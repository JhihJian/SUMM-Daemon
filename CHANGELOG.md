# Changelog

All notable changes to SUMM Daemon will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-02-02

### Added

#### Core Features
- Process management daemon for CLI sessions using tmux
- Unix socket IPC for daemon communication
- Session initialization from directories, .zip, and .tar.gz archives
- Message injection into running sessions
- Session status tracking (running, idle, stopped)
- Session recovery after daemon restart

#### CLI Client
- `summ start` - Create new sessions with initialization
- `summ stop` - Stop running sessions
- `summ list` - List all sessions with optional status filtering
- `summ status` - Query detailed session information
- `summ attach` - Attach to session terminal (Unix only)
- `summ inject` - Inject messages into sessions
- `summ daemon` - Daemon management commands (start/stop/status)

#### Claude Code Integration
- Automatic hook deployment for Claude Code sessions
- `summ-hook` script for status reporting
- Status tracking via SessionStart, Stop, SubagentStop, and SessionEnd hooks
- Idle state detection through hook mechanism

#### System Integration
- systemd user service unit file
- Installation and uninstallation scripts
- sd-notify integration for Type=notify service

#### Configuration
- `~/.summ-daemon/` data directory structure
- Session metadata persistence (meta.json)
- Runtime status files for hook reports

#### Documentation
- Comprehensive README with architecture overview
- Installation guide with platform-specific instructions
- Usage guide with examples and workflows

#### Testing
- Unit tests for all major components
- Integration tests for protocol serialization
- Session lifecycle tests
- Error handling tests

### Technical Details

- **Technology Stack**: Rust, Tokio, tmux, Unix sockets
- **Dependencies**:
  - tokio 1.35 (async runtime)
  - serde/serde_json (serialization)
  - clap 4.4 (CLI parsing)
  - chrono (timestamps)
  - compress-tools (archive extraction)
  - tracing/tracing-subscriber (logging)
  - sd-notify (systemd integration)

### Error Codes

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

### Project Structure

```
SUMM-Daemon/
├── crates/
│   ├── summ-daemon/    # Daemon binary
│   ├── summ-cli/       # CLI client binary
│   └── summ-common/    # Shared types and protocol
├── systemd/            # Systemd unit files
├── scripts/            # Installation scripts
└── docs/               # Documentation
```

[0.1.0]: https://github.com/your-org/SUMM-Daemon/releases/tag/v0.1.0
