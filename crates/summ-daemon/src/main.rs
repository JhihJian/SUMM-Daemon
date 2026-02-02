mod handler;
mod init;
mod ipc;
mod recovery;
mod server;
mod session;
mod tmux;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into()),
        )
        .init();

    let config = summ_common::DaemonConfig::load()?;
    tracing::info!("SUMM Daemon starting...");
    tracing::info!("Sessions directory: {:?}", config.sessions_dir);
    tracing::info!("Logs directory: {:?}", config.logs_dir);
    tracing::info!("Socket path: {:?}", config.socket_path);

    let daemon = server::Daemon::new(config);
    daemon.run().await?;

    Ok(())
}
