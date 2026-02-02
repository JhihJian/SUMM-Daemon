mod handler;
mod hooks;
mod init;
mod ipc;
mod recovery;
mod server;
mod session;
mod tmux;

use anyhow::Result;

/// Initialize logging with proper defaults and environment variable support
fn init_logging() {
    // Set default log level based on RUST_LOG env var, defaulting to info
    // Users can set RUST_LOG=debug, RUST_LOG=summ_daemon=trace, etc.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("info")
                .add_directive("summ_daemon=info".parse().unwrap())
                .add_directive("summ_common=info".parse().unwrap())
        });

    // Configure the subscriber with pretty formatting for development
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();

    let config = summ_common::DaemonConfig::load()?;
    tracing::info!("SUMM Daemon starting...");
    tracing::debug!("Sessions directory: {:?}", config.sessions_dir);
    tracing::debug!("Logs directory: {:?}", config.logs_dir);
    tracing::debug!("Socket path: {:?}", config.socket_path);

    let daemon = server::Daemon::new(config);
    daemon.run().await?;

    Ok(())
}
