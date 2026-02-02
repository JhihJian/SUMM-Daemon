# SUMM-Daemon Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a CLI process management daemon service using Rust and tmux that enables multi-agent collaboration with Session management, initialization support, and message injection.

**Architecture:**
- Three-crate workspace: summ-daemon (daemon binary), summ-cli (client binary), summ-common (shared types)
- tmux for process hosting and terminal management
- Unix Domain Socket for IPC with JSON-over-socket protocol
- Claude Code Hook integration for idle/busy status tracking

**Tech Stack:** Rust, Tokio, tmux, clap, serde, systemd, compress-tools

---

## Phase 1: Project Setup and Core Infrastructure

### Task 1.1: Create Rust Workspace Structure

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/summ-common/Cargo.toml`
- Create: `crates/summ-common/src/lib.rs`
- Create: `crates/summ-daemon/Cargo.toml`
- Create: `crates/summ-daemon/src/main.rs`
- Create: `crates/summ-cli/Cargo.toml`
- Create: `crates/summ-cli/src/main.rs`
- Create: `.gitignore`
- Create: `README.md`

**Step 1: Create workspace Cargo.toml**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/summ-common",
    "crates/summ-daemon",
    "crates/summ-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["SUMM Team"]

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
```

**Step 2: Run cargo check to verify workspace**

Run: `cargo check`
Expected: SUCCESS - workspace compiles with placeholder code

**Step 3: Create summ-common/Cargo.toml**

```toml
[package]
name = "summ-common"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
chrono = { workspace = true }
```

**Step 4: Create summ-common/src/lib.rs placeholder**

```rust
// summ-common/src/lib.rs
pub mod types;
pub mod protocol;
pub mod error;
```

**Step 5: Create summ-daemon/Cargo.toml**

```toml
[package]
name = "summ-daemon"
version.workspace = true
edition.workspace = true

[[bin]]
name = "summ-daemon"
path = "src/main.rs"

[dependencies]
summ-common = { path = "../summ-common" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
compress-tools = "0.14"
sd-notify = "0.4"
dirs = { workspace = true }
```

**Step 6: Create summ-daemon/src/main.rs placeholder**

```rust
// summ-daemon/src/main.rs
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("SUMM Daemon starting...");
    Ok(())
}
```

**Step 7: Create summ-cli/Cargo.toml**

```toml
[package]
name = "summ-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "summ"
path = "src/main.rs"

[dependencies]
summ-common = { path = "../summ-common" }
clap = { version = "4.4", features = ["derive"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
dirs = { workspace = true }
```

**Step 8: Create summ-cli/src/main.rs placeholder**

```rust
// summ-cli/src/main.rs
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    Ok(())
}
```

**Step 9: Create .gitignore**

```
/target
Cargo.lock
**/*.rs.bk
.DS_Store
```

**Step 10: Create basic README.md**

```markdown
# SUMM Daemon

CLI process management daemon service for multi-agent collaboration.

## Status

Development in progress.
```

**Step 11: Run cargo build to verify all crates**

Run: `cargo build`
Expected: SUCCESS - all three crates compile

**Step 12: Commit**

```bash
git add Cargo.toml crates/ .gitignore README.md
git commit -m "feat: setup Rust workspace with three crates"
```

---

### Task 1.2: Define Core Data Types in summ-common

**Files:**
- Modify: `crates/summ-common/src/lib.rs`
- Create: `crates/summ-common/src/types.rs`
- Create: `crates/summ-common/src/error.rs`
- Create: `crates/summ-common/src/protocol.rs`

**Step 1: Write error type tests**

```rust
// summ-common/src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ErrorCode {
    #[error("E001: Init resource not found or inaccessible")]
    E001,
    #[error("E002: Session not found")]
    E002,
    #[error("E003: Session stopped, cannot operate")]
    E003,
    #[error("E004: Archive extraction failed")]
    E004,
    #[error("E005: Process start failed")]
    E005,
    #[error("E006: Message injection failed")]
    E006,
    #[error("E007: Daemon not running")]
    E007,
    #[error("E008: Invalid CLI command")]
    E008,
    #[error("E009: tmux not available")]
    E009,
}

impl ErrorCode {
    pub fn code(&self) -> &'static str {
        match self {
            ErrorCode::E001 => "E001",
            ErrorCode::E002 => "E002",
            ErrorCode::E003 => "E003",
            ErrorCode::E004 => "E004",
            ErrorCode::E005 => "E005",
            ErrorCode::E006 => "E006",
            ErrorCode::E007 => "E007",
            ErrorCode::E008 => "E008",
            ErrorCode::E009 => "E009",
        }
    }
}

#[derive(Debug, Clone, Error)]
pub struct DaemonError {
    pub code: ErrorCode,
    pub message: String,
}

impl DaemonError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn e001(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E001, message)
    }

    pub fn e002(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E002, message)
    }

    pub fn e003(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E003, message)
    }

    pub fn e004(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E004, message)
    }

    pub fn e005(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E005, message)
    }

    pub fn e006(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E006, message)
    }

    pub fn e007(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E007, message)
    }

    pub fn e008(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E008, message)
    }

    pub fn e009(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E009, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::E001.code(), "E001");
        assert_eq!(ErrorCode::E002.code(), "E002");
    }

    #[test]
    fn test_daemon_error_creation() {
        let err = DaemonError::e002("session not found");
        assert_eq!(err.code, ErrorCode::E002);
        assert_eq!(err.message, "session not found");
    }
}
```

**Step 2: Run error tests**

Run: `cargo test -p summ-common error`
Expected: PASS

**Step 3: Write types module**

```rust
// summ-common/src/types.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Idle,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub tmux_session: String,
    pub name: String,
    pub cli: String,
    pub workdir: PathBuf,
    pub init_source: PathBuf,
    pub status: SessionStatus,
    pub pid: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CliState {
    Idle,
    Busy,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStatus {
    pub state: CliState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub base_dir: PathBuf,
    pub sessions_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub socket_path: PathBuf,
    pub cleanup_retention_hours: u64,
    pub tmux_prefix: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let home = dirs::home_dir().expect("HOME directory not found");
        let base = home.join(".summ-daemon");
        Self {
            base_dir: base.clone(),
            sessions_dir: base.join("sessions"),
            logs_dir: base.join("logs"),
            socket_path: base.join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub name: String,
    pub cli: String,
    pub workdir: PathBuf,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
}

impl From<Session> for SessionInfo {
    fn from(session: Session) -> Self {
        Self {
            session_id: session.session_id,
            name: session.name,
            cli: session.cli,
            workdir: session.workdir,
            status: session.status,
            created_at: session.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""running""#);

        let deserialized: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SessionStatus::Running);
    }

    #[test]
    fn test_cli_state_serialization() {
        let state = CliState::Idle;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""idle""#);
    }

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(config.sessions_dir.ends_with("sessions"));
        assert!(config.logs_dir.ends_with("logs"));
        assert_eq!(config.tmux_prefix, "summ-");
    }

    #[test]
    fn test_session_to_info_conversion() {
        let session = Session {
            session_id: "test001".to_string(),
            tmux_session: "summ-test001".to_string(),
            name: "Test Session".to_string(),
            cli: "claude-code".to_string(),
            workdir: PathBuf::from("/tmp/test"),
            init_source: PathBuf::from("/tmp/init"),
            status: SessionStatus::Running,
            pid: Some(1234),
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };

        let info = SessionInfo::from(session);
        assert_eq!(info.session_id, "test001");
        assert_eq!(info.name, "Test Session");
        assert_eq!(info.status, SessionStatus::Running);
    }
}
```

**Step 4: Run type tests**

Run: `cargo test -p summ-common types`
Expected: PASS

**Step 5: Write protocol module**

```rust
// summ-common/src/protocol.rs
use crate::error::DaemonError;
use crate::types::SessionInfo;
use crate::types::SessionStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    Start {
        cli: String,
        init: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Stop {
        session_id: String,
    },
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        status_filter: Option<String>,
    },
    Status {
        session_id: String,
    },
    Inject {
        session_id: String,
        message: String,
    },
    DaemonStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    Success { data: serde_json::Value },
    Error {
        code: String,
        message: String,
    },
}

impl Response {
    pub fn success(data: serde_json::Value) -> Self {
        Self::Success { data }
    }

    pub fn error(err: &DaemonError) -> Self {
        Self::Error {
            code: err.code.code().to_string(),
            message: err.message.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatusResponse {
    pub running: bool,
    pub session_count: usize,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::Start {
            cli: "claude-code".to_string(),
            init: PathBuf::from("/tmp/init.zip"),
            name: Some("test".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"Start""#));
        assert!(json.contains("claude-code"));

        let deserialized: Request = serde_json::from_str(&json).unwrap();
        match deserialized {
            Request::Start { cli, .. } => assert_eq!(cli, "claude-code"),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_response_creation() {
        let err = DaemonError::e002("session not found");
        let resp = Response::error(&err);

        match resp {
            Response::Error { code, message } => {
                assert_eq!(code, "E002");
                assert_eq!(message, "session not found");
            }
            _ => panic!("Expected error response"),
        }
    }

    #[test]
    fn test_list_request_with_filter() {
        let req = Request::List {
            status_filter: Some("idle".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();

        let deserialized: Request = serde_json::from_str(&json).unwrap();
        match deserialized {
            Request::List { status_filter } => {
                assert_eq!(status_filter, Some("idle".to_string()));
            }
            _ => panic!("Wrong type"),
        }
    }
}
```

**Step 6: Run protocol tests**

Run: `cargo test -p summ-common protocol`
Expected: PASS

**Step 7: Update lib.rs to export all modules**

```rust
// summ-common/src/lib.rs
pub mod error;
pub mod protocol;
pub mod types;

pub use error::{DaemonError, ErrorCode};
pub use protocol::{DaemonStatusResponse, Request, Response};
pub use types::{CliState, CliStatus, DaemonConfig, Session, SessionInfo, SessionStatus};
```

**Step 8: Run all summ-common tests**

Run: `cargo test -p summ-common`
Expected: All tests PASS

