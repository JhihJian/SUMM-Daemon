// summ-daemon/src/server.rs
// Unix socket server for daemon IPC
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use summ_common::{DaemonConfig, Session};
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::handler::Handler;
use crate::recovery;
use crate::session::SessionExt;
use crate::tmux::TmuxManager;

/// Daemon server that listens on Unix socket and manages sessions
pub struct Daemon {
    /// Daemon configuration
    config: DaemonConfig,
    /// Map of session_id to Session
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl Daemon {
    /// Create a new Daemon instance
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Run the daemon server - blocks until shutdown
    pub async fn run(&self) -> Result<()> {
        // Check tmux availability
        TmuxManager::check_available()
            .context("tmux is not available. Please install tmux 3.0 or later")?;

        // Ensure directories exist
        self.config.ensure_directories()?;

        // Remove old socket if it exists
        if self.config.socket_path.exists() {
            std::fs::remove_file(&self.config.socket_path)
                .context("Failed to remove old socket file")?;
        }

        // Recover existing sessions
        info!("Recovering existing sessions...");
        let recovered = recovery::recover_sessions(&self.config)?;
        let mut sessions = self.sessions.write().await;
        *sessions = recovered;
        drop(sessions);
        info!(
            "Daemon recovered {} sessions",
            self.sessions.read().await.len()
        );

        // Bind to Unix socket
        let listener = UnixListener::bind(&self.config.socket_path)
            .context("Failed to bind to socket")?;

        // Notify systemd that daemon is ready
        #[cfg(target_os = "linux")]
        {
            if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
                info!("Failed to notify systemd: {}", e);
            } else {
                info!("Notified systemd of ready state");
            }
        }

        info!(
            "SUMM Daemon listening on {}",
            self.config.socket_path.display()
        );

        // Spawn monitoring task
        let sessions_clone = self.sessions.clone();
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            monitor_sessions(sessions_clone, config_clone).await;
        });

        // Accept connections
        let handler = Handler::new(self.sessions.clone(), Arc::new(self.config.clone()));
        let mut join_set = JoinSet::new();

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler_clone = handler.clone();
                    join_set.spawn(async move {
                        if let Err(e) = handler_clone.handle_connection(stream).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                    // Small delay before retry
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            // Clean up completed tasks
            while let Some(result) = join_set.try_join_next() {
                if let Err(e) = result {
                    error!("Error in connection handler task: {}", e);
                }
            }
        }
    }
}

/// Background task that monitors session status
async fn monitor_sessions(
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    _config: DaemonConfig,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let mut sessions = sessions.write().await;
        let mut has_changes = false;

        for (id, session) in sessions.iter_mut() {
            // Get effective status by checking tmux and CLI status
            let new_status = session.get_effective_status();

            if new_status != session.status {
                info!(
                    "Session {} status changed: {:?} -> {:?}",
                    id, session.status, new_status
                );
                session.status = new_status.clone();
                session.pid = if new_status == summ_common::SessionStatus::Stopped {
                    None
                } else {
                    TmuxManager::get_pane_pid(&session.tmux_session).ok().flatten()
                };
                session.save_metadata().ok();
                has_changes = true;
            }

            // Update activity timestamp for non-stopped sessions
            if session.status != summ_common::SessionStatus::Stopped {
                session.last_activity = chrono::Utc::now();
            }
        }

        if has_changes {
            info!("Session monitoring cycle completed with status updates");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_daemon_new() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        let daemon = Daemon::new(config.clone());
        // Daemon is created successfully
        assert_eq!(daemon.config.sessions_dir, config.sessions_dir);
    }
}
