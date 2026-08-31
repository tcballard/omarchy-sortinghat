use clap::{Args, Parser, Subcommand};
use sortinghat_cli::{Request, Response, call};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(version, about = "Review-first file organisation for Omarchy")]
struct Cli {
    #[arg(
        long,
        env = "SORTINGHAT_SOCKET",
        default_value = "/run/user/1000/sortinghat.sock"
    )]
    socket: PathBuf,
    #[arg(long, default_value_t = false)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Status,
    Pause,
    Resume,
    Scan,
    Roots {
        #[command(subcommand)]
        command: RootsCommand,
    },
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    Proposal {
        #[command(subcommand)]
        command: ProposalCommand,
    },
    Rule {
        #[command(subcommand)]
        command: RuleCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Undo(RevisionArgs),
}

#[derive(Subcommand)]
enum RootsCommand {
    List,
    Add {
        path: PathBuf,
        #[arg(long, default_value_t = false)]
        watch: bool,
        #[arg(long, default_value_t = false)]
        destination: bool,
    },
    Remove {
        id: Uuid,
    },
}

#[derive(Subcommand)]
enum QueueCommand {
    List,
}

#[derive(Args)]
struct RevisionArgs {
    id: Uuid,
    #[arg(long)]
    revision: u64,
}

#[derive(Subcommand)]
enum ProposalCommand {
    Show {
        id: Uuid,
    },
    Approve(RevisionArgs),
    Ignore(RevisionArgs),
    Destination {
        id: Uuid,
        folder: String,
        #[arg(long)]
        root: Uuid,
        #[arg(long)]
        revision: u64,
    },
    ChooseFolder {
        id: Uuid,
        folder: PathBuf,
        #[arg(long)]
        revision: u64,
    },
    Rename {
        id: Uuid,
        name: String,
        #[arg(long)]
        revision: u64,
    },
}

#[derive(Subcommand)]
enum RuleCommand {
    List,
    Create {
        #[arg(long)]
        proposal: Uuid,
        #[arg(long, default_value_t = 100)]
        priority: i32,
    },
}

#[derive(Subcommand)]
enum AgentCommand {
    Status,
    Enable {
        executable: String,
        #[arg(long = "arg", action = clap::ArgAction::Append)]
        fixed_args: Vec<String>,
    },
    Disable,
}

fn main() {
    let cli = Cli::parse();
    let (action, proposal_id, revision, argument) = command_request(cli.command);
    let request = Request {
        schema_version: 1,
        correlation_id: Uuid::new_v4(),
        action,
        proposal_id,
        revision,
        argument,
    };
    let response = call(&cli.socket, &request).unwrap_or_else(|error| Response {
        schema_version: 1,
        correlation_id: request.correlation_id,
        ok: false,
        state: "runtime_unavailable".into(),
        message: error.to_string(),
        data: serde_json::Value::Null,
    });
    println!(
        "{}",
        serde_json::to_string(&response).expect("serializable")
    );
    if !response.ok {
        std::process::exit(1);
    }
}

fn command_request(command: Command) -> (String, Option<Uuid>, Option<u64>, Option<String>) {
    match command {
        Command::Status => plain("status"),
        Command::Pause => plain("pause"),
        Command::Resume => plain("resume"),
        Command::Scan => plain("scan"),
        Command::Roots { command } => match command {
            RootsCommand::List => plain("roots_list"),
            RootsCommand::Add {
                path,
                watch,
                destination,
            } => (
                "roots_add".into(),
                None,
                None,
                Some(
                    serde_json::json!({"path": path, "watched": watch, "destination": destination})
                        .to_string(),
                ),
            ),
            RootsCommand::Remove { id } => ("roots_remove".into(), Some(id), None, None),
        },
        Command::Queue {
            command: QueueCommand::List,
        } => plain("queue_list"),
        Command::Proposal { command } => match command {
            ProposalCommand::Show { id } => ("proposal_show".into(), Some(id), None, None),
            ProposalCommand::Approve(args) => revision_request("proposal_approve", args),
            ProposalCommand::Ignore(args) => revision_request("proposal_ignore", args),
            ProposalCommand::Destination {
                id,
                folder,
                root,
                revision,
            } => (
                "proposal_destination".into(),
                Some(id),
                Some(revision),
                Some(serde_json::json!({"root_id": root, "directory": folder}).to_string()),
            ),
            ProposalCommand::ChooseFolder {
                id,
                folder,
                revision,
            } => (
                "proposal_choose_folder".into(),
                Some(id),
                Some(revision),
                folder.to_str().map(str::to_owned),
            ),
            ProposalCommand::Rename { id, name, revision } => (
                "proposal_rename".into(),
                Some(id),
                Some(revision),
                Some(name),
            ),
        },
        Command::Rule { command } => match command {
            RuleCommand::List => plain("rules_list"),
            RuleCommand::Create { proposal, priority } => (
                "rule_create".into(),
                Some(proposal),
                None,
                Some(priority.to_string()),
            ),
        },
        Command::Agent { command } => match command {
            AgentCommand::Status => plain("agent_status"),
            AgentCommand::Enable {
                executable,
                fixed_args,
            } => (
                "agent_enable".into(),
                None,
                None,
                Some(
                    serde_json::json!({"executable": executable, "fixed_args": fixed_args})
                        .to_string(),
                ),
            ),
            AgentCommand::Disable => plain("agent_disable"),
        },
        Command::Undo(args) => revision_request("undo", args),
    }
}

fn plain(action: &str) -> (String, Option<Uuid>, Option<u64>, Option<String>) {
    (action.into(), None, None, None)
}

fn revision_request(
    action: &str,
    args: RevisionArgs,
) -> (String, Option<Uuid>, Option<u64>, Option<String>) {
    (action.into(), Some(args.id), Some(args.revision), None)
}