**Step 9: Commit**

```bash
git add crates/summ-common/
git commit -m "feat(summ-common): define core data types, error codes, and IPC protocol"
```

---

### Task 1.3: Add thiserror dependency and fix compilation

**Files:**
- Modify: `crates/summ-common/Cargo.toml`

**Step 1: Add thiserror to summ-common dependencies**

```toml
[dependencies]
serde = { workspace = true }
chrono = { workspace = true }
thiserror = "1.0"
```

**Step 2: Run cargo build**

Run: `cargo build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add crates/summ-common/Cargo.toml
git commit -m "fix(summ-common): add thiserror dependency for error derive"
```

---

### Task 1.4: Implement TmuxManager

**Files:**
- Create: `crates/summ-daemon/src/tmux.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write TmuxManager tests first**

```rust
// summ-daemon/src/tmux.rs
use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;

const MIN_TMUX_VERSION: (u32, u32) = (3, 0);

pub struct TmuxManager;

impl TmuxManager {
    /// Check if tmux is available and version >= 3.0
    pub fn check_available() -> Result<()> {
        let output = Command::new("tmux")
            .arg("-V")
            .output()
            .context("tmux not found. Please install tmux 3.0 or later")?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_version(&version_str)?;

        Ok(())
    }

    fn parse_version(version_str: &str) -> Result<(u32, u32)> {
        // tmux outputs "tmux 3.3a" or similar
        let parts: Vec<&str> = version_str
            .trim()
            .split_whitespace()
            .collect();

        if parts.len() < 2 {
            anyhow::bail!("Invalid tmux version output: {}", version_str);
        }

        let version_part = parts[1];
        let version_numbers: Vec<&str> = version_part
            .split('.')
            .take(2)
            .collect();

        if version_numbers.len() < 2 {
            anyhow::bail!("Invalid tmux version format: {}", version_part);
        }

        let major: u32 = version_numbers[0]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);

        let minor: u32 = version_numbers[1]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);

        if (major, minor) < MIN_TMUX_VERSION {
            anyhow::bail!(
                "tmux version {}.{} is below minimum required {}.{}",
                major, minor, MIN_TMUX_VERSION.0, MIN_TMUX_VERSION.1
            );
        }

        Ok((major, minor))
    }

    /// Create a new tmux session with the given command
    pub fn create_session(
        session_name: &str,
        workdir: &Path,
        command: &str,
    ) -> Result<()> {
        let workdir_str = workdir
            .to_str()
            .context("Workdir contains invalid UTF-8")?;

        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",                            // detached
                "-s", session_name,
                "-c", workdir_str,
                command,
            ])
            .status()
            .context("Failed to create tmux session")?;

        if !status.success() {
            anyhow::bail!("tmux new-session failed with status: {}", status);
        }
        Ok(())
    }

    /// Check if a tmux session exists
    pub fn session_exists(session_name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the PID of the pane in the session
    pub fn get_pane_pid(session_name: &str) -> Result<Option<u32>> {
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-t", session_name,
                "-F", "#{pane_pid}",
            ])
            .output()
            .context("Failed to get pane PID")?;

        if !output.status.success() {
            return Ok(None);
        }

        let pid_str = String::from_utf8_lossy(&output.stdout);
        let pid = pid_str.trim().parse().ok();
        Ok(pid)
    }

    /// Send keys to a tmux session (for message injection)
    pub fn send_keys(session_name: &str, message: &str, press_enter: bool) -> Result<()> {
        let mut args = vec!["send-keys", "-t", session_name, message];
        if press_enter {
            args.push("Enter");
        }

        let status = Command::new("tmux")
            .args(&args)
            .status()
            .context("Failed to send keys to tmux session")?;

        if !status.success() {
            anyhow::bail!("tmux send-keys failed");
        }
        Ok(())
    }

    /// Kill a tmux session
    pub fn kill_session(session_name: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["kill-session", "-t", session_name])
            .status()
            .context("Failed to kill tmux session")?;

        if !status.success() {
            anyhow::bail!("tmux kill-session failed");
        }
        Ok(())
    }

    /// List all tmux sessions with the summ- prefix
    pub fn list_summ_sessions() -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()
            .context("Failed to list tmux sessions")?;

        // tmux returns non-zero when no sessions exist
        if !output.status.success() {
            return Ok(vec![]);
        }

        let sessions: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|name| name.starts_with("summ-"))
            .map(|s| s.to_string())
            .collect();

        Ok(sessions)
    }

    /// Enable logging for a session
    pub fn enable_logging(session_name: &str, log_path: &Path) -> Result<()> {
        let log_path_str = log_path
            .to_str()
            .context("Log path contains invalid UTF-8")?;

        let status = Command::new("tmux")
            .args([
                "pipe-pane",
                "-t", session_name,
                &format!("cat >> {}", log_path_str),
            ])
            .status()
            .context("Failed to enable logging for session")?;

        if !status.success() {
            anyhow::bail!("Failed to enable logging for session");
        }
        Ok(())
    }

    /// Capture pane content
    pub fn capture_pane(session_name: &str, lines: i32) -> Result<String> {
        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t", session_name,
                "-p",
                "-S", &(-lines).to_string(),
            ])
            .output()
            .context("Failed to capture pane")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let result = TmuxManager::parse_version("tmux 3.3a");
        assert!(result.is_ok());
        let (major, minor) = result.unwrap();
        assert_eq!(major, 3);
        assert_eq!(minor, 3);
    }

    #[test]
    fn test_parse_version_old() {
        let result = TmuxManager::parse_version("tmux 2.9");
        assert!(result.is_ok());
        let (major, minor) = result.unwrap();
        assert_eq!(major, 2);
        assert_eq!(minor, 9);
    }

    #[test]
    fn test_parse_version_invalid() {
        let result = TmuxManager::parse_version("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_tmux_session_name_generation() {
        let session_id = "session_001";
        let tmux_name = format!("summ-{}", session_id);
        assert_eq!(tmux_name, "summ-session_001");
    }
}
```

**Step 2: Run tmux tests**

Run: `cargo test -p summ-daemon tmux`
Expected: PASS (unit tests pass without tmux installed)

**Step 3: Update main.rs to use TmuxManager**

```rust
// summ-daemon/src/main.rs
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
```

**Step 4: Run daemon binary**

Run: `cargo run -p summ-daemon`
Expected: "tmux check passed" message (if tmux is installed) or error message

**Step 5: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement TmuxManager with tmux command abstraction"
```

---

### Task 1.5: Implement Configuration Management

**Files:**
- Create: `crates/summ-daemon/src/config.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write config module with tests**

```rust
// summ-daemon/src/config.rs
use anyhow::{Context, Result};
use summ_common::DaemonConfig;
use std::fs;

impl DaemonConfig {
    /// Load configuration with defaults
    pub fn load() -> Result<Self> {
        let config = Self::default();
        config.ensure_directories()?;
        Ok(config)
    }

