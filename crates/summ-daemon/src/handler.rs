// summ-daemon/src/handler.rs
// Request handler for daemon operations
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use summ_common::{DaemonConfig, Request, Response, Session, SessionStatus, SessionInfo};
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::ipc::{read_request, write_response};
use crate::session::SessionExt;
use crate::tmux::TmuxManager;

/// Handler manages session state and processes requests
pub struct Handler {
    /// Map of session_id to Session
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Daemon configuration
    config: Arc<DaemonConfig>,
}

impl Handler {
    /// Create a new Handler with the given sessions and config
    pub fn new(
        sessions: Arc<RwLock<HashMap<String, Session>>>,
        config: Arc<DaemonConfig>,
    ) -> Self {
        Self { sessions, config }
    }

    /// Handle a single connection (read request, process, write response)
    pub async fn handle_connection(&self, mut stream: UnixStream) -> Result<()> {
        let request = match read_request(&mut stream).await {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to read request: {}", e);
                let response = Response::error(&summ_common::DaemonError::e007(e.to_string()));
                let _ = write_response(&mut stream, &response).await;
                return Ok(());
            }
        };

        let response = self.handle(request).await;

        if let Err(ref e) = response {
            error!("Error handling request: {}", e);
        }

        write_response(&mut stream, &response?).await?;
        Ok(())
    }

    /// Process a request and return a response
    pub async fn handle(&self, request: Request) -> Result<Response> {
        match request {
            Request::Start { cli, init, name } => self.handle_start(cli, init, name).await,
            Request::Stop { session_id } => self.handle_stop(session_id).await,
            Request::List { status_filter } => self.handle_list(status_filter).await,
            Request::Status { session_id } => self.handle_status(session_id).await,
            Request::Inject { session_id, message } => self.handle_inject(session_id, message).await,
            Request::DaemonStatus => self.handle_daemon_status().await,
        }
    }

    /// Handle Start request - create a new session
    async fn handle_start(
        &self,
        cli: String,
        init: std::path::PathBuf,
        name: Option<String>,
    ) -> Result<Response> {
        info!("Start request: cli={}, init={:?}", cli, init);

        // Validate init path exists
        if !init.exists() {
            return Ok(Response::error(&summ_common::DaemonError::e001(
                format!("Initialization source not found: {}", init.display()),
            )));
        }

        // Create the session
        let session = match Session::create(&cli, &init, name, &self.config).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create session: {}", e);
                return Ok(Response::error(&summ_common::DaemonError::e005(e.to_string())));
            }
        };

        // Add to sessions map
        let session_id = session.session_id.clone();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());

        Ok(Response::success(serde_json::to_value(session)?))
    }

    /// Handle Stop request - stop a running session
    async fn handle_stop(&self, session_id: String) -> Result<Response> {
        info!("Stop request: session_id={}", session_id);

        let mut sessions = self.sessions.write().await;

        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => {
                return Ok(Response::error(&summ_common::DaemonError::e002(
                    format!("Session not found: {}", session_id),
                )));
            }
        };

        // Kill tmux session
        if let Err(e) = TmuxManager::kill_session(&session.tmux_session) {
            warn!("Failed to kill tmux session: {}", e);
        }

        // Update status
        let session = sessions.get_mut(&session_id).unwrap();
        session.status = SessionStatus::Stopped;
        session.pid = None;
        session.save_metadata()?;

        Ok(Response::success(serde_json::json!({
            "session_id": session_id,
            "status": "stopped"
        })))
    }

    /// Handle List request - list all sessions, optionally filtered by status
    async fn handle_list(&self, status_filter: Option<SessionStatus>) -> Result<Response> {
        info!("List request: status_filter={:?}", status_filter);

        let sessions = self.sessions.read().await;

        let session_infos: Vec<SessionInfo> = sessions
            .values()
            .filter(|s| {
                if let Some(ref filter) = status_filter {
                    // Update effective status before filtering
                    let effective = s.get_effective_status();
                    &effective == filter
                } else {
                    true
                }
            })
            .cloned()
            .map(SessionInfo::from)
            .collect();

        Ok(Response::success(serde_json::to_value(session_infos)?))
    }

    /// Handle Status request - get detailed session status
    async fn handle_status(&self, session_id: String) -> Result<Response> {
        info!("Status request: session_id={}", session_id);

        let sessions = self.sessions.read().await;

        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => {
                return Ok(Response::error(&summ_common::DaemonError::e002(
                    format!("Session not found: {}", session_id),
                )));
            }
        };

        // Get effective status
        let effective_status = session.get_effective_status();

        Ok(Response::success(serde_json::json!({
            "session_id": session.session_id,
            "name": session.name,
            "cli": session.cli,
            "status": effective_status,
            "pid": session.pid,
            "created_at": session.created_at,
            "last_activity": session.last_activity,
            "workdir": session.workdir,
        })))
    }

    /// Handle Inject request - inject a message into a running session
    async fn handle_inject(&self, session_id: String, message: String) -> Result<Response> {
        info!("Inject request: session_id={}, message_len={}", session_id, message.len());

        let sessions = self.sessions.read().await;

        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => {
                return Ok(Response::error(&summ_common::DaemonError::e002(
                    format!("Session not found: {}", session_id),
                )));
            }
        };

        // Check if session is running
        let effective_status = session.get_effective_status();
        if effective_status == SessionStatus::Stopped {
            return Ok(Response::error(&summ_common::DaemonError::e003(
                format!("Session {} is stopped, cannot inject message", session_id),
            )));
        }

        // Send keys to tmux session
        match TmuxManager::send_keys(&session.tmux_session, &message, true) {
            Ok(()) => {
                info!("Message injected into session {}", session_id);
                Ok(Response::success(serde_json::json!({
                    "session_id": session_id,
                    "message": "injected"
                })))
            }
            Err(e) => {
                error!("Failed to inject message: {}", e);
                Ok(Response::error(&summ_common::DaemonError::e006(e.to_string())))
            }
        }
    }

    /// Handle DaemonStatus request - get daemon status
    async fn handle_daemon_status(&self) -> Result<Response> {
        info!("DaemonStatus request");

        let sessions = self.sessions.read().await;
        let session_count = sessions.len();

        Ok(Response::success(serde_json::json!({
            "running": true,
            "session_count": session_count,
            "version": env!("CARGO_PKG_VERSION")
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_handler_list_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = Arc::new(config);
        let handler = Handler::new(sessions.clone(), config);

        let request = Request::List { status_filter: None };
        let response = handler.handle(request).await.unwrap();

        match response {
            Response::Success { data } => {
                let arr = data.as_array().unwrap();
                assert_eq!(arr.len(), 0);
            }
            _ => panic!("Expected Success response"),
        }
    }

    #[tokio::test]
    async fn test_handler_daemon_status() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = Arc::new(config);
        let handler = Handler::new(sessions, config);

        let request = Request::DaemonStatus;
        let response = handler.handle(request).await.unwrap();

        match response {
            Response::Success { data } => {
                assert_eq!(data["running"], true);
                assert_eq!(data["session_count"], 0);
            }
            _ => panic!("Expected Success response"),
        }
    }

    #[tokio::test]
    async fn test_handler_status_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = Arc::new(config);
        let handler = Handler::new(sessions, config);

        let request = Request::Status {
            session_id: "nonexistent".to_string(),
        };
        let response = handler.handle(request).await.unwrap();

        match response {
            Response::Error { code, .. } => {
                assert_eq!(code, "E002");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handler_list_with_status_filter() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = Arc::new(config);
        let handler = Handler::new(sessions.clone(), config);

        // Add a stopped session
        let session = Session {
            session_id: "test001".to_string(),
            tmux_session: "summ-test001".to_string(),
            name: "Test".to_string(),
            cli: "echo".to_string(),
            workdir: PathBuf::from("/tmp/test001"),
            init_source: PathBuf::from("/tmp"),
            status: SessionStatus::Stopped,
            pid: None,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };
        sessions.write().await.insert("test001".to_string(), session);

        // List with Running filter (should be empty since our session is stopped)
        let request = Request::List {
            status_filter: Some(SessionStatus::Running),
        };
        let response = handler.handle(request).await.unwrap();

        match response {
            Response::Success { data } => {
                let arr = data.as_array().unwrap();
                assert_eq!(arr.len(), 0);
            }
            _ => panic!("Expected Success response"),
        }
    }
}
