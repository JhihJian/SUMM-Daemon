use anyhow::Result;
use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::client::{send_request, socket_path};
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
        DaemonSubcommand::Start { port } => cmd_daemon_start(port).await,
        DaemonSubcommand::Stop => cmd_daemon_stop().await,
        DaemonSubcommand::Status => cmd_daemon_status().await,
    }
}

pub async fn cmd_daemon_start(_port: Option<u16>) -> Result<()> {
    // Find the daemon binary relative to current executable
    let current_exe = std::env::current_exe()?;

    // summ and summ-daemon should be in the same directory
    let daemon_bin = if cfg!(debug_assertions) {
        // Debug build: both in target/debug/
        current_exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find executable directory"))?
            .join("summ-daemon")
    } else {
        // Release build: both in target/release/
        current_exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find executable directory"))?
            .join("summ-daemon")
    };

    if !daemon_bin.exists() {
        anyhow::bail!(
            "Daemon binary not found at {}. Please install summ-daemon.",
            daemon_bin.display()
        );
    }

    // Check if daemon is already running
    if is_daemon_running().await {
        println!("Daemon is already running");
        return Ok(());
    }

    // Start the daemon process
    let status = Command::new(&daemon_bin)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    println!("Daemon started with PID: {}", status.id());

    // Wait a moment and check if it's still running
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if is_daemon_running().await {
        println!("Daemon is running");
    } else {
        anyhow::bail!("Daemon failed to start. Check logs for details.");
    }

    Ok(())
}

pub async fn cmd_daemon_stop() -> Result<()> {
    if !is_daemon_running().await {
        println!("Daemon is not running");
        return Ok(());
    }

    // Try to find the daemon process
    let pid = find_daemon_pid().await?;

    if let Some(pid) = pid {
        // Send SIGTERM to the daemon
        #[cfg(unix)]
        {
            Command::new("kill")
                .arg(pid.to_string())
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to kill daemon: {}", e))?;

            println!("Daemon stopped (PID: {})", pid);

            // Wait a moment and verify
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if !is_daemon_running().await {
                println!("Daemon has stopped");
            } else {
                println!("Warning: Daemon may still be running");
            }
        }

        #[cfg(not(unix))]
        {
            anyhow::bail!("Stopping daemon is not supported on this platform");
        }
    } else {
        anyhow::bail!("Could not find daemon process");
    }

    Ok(())
}

pub async fn cmd_daemon_status() -> Result<()> {
    if is_daemon_running().await {
        let pid = find_daemon_pid().await?;
        if let Some(pid) = pid {
            println!("Daemon is running (PID: {})", pid);
        } else {
            println!("Daemon is running");
        }

        // Try to get detailed status from daemon
        match send_request(Request::DaemonStatus).await {
            Ok(Response::Success { data }) => {
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
            _ => {
                // Daemon is running but status endpoint failed
                println!("(Detailed status unavailable)");
            }
        }
    } else {
        println!("Daemon is not running");
        println!("Start with: summ daemon start");
    }

    Ok(())
}

// Helper functions for daemon management

/// Check if daemon is running by attempting to connect to the socket
async fn is_daemon_running() -> bool {
    let socket = socket_path();
    tokio::net::UnixStream::connect(&socket).await.is_ok()
}

/// Find the daemon process PID using pgrep or ps
async fn find_daemon_pid() -> Result<Option<u32>> {
    #[cfg(unix)]
    {
        // Try pgrep first
        if let Ok(output) = Command::new("pgrep")
            .args(["-f", "summ-daemon"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Ok(pid) = line.trim().parse::<u32>() {
                        // Exclude our own search process
                        if std::process::id() != pid {
                            return Ok(Some(pid));
                        }
                    }
                }
            }
        }

        // Fallback: try ps
        if let Ok(output) = Command::new("ps")
            .args(["aux", "-e"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("summ-daemon") && !line.contains("grep") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(pid) = parts[1].parse::<u32>() {
                            return Ok(Some(pid));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    #[cfg(not(unix))]
    {
        Ok(None)
    }
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
