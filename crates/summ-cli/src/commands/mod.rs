use anyhow::Result;
use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::client::send_request;
use summ_common::{Request, Response, SessionStatus};

/// SUMM CLI subcommands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start a new session
    Start(StartArgs),
    /// Stop a running session
    Stop(StopArgs),
    /// List all sessions
    List(ListArgs),
    /// Query detailed session status
    Status(StatusArgs),
    /// Attach to a session terminal (Unix only)
    Attach(AttachArgs),
    /// Inject a message into a running session
    Inject(InjectArgs),
    /// Daemon management commands
    Daemon(DaemonArgs),
}

impl Commands {
    pub async fn execute(self) -> Result<()> {
        match self {
            Commands::Start(args) => cmd_start(args).await,
            Commands::Stop(args) => cmd_stop(args).await,
            Commands::List(args) => cmd_list(args).await,
            Commands::Status(args) => cmd_status(args).await,
            Commands::Attach(args) => cmd_attach(args).await,
            Commands::Inject(args) => cmd_inject(args).await,
            Commands::Daemon(args) => cmd_daemon(args).await,
        }
    }
}

/// Arguments for the `start` command
#[derive(Debug, Args)]
pub struct StartArgs {
    /// CLI command to execute
    #[clap(long)]
    pub cli: String,

    /// Initialization source path (directory, .zip, or .tar.gz)
    #[clap(long)]
    pub init: String,

    /// Optional custom name for the session
    #[clap(long)]
    pub name: Option<String>,
}

/// Arguments for the `stop` command
#[derive(Debug, Args)]
pub struct StopArgs {
    /// Session ID to stop
    #[clap(value_name = "SESSION_ID")]
    pub session_id: String,
}

/// Arguments for the `list` command
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Optional status filter (running/idle/stopped)
    #[clap(long, value_name = "STATUS")]
    pub status: Option<String>,
}

/// Arguments for the `status` command
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Session ID to query
    #[clap(value_name = "SESSION_ID")]
    pub session_id: String,
}

/// Arguments for the `attach` command
#[derive(Debug, Args)]
pub struct AttachArgs {
    /// Session ID to attach to
    #[clap(value_name = "SESSION_ID")]
    pub session_id: String,
}

/// Arguments for the `inject` command
#[derive(Debug, Args)]
pub struct InjectArgs {
    /// Session ID to inject message into
    #[clap(value_name = "SESSION_ID")]
    pub session_id: String,

    /// Message to inject
    #[clap(long)]
    pub message: Option<String>,

    /// Read message from file
    #[clap(long, value_name = "FILE")]
    pub file: Option<String>,
}

/// Arguments for the `daemon` command
#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[clap(subcommand)]
    pub subcommand: DaemonSubcommand,
}

/// Daemon management subcommands
#[derive(Debug, Subcommand)]
pub enum DaemonSubcommand {
    /// Start the daemon
    Start {
        /// Optional port for the daemon (currently unused, reserved for future)
        #[clap(long)]
        port: Option<u16>,
    },
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
}

// Command implementations

