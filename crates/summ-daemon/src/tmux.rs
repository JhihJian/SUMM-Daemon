// summ-daemon/src/tmux.rs
use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;

const MIN_TMUX_VERSION: (u32, u32) = (3, 0);
const SUMM_SESSION_PREFIX: &str = "summ-";

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

    pub fn create_session(session_name: &str, workdir: &Path, command: &str) -> Result<()> {
        let workdir_str = workdir.to_str().context("Workdir contains invalid UTF-8")?;
        let status = Command::new("tmux")
            .args(["new-session", "-d", "-s", session_name, "-c", workdir_str, command])
            .status()
            .context("Failed to create tmux session")?;
        if !status.success() {
            anyhow::bail!("tmux new-session failed with status: {}", status);
        }
        Ok(())
    }

    pub fn session_exists(session_name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn get_pane_pid(session_name: &str) -> Result<Option<u32>> {
        let output = Command::new("tmux")
            .args(["list-panes", "-t", session_name, "-F", "#{pane_pid}"])
            .output()
            .context("Failed to get pane PID")?;
        if !output.status.success() {
            return Ok(None);
        }
        let pid_str = String::from_utf8_lossy(&output.stdout);
        Ok(pid_str.trim().parse().ok())
    }

    pub fn send_keys(session_name: &str, message: &str, press_enter: bool) -> Result<()> {
        let mut args = vec!["send-keys", "-t", session_name, message];
        if press_enter {
            args.push("Enter");
        }
        let status = Command::new("tmux").args(&args).status()
            .context("Failed to send keys to tmux session")?;
        if !status.success() {
            anyhow::bail!("tmux send-keys failed");
        }
        Ok(())
    }

    pub fn kill_session(session_name: &str) -> Result<()> {
        let status = Command::new("tmux").args(["kill-session", "-t", session_name]).status()
            .context("Failed to kill tmux session")?;
        if !status.success() {
            anyhow::bail!("tmux kill-session failed");
        }
        Ok(())
    }

    pub fn list_summ_sessions() -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()
            .context("Failed to list tmux sessions")?;
        if !output.status.success() {
            return Ok(vec![]);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines().filter(|n| n.starts_with(SUMM_SESSION_PREFIX)).map(|s| s.to_string()).collect())
    }

    pub fn enable_logging(session_name: &str, log_path: &Path) -> Result<()> {
        let log_path_str = log_path.to_str().context("Log path contains invalid UTF-8")?;
        let status = Command::new("tmux")
            .args(["pipe-pane", "-t", session_name, &format!("cat >> {}", log_path_str)])
            .status()
            .context("Failed to enable logging for session")?;
        if !status.success() {
            anyhow::bail!("tmux pipe-pane failed");
        }
        Ok(())
    }

    pub fn capture_pane(session_name: &str, lines: i32) -> Result<String> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", session_name, "-p", "-S", &(-lines).to_string()])
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
        // Versions below 3.0 should fail
        let result = TmuxManager::parse_version("tmux 2.9");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("below minimum required"));
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
