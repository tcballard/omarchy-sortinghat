use clap::Parser;
use sortinghat_cli::{MAX_IPC_BYTES, Request, Response};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    socket: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.socket.exists() {
        fs::remove_file(&args.socket)?;
    }
    let listener = UnixListener::bind(&args.socket)?;
    fs::set_permissions(&args.socket, fs::Permissions::from_mode(0o600))?;
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let _ = handle(stream);
        }
    }
    Ok(())
}

fn handle(mut stream: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut line = Vec::new();
    BufReader::new(stream.try_clone()?)
        .take((MAX_IPC_BYTES + 1) as u64)
        .read_until(b'\n', &mut line)?;
    if line.len() > MAX_IPC_BYTES {
        return Ok(());
    }
    let request: Request = serde_json::from_slice(&line)?;
    let (ok, state, message) = match request.action.as_str() {
        "status" => (true, "running", "watcher running"),
        "pause" => (true, "paused", "watcher paused"),
        "resume" => (true, "running", "watcher resumed"),
        "queue_list" => (true, "running", "queue is empty"),
        _ => (false, "unsupported", "action not yet available"),
    };
    let response = Response {
        schema_version: 1,
        correlation_id: request.correlation_id,
        ok,
        state: state.into(),
        message: message.into(),
    };
    serde_json::to_writer(&mut stream, &response)?;
    stream.write_all(b"\n")?;
    Ok(())
}