pub async fn cmd_start(args: StartArgs) -> Result<()> {
    // Expand path with shell expansion (e.g., ~, $HOME)
    let init_path = shellexpand::full(&args.init)
        .map_err(|e| anyhow::anyhow!("Failed to expand init path: {}", e))?;
    let init_path = PathBuf::from(init_path.as_ref());

    let req = Request::Start {
        cli: args.cli,
        init: init_path,
        name: args.name,
    };

    let resp = send_request(req).await?;

    match resp {
        Response::Success { data } => {
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        Response::Error { code, message } => {
            anyhow::bail!("{}: {}", code, message);
        }
    }
}

pub async fn cmd_stop(args: StopArgs) -> Result<()> {
    let req = Request::Stop {
        session_id: args.session_id,
    };

    let resp = send_request(req).await?;

    match resp {
        Response::Success { data } => {
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        Response::Error { code, message } => {
            anyhow::bail!("{}: {}", code, message);
        }
    }
}

pub async fn cmd_list(args: ListArgs) -> Result<()> {
    let status_filter = parse_status_filter(args.status)?;

    let req = Request::List { status_filter };

    let resp = send_request(req).await?;

    match resp {
        Response::Success { data } => {
            // Try to parse as array of sessions
            if let Some(sessions) = data.as_array() {
                print_colored_list(sessions);
            } else {
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
            Ok(())
        }
        Response::Error { code, message } => {
            anyhow::bail!("{}: {}", code, message);
        }
    }
}

pub async fn cmd_status(args: StatusArgs) -> Result<()> {
    let req = Request::Status {
        session_id: args.session_id,
    };

    let resp = send_request(req).await?;

    match resp {
        Response::Success { data } => {
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        Response::Error { code, message } => {
            anyhow::bail!("{}: {}", code, message);
        }
    }
}

pub async fn cmd_attach(args: AttachArgs) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let tmux_session = format!("summ-{}", args.session_id);

        // First verify session exists via daemon
        let req = Request::Status {
            session_id: args.session_id.clone(),
        };
        let resp = send_request(req).await?;

        match resp {
            Response::Success { .. } => {
                // Session exists, use exec to replace current process with tmux attach
                let err = Command::new("tmux")
                    .args(["attach-session", "-t", &tmux_session])
                    .exec();

                // exec only returns on failure
                Err(anyhow::anyhow!("Failed to attach to tmux session: {}", err))
            }
            Response::Error { code, message } => {
                anyhow::bail!("{}: {}", code, message);
            }
        }
    }

    #[cfg(not(unix))]
    {
        Err(anyhow::anyhow!(
            "attach command is only supported on Unix systems with tmux"
        ))
    }
}

pub async fn cmd_inject(args: InjectArgs) -> Result<()> {
    // Get message from --message or --file
    let message = if let Some(msg) = args.message {
        msg
    } else if let Some(file_path) = args.file {
        // Expand path
        let expanded = shellexpand::full(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to expand file path: {}", e))?;
        let path = PathBuf::from(expanded.as_ref());

        // Read file content
        fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?
    } else {
        anyhow::bail!("Either --message or --file must be provided");
    };

    let req = Request::Inject {
        session_id: args.session_id,
        message,
    };

    let resp = send_request(req).await?;

    match resp {
        Response::Success { data } => {
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        Response::Error { code, message } => {
            anyhow::bail!("{}: {}", code, message);
        }
    }
}

pub async fn cmd_daemon(args: DaemonArgs) -> Result<()> {
    match args.subcommand {
        DaemonSubcommand::Start { .. } => cmd_daemon_start().await,
        DaemonSubcommand::Stop => cmd_daemon_stop().await,
        DaemonSubcommand::Status => cmd_daemon_status().await,
    }
}

pub async fn cmd_daemon_start() -> Result<()> {
    println!("Daemon start command will be implemented in Task 4.5");
    Ok(())
}

pub async fn cmd_daemon_stop() -> Result<()> {
    println!("Daemon stop command will be implemented in Task 4.5");
    Ok(())
}

pub async fn cmd_daemon_status() -> Result<()> {
    println!("Daemon status command will be implemented in Task 4.5");
    Ok(())
}

// Helper function to parse status filter from string

pub fn parse_status_filter(s: Option<String>) -> Result<Option<SessionStatus>> {
    match s.as_deref() {
        None => Ok(None),
        Some("running") => Ok(Some(SessionStatus::Running)),
        Some("idle") => Ok(Some(SessionStatus::Idle)),
        Some("stopped") => Ok(Some(SessionStatus::Stopped)),
        Some(other) => anyhow::bail!("Invalid status filter: {}. Use: running, idle, or stopped", other),
    }
}

// Helper function to print colored list output

fn print_colored_list(sessions: &[serde_json::Value]) {
    use ansi_term::Colour;

    if sessions.is_empty() {
        println!("{}", Colour::Purple.dimmed().paint("No sessions found."));
        return;
    }

    for session in sessions {
        let session_id = session["session_id"].as_str().unwrap_or("unknown");
        let name = session["name"].as_str().unwrap_or("");
        let cli = session["cli"].as_str().unwrap_or("unknown");
        let status = session["status"].as_str().unwrap_or("unknown");

        let status_colored = match status {
            "running" => Colour::Green.paint(status),
            "idle" => Colour::Yellow.paint(status),
            "stopped" => Colour::Red.paint(status),
            _ => Colour::White.paint(status),
        };

        println!(
            "{} {} {} {}",
            Colour::Cyan.bold().paint(session_id),
            Colour::White.dimmed().paint(format!("({})", cli)),
            status_colored,
            if name.is_empty() {
                String::new()
            } else {
                format!("- {}", Colour::White.paint(name))
            }
        );
    }
}
