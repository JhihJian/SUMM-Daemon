use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Session status represents the current state of a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// CLI is executing a task
    Running,
    /// CLI is idle, waiting for new tasks (reported via Hook)
    Idle,
    /// tmux session has exited
    Stopped,
}

/// Session metadata stored in meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub session_id: String,
    /// tmux session name (summ-{session_id})
    pub tmux_session: String,
    /// User-readable name
    pub name: String,
    /// CLI command
    pub cli: String,
    /// Working directory
    pub workdir: PathBuf,
    /// Initialization source path
    pub init_source: PathBuf,
    /// Current session status
    pub status: SessionStatus,
    /// CLI process PID (informational only)
    pub pid: Option<u32>,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

/// CLI state reported by hooks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CliState {
    /// CLI is idle, waiting for input
    Idle,
    /// CLI is processing a task
    Busy,
    /// CLI has stopped
    Stopped,
}

/// CLI status reported via hook mechanism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStatus {
    /// Current CLI state
    pub state: CliState,
    /// Optional status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional event type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Status update timestamp
    pub timestamp: DateTime<Utc>,
}

/// Daemon configuration loaded from config.json or using defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Directory for session data (default: ~/.summ-daemon/sessions)
    pub sessions_dir: PathBuf,
    /// Directory for logs (default: ~/.summ-daemon/logs)
    pub logs_dir: PathBuf,
    /// Path to Unix socket (default: ~/.summ-daemon/daemon.sock)
    pub socket_path: PathBuf,
    /// Hours to retain stopped sessions before cleanup (default: 24)
    pub cleanup_retention_hours: u64,
    /// Prefix for tmux session names (default: "summ-")
    pub tmux_prefix: String,
}

impl DaemonConfig {
    /// Load the daemon configuration and ensure all required directories exist
    pub fn load() -> anyhow::Result<Self> {
        let config = Self::default();
        config.ensure_directories()?;
        Ok(config)
    }

    /// Create all required directories for the daemon
    pub fn ensure_directories(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.sessions_dir)
            .context("Failed to create sessions directory")?;
        std::fs::create_dir_all(&self.logs_dir)
            .context("Failed to create logs directory")?;
        Ok(())
    }

    /// Get the path to a session's meta.json file
    pub fn session_meta_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id).join("meta.json")
    }

    /// Get the path to a session's runtime status.json file
    pub fn session_status_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id).join("runtime").join("status.json")
    }

    /// Get the path to a session's workspace directory
    pub fn session_workspace_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id).join("workspace")
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let home = dirs::home_dir().expect("HOME directory not found");
        let base = home.join(".summ-daemon");
        Self {
            sessions_dir: base.join("sessions"),
            logs_dir: base.join("logs"),
            socket_path: base.join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        }
    }
}

/// Session information returned by list commands (subset of Session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,
    /// User-readable name
    pub name: String,
    /// CLI command
    pub cli: String,
    /// Current session status
    pub status: SessionStatus,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl From<Session> for SessionInfo {
    fn from(session: Session) -> Self {
        Self {
            session_id: session.session_id,
            name: session.name,
            cli: session.cli,
            status: session.status,
            created_at: session.created_at,
            last_activity: session.last_activity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_status_serialization() {
        let running = SessionStatus::Running;
        assert_eq!(serde_json::to_string(&running).unwrap(), r#""running""#);

        let idle = SessionStatus::Idle;
        assert_eq!(serde_json::to_string(&idle).unwrap(), r#""idle""#);

        let stopped = SessionStatus::Stopped;
        assert_eq!(serde_json::to_string(&stopped).unwrap(), r#""stopped""#);
    }

    #[test]
    fn test_session_status_deserialization() {
        let running: SessionStatus = serde_json::from_str(r#""running""#).unwrap();
        assert_eq!(running, SessionStatus::Running);

        let idle: SessionStatus = serde_json::from_str(r#""idle""#).unwrap();
        assert_eq!(idle, SessionStatus::Idle);

        let stopped: SessionStatus = serde_json::from_str(r#""stopped""#).unwrap();
        assert_eq!(stopped, SessionStatus::Stopped);
    }

    #[test]
    fn test_cli_state_serialization() {
        let idle = CliState::Idle;
        assert_eq!(serde_json::to_string(&idle).unwrap(), r#""idle""#);

        let busy = CliState::Busy;
        assert_eq!(serde_json::to_string(&busy).unwrap(), r#""busy""#);

        let stopped = CliState::Stopped;
        assert_eq!(serde_json::to_string(&stopped).unwrap(), r#""stopped""#);
    }

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(config.sessions_dir.ends_with(".summ-daemon/sessions"));
        assert!(config.logs_dir.ends_with(".summ-daemon/logs"));
        assert!(config.socket_path.ends_with(".summ-daemon/daemon.sock"));
        assert_eq!(config.cleanup_retention_hours, 24);
        assert_eq!(config.tmux_prefix, "summ-");
    }

    #[test]
    fn test_session_from_conversion() {
        let session = Session {
            session_id: "test-session".to_string(),
            tmux_session: "summ-test-session".to_string(),
            name: "Test Session".to_string(),
            cli: "claude".to_string(),
            workdir: PathBuf::from("/tmp/test"),
            init_source: PathBuf::from("/tmp/init"),
            status: SessionStatus::Running,
            pid: Some(12345),
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };

        let info: SessionInfo = session.clone().into();
        assert_eq!(info.session_id, session.session_id);
        assert_eq!(info.name, session.name);
        assert_eq!(info.cli, session.cli);
        assert_eq!(info.status, session.status);
        assert_eq!(info.created_at, session.created_at);
        assert_eq!(info.last_activity, session.last_activity);
    }

    #[test]
    fn test_cli_status_serialization_with_optional_fields() {
        let status = CliStatus {
            state: CliState::Idle,
            message: Some("Ready for tasks".to_string()),
            event: Some("SessionStart".to_string()),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""state":"idle""#));
        assert!(json.contains(r#""message":"Ready for tasks""#));
        assert!(json.contains(r#""event":"SessionStart""#));
    }

    #[test]
    fn test_cli_status_serialization_without_optional_fields() {
        let status = CliStatus {
            state: CliState::Busy,
            message: None,
            event: None,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""state":"busy""#));
        assert!(!json.contains(r#""message""#));
        assert!(!json.contains(r#""event""#));
    }

    #[test]
    fn test_config_load_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };
        assert!(config.ensure_directories().is_ok());
        assert!(config.sessions_dir.exists());
        assert!(config.logs_dir.exists());
    }

    #[test]
    fn test_session_meta_path() {
        let config = DaemonConfig::default();
        let path = config.session_meta_path("test001");
        assert!(path.ends_with("sessions/test001/meta.json"));
    }

    #[test]
    fn test_session_status_path() {
        let config = DaemonConfig::default();
        let path = config.session_status_path("test001");
        assert!(path.ends_with("sessions/test001/runtime/status.json"));
    }

    #[test]
    fn test_session_workspace_path() {
        let config = DaemonConfig::default();
        let path = config.session_workspace_path("test001");
        assert!(path.ends_with("sessions/test001/workspace"));
    }
}
