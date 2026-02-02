mod session;
mod tmux;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    if let Err(e) = tmux::TmuxManager::check_available() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let config = summ_common::DaemonConfig::load()?;
    tracing::info!("SUMM Daemon starting...");
    tracing::info!("Sessions directory: {:?}", config.sessions_dir);
    tracing::info!("Logs directory: {:?}", config.logs_dir);
    tracing::info!("Socket path: {:?}", config.socket_path);

    Ok(())
}
