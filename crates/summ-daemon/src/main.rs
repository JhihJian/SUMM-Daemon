mod tmux;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    if let Err(e) = tmux::TmuxManager::check_available() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    tracing::info!("SUMM Daemon starting...");
    tracing::info!("tmux check passed");

    Ok(())
}
