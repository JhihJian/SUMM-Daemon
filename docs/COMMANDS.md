# SUMM Daemon Commands

This reference summarizes the CLI commands provided by `summ`.

## Daemon Management

```bash
summ daemon start
summ daemon stop
summ daemon status
```

Notes:
- Stopping the daemon does not terminate running sessions; they continue in tmux.

## Session Lifecycle

### Create a Session

```bash
summ start --cli "<command>" --init <path> [--name "<name>"]
```

Arguments:
- `--cli <command>`: CLI command to run (e.g., `claude`, `aider-chat`).
- `--init <path>`: Initialization source (directory, `.zip`, or `.tar.gz`).
- `--name <name>`: Optional custom session name.

Examples:
```bash
summ start --cli "claude" --init ./my-project.zip
summ start --cli "claude" --init ~/projects/my-codebase --name "frontend-work"
```

### List Sessions

```bash
summ list
summ list --status running
summ list --status idle
summ list --status stopped
```

Status values:
- `running`: CLI is processing a task.
- `idle`: CLI is waiting for input (Claude Code only).
- `stopped`: Session has terminated.

### Query Session Status

```bash
summ status <session_id>
```

### Stop a Session

```bash
summ stop <session_id>
```

## Session Interaction

### Attach to a Session (Unix + tmux)

```bash
summ attach <session_id>
```

tmux key bindings:
- `Ctrl+B, D`: Detach.
- `Ctrl+B, [`: Scroll/copy mode.
- `Ctrl+B, ?`: List key bindings.

### Inject Messages

```bash
summ inject <session_id> --message "<text>"
summ inject <session_id> --file <path>
```

Examples:
```bash
summ inject session_abc123 --message "Process this file"
summ inject session_abc123 --file ./instructions.txt
```

## Common Workflows

### Multi-Agent Setup

```bash
summ start --cli "claude" --init ./main-project --name "main-agent"
summ start --cli "claude" --init ./test-suite --name "test-agent"
summ list
summ attach main-agent
summ inject test-agent --message "Run the test suite"
```

### Initialize from Archive

```bash
cd my-project
zip -r ../my-project.zip .
summ start --cli "claude" --init ../my-project.zip --name "my-project"
summ attach my-project
```

## Error Codes (CLI)

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