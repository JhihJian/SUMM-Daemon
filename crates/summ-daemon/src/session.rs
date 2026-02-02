// summ-daemon/src/session.rs
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use serde_json;
use std::fs;
use std::path::Path;
use summ_common::{CliStatus, CliState, DaemonConfig, Session, SessionStatus};
use uuid::Uuid;

/// Session extension trait providing additional methods for Session management
pub trait SessionExt {
    /// Generate a unique session ID
    fn generate_id() -> String;

    /// Create a new session with tmux and workspace initialization
    async fn create(
        cli: &str,
        init_path: &Path,
        name: Option<String>,
        config: &DaemonConfig,
    ) -> Result<Session>;

    /// Get the effective status by checking tmux and CLI status
    fn get_effective_status(&self) -> SessionStatus;

    /// Read the CLI status from the runtime/status.json file
    fn read_cli_status(&self) -> Option<CliStatus>;

    /// Save session metadata to meta.json
    fn save_metadata(&self) -> Result<()>;

    /// Load session metadata from meta.json in the given directory
    fn load_metadata(workdir: &Path) -> Result<Session>;
}

impl SessionExt for Session {
    fn generate_id() -> String {
        format!("session_{}", Uuid::new_v4().to_string().split('-').next().unwrap())
    }

    async fn create(
        cli: &str,
        init_path: &Path,
        name: Option<String>,
        config: &DaemonConfig,
    ) -> Result<Session> {
        let session_id = Self::generate_id();
        let display_name = name.unwrap_or_else(|| session_id.clone());
        let tmux_session = format!("summ-{}", session_id);

        // Create session directory structure
        let session_dir = config.sessions_dir.join(&session_id);
        crate::init::create_session_structure(&session_dir)?;

        // Initialize workspace from init_path
        let workspace_dir = session_dir.join("workspace");
        crate::init::initialize_workdir(&workspace_dir, init_path)?;

        // Deploy CLI hooks (e.g., Claude Code hooks)
        let runtime_dir = session_dir.join("runtime");
        crate::hooks::deploy_cli_hooks(&workspace_dir, cli, &session_id, &runtime_dir)?;

        // Install hook script on first session creation
        if let Err(e) = crate::hooks::install_hook_script(&config.sessions_dir.join("..")) {
            tracing::warn!("Failed to install hook script: {}", e);
        }

        // Create tmux session in the workspace directory
        crate::tmux::TmuxManager::create_session(&tmux_session, &workspace_dir, cli)?;

        // Enable logging
        let log_path = config.logs_dir.join(format!("{}.log", session_id));
        crate::tmux::TmuxManager::enable_logging(&tmux_session, &log_path)?;

        // Get CLI process PID
        let pid = crate::tmux::TmuxManager::get_pane_pid(&tmux_session)?;

        let now = Utc::now();
        let session = Session {
            session_id: session_id.clone(),
            tmux_session,
            name: display_name,
            cli: cli.to_string(),
            workdir: session_dir,
            init_source: init_path.to_path_buf(),
            status: SessionStatus::Running,
            pid,
            created_at: now,
            last_activity: now,
        };

        // Save metadata
        session.save_metadata()?;

        tracing::info!("Created session: {} ({})", session_id, cli);

        Ok(session)
    }

    fn get_effective_status(&self) -> SessionStatus {
        if !crate::tmux::TmuxManager::session_exists(&self.tmux_session) {
            return SessionStatus::Stopped;
        }

        if let Some(cli_status) = self.read_cli_status() {
            let age = Utc::now() - cli_status.timestamp;
            if age > Duration::seconds(120) {
                return SessionStatus::Running;
            }
            match cli_status.state {
                CliState::Idle => SessionStatus::Idle,
                CliState::Busy => SessionStatus::Running,
                CliState::Stopped => SessionStatus::Stopped,
            }
        } else {
            SessionStatus::Running
        }
    }

    fn read_cli_status(&self) -> Option<CliStatus> {
        let status_file = self.workdir.join("runtime/status.json");
        if !status_file.exists() {
            return None;
        }
        let content = fs::read_to_string(&status_file).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_metadata(&self) -> Result<()> {
        let meta_path = self.workdir.join("meta.json");
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize session metadata")?;
        fs::write(&meta_path, json)
            .context("Failed to write session metadata")?;
        Ok(())
    }

    fn load_metadata(workdir: &Path) -> Result<Session> {
        let meta_path = workdir.join("meta.json");
        let content = fs::read_to_string(&meta_path)
            .context("Failed to read session metadata")?;
        let session: Session = serde_json::from_str(&content)
            .context("Failed to parse session metadata")?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_generate_session_id() {
        let id1 = Session::generate_id();
        let id2 = Session::generate_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("session_"));
        assert!(id2.starts_with("session_"));
    }

    #[test]
    fn test_save_and_load_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let workdir = temp_dir.path();

        let session = Session {
            session_id: "test001".to_string(),
            tmux_session: "summ-test001".to_string(),
            name: "Test Session".to_string(),
            cli: "claude-code".to_string(),
            workdir: workdir.to_path_buf(),
            init_source: PathBuf::from("/tmp/init"),
            status: SessionStatus::Running,
            pid: Some(1234),
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };

        session.save_metadata().unwrap();
        assert!(workdir.join("meta.json").exists());

        let loaded = Session::load_metadata(workdir).unwrap();
        assert_eq!(loaded.session_id, session.session_id);
        assert_eq!(loaded.name, session.name);
        assert_eq!(loaded.cli, session.cli);
    }

    #[test]
    fn test_cli_status_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let runtime_dir = temp_dir.path().join("runtime");
        fs::create_dir_all(&runtime_dir).unwrap();

        let status_file = runtime_dir.join("status.json");
        let status_json = r#"{"state":"idle","message":"Ready","event":"test","timestamp":"2025-02-01T10:00:00Z"}"#;
        fs::write(&status_file, status_json).unwrap();

        let session = Session {
            session_id: "test001".to_string(),
            tmux_session: "summ-test001".to_string(),
            name: "Test".to_string(),
            cli: "claude".to_string(),
            workdir: temp_dir.path().to_path_buf(),
            init_source: PathBuf::from("/tmp"),
            status: SessionStatus::Running,
            pid: None,
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };

        let cli_status = session.read_cli_status().unwrap();
        assert_eq!(cli_status.state, CliState::Idle);
        assert_eq!(cli_status.message, Some("Ready".to_string()));
    }

    #[test]
    fn test_tmux_session_naming() {
        // Test that tmux session names follow the expected format
        let session_id = "session_abc123";
        let tmux_name = format!("summ-{}", session_id);
        assert_eq!(tmux_name, "summ-session_abc123");
        assert!(tmux_name.starts_with("summ-"));
    }
}
