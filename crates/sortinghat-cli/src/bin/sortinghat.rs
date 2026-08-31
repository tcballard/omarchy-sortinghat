use clap::{Parser, Subcommand};
use sortinghat_cli::{Request, call};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
struct Cli {
    #[arg(
        long,
        env = "SORTINGHAT_SOCKET",
        default_value = "/run/user/1000/sortinghat.sock"
    )]
    socket: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Status,
    Pause,
    Resume,
    Queue,
    Approve { id: Uuid, revision: u64 },
    Ignore { id: Uuid, revision: u64 },
    Undo { id: Uuid, revision: u64 },
}

fn main() {
    let cli = Cli::parse();
    let (action, id, revision) = match cli.command {
        Command::Status => ("status", None, None),
        Command::Pause => ("pause", None, None),
        Command::Resume => ("resume", None, None),
        Command::Queue => ("queue_list", None, None),
        Command::Approve { id, revision } => ("proposal_approve", Some(id), Some(revision)),
        Command::Ignore { id, revision } => ("proposal_ignore", Some(id), Some(revision)),
        Command::Undo { id, revision } => ("undo", Some(id), Some(revision)),
    };
    let request = Request {
        schema_version: 1,
        correlation_id: Uuid::new_v4(),
        action: action.into(),
        proposal_id: id,
        revision,
        argument: None,
    };
    match call(&cli.socket, &request) {
        Ok(response) => println!(
            "{}",
            serde_json::to_string(&response).expect("serializable")
        ),
        Err(error) => {
            let response = sortinghat_cli::Response {
                schema_version: 1,
                correlation_id: request.correlation_id,
                ok: false,
                state: "runtime_unavailable".into(),
                message: error.to_string(),
            };
            println!(
                "{}",
                serde_json::to_string(&response).expect("serializable")
            );
            std::process::exit(1);
        }
    }
}
