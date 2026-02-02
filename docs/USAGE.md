# SUMM Daemon Usage Guide

This guide covers how to use SUMM Daemon to manage CLI sessions.

## Starting the Daemon

### Manual Start

```bash
summ daemon start
```

### Systemd Start (Linux)

```bash
systemctl --user start summ-daemon
```

## Session Management

### Creating a Session

Use `summ start` to create a new session:

```bash
# Basic usage
summ start --cli "claude" --init ./my-project.zip

# With a directory as initialization source
summ start --cli "claude" --init ~/projects/my-codebase

# With a custom name
summ start --cli "claude" --init ./project.zip --name "frontend-work"
```

**Arguments:**
- `--cli <command>`: The CLI command to run (e.g., "claude", "aider-chat")
- `--init <path>`: Path to initialization source (directory, .zip, or .tar.gz)
- `--name <name>`: Optional custom name for the session

### Listing Sessions

```bash
# List all sessions
summ list

# List only running sessions
summ list --status running

# List only idle sessions
summ list --status idle

# List only stopped sessions
summ list --status stopped
```

**Status Types:**
- `running`: The CLI is actively processing a task
- `idle`: The CLI is waiting for input (Claude Code only)
- `stopped`: The session has terminated

### Querying Session Status

```bash
summ status <session_id>
```

Example output:
```json
{
  "session_id": "session_abc123",
  "name": "my-session",
  "cli": "claude",
  "status": "idle",
  "pid": 12345,
  "created_at": "2025-02-01T10:00:00Z",
  "last_activity": "2025-02-01T10:30:00Z"
}
```

### Stopping a Session

```bash
summ stop <session_id>
```

## Interacting with Sessions

### Attaching to a Session

Use `summ attach` to connect to a running session's terminal:

```bash
summ attach session_abc123
```

**Keybindings (tmux):**
- `Ctrl+B, D`: Detach from session
- `Ctrl+B, [` : Enter scroll/copy mode
- `Ctrl+B, ?`: List all keybindings

**Note:** Attach is Unix-only and requires tmux.

### Injecting Messages

Send messages to a running session:

```bash
# Send a text message
summ inject session_abc123 --message "Process this file"

# Send from a file
summ inject session_abc123 --file ./instructions.txt

# Send a multiline message
summ inject session_abc123 --message "$(cat <<EOF
First task
Second task
Third task
EOF
)"
```

## Daemon Management

### Check Daemon Status

```bash
summ daemon status
```

### Stop the Daemon

```bash
summ daemon stop
```

**Note:** Stopping the daemon does NOT terminate running sessions. Sessions continue running in tmux.

### Restart the Daemon

```bash
# Stop and start
summ daemon stop
summ daemon start
```

## Common Workflows

### Multi-Agent Development

```bash
# Start a main agent session
summ start --cli "claude" --init ./main-project --name "main-agent"

# Start a sub-agent session for testing
summ start --cli "claude" --init ./test-suite --name "test-agent"

# List sessions
summ list

# Attach to the main agent
summ attach main-agent
# (Ctrl+B, D to detach)

# Send a message to the test agent
summ inject test-agent --message "Run the test suite"
```

### Project Initialization from Archive

```bash
# Prepare your project
cd my-project
zip -r ../my-project.zip .

# Start a session with the archive
summ start --cli "claude" --init ../my-project.zip --name "my-project"

# Attach and start working
summ attach my-project
```

## Troubleshooting

### Daemon Won't Start

```bash
# Check if tmux is installed
tmux -V

# Check for existing socket
ls -la ~/.summ-daemon/daemon.sock

# Remove stale socket if daemon crashed
rm ~/.summ-daemon/daemon.sock
summ daemon start
```

### Session Shows as Stopped

```bash
# Check tmux sessions
tmux list-sessions

# If tmux session still exists, daemon will recover on restart
summ daemon stop
summ daemon start
```

### Can't Attach to Session

```bash
# Verify session is running
summ status <session_id>

# Check tmux session name (should be summ-<session_id>)
tmux list-sessions | grep summ

# Attach directly via tmux (if attach command fails)
tmux attach-session -t summ-<session_id>
```
