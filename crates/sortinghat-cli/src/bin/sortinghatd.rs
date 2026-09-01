use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sortinghat_cli::{MAX_IPC_BYTES, Request, Response, validate_request};
use sortinghat_service::{Service, ServiceError};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    socket: PathBuf,
    #[arg(long)]
    state_dir: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootAdd {
    path: PathBuf,
    watched: bool,
    destination: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DestinationArg {
    root_id: Uuid,
    directory: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentArg {
    executable: String,
    fixed_args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(parent) = args.socket.parent() {
        fs::create_dir_all(parent)?;
    }
    if args.socket.exists() {
        fs::remove_file(&args.socket)?;
    }
    let listener = UnixListener::bind(&args.socket)?;
    fs::set_permissions(&args.socket, fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let mut service = Service::open(&args.state_dir)?;
    let mut last_scan = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                if peer_is_owner(&stream) {
                    let _ = handle(stream, &mut service);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        if last_scan.elapsed() >= Duration::from_secs(2) {
            let _ = service.scan_once();
            last_scan = Instant::now();
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn peer_is_owner(stream: &UnixStream) -> bool {
    rustix::net::sockopt::socket_peercred(stream)
        .map(|credentials| credentials.uid == rustix::process::getuid())
        .unwrap_or(false)
}

fn handle(mut stream: UnixStream, service: &mut Service) -> Result<(), Box<dyn std::error::Error>> {
    let mut line = Vec::new();
    BufReader::new(stream.try_clone()?)
        .take((MAX_IPC_BYTES + 1) as u64)
        .read_until(b'\n', &mut line)?;
    if line.len() > MAX_IPC_BYTES {
        return Ok(());
    }
    let request: Request = serde_json::from_slice(&line)?;
    let response = if validate_request(&request).is_err() {
        Response {
            schema_version: 1,
            correlation_id: request.correlation_id,
            ok: false,
            state: "rejected".into(),
            message: "invalid request".into(),
            data: Value::Null,
        }
    } else {
        dispatch(service, &request).unwrap_or_else(|error| Response {
            schema_version: 1,
            correlation_id: request.correlation_id,
            ok: false,
            state: error_state(&error).into(),
            message: error.to_string(),
            data: Value::Null,
        })
    };
    serde_json::to_writer(&mut stream, &response)?;
    stream.write_all(b"\n")?;
    Ok(())
}

fn dispatch(service: &mut Service, request: &Request) -> Result<Response, ServiceError> {
    let data = match request.action.as_str() {
        "status" => {
            json!({"paused": service.paused(), "awaiting_review": service.proposals().iter().filter(|p| p.status == "proposed").count()})
        }
        "pause" => {
            service.set_paused(true)?;
            json!({"paused": true})
        }
        "resume" => {
            service.set_paused(false)?;
            json!({"paused": false})
        }
        "scan" => json!({"created": service.scan_once()?}),
        "roots_list" => json!(service.roots()),
        "roots_add" => {
            let arg: RootAdd = parse_argument(request)?;
            json!(service.add_root(&arg.path, arg.watched, arg.destination)?)
        }
        "roots_remove" => {
            service.remove_root(id(request)?)?;
            Value::Null
        }
        "queue_list" => json!(
            service
                .proposals()
                .iter()
                .filter(|proposal| proposal.status != "ignored" && proposal.status != "undone")
                .rev()
                .take(100)
                .collect::<Vec<_>>()
        ),
        "proposal_show" => json!(service.proposal(id(request)?)?),
        "proposal_destination" => {
            let arg: DestinationArg = parse_argument(request)?;
            json!(service.choose_destination(
                id(request)?,
                revision(request)?,
                arg.root_id,
                &arg.directory
            )?)
        }
        "proposal_choose_folder" => json!(
            service.choose_destination_path(
                id(request)?,
                revision(request)?,
                PathBuf::from(
                    request
                        .argument
                        .as_deref()
                        .ok_or(ServiceError::MalformedState)?
                )
                .as_path(),
            )?
        ),
        "proposal_rename" => json!(
            service.rename(
                id(request)?,
                revision(request)?,
                request
                    .argument
                    .as_deref()
                    .ok_or(ServiceError::MalformedState)?
            )?
        ),
        "proposal_approve" => json!(service.approve(id(request)?, revision(request)?)?),
        "proposal_ignore" => json!(service.ignore(id(request)?, revision(request)?)?),
        "rules_list" => json!(service.rules()),
        "rule_create" => {
            let priority = request
                .argument
                .as_deref()
                .ok_or(ServiceError::MalformedState)?
                .parse()
                .map_err(|_| ServiceError::MalformedState)?;
            json!(service.create_rule_from_proposal(id(request)?, priority)?)
        }
        "agent_status" => json!(service.agent_settings()),
        "agent_enable" => {
            let arg: AgentArg = parse_argument(request)?;
            json!(service.configure_metadata_agent(arg.executable, arg.fixed_args)?)
        }
        "agent_disable" => json!(service.disable_agent()?),
        "undo" => json!(service.undo(id(request)?, revision(request)?)?),
        _ => return Err(ServiceError::NotFound),
    };
    Ok(Response {
        schema_version: 1,
        correlation_id: request.correlation_id,
        ok: true,
        state: if service.paused() {
            "paused"
        } else {
            "running"
        }
        .into(),
        message: "ok".into(),
        data,
    })
}

fn parse_argument<T: for<'de> Deserialize<'de>>(request: &Request) -> Result<T, ServiceError> {
    serde_json::from_str(
        request
            .argument
            .as_deref()
            .ok_or(ServiceError::MalformedState)?,
    )
    .map_err(|_| ServiceError::MalformedState)
}

fn id(request: &Request) -> Result<Uuid, ServiceError> {
    request.proposal_id.ok_or(ServiceError::MalformedState)
}

fn revision(request: &Request) -> Result<u64, ServiceError> {
    request.revision.ok_or(ServiceError::MalformedState)
}

fn error_state(error: &ServiceError) -> &'static str {
    match error {
        ServiceError::Collision => "conflict",
        ServiceError::RevisionConflict => "stale_revision",
        ServiceError::Limit => "paused_limit",
        ServiceError::Io(_) | ServiceError::Fs(_) => "filesystem_error",
        _ => "rejected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sortinghat_service::ServiceError;

    #[test]
    fn permission_denial_is_reported_as_a_filesystem_error() {
        let error = ServiceError::Fs(sortinghat_fs::FsError::Io(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        )));
        assert_eq!(error_state(&error), "filesystem_error");
    }
}
