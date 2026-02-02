// summ-daemon/src/hooks.rs
// Claude Code Hook integration for status reporting
use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// The summ-hook script content
pub const SUMM_HOOK_SCRIPT: &str = r#"#!/bin/bash
# summ-hook: Claude Code Hook handler script
# Usage: summ-hook <event> [args...]

set -e

EVENT="$1"
shift
RUNTIME_DIR="${SUMM_RUNTIME_DIR:-$PWD/../runtime}"
STATUS_FILE="$RUNTIME_DIR/status.json"

# From stdin read Hook input (JSON)
INPUT=$(cat)

# Extract session_id (from environment variable or input)
SESSION_ID="${SUMM_SESSION_ID:-unknown}"

# Ensure runtime directory exists
mkdir -p "$(dirname "$STATUS_FILE")"

write_status() {
    local state="$1"
    local message="$2"

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
        # Claude main agent completed response
        write_status "idle" "Task completed"
        ;;

    subagent-stop)
        # Subagent completed
        write_status "idle" "Subagent task completed"
        ;;

    session-end)
        # Session ended
        REASON=$(echo "$INPUT" | jq -r '.reason // "unknown"' 2>/dev/null || echo "unknown")
        write_status "stopped" "Session ended: $REASON"
        ;;

    *)
        echo "Unknown event: $EVENT" >&2
        exit 1
        ;;
esac

exit 0
"#;

/// Deploy Claude Code hooks to the workspace directory
pub fn deploy_claude_code_hooks(
    workspace_dir: &Path,
    session_id: &str,
    runtime_dir: &Path,
) -> Result<()> {
    let claude_dir = workspace_dir.join(".claude");
    fs::create_dir_all(&claude_dir)
        .context(format!("Failed to create .claude directory: {}", claude_dir.display()))?;

    // Build the hook command with environment variables
    let hook_base = format!(
        "SUMM_SESSION_ID={} SUMM_RUNTIME_DIR={} ~/.summ-daemon/bin/summ-hook",
        session_id,
        runtime_dir.display()
    );

    // Create the hooks configuration
    let hooks_config = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} session-start", hook_base)
                }]
            }],
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} stop", hook_base)
                }]
            }],
            "SubagentStop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} subagent-stop", hook_base)
                }]
            }],
            "SessionEnd": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{} session-end", hook_base)
                }]
            }]
        }
    });

    let settings_path = claude_dir.join("settings.local.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&hooks_config)?.as_bytes(),
    )
    .context(format!(
        "Failed to write Claude Code hooks: {}",
        settings_path.display()
    ))?;

    tracing::info!("Deployed Claude Code hooks to {}", settings_path.display());

    Ok(())
}

/// Deploy hooks for other CLI tools (placeholder for future expansion)
pub fn deploy_cli_hooks(
    workspace_dir: &Path,
    cli: &str,
    session_id: &str,
    runtime_dir: &Path,
) -> Result<()> {
    if cli.contains("claude") {
        deploy_claude_code_hooks(workspace_dir, session_id, runtime_dir)?;
    } else if cli.contains("aider") {
        // aider doesn't currently support hooks, log info
        tracing::info!(
            "CLI '{}' does not support hooks, status detection will be limited",
            cli
        );
    }
    // Extend here for other CLI tools in the future
    Ok(())
}

/// Install the summ-hook script to ~/.summ-daemon/bin/
pub fn install_hook_script(base_dir: &Path) -> Result<()> {
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .context(format!("Failed to create bin directory: {}", bin_dir.display()))?;

    let hook_script = bin_dir.join("summ-hook");
    fs::write(&hook_script, SUMM_HOOK_SCRIPT).context(format!(
        "Failed to write hook script: {}",
        hook_script.display()
    ))?;

    // Make the script executable
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&hook_script)
            .context(format!("Failed to get metadata: {}", hook_script.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_script, perms)
            .context(format!("Failed to set permissions: {}", hook_script.display()))?;
    }

    tracing::info!("Installed summ-hook script to {}", hook_script.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_summ_hook_script_content() {
        // Verify the script contains expected content
        assert!(SUMM_HOOK_SCRIPT.contains("#!/bin/bash"));
        assert!(SUMM_HOOK_SCRIPT.contains("session-start"));
        assert!(SUMM_HOOK_SCRIPT.contains("stop"));
        assert!(SUMM_HOOK_SCRIPT.contains("subagent-stop"));
        assert!(SUMM_HOOK_SCRIPT.contains("session-end"));
        assert!(SUMM_HOOK_SCRIPT.contains("write_status"));
    }

    #[test]
    fn test_install_hook_script() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let result = install_hook_script(base_dir);
        assert!(result.is_ok());

        let hook_script = base_dir.join("bin/summ-hook");
        assert!(hook_script.exists());
        assert!(hook_script.is_file());

        let content = fs::read_to_string(&hook_script).unwrap();
        assert!(content.contains("#!/bin/bash"));
    }

    #[test]
    fn test_deploy_claude_code_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let runtime_dir = temp_dir.path().join("runtime");
        fs::create_dir_all(&workspace_dir).unwrap();
        fs::create_dir_all(&runtime_dir).unwrap();

        let result = deploy_claude_code_hooks(&workspace_dir, "test_session", &runtime_dir);
        assert!(result.is_ok());

        let settings_path = workspace_dir.join(".claude/settings.local.json");
        assert!(settings_path.exists());

        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("SessionStart"));
        assert!(content.contains("Stop"));
        assert!(content.contains("SubagentStop"));
        assert!(content.contains("SessionEnd"));
        assert!(content.contains("summ-hook"));
    }

    #[test]
    fn test_deploy_cli_hooks_claude() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let runtime_dir = temp_dir.path().join("runtime");
        fs::create_dir_all(&workspace_dir).unwrap();
        fs::create_dir_all(&runtime_dir).unwrap();

        let result = deploy_cli_hooks(&workspace_dir, "claude-code", "test_session", &runtime_dir);
        assert!(result.is_ok());

        let settings_path = workspace_dir.join(".claude/settings.local.json");
        assert!(settings_path.exists());
    }

    #[test]
    fn test_deploy_cli_hooks_aider() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let runtime_dir = temp_dir.path().join("runtime");
        fs::create_dir_all(&workspace_dir).unwrap();
        fs::create_dir_all(&runtime_dir).unwrap();

        // Should not error, just log info
        let result = deploy_cli_hooks(&workspace_dir, "aider-chat", "test_session", &runtime_dir);
        assert!(result.is_ok());

        // No hooks should be created for aider
        let settings_path = workspace_dir.join(".claude/settings.local.json");
        assert!(!settings_path.exists());
    }
}
