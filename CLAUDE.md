# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SUMM Daemon is a CLI process management daemon service designed to manage and coordinate multiple CLI processes (called "Sessions"). It enables multi-agent collaboration by providing process lifecycle management and inter-process message injection capabilities.

**Current Status**: This repository is in the specification phase. The PRD is complete (`docs/desgin/SUMM-Daemon-PRD-v1.0.md`), but no implementation code exists yet.

## Core Architecture

### Three Main Capabilities

1. **Process Management**: Start, stop, and query CLI processes (Sessions)
2. **Initialization Management**: Initialize Session working environments from compressed packages (.zip/.tar.gz) or directories
3. **Message Injection**: Send messages to running Sessions to enable inter-agent communication

### Session Concept

A "Session" is a managed CLI process instance with:
- Unique session_id (auto-generated)
- Optional custom name
- Working directory under `~/.summ-daemon/sessions/<session_id>/`
- Metadata tracking (PID, status, timestamps, initialization source)
- Status: running, idle, or stopped

### Configuration Structure

```
~/.summ-daemon/
├── config.json              # Global daemon configuration
├── sessions/                # Session runtime directories
│   ├── session_001/         # Each session has isolated workdir
│   │   └── meta.json        # Session metadata
│   └── session_002/
└── logs/                    # Daemon and session logs
```

## Planned Command Interface

The daemon will expose these commands:

### Session Management
- `summ start --cli <command> --init <path> [--name <name>]` - Start new Session with initialization
- `summ stop <session-id>` - Stop a Session
- `summ list [--status <status>]` - List all Sessions (optionally filtered by status)
- `summ status <session-id>` - Query detailed Session status
- `summ attach <session-id>` - Connect to Session terminal for interactive use

### Message Injection
- `summ inject <session-id> --message <message>` - Inject message into running Session
- `summ inject <session-id> --file <file-path>` - Inject message from file

### Daemon Control
- `summ daemon start [--port <port>]` - Start the daemon process
- `summ daemon stop` - Stop the daemon
- `summ daemon status` - Check daemon status

## Session Initialization Behavior

When starting a Session with `summ start`:
1. Generate unique Session ID
2. Create directory: `~/.summ-daemon/sessions/<session_id>/`
3. If `--init` is a compressed file: extract contents to session directory
4. If `--init` is a directory: copy contents to session directory
5. Execute the `--cli` command in the session directory
6. Record metadata in `meta.json`

## Error Codes

The PRD defines these error codes:
- E001: Initialization resource not found or inaccessible
- E002: Session not found
- E003: Session stopped, cannot operate
- E004: Archive extraction failed
- E005: Process start failed
- E006: Message injection failed
- E007: Daemon not running
- E008: Invalid or non-existent CLI command

## Key Design Considerations

### Multi-Agent Coordination
The message injection feature (`summ inject`) is designed to enable a main SUMM agent to coordinate with multiple sub-Sessions by:
- Notifying Sessions of task changes
- Passing output results between Sessions
- Sending system-level notifications or commands

### Process Monitoring
The daemon should:
- Monitor all child CLI process states
- Detect abnormal process exits and update Session status
- Periodically clean up terminated Session data (configurable retention)

### Output Format
All commands return JSON output for programmatic consumption.

## Documentation

- Primary specification: `docs/desgin/SUMM-Daemon-PRD-v1.0.md` (Chinese)
- The PRD is version 1.0, dated February 2025
- System architecture section (Section 2) is marked as "待补充" (to be completed)
