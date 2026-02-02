# SUMM Daemon

SUMM Daemon is a CLI process management daemon service designed to manage and coordinate multiple CLI processes (called "Sessions"). It enables multi-agent collaboration by providing process lifecycle management and inter-process message injection capabilities.

## Features

- **Process Management**: Start, stop, and query CLI processes (Sessions)
- **Initialization Management**: Initialize Session working environments from compressed packages (.zip/.tar.gz) or directories
- **Message Injection**: Send messages to running Sessions to enable inter-agent communication
- **Claude Code Hook Integration**: Automatic status reporting via Claude Code hooks
- **tmux Integration**: Sessions run in tmux for persistent, attachable terminals
- **Unix Socket IPC**: Fast local communication between CLI and daemon

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

## Requirements

- **tmux** 3.0 or later (required for session management)
- **Rust** 1.70 or later (for building from source)

### Installing tmux

```bash
# Debian/Ubuntu
apt install tmux

# RHEL/CentOS
yum install tmux

# macOS
brew install tmux
```

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/SUMM-Daemon.git
cd SUMM-Daemon

# Build release binaries
cargo build --release

# Install binaries
sudo install -m 755 target/release/summ-daemon ~/.cargo/bin/
sudo install -m 755 target/release/summ ~/.cargo/bin/
```

### Systemd Installation (Linux)

```bash
# Run the installation script
./scripts/install.sh

# Start the daemon
systemctl --user start summ-daemon

# Enable on login
systemctl --user enable summ-daemon

# Check status
systemctl --user status summ-daemon
```

## Usage

### Starting a Session

```bash
# Start a new session with a CLI command
summ start --cli "claude" --init ./my-project.zip

# Start with a directory as init source
summ start --cli "claude" --init ~/projects/my-project

# Start with a custom name
summ start --cli "claude" --init ./project.zip --name "my-session"
```

### Managing Sessions

```bash
# List all sessions
summ list

# List only running sessions
summ list --status running

# List only idle sessions
summ list --status idle

# Get detailed session status
summ status session_abc123
```

### Attaching to a Session

```bash
# Attach to a running session (Unix only)
summ attach session_abc123

# Detach: Ctrl+B, then D
```

### Injecting Messages

```bash
# Inject a text message
summ inject session_abc123 --message "Hello from the CLI"

# Inject from a file
summ inject session_abc123 --file ./message.txt
```

### Daemon Management

```bash
# Start the daemon manually
summ daemon start

# Stop the daemon
summ daemon stop

# Check daemon status
summ daemon status
```

## Configuration

SUMM Daemon stores data in `~/.summ-daemon/`:

```
~/.summ-daemon/
├── config.json              # Global daemon configuration
├── daemon.sock              # Unix socket for IPC
├── bin/
│   └── summ-hook            # Hook script for Claude Code
├── sessions/                # Session runtime directories
│   ├── session_001/
│   │   ├── meta.json        # Session metadata
│   │   ├── runtime/         # Hook status files
│   │   └── workspace/       # Actual working directory
│   └── session_002/
└── logs/                    # Daemon and session logs
```

## Claude Code Integration

When starting a session with a Claude Code CLI, SUMM Daemon automatically:

1. Creates `.claude/settings.local.json` in the workspace
2. Installs the `summ-hook` script
3. Configures hooks for:
   - `SessionStart`: Reports idle status
   - `Stop`: Reports idle after task completion
   - `SubagentStop`: Reports subagent completion
   - `SessionEnd`: Reports stopped status

This enables the daemon to track the actual state of Claude Code sessions.

## Error Codes

| Code | Description |
|------|-------------|
| E001 | Initialization resource not found or inaccessible |
| E002 | Session not found |
| E003 | Session stopped, cannot operate |
| E004 | Archive extraction failed |
| E005 | Process start failed |
| E006 | Message injection failed |
| E007 | Daemon not running |
| E008 | Invalid or non-existent CLI command |
| E009 | tmux not available |

## Development

### Building

```bash
cargo build --workspace
```

### Testing

```bash
cargo test --workspace
```

### Project Structure

```
SUMM-Daemon/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── summ-daemon/          # Main daemon binary
│   │   ├── src/
│   │   │   ├── main.rs       # Daemon entry point
│   │   │   ├── server.rs     # Core daemon logic
│   │   │   ├── session.rs    # Session management
│   │   │   ├── tmux.rs       # tmux command abstraction
│   │   │   ├── ipc.rs        # Unix socket server
│   │   │   ├── handler.rs    # Request handlers
│   │   │   ├── init.rs       # Initialization logic
│   │   │   ├── recovery.rs   # Session recovery
│   │   │   └── hooks.rs      # Claude Code hook integration
│   │   └── Cargo.toml
│   ├── summ-cli/             # CLI client binary
│   │   ├── src/
│   │   │   ├── main.rs       # CLI entry point
│   │   │   ├── client.rs     # IPC client
│   │   │   └── commands/     # Command implementations
│   │   └── Cargo.toml
│   └── summ-common/          # Shared types and protocol
│       ├── src/
│       │   ├── lib.rs
│       │   ├── protocol.rs   # IPC message types
│       │   ├── types.rs      # Session metadata types
│       │   └── error.rs      # Error types
│       └── Cargo.toml
├── systemd/
│   └── summ-daemon.service   # Systemd unit file
├── scripts/
│   ├── install.sh            # Installation script
│   └── uninstall.sh          # Uninstallation script
└── README.md
```

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Authors

SUMM Team
