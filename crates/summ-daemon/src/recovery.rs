// summ-daemon/src/recovery.rs
// Session recovery functionality for daemon restart
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use summ_common::{DaemonConfig, Session, SessionStatus};
use tracing::{info, warn};
use crate::session::SessionExt;

/// Recover existing sessions from tmux and metadata files
/// This should be called on daemon startup to reconnect to existing tmux sessions
pub fn recover_sessions(config: &DaemonConfig) -> Result<HashMap<String, Session>> {
    let mut sessions = HashMap::new();

    // Get all summ- prefixed tmux sessions
    let tmux_sessions = crate::tmux::TmuxManager::list_summ_sessions()
        .unwrap_or_default();
    let tmux_set: std::collections::HashSet<&str> =
        tmux_sessions.iter().map(|s| s.as_str()).collect();

    // Scan sessions directory for meta.json files
    let sessions_dir = &config.sessions_dir;

    if !sessions_dir.exists() {
        fs::create_dir_all(sessions_dir)
            .context("Failed to create sessions directory during recovery")?;
        info!("Created sessions directory during recovery");
        return Ok(sessions);
    }

    for entry in fs::read_dir(sessions_dir)
        .context("Failed to read sessions directory during recovery")?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let entry_path = entry.path();

        // Skip if not a directory
        if !entry_path.is_dir() {
            continue;
        }

        let meta_path = entry_path.join("meta.json");

        // Skip directories without meta.json
        if !meta_path.exists() {
            continue;
        }

        // Load session metadata
        let mut session: Session = <Session as SessionExt>::load_metadata(&entry_path)
            .with_context(|| format!("Failed to load metadata from {:?}", meta_path))?;

        // Reconcile with tmux state
        if tmux_set.contains(session.tmux_session.as_str()) {
            // tmux session exists, recover as running
            session.status = SessionStatus::Running;
            session.pid = crate::tmux::TmuxManager::get_pane_pid(&session.tmux_session)
                .ok()
                .flatten();
            info!(
                "Recovered running session: {} (tmux: {})",
                session.session_id, session.tmux_session
            );
        } else if session.status == SessionStatus::Running {
            // meta shows running but tmux session is gone, update to stopped
            session.status = SessionStatus::Stopped;
            session.pid = None;
            session.save_metadata().ok();
            info!(
                "Session {} marked as stopped (tmux session gone)",
                session.session_id
            );
        }

        sessions.insert(session.session_id.clone(), session);
    }

    // Check for orphan tmux sessions (without meta.json)
    for tmux_name in &tmux_sessions {
        let session_id = tmux_name.strip_prefix("summ-").unwrap_or(tmux_name);
        if !sessions.contains_key(session_id) {
            warn!(
                "Found orphan tmux session {} without meta.json, consider manual cleanup",
                tmux_name
            );
        }
    }

    info!(
        "Recovered {} sessions from disk and tmux",
        sessions.len()
    );

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_session_meta(session_dir: &std::path::Path, session_id: &str) -> Result<()> {
        let meta_path = session_dir.join("meta.json");
        let session = serde_json::json!({
            "session_id": session_id,
            "tmux_session": format!("summ-{}", session_id),
            "name": format!("Test Session {}", session_id),
            "cli": "echo test",
            "workdir": session_dir,
            "init_source": "/tmp/init",
            "status": "running",
            "pid": null,
            "created_at": "2025-01-01T00:00:00Z",
            "last_activity": "2025-01-01T00:00:00Z"
        });
        fs::write(&meta_path, serde_json::to_string_pretty(&session).unwrap())?;
        Ok(())
    }

    #[test]
    fn test_recover_from_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        // Create empty sessions directory
        fs::create_dir_all(&config.sessions_dir).unwrap();

        let result = recover_sessions(&config);
        assert!(result.is_ok());

        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn test_recover_skips_non_session_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        // Create sessions directory with various entries
        fs::create_dir_all(&config.sessions_dir).unwrap();

        // Create a directory without meta.json (should be skipped)
        let no_meta_dir = config.sessions_dir.join("no_meta");
        fs::create_dir(&no_meta_dir).unwrap();

        // Create a file (should be skipped)
        let random_file = config.sessions_dir.join("random_file.txt");
        File::create(&random_file).unwrap().write_all(b"content").unwrap();

        let result = recover_sessions(&config);
        assert!(result.is_ok());

        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn test_recover_loads_valid_session() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        fs::create_dir_all(&config.sessions_dir).unwrap();

        // Create a session directory with meta.json
        let session_dir = config.sessions_dir.join("session_test001");
        fs::create_dir_all(&session_dir).unwrap();

        let meta_content = r#"{
            "session_id": "session_test001",
            "tmux_session": "summ-session_test001",
            "name": "Test Session",
            "cli": "echo test",
            "workdir": "/tmp/session_test001",
            "init_source": "/tmp/init",
            "status": "running",
            "pid": null,
            "created_at": "2025-01-01T00:00:00Z",
            "last_activity": "2025-01-01T00:00:00Z"
        }"#;

        let meta_path = session_dir.join("meta.json");
        File::create(&meta_path).unwrap().write_all(meta_content.as_bytes()).unwrap();

        let result = recover_sessions(&config);
        assert!(result.is_ok());

        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key("session_test001"));

        let session = &sessions["session_test001"];
        assert_eq!(session.session_id, "session_test001");
        assert_eq!(session.tmux_session, "summ-session_test001");
        // Since tmux session doesn't actually exist, status should be updated to stopped
        // by the recovery logic when tmux_set doesn't contain the session
    }

    #[test]
    fn test_recover_creates_directory_if_missing() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            sessions_dir: temp_dir.path().join("nonexistent_sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        // Don't create the sessions directory - let recovery do it
        let result = recover_sessions(&config);
        assert!(result.is_ok());

        // Directory should have been created
        assert!(config.sessions_dir.exists());

        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 0);
    }
}