    /// Ensure all required directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.base_dir)
            .context("Failed to create base directory")?;
        fs::create_dir_all(&self.sessions_dir)
            .context("Failed to create sessions directory")?;
        fs::create_dir_all(&self.logs_dir)
            .context("Failed to create logs directory")?;
        Ok(())
    }

    /// Get the path to meta.json for a session
    pub fn session_meta_path(&self, session_id: &str) -> std::path::PathBuf {
        self.sessions_dir.join(session_id).join("meta.json")
    }

    /// Get the path to runtime/status.json for a session
    pub fn session_status_path(&self, session_id: &str) -> std::path::PathBuf {
        self.sessions_dir
            .join(session_id)
            .join("runtime")
            .join("status.json")
    }

    /// Get the workspace directory for a session
    pub fn session_workspace_path(&self, session_id: &str) -> std::path::PathBuf {
        self.sessions_dir
            .join(session_id)
            .join("workspace")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_load_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            base_dir: temp_dir.path().to_path_buf(),
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
```

**Step 2: Add tempfile to dev-dependencies**

```toml
# summ-daemon/Cargo.toml

[dev-dependencies]
tempfile = "3.8"
tokio-test = "0.4"
```

**Step 3: Run config tests**

Run: `cargo test -p summ-daemon config`
Expected: PASS

**Step 4: Update main.rs to use config**

```rust
// summ-daemon/src/main.rs
mod config;
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
```

**Step 5: Run daemon binary**

Run: `cargo run -p summ-daemon`
Expected: Shows directories paths

**Step 6: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement configuration management with directory setup"
```

---

## Phase 2: Session Management

### Task 2.1: Implement Session Type with Metadata Persistence

**Files:**
- Create: `crates/summ-daemon/src/session.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write Session tests**

```rust
// summ-daemon/src/session.rs
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use summ_common::{CliStatus, CliState, Session, SessionStatus};
use uuid::Uuid;

impl Session {
    /// Generate a new unique session ID
    pub fn generate_id() -> String {
        format!("session_{}", Uuid::new_v4().to_string().split('-').next().unwrap())
    }

    /// Get the effective status based on tmux state and hook-reported status
    pub fn get_effective_status(&self) -> SessionStatus {
        // First check if tmux session exists
        if !crate::tmux::TmuxManager::session_exists(&self.tmux_session) {
            return SessionStatus::Stopped;
        }

        // Read hook-reported status if available
        if let Some(cli_status) = self.read_cli_status() {
            // Check if status is stale (> 120 seconds old)
            let age = Utc::now() - cli_status.timestamp;
            if age.num_seconds() > 120 {
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

    /// Read CLI status from runtime/status.json
    pub fn read_cli_status(&self) -> Option<CliStatus> {
        let status_file = self.workdir.join("runtime/status.json");

        if !status_file.exists() {
            return None;
        }

        let content = fs::read_to_string(&status_file).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save session metadata to meta.json
    pub fn save_metadata(&self) -> Result<()> {
        let meta_path = self.workdir.join("meta.json");
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize session metadata")?;

        fs::write(&meta_path, json)
            .context("Failed to write session metadata")?;

        Ok(())
    }

    /// Load session metadata from meta.json
    pub fn load_metadata(workdir: &Path) -> Result<Session> {
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
}
```

**Step 2: Run session tests**

Run: `cargo test -p summ-daemon session`
Expected: PASS

**Step 3: Update main.rs to include session module**

```rust
mod config;
mod session;
mod tmux;
```

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement Session type with metadata persistence"
```

---

### Task 2.2: Implement Initialization Functions (Directory, ZIP, tar.gz)

**Files:**
- Create: `crates/summ-daemon/src/init.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write initialization module with tests**

```rust
// summ-daemon/src/init.rs
use anyhow::{Context, Result};
use compress_tools::CompressionFormat;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Initialize workdir from a source (directory, zip, or tar.gz)
pub fn initialize_workdir(workdir: &Path, init_path: &Path) -> Result<()> {
    if !init_path.exists() {
        anyhow::bail!(
            "Initialization source not found: {:?}",
            init_path
        );
    }

    // Ensure workdir exists
    fs::create_dir_all(workdir)
        .context("Failed to create workdir")?;

    if init_path.is_dir() {
        copy_dir_contents(init_path, workdir)?;
    } else {
        let extension = init_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "zip" => extract_zip(init_path, workdir)?,
            "gz" => {
                if init_path.to_string_lossy().ends_with(".tar.gz") {
                    extract_tar_gz(init_path, workdir)?;
                } else {
                    anyhow::bail!("Unsupported archive format: .gz (only .tar.gz supported)");
                }
            }
            _ => anyhow::bail!(
                "Unsupported initialization source: {:?}. Expected directory, .zip, or .tar.gz",
                init_path
            ),
        }
    }

    Ok(())
}

/// Copy directory contents to workdir
fn copy_dir_contents(source: &Path, dest: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("Failed to read source directory: {:?}", source))?
    {
        let entry = entry?;
        let src = entry.path();
        let dst = dest.join(entry.file_name());

        if src.is_dir() {
            fs::create_dir_all(&dst)?;
            copy_dir_contents(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)
                .with_context(|| format!("Failed to copy file: {:?}", src))?;
        }
    }
    Ok(())
}

/// Extract ZIP archive to workdir
fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let mut archive_file = fs::File::open(archive)
        .with_context(|| format!("Failed to open archive: {:?}", archive))?;

    compress_tools::extract_source(
        &mut archive_file,
        dest,
        CompressionFormat::Zip,
    ).context("Failed to extract ZIP archive")?;

    Ok(())
}

/// Extract tar.gz archive to workdir
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let mut archive_file = fs::File::open(archive)
        .with_context(|| format!("Failed to open archive: {:?}", archive))?;

    compress_tools::extract_source(
        &mut archive_file,
        dest,
        CompressionFormat::Gzip,
    ).context("Failed to extract tar.gz archive")?;

    Ok(())
}

/// Create initialization directory structure
pub fn create_session_structure(base_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let workspace_dir = base_dir.join("workspace");
    let runtime_dir = base_dir.join("runtime");

    fs::create_dir_all(&workspace_dir)
        .context("Failed to create workspace directory")?;
    fs::create_dir_all(&runtime_dir)
        .context("Failed to create runtime directory")?;

    Ok((workspace_dir, runtime_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_copy_dir_contents() {
        let temp_src = TempDir::new().unwrap();
        let temp_dst = TempDir::new().unwrap();

        // Create source structure
        let src_file = temp_src.path().join("test.txt");
        let mut f = fs::File::create(&src_file).unwrap();
        f.write_all(b"test content").unwrap();

        let src_subdir = temp_src.path().join("subdir");
        fs::create_dir_all(&src_subdir).unwrap();
        let src_subfile = src_subdir.join("nested.txt");
        let mut f2 = fs::File::create(&src_subfile).unwrap();
        f2.write_all(b"nested content").unwrap();

        // Copy
        copy_dir_contents(temp_src.path(), temp_dst.path()).unwrap();

        // Verify
        assert!(temp_dst.path().join("test.txt").exists());
        assert!(temp_dst.path().join("subdir/nested.txt").exists());

        let content = fs::read_to_string(temp_dst.path().join("test.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_create_session_structure() {
        let temp_dir = TempDir::new().unwrap();
        let (workspace, runtime) = create_session_structure(temp_dir.path()).unwrap();

        assert!(workspace.ends_with("workspace"));
        assert!(runtime.ends_with("runtime"));
        assert!(workspace.exists());
        assert!(runtime.exists());
    }

    #[test]
    fn test_initialize_workdir_from_directory() {
        let temp_src = TempDir::new().unwrap();
        let temp_dst = TempDir::new().unwrap();

        // Create source with content
        let src_file = temp_src.path().join("file.txt");
        fs::write(&src_file, "test content").unwrap();

        // Initialize
        initialize_workdir(temp_dst.path(), temp_src.path()).unwrap();

        // Verify
        assert!(temp_dst.path().join("file.txt").exists());
        let content = fs::read_to_string(temp_dst.path().join("file.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_initialize_workdir_nonexistent_source() {
        let temp_dst = TempDir::new().unwrap();
        let result = initialize_workdir(temp_dst.path(), PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }
}
```

**Step 2: Run init tests**

Run: `cargo test -p summ-daemon init`
Expected: PASS

**Step 3: Update main.rs**

```rust
mod config;
mod init;
mod session;
mod tmux;
```

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement workdir initialization (directory, zip, tar.gz)"
```

---

### Task 2.3: Implement Session Creation

**Files:**
- Modify: `crates/summ-daemon/src/session.rs`

**Step 1: Write session creation function with tests**

```rust
// Add to summ-daemon/src/session.rs

use crate::init::{create_session_structure, initialize_workdir};
use crate::tmux::TmuxManager;
use summ_common::DaemonConfig;

impl Session {
    /// Create a new session
    pub async fn create(
        cli: &str,
        init_path: &Path,
        name: Option<String>,
        config: &DaemonConfig,
    ) -> Result<Self> {
        // 1. Generate unique session_id
        let session_id = Self::generate_id();
        let display_name = name.unwrap_or_else(|| session_id.clone());
        let tmux_session = format!("{}{}", config.tmux_prefix, &session_id);

        // 2. Create session directory structure
        let workdir = config.sessions_dir.join(&session_id);
        let (workspace_dir, runtime_dir) = create_session_structure(&workdir)?;

        // 3. Initialize workspace from init source
        initialize_workdir(&workspace_dir, init_path)?;

        // 4. Create tmux session in workspace directory
        TmuxManager::create_session(&tmux_session, &workspace_dir, cli)?;

        // 5. Get PID
        let pid = TmuxManager::get_pane_pid(&tmux_session)?;

        // 6. Enable logging
        let log_path = config.logs_dir.join(format!("{}.log", session_id));
        TmuxManager::enable_logging(&tmux_session, &log_path)?;

        // 7. Build session metadata
        let session = Session {
            session_id: session_id.clone(),
            tmux_session,
            name: display_name,
            cli: cli.to_string(),
            workdir,
            init_source: init_path.to_path_buf(),
            status: SessionStatus::Running,
            pid,
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };

        // 8. Save metadata
        session.save_metadata()?;

        tracing::info!("Created session: {}", session_id);

        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[test]
    fn test_tmux_session_naming() {
        let prefix = "summ-";
        let session_id = "session_001";
        let tmux_name = format!("{}{}", prefix, session_id);
        assert_eq!(tmux_name, "summ-session_001");
    }
}
```

**Step 2: Run session tests**

Run: `cargo test -p summ-daemon session`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement session creation function"
```

---

### Task 2.4: Implement Session Recovery

**Files:**
- Create: `crates/summ-daemon/src/recovery.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write recovery module with tests**

```rust
// summ-daemon/src/recovery.rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use summ_common::{DaemonConfig, Session, SessionStatus};
use crate::tmux::TmuxManager;

/// Recover existing sessions from tmux and metadata
pub fn recover_sessions(config: &DaemonConfig) -> Result<HashMap<String, Session>> {
    let mut sessions = HashMap::new();

    // 1. Get all summ- prefixed tmux sessions
    let tmux_sessions = TmuxManager::list_summ_sessions()?;
    let tmux_set: std::collections::HashSet<_> =
        tmux_sessions.into_iter().collect();

    // 2. Scan for meta.json files
    if !config.sessions_dir.exists() {
        tracing::info!("Sessions directory does not exist, starting fresh");
        return Ok(sessions);
    }

    for entry in fs::read_dir(&config.sessions_dir)
        .context("Failed to read sessions directory")?
    {
        let entry = entry?;
        let session_dir = entry.path();

        // Skip if not a directory
        if !session_dir.is_dir() {
            continue;
        }

        let meta_path = session_dir.join("meta.json");
        if !meta_path.exists() {
            continue;
        }

        match Session::load_metadata(&session_dir) {
            Ok(mut session) => {
                // 3. Reconcile with tmux state
                if tmux_set.contains(&session.tmux_session) {
                    // tmux session exists, mark as running
                    let old_status = session.status;
                    session.status = session.get_effective_status();
                    session.pid = TmuxManager::get_pane_pid(&session.tmux_session)?;

                    if old_status != session.status {
                        session.save_metadata().ok();
                    }

                    tracing::info!(
                        "Recovered session: {} (status: {:?})",
                        session.session_id,
                        session.status
                    );
                } else if session.status == SessionStatus::Running
                    || session.status == SessionStatus::Idle
                {
                    // Meta says running/idle but tmux session gone
                    session.status = SessionStatus::Stopped;
                    session.save_metadata().ok();
                    tracing::info!(
                        "Session {} marked as stopped (tmux session gone)",
                        session.session_id
                    );
                }

                sessions.insert(session.session_id.clone(), session);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load session metadata from {:?}: {}",
                    session_dir,
                    e
                );
            }
        }
    }

    // 4. Check for orphan tmux sessions
    for tmux_name in tmux_set {
        let session_id = tmux_name.strip_prefix("summ-").unwrap_or(&tmux_name);
        if !sessions.contains_key(session_id) {
            tracing::warn!(
                "Found orphan tmux session {} without meta.json",
                tmux_name
            );
        }
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_recover_from_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            base_dir: temp_dir.path().to_path_buf(),
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        // Create empty sessions directory
        fs::create_dir_all(&config.sessions_dir).unwrap();

        let sessions = recover_sessions(&config).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_recover_skips_non_session_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            base_dir: temp_dir.path().to_path_buf(),
            sessions_dir: temp_dir.path().join("sessions"),
            logs_dir: temp_dir.path().join("logs"),
            socket_path: temp_dir.path().join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        };

        // Create a file instead of directory
        fs::create_dir_all(&config.sessions_dir).unwrap();
        fs::write(config.sessions_dir.join("not_a_dir"), "test").unwrap();

        let sessions = recover_sessions(&config).unwrap();
        assert!(sessions.is_empty());
    }
}
```

**Step 2: Run recovery tests**

Run: `cargo test -p summ-daemon recovery`
Expected: PASS

**Step 3: Update main.rs**

```rust
mod config;
mod init;
mod recovery;
mod session;
mod tmux;
```

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement session recovery on daemon restart"
```

---

## Phase 3: IPC and Daemon Server

### Task 3.1: Implement IPC Protocol Handler

**Files:**
- Create: `crates/summ-daemon/src/ipc.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write IPC handler with frame parsing**

```rust
// summ-daemon/src/ipc.rs
use anyhow::{Context, Result};
use serde_json;
use summ_common::{Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Maximum request size (16MB)
const MAX_REQUEST_SIZE: usize = 16 * 1024 * 1024;

/// Read a length-prefixed JSON message from the stream
pub async fn read_request(stream: &mut UnixStream) -> Result<Request> {
    // Read 4-byte length prefix (big-endian u32)
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await
        .context("Failed to read message length")?;

    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_REQUEST_SIZE {
        anyhow::bail!(
            "Request size {} exceeds maximum {}",
            len,
            MAX_REQUEST_SIZE
        );
    }

    if len == 0 {
        anyhow::bail!("Empty request received");
    }

    // Read JSON payload
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await
        .context("Failed to read request payload")?;

    // Parse JSON
    let request: Request = serde_json::from_slice(&buf)
        .context("Failed to parse request JSON")?;

    Ok(request)
}

/// Write a length-prefixed JSON response to the stream
pub async fn write_response(stream: &mut UnixStream, response: &Response) -> Result<()> {
    let json = serde_json::to_vec(response)
        .context("Failed to serialize response")?;

    let len = json.len() as u32;

    // Write length prefix
    stream.write_all(&len.to_be_bytes()).await
        .context("Failed to write response length")?;

    // Write JSON payload
    stream.write_all(&json).await
        .context("Failed to write response payload")?;

    stream.flush().await
        .context("Failed to flush response")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::List {
            status_filter: Some("idle".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("List"));
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::Success {
            data: serde_json::json!({"test": "value"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Success"));
    }

    #[test]
    fn test_length_prefix_encoding() {
        let value: u32 = 1234;
        let bytes = value.to_be_bytes();
        let decoded = u32::from_be_bytes(bytes);
        assert_eq!(decoded, 1234);
    }
}
```

**Step 2: Run IPC tests**

Run: `cargo test -p summ-daemon ipc`
Expected: PASS

**Step 3: Update main.rs**

```rust
mod config;
mod init;
mod ipc;
mod recovery;
mod session;
mod tmux;
```

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement IPC protocol handler with length-prefixed framing"
```

---

### Task 3.2: Implement Request Handler

**Files:**
- Create: `crates/summ-daemon/src/handler.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write request handler**

```rust
// summ-daemon/src/handler.rs
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use summ_common::{DaemonConfig, Request, Response, SessionInfo, SessionStatus};
use tokio::sync::RwLock;

pub struct Handler {
    sessions: Arc<RwLock<HashMap<String, summ_common::Session>>>,
    config: Arc<DaemonConfig>,
}

impl Handler {
    pub fn new(
        sessions: Arc<RwLock<HashMap<String, summ_common::Session>>>,
        config: Arc<DaemonConfig>,
    ) -> Self {
        Self { sessions, config }
    }

    pub async fn handle(&self, request: Request) -> Response {
        match request {
            Request::Start { cli, init, name } => self.handle_start(cli, init, name).await,
            Request::Stop { session_id } => self.handle_stop(session_id).await,
            Request::List { status_filter } => self.handle_list(status_filter).await,
            Request::Status { session_id } => self.handle_status(session_id).await,
            Request::Inject { session_id, message } => {
                self.handle_inject(session_id, message).await
            }
            Request::DaemonStatus => self.handle_daemon_status().await,
        }
    }

    async fn handle_start(
        &self,
        cli: String,
        init: std::path::PathBuf,
        name: Option<String>,
    ) -> Response {
        // Validate init path exists
        if !init.exists() {
            return Response::Error {
                code: "E001".to_string(),
                message: format!("Initialization source not found: {:?}", init),
            };
        }

        // Validate CLI command is not empty
        if cli.trim().is_empty() {
            return Response::Error {
                code: "E008".to_string(),
                message: "CLI command cannot be empty".to_string(),
            };
        }

        match summ_common::Session::create(&cli, &init, name, &self.config).await {
            Ok(session) => {
                // Add to sessions map
                let session_id = session.session_id.clone();
                let mut sessions = self.sessions.write().await;
                sessions.insert(session_id.clone(), session);

                let session_ref = sessions.get(&session_id).unwrap();
                Response::success(json!(session_ref))
            }
            Err(e) => {
                tracing::error!("Failed to create session: {}", e);
                Response::Error {
                    code: "E005".to_string(),
                    message: format!("Failed to create session: {}", e),
                }
            }
        }
    }

    async fn handle_stop(&self, session_id: String) -> Response {
        let mut sessions = self.sessions.write().await;

        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => {
                return Response::Error {
                    code: "E002".to_string(),
                    message: format!("Session not found: {}", session_id),
                };
            }
        };

        // Kill tmux session
        if let Err(e) = crate::tmux::TmuxManager::kill_session(&session.tmux_session) {
            tracing::warn!("Failed to kill tmux session: {}", e);
        }

        // Update status
        let session = sessions.get_mut(&session_id).unwrap();
        session.status = SessionStatus::Stopped;
        session.last_activity = chrono::Utc::now();
        let _ = session.save_metadata();

        Response::success(json!({
            "session_id": session_id,
            "status": "stopped"
        }))
    }

    async fn handle_list(&self, status_filter: Option<String>) -> Response {
        let sessions = self.sessions.read().await;
        let mut session_infos: Vec<SessionInfo> = sessions
            .values()
            .map(|s| s.clone().into())
            .collect();

        // Apply status filter if provided
        if let Some(filter) = status_filter {
            let filter_status = match filter.as_str() {
                "running" => Some(SessionStatus::Running),
                "idle" => Some(SessionStatus::Idle),
                "stopped" => Some(SessionStatus::Stopped),
                _ => None,
            };

            if let Some(fs) = filter_status {
                session_infos.retain(|s| s.status == fs);
            }
        }

        // Sort by creation time (newest first)
        session_infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Response::success(json!({ "sessions": session_infos }))
    }

    async fn handle_status(&self, session_id: String) -> Response {
        let sessions = self.sessions.read().await;

        match sessions.get(&session_id) {
            Some(session) => {
                // Update effective status
                let effective_status = session.get_effective_status();
                let mut session_json = serde_json::to_value(session).unwrap();
                session_json["status"] = serde_json::to_value(effective_status).unwrap();

                Response::success(session_json)
            }
            None => Response::Error {
                code: "E002".to_string(),
                message: format!("Session not found: {}", session_id),
            },
        }
    }

    async fn handle_inject(&self, session_id: String, message: String) -> Response {
        let sessions = self.sessions.read().await;

        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => {
                return Response::Error {
                    code: "E002".to_string(),
                    message: format!("Session not found: {}", session_id),
                };
            }
        };

        // Check if session is stopped
        if !crate::tmux::TmuxManager::session_exists(&session.tmux_session) {
            return Response::Error {
                code: "E003".to_string(),
                message: format!("Session is stopped: {}", session_id),
            };
        }

        // Send keys to tmux session
        match crate::tmux::TmuxManager::send_keys(&session.tmux_session, &message, true) {
            Ok(_) => Response::success(json!({
                "session_id": session_id,
                "injected": true,
                "message_length": message.len()
            })),
            Err(e) => Response::Error {
                code: "E006".to_string(),
                message: format!("Failed to inject message: {}", e),
            },
        }
    }

    async fn handle_daemon_status(&self) -> Response {
        let sessions = self.sessions.read().await;
        let running_count = sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Running | SessionStatus::Idle))
            .count();

        Response::success(json!({
            "running": true,
            "session_count": running_count,
            "version": env!("CARGO_PKG_VERSION")
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler_list_empty() {
        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = Arc::new(DaemonConfig::default());
        let handler = Handler::new(sessions, config);

        let response = handler.handle(Request::List { status_filter: None }).await;
        match response {
            Response::Success { data } => {
                let sessions_array = data["sessions"].as_array().unwrap();
                assert!(sessions_array.is_empty());
            }
            _ => panic!("Expected success response"),
        }
    }
}
```

**Step 2: Run handler tests**

Run: `cargo test -p summ-daemon handler`
Expected: PASS

**Step 3: Update main.rs**

```rust
mod config;
mod handler;
mod init;
mod ipc;
mod recovery;
mod session;
mod tmux;
```

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement request handler for all operations"
```

---

### Task 3.3: Implement Unix Socket Server

**Files:**
- Create: `crates/summ-daemon/src/server.rs`
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Write Unix socket server**

```rust
// summ-daemon/src/server.rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use summ_common::Session;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::handler::Handler;
use crate::ipc::{read_request, write_response};
use crate::recovery;

/// Request timeout in seconds
const REQUEST_TIMEOUT: u64 = 30;

pub struct Daemon {
    config: summ_common::DaemonConfig,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl Daemon {
    pub fn new(config: summ_common::DaemonConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the daemon server
    pub async fn run(self) -> Result<()> {
        // Recover existing sessions
        let sessions = recovery::recover_sessions(&self.config)?;
        let session_count = sessions.len();
        *self.sessions.write().await = sessions;

        info!("Recovered {} sessions", session_count);

        // Remove existing socket if present
        if self.config.socket_path.exists() {
            std::fs::remove_file(&self.config.socket_path)
                .context("Failed to remove existing socket")?;
        }

        // Create socket directory if needed
        if let Some(parent) = self.config.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Bind to socket
        let listener = UnixListener::bind(&self.config.socket_path)
            .context("Failed to bind to socket")?;

        // Set socket permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.config.socket_path)?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.config.socket_path, perms)?;
        }

        info!("Listening on {:?}", self.config.socket_path);

        // Create handler
        let handler = Handler::new(self.sessions.clone(), Arc::new(self.config.clone()));

        // Start monitoring task
        self.start_monitoring();

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler_clone = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, handler_clone).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    fn start_monitoring(&self) {
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                let mut sessions = sessions.write().await;
                for (id, session) in sessions.iter_mut() {
                    let new_status = session.get_effective_status();

                    if new_status != session.status {
                        info!(
                            "Session {} status changed: {:?} -> {:?}",
                            id, session.status, new_status
                        );
                        session.status = new_status;
                        let _ = session.save_metadata();
                    }

                    if session.status != summ_common::SessionStatus::Stopped {
                        session.last_activity = chrono::Utc::now();
                    }
                }
            }
        });
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    handler: Handler,
) -> Result<()> {
    // Set read timeout
    stream.set_read_timeout(Some(Duration::from_secs(REQUEST_TIMEOUT)))?;

    // Read request
    let request = tokio::time::timeout(
        Duration::from_secs(REQUEST_TIMEOUT),
        read_request(&mut stream),
    )
    .await
    .context("Request read timeout")?
    .context("Failed to read request")?;

    // Handle request
    let response = handler.handle(request).await;

    // Write response
    write_response(&mut stream, &response).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_creation() {
        let config = DaemonConfig::default();
        let daemon = Daemon::new(config);
        assert!(daemon.sessions.read().await.is_empty());
    }
}
```

**Step 2: Update main.rs to run server**

```rust
mod config;
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    // Check tmux availability
    if let Err(e) = tmux::TmuxManager::check_available() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Load configuration
    let config = summ_common::DaemonConfig::load()?;
    tracing::info!("SUMM Daemon starting...");
    tracing::info!("Sessions: {:?}", config.sessions_dir);
    tracing::info!("Logs: {:?}", config.logs_dir);

    // Create and run daemon
    let daemon = server::Daemon::new(config);
    daemon.run().await?;

    Ok(())
}
```

**Step 3: Build and test daemon**

Run: `cargo build -p summ-daemon`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement Unix socket server with connection handling"
```

---

## Phase 4: CLI Client

### Task 4.1: Implement CLI Structure with clap

**Files:**
- Modify: `crates/summ-cli/src/main.rs`
- Create: `crates/summ-cli/src/commands/mod.rs`

**Step 1: Write CLI main with clap derive**

```rust
// summ-cli/src/main.rs
mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;

#[derive(Parser)]
#[command(name = "summ")]
#[command(about = "SUMM Daemon CLI - Process management for multi-agent collaboration", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new session
    Start {
        /// CLI command to run (e.g., "claude-code", "aider")
        #[arg(long)]
        cli: String,

        /// Initialization source (directory, .zip, or .tar.gz)
        #[arg(long)]
        init: String,

        /// Optional session name
        #[arg(long)]
        name: Option<String>,
    },

    /// Stop a session
    Stop {
        /// Session ID
        session_id: String,
    },

    /// List all sessions
    List {
        /// Filter by status (running, idle, stopped)
        #[arg(long)]
        status: Option<String>,
    },

    /// Get session status
    Status {
        /// Session ID
        session_id: String,
    },

    /// Attach to a session terminal
    Attach {
        /// Session ID
        session_id: String,
    },

    /// Inject a message into a session
    Inject {
        /// Session ID
        session_id: String,

        /// Message to inject
        #[arg(long)]
        message: Option<String>,

        /// Read message from file
        #[arg(long)]
        file: Option<String>,
    },

    /// Daemon management
    Daemon {
        #[command(subcommand)]
        daemon_command: DaemonCommands,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Start the daemon
    Start,

    /// Stop the daemon
    Stop,

    /// Check daemon status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { cli, init, name } => {
            cmd_start(&cli, &init, name).await
        }
        Commands::Stop { session_id } => {
            cmd_stop(&session_id).await
        }
        Commands::List { status } => {
            cmd_list(status.as_deref()).await
        }
        Commands::Status { session_id } => {
            cmd_status(&session_id).await
        }
        Commands::Attach { session_id } => {
            cmd_attach(&session_id).await
        }
        Commands::Inject { session_id, message, file } => {
            cmd_inject(&session_id, message.as_deref(), file.as_deref()).await
        }
        Commands::Daemon { daemon_command } => {
            match daemon_command {
                DaemonCommands::Start => cmd_daemon_start().await,
                DaemonCommands::Stop => cmd_daemon_stop().await,
                DaemonCommands::Status => cmd_daemon_status().await,
            }
        }
    }
}
```

**Step 2: Build to check compilation**

Run: `cargo build -p summ-cli`
Expected: May have errors for missing commands (expected)

**Step 3: Commit**

```bash
git add crates/summ-cli/
git commit -m "feat(summ-cli): define CLI structure with clap derive macros"
```

---

### Task 4.2: Implement IPC Client for CLI

**Files:**
- Create: `crates/summ-cli/src/client.rs`
- Create: `crates/summ-cli/src/commands/mod.rs`

**Step 1: Write IPC client module**

```rust
// summ-cli/src/client.rs
use anyhow::{Context, Result};
use serde_json;
use summ_common::{Request, Response};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use dirs::home_dir;

/// Get the default socket path
pub fn socket_path() -> PathBuf {
    home_dir()
        .map(|h| h.join(".summ-daemon/daemon.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/.summ-daemon.sock"))
}

/// Send a request to the daemon and get a response
pub async fn send_request(request: &Request) -> Result<Response> {
    let socket_addr = socket_path();

    let mut stream = UnixStream::connect(&socket_addr)
        .await
        .with_context(|| {
            format!(
                "Failed to connect to daemon. Is it running?\nSocket: {:?}",
                socket_addr
            )
        })?;

    // Serialize request
    let json = serde_json::to_vec(request)
        .context("Failed to serialize request")?;

    // Write length prefix
    stream.write_all(&(json.len() as u32).to_be_bytes()).await
        .context("Failed to write request length")?;

    // Write JSON payload
    stream.write_all(&json).await
        .context("Failed to write request payload")?;

    stream.flush().await
        .context("Failed to flush request")?;

    // Read response length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await
        .context("Failed to read response length")?;

    let len = u32::from_be_bytes(len_buf) as usize;

    // Read response payload
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await
        .context("Failed to read response payload")?;

    // Parse response
    let response: Response = serde_json::from_slice(&buf)
        .context("Failed to parse response JSON")?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.ends_with("daemon.sock"));
    }

    #[test]
    fn test_request_serialization() {
        let req = Request::List {
            status_filter: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("List"));
    }
}
```

**Step 2: Write commands module placeholder**

```rust
// summ-cli/src/commands/mod.rs

use anyhow::Result;

pub async fn cmd_start(_cli: &str, _init: &str, _name: Option<String>) -> Result<()> {
    println!("Start command - not yet implemented");
    Ok(())
}

pub async fn cmd_stop(_session_id: &str) -> Result<()> {
    println!("Stop command - not yet implemented");
    Ok(())
}

pub async fn cmd_list(_status: Option<&str>) -> Result<()> {
    println!("List command - not yet implemented");
    Ok(())
}

pub async fn cmd_status(_session_id: &str) -> Result<()> {
    println!("Status command - not yet implemented");
    Ok(())
}

pub async fn cmd_attach(_session_id: &str) -> Result<()> {
    println!("Attach command - not yet implemented");
    Ok(())
}

pub async fn cmd_inject(_session_id: &str, _message: Option<&str>, _file: Option<&str>) -> Result<()> {
    println!("Inject command - not yet implemented");
    Ok(())
}

pub async fn cmd_daemon_start() -> Result<()> {
    println!("Daemon start - not yet implemented");
    Ok(())
}

pub async fn cmd_daemon_stop() -> Result<()> {
    println!("Daemon stop - not yet implemented");
    Ok(())
}

pub async fn cmd_daemon_status() -> Result<()> {
    println!("Daemon status - not yet implemented");
    Ok(())
}
```

**Step 3: Update main.rs**

```rust
mod client;
mod commands;

// ... rest of main.rs unchanged
```

**Step 4: Build CLI**

Run: `cargo build -p summ-cli`
Expected: SUCCESS

**Step 5: Test CLI help**

Run: `cargo run -p summ-cli -- --help`
Expected: Shows help message

**Step 6: Commit**

```bash
git add crates/summ-cli/
git commit -m "feat(summ-cli): implement IPC client for daemon communication"
```

---

### Task 4.3: Implement start, stop, list, status Commands

**Files:**
- Modify: `crates/summ-cli/src/commands/mod.rs`

**Step 1: Implement list command**

```rust
// Add to commands/mod.rs

use crate::client::send_request;
use summ_common::{Request, SessionStatus};
use anyhow::Result;

pub async fn cmd_list(status: Option<&str>) -> Result<()> {
    let request = Request::List {
        status_filter: status.map(|s| s.to_string()),
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            if let Some(sessions) = data.get("sessions").and_then(|v| v.as_array()) {
                if sessions.is_empty() {
                    println!("No sessions found.");
                    return Ok(());
                }

                println!("{:<20} {:<20} {:<15} {:<20} {}", "SESSION ID", "NAME", "STATUS", "CLI", "WORKDIR");
                println!("{}", "-".repeat(120));

                for session in sessions {
                    let session_id = session.get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");
                    let name = session.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");
                    let status = session.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let cli = session.get("cli")
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");
                    let workdir = session.get("workdir")
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");

                    let status_color = match status {
                        "running" => "\x1b[31mrunning\x1b[0m",  // red
                        "idle" => "\x1b[32midle\x1b[0m",          // green
                        "stopped" => "\x1b[90mstopped\x1b[0m",    // gray
                        _ => status,
                    };

                    println!("{:<20} {:<20} {:<25} {:<20} {}",
                        session_id, name, status_color, cli, workdir);
                }
            } else {
                println!("No sessions found.");
            }
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 2: Implement status command**

```rust
pub async fn cmd_status(session_id: &str) -> Result<()> {
    let request = Request::Status {
        session_id: session_id.to_string(),
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            println!("Session Details:");
            println!("  ID:        {}", data.get("session_id").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Name:      {}", data.get("name").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Status:    {}", data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"));
            println!("  CLI:       {}", data.get("cli").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Workdir:   {}", data.get("workdir").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  PID:       {}", data.get("pid").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Created:   {}", data.get("created_at").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Activity:  {}", data.get("last_activity").and_then(|v| v.as_str()).unwrap_or("N/A"));
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 3: Implement stop command**

```rust
pub async fn cmd_stop(session_id: &str) -> Result<()> {
    let request = Request::Stop {
        session_id: session_id.to_string(),
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            println!("Session stopped: {}", session_id);
            if let Some(serde_json::Value::Object(map)) = data {
                for (key, value) in map {
                    println!("  {}: {}", key, value);
                }
            }
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 4: Implement start command**

```rust
use std::path::PathBuf;

pub async fn cmd_start(cli: &str, init: &str, name: Option<String>) -> Result<()> {
    // Expand init path
    let init_path = shellexpand::tilde(init);
    let init_path = PathBuf::from(init_path.as_ref());

    let request = Request::Start {
        cli: cli.to_string(),
        init: init_path,
        name,
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            println!("Session created:");
            println!("  ID:        {}", data.get("session_id").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Name:      {}", data.get("name").and_then(|v| v.as_str()).unwrap_or("N/A"));
            println!("  Status:    {}", data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"));
            println!("  Workdir:   {}", data.get("workdir").and_then(|v| v.as_str()).unwrap_or("N/A"));
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 5: Add shellexpand dependency**

```toml
# summ-cli/Cargo.toml

[dependencies]
# ... existing dependencies ...
shellexpand = "3.1"
```

**Step 6: Build and test**

Run: `cargo build -p summ-cli`
Expected: SUCCESS

**Step 7: Commit**

```bash
git add crates/summ-cli/
git commit -m "feat(summ-cli): implement start, stop, list, status commands"
```

---

### Task 4.4: Implement inject and attach Commands

**Files:**
- Modify: `crates/summ-cli/src/commands/mod.rs`

**Step 1: Implement inject command**

```rust
pub async fn cmd_inject(session_id: &str, message: Option<&str>, file: Option<&str>) -> Result<()> {
    let message_content = if let Some(file_path) = file {
        // Read from file
        let expanded = shellexpand::tilde(file_path);
        tokio::fs::read_to_string(expanded.as_ref())
            .await
            .with_context(|| format!("Failed to read file: {}", file_path))?
    } else if let Some(msg) = message {
        msg.to_string()
    } else {
        anyhow::bail!("Either --message or --file must be provided");
    };

    let request = Request::Inject {
        session_id: session_id.to_string(),
        message: message_content,
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            println!("Message injected to session {}", session_id);
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 2: Implement attach command**

```rust
use std::os::unix::process::CommandExt;

pub async fn cmd_attach(session_id: &str) -> Result<()> {
    // First verify session exists via daemon
    let request = Request::Status {
        session_id: session_id.to_string(),
    };

    let response = send_request(&request).await?;

    match response {
        summ_common::Response::Success { data } => {
            let tmux_session = format!("summ-{}", session_id);

            // Use exec to replace current process with tmux attach
            let err = std::process::Command::new("tmux")
                .args(["attach-session", "-t", &tmux_session])
                .exec();

            // exec only returns on failure
            anyhow::bail!("Failed to attach to tmux session: {}", err);
        }
        summ_common::Response::Error { code, message } => {
            eprintln!("Error [{}]: {}", code, message);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Build CLI**

Run: `cargo build -p summ-cli`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/summ-cli/
git commit -m "feat(summ-cli): implement inject and attach commands"
```

---

### Task 4.5: Implement Daemon Management Commands

**Files:**
- Modify: `crates/summ-cli/src/commands/mod.rs`

**Step 1: Implement daemon commands**

```rust
use std::process::Stdio;

pub async fn cmd_daemon_start() -> Result<()> {
    let socket = crate::client::socket_path();

    // Check if daemon is already running
    if socket.exists() {
        // Try to connect
        if let Ok(_) = Request::DaemonStatus;
        let response = send_request(&Request::DaemonStatus).await {
            match response {
                summ_common::Response::Success { .. } => {
                    println!("Daemon is already running");
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    // Start daemon as background process
    println!("Starting SUMM Daemon...");

    let status = std::process::Command::new("summ-daemon")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start daemon. Is summ-daemon installed in PATH?")?;

    println!("Daemon started with PID: {}", status.id());
    println!("Socket: {:?}", socket);

    // Wait a bit and check if it's running
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    match send_request(&Request::DaemonStatus).await {
        Ok(summ_common::Response::Success { .. }) => {
            println!("Daemon is running");
        }
        _ => {
            eprintln!("Warning: Daemon started but not responding");
        }
    }

    Ok(())
}

pub async fn cmd_daemon_stop() -> Result<()> {
    // Find daemon process and terminate
    let output = std::process::Command::new("pgrep")
        .args(["-f", "summ-daemon"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let pid_str = String::from_utf8_lossy(&out.stdout);
                let pid = pid_str.trim().parse::<u32>().unwrap();

                println!("Stopping daemon (PID: {})...", pid);

                std::process::Command::new("kill")
                    .arg(pid.to_string())
                    .output()
                    .context("Failed to send SIGTERM to daemon")?;

                println!("Daemon stopped");
            } else {
                println!("Daemon is not running");
            }
        }
        Err(_) => {
            println!("Could not determine daemon status");
        }
    }

    Ok(())
}

pub async fn cmd_daemon_status() -> Result<()> {
    match send_request(&Request::DaemonStatus).await {
        Ok(response) => {
            match response {
                summ_common::Response::Success { data } => {
                    println!("Daemon Status:");
                    println!("  Running: {}", data.get("running").unwrap_or(&serde_json::Value::Bool(false)));
                    println!("  Sessions: {}", data.get("session_count").unwrap_or(&serde_json::Value::Number(0.into())));
                    println!("  Version: {}", data.get("version").and_then(|v| v.as_str()).unwrap_or("unknown"));
                }
                summ_common::Response::Error { code, message } => {
                    if code == "E007" {
                        println!("Daemon is not running");
                    } else {
                        eprintln!("Error [{}]: {}", code, message);
                    }
                }
            }
        }
        Err(e) => {
            println!("Daemon is not running");
            println!("Error: {}", e);
        }
    }

    Ok(())
}
```

**Step 2: Fix daemon start command (proper implementation)**

```rust
pub async fn cmd_daemon_start() -> Result<()> {
    let socket = crate::client::socket_path();

    // Check if daemon is already running
    match send_request(&Request::DaemonStatus).await {
        Ok(summ_common::Response::Success { .. }) => {
            println!("Daemon is already running");
            return Ok(());
        }
        _ => {}
    }

    // Get the path to summ-daemon binary
    let daemon_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("summ-daemon")))
        .unwrap_or_else(|| PathBuf::from("summ-daemon"));

    println!("Starting SUMM Daemon...");
    println!("Binary: {:?}", daemon_path);

    // Start daemon as background process
    let status = std::process::Command::new(&daemon_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| {
            format!("Failed to start daemon. Try: cargo run -p summ-daemon --bin summ-daemon")
        })?;

    println!("Daemon started with PID: {}", status.id());

    // Wait and verify
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    match send_request(&Request::DaemonStatus).await {
        Ok(summ_common::Response::Success { .. }) => {
            println!("Daemon is running");
        }
        _ => {
            eprintln!("Warning: Daemon started but not yet responding");
        }
    }

    Ok(())
}
```

**Step 3: Build CLI**

Run: `cargo build -p summ-cli`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/summ-cli/
git commit -m "feat(summ-cli): implement daemon management commands (start, stop, status)"
```

---

## Phase 5: Claude Code Hook Integration

### Task 5.1: Create summ-hook Script

**Files:**
- Create: `crates/summ-daemon/scripts/summ-hook.sh`
- Create: `crates/summ-daemon/src/hooks.rs`

**Step 1: Write summ-hook script**

```bash
#!/bin/bash
# summ-hook: Claude Code Hook handler
# Usage: summ-hook <event> [args...]

set -e

EVENT="$1"
RUNTIME_DIR="${SUMM_RUNTIME_DIR:-$PWD/../runtime}"
STATUS_FILE="$RUNTIME_DIR/status.json"

# Read Hook input from stdin (JSON)
INPUT=$(cat)

# Extract session_id from environment or default
SESSION_ID="${SUMM_SESSION_ID:-unknown}"

# Function to write status file
write_status() {
    local state="$1"
    local message="$2"

    mkdir -p "$(dirname "$STATUS_FILE")"
    cat > "$STATUS_FILE" << EOF
{
  "state": "$state",
  "message": "$message",
  "event": "$EVENT",
  "timestamp": "$(date -Iseconds)"
}
EOF
}

case "$EVENT" in
    session-start)
        write_status "idle" "Session started, ready for tasks"
        ;;

    stop)
        write_status "idle" "Task completed"
        ;;

    subagent-stop)
        write_status "idle" "Subagent task completed"
        ;;

    session-end)
        REASON=$(echo "$INPUT" | jq -r '.reason // "unknown"')
        write_status "stopped" "Session ended: $REASON"
        ;;

    *)
        echo "Unknown event: $EVENT" >&2
        exit 1
        ;;
esac

exit 0
```

**Step 2: Write hooks module**

```rust
// summ-daemon/src/hooks.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Deploy Claude Code hook configuration
pub fn deploy_claude_code_hooks(
    workspace_dir: &Path,
    session_id: &str,
    runtime_dir: &Path,
) -> Result<()> {
    let claude_dir = workspace_dir.join(".claude");
    fs::create_dir_all(&claude_dir)
        .context("Failed to create .claude directory")?;

    let hook_command = format!(
        "SUMM_SESSION_ID={} SUMM_RUNTIME_DIR={} ~/.summ-daemon/bin/summ-hook",
        session_id,
        runtime_dir.display()
    );

    let settings = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} session-start", hook_command)
                }]
            }],
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} stop", hook_command)
                }]
            }],
            "SubagentStop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} subagent-stop", hook_command)
                }]
            }],
            "SessionEnd": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} session-end", hook_command)
                }]
            }]
        }
    });

    let settings_path = claude_dir.join("settings.local.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings)?
    ).context("Failed to write Claude Code settings")?;

    tracing::info!("Deployed Claude Code hooks to {:?}", settings_path);

    Ok(())
}

/// Deploy hooks for the specified CLI type
pub fn deploy_cli_hooks(
    workspace_dir: &Path,
    cli: &str,
    session_id: &str,
    runtime_dir: &Path,
) -> Result<()> {
    if cli.contains("claude") {
        deploy_claude_code_hooks(workspace_dir, session_id, runtime_dir)?;
    } else {
        tracing::info!(
            "CLI '{}' does not support hooks, status detection limited",
            cli
        );
    }

    Ok(())
}

/// Install the summ-hook script to ~/.summ-daemon/bin/
pub fn install_hook_script(base_dir: &Path) -> Result<()> {
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .context("Failed to create bin directory")?;

    let hook_script = bin_dir.join("summ-hook");

    // Include the script from the scripts directory
    let script_content = include_str!("../scripts/summ-hook.sh");
    fs::write(&hook_script, script_content)
        .context("Failed to write hook script")?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_script)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_script, perms)?;
    }

    tracing::info!("Installed hook script to {:?}", hook_script);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_deploy_claude_code_hooks() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let runtime = temp.path().join("runtime");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&runtime).unwrap();

        let result = deploy_claude_code_hooks(&workspace, "test001", &runtime);
        assert!(result.is_ok());

        let settings_path = workspace.join(".claude/settings.local.json");
        assert!(settings_path.exists());

        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("SessionStart"));
        assert!(content.contains("summ-hook"));
    }

    #[test]
    fn test_install_hook_script() {
        let temp = TempDir::new().unwrap();
        let result = install_hook_script(temp.path());
        assert!(result.is_ok());

        let hook_script = temp.path().join("bin/summ-hook");
        assert!(hook_script.exists());

        #[cfg(unix)]
        {
            let perms = fs::metadata(&hook_script).unwrap();
            assert_eq!(perms.permissions().mode() & 0o755, 0o755);
        }
    }
}
```

**Step 3: Create scripts directory and add hook script**

```bash
mkdir -p crates/summ-daemon/scripts
```

**Step 4: Update session creation to use hooks**

```rust
// Modify summ-daemon/src/session.rs

use crate::hooks;

// In Session::create method, after creating workspace directory:

// Deploy hooks if CLI is Claude Code
hooks::deploy_cli_hooks(&workspace_dir, cli, &session_id, &runtime_dir)?;
```

**Step 5: Update config to install hooks on startup**

```rust
// Modify summ-daemon/src/config.rs

use crate::hooks::install_hook_script;

impl DaemonConfig {
    pub fn load() -> Result<Self> {
        let config = Self::default();
        config.ensure_directories()?;

        // Install hook script
        let _ = install_hook_script(&config.base_dir);

        Ok(config)
    }
}
```

**Step 6: Run tests**

Run: `cargo test -p summ-daemon hooks`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/summ-daemon/
git commit -m "feat(summ-daemon): implement Claude Code hook integration"
```

---

## Phase 6: Systemd Integration

### Task 6.1: Create Systemd Unit File

**Files:**
- Create: `systemd/summ-daemon.service`

**Step 1: Write systemd unit file**

```ini
# systemd user service for SUMM Daemon
# Install to: ~/.config/systemd/user/summ-daemon.service

[Unit]
Description=SUMM Daemon - CLI Process Management Service
After=default.target

[Service]
Type=notify
ExecStart=%h/.cargo/bin/summ-daemon
Restart=on-failure
RestartSec=5s

# Environment
Environment="RUST_LOG=info"

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=summ-daemon

[Install]
WantedBy=default.target
```

**Step 2: Update Cargo.toml to include systemd files in package**

```toml
# summ-daemon/Cargo.toml

[package]
# ... existing ...

[metadata.systemd]
units = ["../../systemd/summ-daemon.service"]
```

**Step 3: Add sd-notify to daemon**

```rust
// summ-daemon/src/main.rs

fn main() -> Result<()> {
    // ... existing setup ...

    // Notify systemd we're ready
    #[cfg(target_os = "linux")]
    {
        if let Ok(_) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
            tracing::info!("Notified systemd of ready state");
        }
    }

    // ... rest of main ...
}
```

**Step 4: Create installation script**

```bash
#!/bin/bash
# scripts/install.sh

set -e

echo "Installing SUMM Daemon..."

# Create systemd directory
mkdir -p ~/.config/systemd/user/

# Copy unit file
cp systemd/summ-daemon.service ~/.config/systemd/user/

# Reload systemd
systemctl --user daemon-reload

echo "Installation complete."
echo ""
echo "To enable and start the daemon:"
echo "  systemctl --user enable summ-daemon"
echo "  systemctl --user start summ-daemon"
echo ""
echo "To check status:"
echo "  systemctl --user status summ-daemon"
```

**Step 5: Create uninstall script**

```bash
#!/bin/bash
# scripts/uninstall.sh

set -e

echo "Stopping SUMM Daemon..."
systemctl --user stop summ-daemon 2>/dev/null || true
systemctl --user disable summ-daemon 2>/dev/null || true

echo "Removing systemd unit..."
rm -f ~/.config/systemd/user/summ-daemon.service
systemctl --user daemon-reload

echo "Uninstallation complete."
```

**Step 6: Commit**

```bash
git add systemd/ scripts/
git commit -m "feat: add systemd unit file and installation scripts"
```

---

## Phase 7: Testing and Documentation

### Task 7.1: Add Integration Tests

**Files:**
- Create: `crates/summ-daemon/tests/integration_tests.rs`

**Step 1: Write integration tests**

```rust
// summ-daemon/tests/integration_tests.rs

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

#[tokio::test]
#[ignore] // Requires tmux
async fn test_full_session_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let init_dir = temp_dir.path().join("init");
    std::fs::create_dir(&init_dir).unwrap();

    // Create a test file in init directory
    std::fs::write(init_dir.join("test.txt"), "test content").unwrap();

    // Note: This test requires a running daemon
    // In a real scenario, you'd start the daemon programmatically

    // For now, this is a placeholder showing the test structure
}

#[tokio::test]
async fn test_protocol_serialization() {
    use summ_common::{Request, Response};

    let req = Request::List {
        status_filter: Some("idle".to_string()),
    };

    let json = serde_json::to_string(&req).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();

    match parsed {
        Request::List { status_filter } => {
            assert_eq!(status_filter, Some("idle".to_string()));
        }
        _ => panic!("Wrong type"),
    }
}

#[tokio::test]
async fn test_error_response_format() {
    let resp = Response::Error {
        code: "E002".to_string(),
        message: "Session not found".to_string(),
    };

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("E002"));
    assert!(json.contains("Session not found"));
}
```

**Step 2: Run tests**

Run: `cargo test --package summ-daemon --test integration_tests`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/summ-daemon/tests/
git commit -m "test(summ-daemon): add integration tests"
```

---

### Task 7.2: Write README and Documentation

**Files:**
- Modify: `README.md`
- Create: `docs/INSTALLATION.md`
- Create: `docs/USAGE.md`

**Step 1: Write comprehensive README**

```markdown
# SUMM Daemon

A CLI process management daemon service for multi-agent collaboration.

## Overview

SUMM Daemon manages multiple CLI processes (called "Sessions") with support for:
- **Process Management**: Start, stop, and query CLI processes
- **Initialization**: Set up Session environments from compressed packages or directories
- **Message Injection**: Send messages to running Sessions for inter-agent communication
- **Interactive Attachment**: Connect to Session terminals for direct interaction

## Features

- **tmux Integration**: Uses tmux for robust process hosting and terminal management
- **Claude Code Hooks**: Native support for Claude Code hook-based status tracking
- **Idle Detection**: Track when Sessions are busy vs idle for efficient task dispatch
- **Session Recovery**: Daemon restarts don't affect running Sessions
- **Multi-client Attach**: Multiple terminals can attach to the same Session

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Install binaries
cargo install --path crates/summ-daemon
cargo install --path crates/summ-cli

# Or use systemd
./scripts/install.sh
```

### Starting the Daemon

```bash
# Start manually
summ-daemon

# Or via systemd
systemctl --user start summ-daemon
```

### Creating a Session

```bash
summ start --cli "claude-code" --init ./my-project --name "dev-session"
```

### Listing Sessions

```bash
summ list
summ list --status idle
```

### Attaching to a Session

```bash
summ attach session_abc123
```

### Stopping a Session

```bash
summ stop session_abc123
```

## Architecture

```
┌─────────────┐     Unix Socket      ┌─────────────┐      tmux commands      ┌─────────────┐
│  summ CLI   │ ◄──────────────────► │ summ-daemon │ ◄─────────────────────► │ tmux server │
└─────────────┘   JSON over socket   └─────────────┘    subprocess calls     └──────┬──────┘
                                                                                    │
                                                                         ┌──────────┼──────────┐
                                                                         │          │          │
                                                                    ┌────▼────┐ ┌───▼────┐ ┌───▼────┐
                                                                    │ CLI 1   │ │ CLI 2  │ │ CLI 3  │
                                                                    │(claude) │ │(aider) │ │(custom)│
                                                                    └─────────┘ └────────┘ └────────┘
```

## Project Structure

```
summ-daemon/
├── crates/
│   ├── summ-daemon/    # Main daemon binary
│   ├── summ-cli/       # CLI client binary
│   └── summ-common/    # Shared types and protocol
├── systemd/            # Systemd unit files
└── docs/               # Documentation
```

## Commands

| Command | Description |
|---------|-------------|
| `summ start` | Start a new Session |
| `summ stop` | Stop a Session |
| `summ list` | List all Sessions |
| `summ status` | Get Session details |
| `summ attach` | Attach to Session terminal |
| `summ inject` | Inject message into Session |
| `summ daemon start` | Start the daemon |
| `summ daemon stop` | Stop the daemon |
| `summ daemon status` | Check daemon status |

## Requirements

- Rust 1.70+
- tmux 3.0+
- Linux/macOS

## License

MIT
```

**Step 2: Write installation guide**

```markdown
# Installation Guide

## Prerequisites

- **tmux 3.0+**: Required for process management
- **Rust 1.70+**: For building from source

### Installing tmux

```bash
# Debian/Ubuntu
sudo apt install tmux

# RHEL/CentOS
sudo yum install tmux

# macOS
brew install tmux

# Verify version
tmux -V
```

## Building from Source

```bash
# Clone repository
git clone https://github.com/your-org/SUMM-Daemon.git
cd SUMM-Daemon

# Build release binaries
cargo build --release

# Install to ~/.cargo/bin
cargo install --path crates/summ-daemon
cargo install --path crates/summ-cli
```

## Systemd Installation

For automatic startup on login:

```bash
# Run installation script
./scripts/install.sh

# Enable and start
systemctl --user enable summ-daemon
systemctl --user start summ-daemon

# Check status
systemctl --user status summ-daemon
```

## Manual Installation

1. Copy binaries to your PATH:
```bash
cp target/release/summ-daemon ~/.local/bin/
cp target/release/summ ~/.local/bin/
```

2. Start the daemon:
```bash
summ-daemon
```

## Configuration

The daemon uses `~/.summ-daemon/` for configuration and data:

```
~/.summ-daemon/
├── config.json              # Global configuration (optional)
├── sessions/                # Session data
│   ├── session_001/         # Per-session directories
│   │   ├── meta.json
│   │   ├── workspace/
│   │   └── runtime/
├── logs/                    # Log files
├── daemon.sock              # IPC socket
└── bin/
    └── summ-hook            # Hook script
```
```

**Step 3: Write usage guide**

```markdown
# Usage Guide

## Session Lifecycle

```
                    summ start
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Generate session_id                                         │
│  2. Create workdir: ~/.summ-daemon/sessions/{session_id}/        │
│  3. Extract/copy --init contents to workspace                    │
│  4. tmux new-session -d -s summ-{session_id}                     │
│  5. Save meta.json                                              │
│  6. Return session info                                         │
└─────────────────────────────────────────────────────────────────┘
                         │
                         ▼
                   ┌──────────┐
                   │ running  │
                   └────┬─────┘
                        │
            ┌───────────┼───────────┐
            ▼           ▼           ▼
       summ inject  summ attach  CLI completes
            │           │           │
            └───────────┴───────────┘
                        │
              (CLI process exits)
                        │
                        ▼
                   ┌──────────┐
                   │ stopped  │
                   └──────────┘
```

## Starting Sessions

### Basic Start

```bash
summ start --cli "claude-code" --init ./my-project
```

### With Custom Name

```bash
summ start --cli "aider" --init ./codebase --name "code-review"
```

### From Archive

```bash
summ start --cli "claude-code" --init ./project.zip
summ start --cli "claude-code" --init ./project.tar.gz
```

## Listing Sessions

```bash
# All sessions
summ list

# Filter by status
summ list --status running
summ list --status idle
summ list --status stopped
```

Output:
```
SESSION ID           NAME                 STATUS               CLI                  WORKDIR
session_abc123       dev-session          running              claude-code           /home/user/.summ-daemon/sessions/session_abc123
session_def456       test-session         idle                 aider                 /home/user/.summ-daemon/sessions/session_def456
```

## Session Status

```bash
summ status session_abc123
```

Output:
```json
Session Details:
  ID:        session_abc123
  Name:      dev-session
  Status:    running
  CLI:       claude-code
  Workdir:   /home/user/.summ-daemon/sessions/session_abc123
  PID:       12345
  Created:   2025-02-01T10:00:00Z
  Activity:  2025-02-01T12:30:00Z
```

## Attaching to Sessions

```bash
summ attach session_abc123
```

- Use `Ctrl+B` then `D` to detach without stopping the session
- Multiple terminals can attach simultaneously

## Injecting Messages

```bash
# Direct message
summ inject session_abc123 --message "Hello from main agent"

# From file
summ inject session_abc123 --file ./task.json
```

## Stopping Sessions

```bash
summ stop session_abc123
```

## Daemon Management

```bash
# Check daemon status
summ daemon status

# Start daemon (if not running)
summ daemon start

# Stop daemon
summ daemon stop
```

## Claude Code Integration

When starting a session with a Claude Code CLI, the daemon automatically:

1. Creates `workspace/` directory for your project
2. Creates `runtime/` directory for status tracking
3. Generates `.claude/settings.local.json` with hooks

The hooks track:
- **SessionStart**: Session is ready
- **Stop**: Task completed, back to idle
- **SubagentStop**: Subagent task completed
- **SessionEnd**: Session terminated

## Error Codes

| Code | Description |
|------|-------------|
| E001 | Initialization resource not found |
| E002 | Session not found |
| E003 | Session stopped, cannot operate |
| E004 | Archive extraction failed |
| E005 | Process start failed |
| E006 | Message injection failed |
| E007 | Daemon not running |
| E008 | Invalid CLI command |
| E009 | tmux not available |
```

**Step 4: Commit**

```bash
git add README.md docs/
git commit -m "docs: add comprehensive README and usage documentation"
```

---

## Phase 8: Final Integration and Polish

### Task 8.1: Fix Unix Target for CLI

**Files:**
- Modify: `crates/summ-cli/Cargo.toml`

**Step 1: Add Unix-specific cfg for attach command**

```toml
# summ-cli/Cargo.toml

[package]
name = "summ-cli"
# ...

[target.'cfg(unix)'.dependencies]
# Unix-specific dependencies
```

**Step 2: Update attach command to handle non-Unix**

```rust
// summ-cli/src/commands/mod.rs

pub async fn cmd_attach(session_id: &str) -> Result<()> {
    #[cfg(unix)]
    {
        // First verify session exists
        let request = Request::Status {
            session_id: session_id.to_string(),
        };

        let response = send_request(&request).await?;

        match response {
            summ_common::Response::Success { .. } => {
                let tmux_session = format!("summ-{}", session_id);

                // Use exec to replace current process
                let err = std::process::Command::new("tmux")
                    .args(["attach-session", "-t", &tmux_session])
                    .exec();

                anyhow::bail!("Failed to attach: {}", err);
            }
            summ_common::Response::Error { code, message } => {
                eprintln!("Error [{}]: {}", code, message);
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(unix))]
    {
        eprintln!("Error: attach command is only supported on Unix systems");
        std::process::exit(1);
    }
}
```

**Step 3: Build and test**

Run: `cargo build --release`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/summ-cli/
git commit -m "fix(summ-cli): add proper cfg handling for attach command"
```

---

### Task 8.2: Add Logging Configuration

**Files:**
- Modify: `crates/summ-daemon/src/main.rs`

**Step 1: Improve logging setup**

```rust
// summ-daemon/src/main.rs

use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let env_filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("summ_daemon=debug".parse().unwrap());

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // ... rest of main
}
```

**Step 2: Commit**

```bash
git add crates/summ-daemon/src/main.rs
git commit -m "feat(summ-daemon): improve logging configuration"
```

---

### Task 8.3: Final Verification

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Build release**

Run: `cargo build --release`
Expected: All binaries build successfully

**Step 3: Verify project structure**

Run: `ls -la crates/`
Expected: Shows summ-common, summ-daemon, summ-cli

**Step 4: Create CHANGELOG**

```markdown
# Changelog

## [0.1.0] - 2025-02-02

### Added
- Initial release of SUMM Daemon
- Session management (start, stop, list, status)
- tmux integration for process hosting
- Message injection capability
- Interactive session attachment
- Claude Code hook integration
- Systemd service support
- Unix socket IPC
- Session recovery on daemon restart
```

**Step 5: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add initial changelog"
```

---

## Summary

This implementation plan breaks down the SUMM-Daemon project into 8 phases with 33 discrete tasks. Each task follows TDD principles with:

1. **Write the test first** - Every module includes unit tests
2. **Run it to verify failure** - Tests fail before implementation
3. **Write minimal implementation** - Just enough to pass the test
4. **Run to verify passing** - Tests pass after implementation
5. **Commit** - Small, focused commits with descriptive messages

### Key Architecture Decisions

- **Three-crate workspace** for separation of concerns
- **tmux for process management** leverages mature terminal multiplexing
- **Unix socket IPC** for fast local communication
- **Claude Code hooks** for idle/busy status tracking
- **Systemd integration** for production deployment

### Estimated Timeline

- Phase 1: 3-5 tasks (infrastructure setup)
- Phase 2: 4 tasks (session management)
- Phase 3: 3 tasks (IPC and server)
- Phase 4: 5 tasks (CLI client)
- Phase 5: 1 task (hooks integration)
- Phase 6: 1 task (systemd)
- Phase 7: 2 tasks (testing/docs)
- Phase 8: 3 tasks (polish)

Total: ~27 tasks, each commit-ready and independently testable.
