# SUMM-Daemon Design Document

**Date:** 2026-02-02  
**Version:** 2.0  
**Status:** Draft for Review

---

## Overview

SUMM-Daemon is a CLI process management daemon service that enables multi-agent collaboration by managing multiple CLI processes (Sessions) with support for initialization, message injection, and interactive attachment.

## Technology Stack

| 组件 | 选型 | 说明 |
|-----|------|------|
| Language | Rust | 内存安全、高性能 |
| Async Runtime | Tokio | 异步 I/O |
| IPC | Unix Domain Sockets | 本地快速通信 |
| Session 管理 | tmux | 进程托管、终端复用 |
| CLI Framework | clap (derive macros) | 命令行解析 |
| Archive Extraction | compress-tools | 压缩包处理 |
| Logging | tracing + tracing-subscriber | 结构化日志 |
| Process Management | systemd (user service) | daemon 生命周期 |

### 前置依赖

```bash
# tmux 是必需依赖
# Debian/Ubuntu
apt install tmux

# RHEL/CentOS
yum install tmux

# macOS
brew install tmux

# 最低版本要求: tmux 3.0+
tmux -V
```

---

## 1. Overall Architecture

### 架构概览

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

### Project Structure

```
summ-daemon/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── summ-daemon/          # Main daemon binary
│   │   ├── src/
│   │   │   ├── main.rs       # Daemon entry point
│   │   │   ├── daemon.rs     # Core daemon logic
│   │   │   ├── session.rs    # Session management
│   │   │   ├── tmux.rs       # tmux command abstraction
│   │   │   └── ipc.rs        # Unix socket server
│   │   └── Cargo.toml
│   ├── summ-cli/             # CLI client binary
│   │   ├── src/
│   │   │   ├── main.rs       # CLI entry point
│   │   │   └── commands/     # Command implementations
│   │   └── Cargo.toml
│   └── summ-common/          # Shared types and protocol
│       ├── src/
│       │   ├── lib.rs
│       │   ├── protocol.rs   # IPC message types
│       │   └── types.rs      # Session metadata types
│       └── Cargo.toml
├── systemd/
│   └── summ-daemon.service   # Systemd unit file
└── README.md
```

### Communication Flow

1. User runs `summ start ...` (CLI client)
2. CLI serializes command to JSON
3. CLI connects to Unix socket at `~/.summ-daemon/daemon.sock`
4. Daemon receives request, processes it
5. Daemon responds with JSON result
6. CLI displays formatted output

### Key Design Decisions

| 决策 | 理由 |
|-----|------|
| 使用 tmux 管理 Session | 天然支持 detach/reattach、多客户端 attach、进程与 daemon 生命周期解耦 |
| Separate binaries | daemon 和 CLI 分离，职责清晰 |
| Shared crate | protocol 类型共享，保证一致性 |
| Systemd user service | 自动重启、日志集成 |
| Unix socket IPC | 快速、安全、仅本地 |

---

## 2. tmux Integration Layer

### tmux Session 命名规范

```
summ-{session_id}

示例:
summ-session_001
summ-dev-frontend
summ-a1b2c3d4
```

使用 `summ-` 前缀避免与用户自己的 tmux session 冲突。

### tmux 命令封装

```rust
// crates/summ-daemon/src/tmux.rs

use std::process::Command;
use anyhow::{Result, Context};

pub struct TmuxManager;

impl TmuxManager {
    /// 检查 tmux 是否可用
    pub fn check_available() -> Result<()> {
        let output = Command::new("tmux")
            .arg("-V")
            .output()
            .context("tmux not found. Please install tmux 3.0+")?;
        
        // 解析版本号，确保 >= 3.0
        let version = String::from_utf8_lossy(&output.stdout);
        // ... 版本检查逻辑
        Ok(())
    }

    /// 创建新 session 并执行命令
    pub fn create_session(
        session_name: &str,
        workdir: &Path,
        command: &str,
    ) -> Result<()> {
        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",                            // detached
                "-s", session_name,              // session name
                "-c", workdir.to_str().unwrap(), // working directory
                command,                         // command to run
            ])
            .status()
            .context("Failed to create tmux session")?;

        if !status.success() {
            anyhow::bail!("tmux new-session failed with status: {}", status);
        }
        Ok(())
    }

    /// 检查 session 是否存在
    pub fn session_exists(session_name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// 获取 session 内进程的 PID
    pub fn get_pane_pid(session_name: &str) -> Result<Option<u32>> {
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-t", session_name,
                "-F", "#{pane_pid}",
            ])
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let pid_str = String::from_utf8_lossy(&output.stdout);
        Ok(pid_str.trim().parse().ok())
    }

    /// 向 session 发送按键（用于消息注入）
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

    /// 终止 session
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

    /// 列出所有 summ- 前缀的 session
    pub fn list_summ_sessions() -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()?;

        if !output.status.success() {
            // tmux 没有 session 时返回非零，这是正常情况
            return Ok(vec![]);
        }

        let sessions: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|name| name.starts_with("summ-"))
            .map(|s| s.to_string())
            .collect();

        Ok(sessions)
    }

    /// 开启日志记录
    pub fn enable_logging(session_name: &str, log_path: &Path) -> Result<()> {
        let status = Command::new("tmux")
            .args([
                "pipe-pane",
                "-t", session_name,
                &format!("cat >> {}", log_path.display()),
            ])
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to enable logging for session");
        }
        Ok(())
    }

    /// 捕获 pane 内容
    pub fn capture_pane(session_name: &str, lines: i32) -> Result<String> {
        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t", session_name,
                "-p",                           // print to stdout
                "-S", &(-lines).to_string(),    // start line (negative = from end)
            ])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
```

### Attach 实现

`summ attach` 命令直接调用 tmux attach，让用户进入交互模式：

```rust
// crates/summ-cli/src/commands/attach.rs

use std::os::unix::process::CommandExt;

pub fn attach_session(session_id: &str) -> Result<()> {
    let tmux_session = format!("summ-{}", session_id);
    
    // 先通过 daemon 确认 session 存在
    let response = send_to_daemon(Request::Status { session_id: session_id.to_string() })?;
    if let Response::Error { .. } = response {
        anyhow::bail!("Session not found: {}", session_id);
    }

    // 使用 exec 替换当前进程为 tmux attach
    let err = Command::new("tmux")
        .args(["attach-session", "-t", &tmux_session])
        .exec();

    // exec 只在失败时返回
    Err(anyhow::anyhow!("Failed to attach: {}", err))
}
```

### 并发 Attach 处理

tmux 原生支持多个客户端同时 attach 到同一个 session：
- 所有客户端看到相同输出
- 任何客户端的输入都会发送到 session
- 所有 attach 的终端大小会同步到最小的那个

如果需要限制只允许一个 attach，可以在 daemon 层面加锁：

```rust
// 可选：单 attach 限制
pub struct Session {
    // ...
    attached_client: Option<String>,
}

// 在 attach 请求时检查
if session.attached_client.is_some() {
    return Err(DaemonError::new(ErrorCode::E009, "Session already attached"));
}
```

---

## 3. Session Management

### Session Lifecycle

```
                    summ start
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. 生成 session_id                                             │
│  2. 创建 workdir: ~/.summ-daemon/sessions/{session_id}/         │
│  3. 解压/复制 --init 内容到 workdir                              │
│  4. tmux new-session -d -s summ-{session_id} -c {workdir} {cli} │
│  5. tmux pipe-pane 开启日志                                      │
│  6. 保存 meta.json                                               │
│  7. 返回 session info                                            │
└─────────────────────────────────────────────────────────────────┘
                         │
                         ▼
                   ┌──────────┐
                   │ running  │◄────────────────┐
                   └────┬─────┘                 │
                        │                       │
            ┌───────────┼───────────┐           │
            │           │           │           │
            ▼           ▼           ▼           │
       summ inject  summ attach  CLI 完成任务   │
            │           │           │           │
            └───────────┴───────────┴───────────┘
                        │
              (CLI 进程退出)
                        │
                        ▼
                   ┌──────────┐
                   │ stopped  │
                   └──────────┘
```

### Session Creation Flow

```rust
// crates/summ-daemon/src/session.rs

pub async fn create_session(
    cli: &str,
    init_path: &Path,
    name: Option<String>,
    config: &DaemonConfig,
) -> Result<Session> {
    // 1. 生成唯一 session_id
    let session_id = generate_session_id();
    let display_name = name.unwrap_or_else(|| session_id.clone());
    let tmux_session = format!("summ-{}", session_id);

    // 2. 创建 workdir
    let workdir = config.sessions_dir.join(&session_id);
    fs::create_dir_all(&workdir)?;

    // 3. 初始化工作目录
    initialize_workdir(&workdir, init_path)?;

    // 4. 创建 tmux session
    TmuxManager::create_session(&tmux_session, &workdir, cli)?;

    // 5. 开启日志
    let log_path = config.logs_dir.join(format!("{}.log", session_id));
    TmuxManager::enable_logging(&tmux_session, &log_path)?;

    // 6. 获取 CLI 进程 PID
    let pid = TmuxManager::get_pane_pid(&tmux_session)?;

    // 7. 构建并保存元信息
    let session = Session {
        session_id: session_id.clone(),
        tmux_session,
        name: display_name,
        cli: cli.to_string(),
        workdir: workdir.clone(),
        init_source: init_path.to_path_buf(),
        status: SessionStatus::Running,
        pid,
        created_at: Utc::now(),
        last_activity: Utc::now(),
    };

    session.save_metadata()?;

    Ok(session)
}

fn initialize_workdir(workdir: &Path, init_path: &Path) -> Result<()> {
    if init_path.is_dir() {
        copy_dir_contents(init_path, workdir)?;
    } else if init_path.extension().map_or(false, |e| e == "zip") {
        extract_zip(init_path, workdir)?;
    } else if init_path.to_string_lossy().ends_with(".tar.gz") {
        extract_tar_gz(init_path, workdir)?;
    } else {
        anyhow::bail!("Unsupported init source: {:?}", init_path);
    }
    Ok(())
}
```

### Session Recovery on Daemon Restart

Daemon 启动时自动恢复对现有 Session 的管理：

```rust
// crates/summ-daemon/src/daemon.rs

pub async fn recover_sessions(config: &DaemonConfig) -> Result<HashMap<String, Session>> {
    let mut sessions = HashMap::new();

    // 1. 获取所有 summ- 前缀的 tmux sessions
    let tmux_sessions = TmuxManager::list_summ_sessions()?;
    let tmux_set: HashSet<_> = tmux_sessions.into_iter().collect();

    // 2. 扫描 meta.json 文件
    for entry in fs::read_dir(&config.sessions_dir)? {
        let entry = entry?;
        let meta_path = entry.path().join("meta.json");

        if !meta_path.exists() {
            continue;
        }

        let mut session: Session = serde_json::from_reader(
            fs::File::open(&meta_path)?
        )?;

        // 3. 与 tmux 状态对账
        if tmux_set.contains(&session.tmux_session) {
            // tmux session 存在，恢复为 running
            session.status = SessionStatus::Running;
            session.pid = TmuxManager::get_pane_pid(&session.tmux_session)?;
            tracing::info!("Recovered running session: {}", session.session_id);
        } else if session.status == SessionStatus::Running {
            // meta 显示 running 但 tmux session 不存在，更新为 stopped
            session.status = SessionStatus::Stopped;
            session.save_metadata()?;
            tracing::info!("Session {} marked as stopped (tmux session gone)", session.session_id);
        }

        sessions.insert(session.session_id.clone(), session);
    }

    // 4. 检查是否有 tmux session 但没有 meta.json（异常情况）
    for tmux_name in tmux_set {
        let session_id = tmux_name.strip_prefix("summ-").unwrap_or(&tmux_name);
        if !sessions.contains_key(session_id) {
            tracing::warn!(
                "Found orphan tmux session {} without meta.json, consider manual cleanup",
                tmux_name
            );
        }
    }

    Ok(sessions)
}
```

---

## 4. Claude Code Hook 集成

### 概述

Claude Code 原生支持 Hook 机制，在特定事件发生时自动执行外部脚本。SUMM Daemon 利用此机制实现状态同步，无需修改 Claude Code 本身。

### Claude Code Hook 事件

| Hook 事件 | 触发时机 | SUMM 用途 |
|----------|---------|----------|
| **SessionStart** | CLI 启动/恢复会话 | 上报 `idle` 状态 |
| **Stop** | 主代理完成响应 | 上报 `idle` 状态 + 任务结果 |
| **SubagentStop** | 子代理完成任务 | 上报子任务完成 |
| **SessionEnd** | 会话结束退出 | 上报 `stopped` 状态 |

### 运行时目录结构

```
~/.summ-daemon/sessions/session_001/
├── meta.json                 # Daemon 管理的元信息
├── runtime/                  # 运行时状态目录
│   └── status.json           # Hook 脚本写入的状态
└── workspace/                # 实际工作目录（从 --init 初始化）
    └── ...
```

### Claude Code 配置

Session 创建时，在 workspace 目录生成 `.claude/settings.local.json`：

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.summ-daemon/bin/summ-hook session-start"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.summ-daemon/bin/summ-hook stop"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.summ-daemon/bin/summ-hook subagent-stop"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.summ-daemon/bin/summ-hook session-end"
          }
        ]
      }
    ]
  }
}
```

### summ-hook 脚本实现

安装到 `~/.summ-daemon/bin/summ-hook`：

```bash
#!/bin/bash
# summ-hook: Claude Code Hook 处理脚本
# 用法: summ-hook <event> [args...]

set -e

EVENT="$1"
RUNTIME_DIR="${SUMM_RUNTIME_DIR:-$PWD/../runtime}"
STATUS_FILE="$RUNTIME_DIR/status.json"

# 从 stdin 读取 Hook 输入 (JSON)
INPUT=$(cat)

# 提取 session_id（从环境变量或输入）
SESSION_ID="${SUMM_SESSION_ID:-unknown}"

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
        # Claude 主代理完成响应
        write_status "idle" "Task completed"
        ;;
    
    subagent-stop)
        # 子代理完成
        write_status "idle" "Subagent task completed"
        ;;
    
    session-end)
        # 会话结束
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

### Daemon 端 Session 创建

```rust
pub async fn create_session(
    cli: &str,
    init_path: &Path,
    name: Option<String>,
    config: &DaemonConfig,
) -> Result<Session> {
    // ... 生成 session_id, 创建 workdir ...
    
    let workspace_dir = workdir.join("workspace");
    let runtime_dir = workdir.join("runtime");
    fs::create_dir_all(&workspace_dir)?;
    fs::create_dir_all(&runtime_dir)?;
    
    // 初始化 workspace（从 --init 解压/复制）
    initialize_workdir(&workspace_dir, init_path)?;
    
    // 生成 Claude Code Hook 配置
    if cli.contains("claude") {
        deploy_claude_code_hooks(&workspace_dir, &session_id, &runtime_dir)?;
    }
    
    // 创建 tmux session，设置环境变量
    TmuxManager::create_session_with_env(
        &tmux_session,
        &workspace_dir,  // CLI 在 workspace 目录运行
        cli,
        &[
            ("SUMM_SESSION_ID", &session_id),
            ("SUMM_RUNTIME_DIR", runtime_dir.to_str().unwrap()),
        ],
    )?;
    
    // ...
}

fn deploy_claude_code_hooks(
    workspace_dir: &Path,
    session_id: &str,
    runtime_dir: &Path,
) -> Result<()> {
    let claude_dir = workspace_dir.join(".claude");
    fs::create_dir_all(&claude_dir)?;
    
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
    fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    
    Ok(())
}
```

### CLI 状态数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStatus {
    pub state: CliState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CliState {
    Idle,
    Busy,
    Stopped,
}
```

### Daemon 状态读取

```rust
impl Session {
    /// 从 runtime/status.json 读取 CLI 上报的状态
    pub fn read_cli_status(&self) -> Option<CliStatus> {
        let status_file = self.workdir.join("runtime/status.json");
        
        if !status_file.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&status_file).ok()?;
        serde_json::from_str(&content).ok()
    }
    
    /// 获取综合状态（结合 tmux 和 Hook 上报）
    pub fn get_effective_status(&self) -> SessionStatus {
        // 1. 检查 tmux session 是否存在
        if !TmuxManager::session_exists(&self.tmux_session) {
            return SessionStatus::Stopped;
        }
        
        // 2. 读取 Hook 上报的状态
        match self.read_cli_status() {
            Some(cli_status) => {
                // 检查状态是否过期（超过 120 秒未更新视为 busy）
                let age = Utc::now() - cli_status.timestamp;
                if age > chrono::Duration::seconds(120) {
                    return SessionStatus::Running;
                }
                
                match cli_status.state {
                    CliState::Idle => SessionStatus::Idle,
                    CliState::Busy => SessionStatus::Running,
                    CliState::Stopped => SessionStatus::Stopped,
                }
            }
            None => SessionStatus::Running, // 无状态文件，假设在运行
        }
    }
}
```

### 状态流转

```
                Claude Code 启动
                       │
                       ▼
              SessionStart Hook
              state: "idle"
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
    用户输入任务                  等待输入中
         │                      (保持 idle)
         ▼
    Claude 处理中
    (状态文件未更新，
     超过120秒视为 busy)
         │
         ▼
      Stop Hook
    state: "idle"
         │
         ▼
    等待下一个任务...
         │
         ▼
   SessionEnd Hook
   state: "stopped"
```

### 安装 summ-hook 脚本

Daemon 安装时部署：

```rust
fn install_hook_script(config: &DaemonConfig) -> Result<()> {
    let bin_dir = config.base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    
    let hook_script = bin_dir.join("summ-hook");
    fs::write(&hook_script, include_str!("scripts/summ-hook.sh"))?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_script, fs::Permissions::from_mode(0o755))?;
    }
    
    Ok(())
}
```

### 其他 CLI 工具支持

对于非 Claude Code 的 CLI 工具（如 aider），可以：

1. **方案 A**：如果工具支持类似 Hook 机制，配置调用 `summ-hook`
2. **方案 B**：回退到仅检测 tmux session 存活状态，不支持 idle 检测

```rust
fn deploy_cli_hooks(workspace_dir: &Path, cli: &str, session_id: &str, runtime_dir: &Path) -> Result<()> {
    if cli.contains("claude") {
        deploy_claude_code_hooks(workspace_dir, session_id, runtime_dir)?;
    } else if cli.contains("aider") {
        // aider 暂不支持 hook，跳过
        tracing::info!("CLI '{}' does not support hooks, status detection limited", cli);
    }
    // 其他 CLI 工具可在此扩展
    Ok(())
}
```

---

## 5. Data Model

### Session Metadata (meta.json)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub tmux_session: String,      // tmux session 名称 (summ-{session_id})
    pub name: String,              // 用户可读名称
    pub cli: String,               // CLI 命令
    pub workdir: PathBuf,          // 工作目录
    pub init_source: PathBuf,      // 初始化来源
    pub status: SessionStatus,
    pub pid: Option<u32>,          // CLI 进程 PID（信息性）
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Running,    // CLI 正在执行任务
    Idle,       // CLI 空闲，等待新任务（通过 Hook 上报）
    Stopped,    // tmux session 已退出
}
```

### Configuration File

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub sessions_dir: PathBuf,          // Default: ~/.summ-daemon/sessions
    pub logs_dir: PathBuf,              // Default: ~/.summ-daemon/logs
    pub socket_path: PathBuf,           // Default: ~/.summ-daemon/daemon.sock
    pub cleanup_retention_hours: u64,   // Default: 24
    pub tmux_prefix: String,            // Default: "summ-"
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let home = dirs::home_dir().expect("HOME not set");
        let base = home.join(".summ-daemon");
        Self {
            sessions_dir: base.join("sessions"),
            logs_dir: base.join("logs"),
            socket_path: base.join("daemon.sock"),
            cleanup_retention_hours: 24,
            tmux_prefix: "summ-".to_string(),
        }
    }
}
```

---

## 6. IPC Protocol

### Protocol Format

JSON over Unix socket，长度前缀帧格式：

```
[4 bytes: message length (u32, big-endian)][JSON payload]
```

### Request/Response Types

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    Start {
        cli: String,
        init: PathBuf,
        name: Option<String>,
    },
    Stop {
        session_id: String,
    },
    List {
        status_filter: Option<SessionStatus>,  // running / idle / error / stopped
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    Success { data: serde_json::Value },
    Error { code: String, message: String },
}
```

### Socket Handling

- Daemon 在 `~/.summ-daemon/daemon.sock` 创建 socket
- Socket 权限设置为 0600（仅所有者可访问）
- 使用 Tokio 的 `UnixListener` 异步接受连接
- 每个连接处理一个请求后关闭
- 请求超时：30 秒

---

## 7. Process Monitoring

### Monitoring Task

```rust
async fn monitor_sessions(
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    config: Arc<DaemonConfig>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let mut sessions = sessions.write().await;
        for (id, session) in sessions.iter_mut() {
            // 获取综合状态（结合 tmux 存活检测和 Hook 上报）
            let new_status = session.get_effective_status();
            
            if new_status != session.status {
                tracing::info!(
                    "Session {} status changed: {:?} -> {:?}",
                    id, session.status, new_status
                );
                session.status = new_status;
                session.save_metadata().ok();
            }

            // 更新活动时间
            if session.status != SessionStatus::Stopped {
                session.last_activity = Utc::now();
            }
        }
    }
}
```

### Cleanup Task

```rust
async fn cleanup_old_sessions(
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    config: Arc<DaemonConfig>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;

        let retention = Duration::from_secs(config.cleanup_retention_hours * 3600);
        let cutoff = Utc::now() - chrono::Duration::from_std(retention).unwrap();

        let mut sessions = sessions.write().await;
        let to_remove: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.status == SessionStatus::Stopped && s.last_activity < cutoff)
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            if let Some(session) = sessions.remove(&id) {
                if let Err(e) = fs::remove_dir_all(&session.workdir) {
                    tracing::error!("Failed to cleanup session {}: {}", id, e);
                } else {
                    tracing::info!("Cleaned up old session: {}", id);
                }
            }
        }
    }
}
```

### Graceful Shutdown

On SIGTERM/SIGINT:
1. 停止接受新连接
2. 等待进行中的请求完成（带超时）
3. 保存所有 session 元数据
4. 关闭 socket
5. 退出（tmux sessions 继续运行）

---

## 8. Systemd Integration

### Systemd Unit File

```ini
# ~/.config/systemd/user/summ-daemon.service

[Unit]
Description=SUMM Daemon - CLI Process Management Service
After=default.target

[Service]
Type=notify
ExecStart=%h/.cargo/bin/summ-daemon
Restart=on-failure
RestartSec=5s

# 环境变量
Environment="RUST_LOG=info"

# 日志
StandardOutput=journal
StandardError=journal
SyslogIdentifier=summ-daemon

[Install]
WantedBy=default.target
```

### Systemd Notify Integration

```rust
async fn start_daemon(config: DaemonConfig) -> Result<()> {
    // 初始化
    setup_directories(&config)?;
    TmuxManager::check_available()?;
    
    let listener = create_socket(&config.socket_path).await?;
    let sessions = recover_sessions(&config).await?;
    let sessions = Arc::new(RwLock::new(sessions));

    // 启动后台任务
    let sessions_clone = sessions.clone();
    let config_clone = Arc::new(config.clone());
    tokio::spawn(monitor_sessions(sessions_clone, config_clone.clone()));
    tokio::spawn(cleanup_old_sessions(sessions.clone(), config_clone));

    // 通知 systemd 准备就绪
    #[cfg(target_os = "linux")]
    sd_notify::notify(true, &[sd_notify::NotifyState::Ready])?;

    tracing::info!("SUMM Daemon started, listening on {:?}", config.socket_path);

    // 主循环
    accept_loop(listener, sessions, config).await
}
```

### Installation Commands

```bash
# 安装二进制
cargo install --path crates/summ-daemon
cargo install --path crates/summ-cli

# 安装 systemd unit
mkdir -p ~/.config/systemd/user/
cp systemd/summ-daemon.service ~/.config/systemd/user/

# 启用并启动
systemctl --user daemon-reload
systemctl --user enable summ-daemon
systemctl --user start summ-daemon

# 验证
systemctl --user status summ-daemon
summ daemon status
```

---

## 9. Error Handling

### Error Codes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    E001,  // Init resource not found/inaccessible
    E002,  // Session not found
    E003,  // Session stopped, cannot operate
    E004,  // Archive extraction failed
    E005,  // Process start failed
    E006,  // Message injection failed
    E007,  // Daemon not running
    E008,  // Invalid CLI command
    E009,  // tmux not available
}
```

### Error Handling Strategy

| 错误场景 | 处理方式 |
|---------|---------|
| tmux 未安装 | daemon 启动时检查，返回 E009 |
| tmux session 创建失败 | 返回 E005，包含 tmux 错误信息 |
| Session 不存在 | 返回 E002 |
| 向已停止 Session 注入消息 | 返回 E003 |
| 压缩包解压失败 | 返回 E004 |
| Socket 连接失败 | CLI 返回 E007 |

### Error Output Format

```json
{
  "error": true,
  "code": "E002",
  "message": "Session not found: session_999"
}
```

---

## 10. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tmux_session_naming() {
        assert_eq!(
            format!("summ-{}", "session_001"),
            "summ-session_001"
        );
    }

    #[test]
    fn test_config_default() {
        let config = DaemonConfig::default();
        assert!(config.sessions_dir.ends_with("sessions"));
    }

    #[tokio::test]
    async fn test_session_metadata_serialization() {
        let session = Session {
            session_id: "test".to_string(),
            tmux_session: "summ-test".to_string(),
            // ...
        };
        let json = serde_json::to_string(&session).unwrap();
        let parsed: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session.session_id, parsed.session_id);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // 需要 tmux 环境
async fn test_full_session_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let config = DaemonConfig {
        sessions_dir: temp_dir.path().join("sessions"),
        logs_dir: temp_dir.path().join("logs"),
        socket_path: temp_dir.path().join("test.sock"),
        ..Default::default()
    };

    // 1. 创建 session
    let session = create_session(
        "echo 'hello'; sleep 10",
        &temp_dir.path().join("init"),
        Some("test-session".to_string()),
        &config,
    ).await.unwrap();

    assert_eq!(session.status, SessionStatus::Running);
    assert!(TmuxManager::session_exists(&session.tmux_session));

    // 2. 注入消息
    TmuxManager::send_keys(&session.tmux_session, "echo injected", true).unwrap();

    // 3. 停止 session
    TmuxManager::kill_session(&session.tmux_session).unwrap();
    assert!(!TmuxManager::session_exists(&session.tmux_session));
}
```

### Manual Testing Checklist

1. [ ] tmux 未安装时 daemon 报错信息清晰
2. [ ] 目录初始化正常
3. [ ] .zip 初始化正常
4. [ ] .tar.gz 初始化正常
5. [ ] Claude Code session 创建时 `.claude/settings.local.json` 正确生成
6. [ ] summ-hook 脚本正确安装到 `~/.summ-daemon/bin/`
7. [ ] Claude Code 启动后 SessionStart Hook 触发，状态变为 idle
8. [ ] Claude Code 完成任务后 Stop Hook 触发，状态变为 idle
9. [ ] Claude Code 退出后 SessionEnd Hook 触发，状态变为 stopped
10. [ ] summ list --status idle 正确过滤
11. [ ] summ list 正确显示所有 session
12. [ ] summ attach 能正常交互
13. [ ] summ attach 后 Ctrl+B D 能正常 detach
14. [ ] summ inject 消息能被 CLI 接收
15. [ ] 多个终端同时 attach 同一 session
16. [ ] 停止 daemon 后 tmux session 继续运行
17. [ ] 重启 daemon 后自动恢复对 session 的管理
18. [ ] CLI 进程退出后 session 状态更新为 stopped
19. [ ] 清理任务正确删除过期 session
20. [ ] 非 Claude Code CLI（如 aider）正常工作（无 idle 检测）

---

## 11. Key Dependencies

### Core Dependencies

```toml
# summ-daemon/Cargo.toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
compress-tools = "0.14"
sd-notify = "0.4"
dirs = "5.0"

# summ-cli/Cargo.toml
[dependencies]
clap = { version = "4.4", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["net", "io-util", "rt"] }
anyhow = "1.0"
dirs = "5.0"

# summ-common/Cargo.toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Development Dependencies

```toml
[dev-dependencies]
tempfile = "3.8"
tokio-test = "0.4"
```

---

## 12. Implementation Phases

### Phase 1: Core Infrastructure (1 week)

- [ ] 设置 Rust workspace
- [ ] 实现 TmuxManager 基础功能
- [ ] 实现 Unix socket server
- [ ] 实现 IPC 协议
- [ ] 基础 CLI 结构 (clap)
- [ ] 配置文件加载

### Phase 2: Session Management (1 week)

- [ ] Session 创建（无 init）
- [ ] Session 元数据持久化
- [ ] Session 列表和状态查询
- [ ] Session 停止
- [ ] Daemon 重启恢复

### Phase 3: Initialization Support (3 days)

- [ ] 目录复制
- [ ] .zip 解压
- [ ] .tar.gz 解压
- [ ] 错误处理

### Phase 4: Claude Code Hook 集成 (3 days)

- [ ] summ-hook 脚本实现
- [ ] 安装脚本部署到 ~/.summ-daemon/bin/
- [ ] Claude Code settings.local.json 生成
- [ ] runtime/status.json 读取
- [ ] 综合状态判断逻辑 (get_effective_status)
- [ ] 状态过期检测

### Phase 5: Message Injection and Attach (3 days)

- [ ] summ inject 实现 (tmux send-keys)
- [ ] summ attach 实现 (exec tmux)
- [ ] 日志记录 (tmux pipe-pane)

### Phase 6: Monitoring and Cleanup (3 days)

- [ ] Session 监控任务（集成 Hook 状态）
- [ ] 自动状态更新
- [ ] 清理任务

### Phase 7: Systemd Integration (2 days)

- [ ] Systemd unit file
- [ ] sd-notify 集成
- [ ] daemon start/stop/status 命令

### Phase 8: Testing and Documentation (1 week)

- [ ] 单元测试
- [ ] 集成测试（含 Claude Code Hook 测试）
- [ ] README 和使用文档
- [ ] 错误处理完善

**预计总工期：5 周**

---

## Appendix A: tmux 常用命令参考

```bash
# 创建 session
tmux new-session -d -s {name} -c {workdir} {command}

# 检查 session 存在
tmux has-session -t {name}

# 列出 sessions
tmux list-sessions -F "#{session_name}"

# 获取 pane PID
tmux list-panes -t {name} -F "#{pane_pid}"

# 发送按键
tmux send-keys -t {name} "message" Enter

# 终止 session
tmux kill-session -t {name}

# 附加到 session
tmux attach-session -t {name}

# 开启日志
tmux pipe-pane -t {name} "cat >> /path/to/log"

# 捕获输出
tmux capture-pane -t {name} -p -S -100
```

---

## Conclusion

本设计采用 tmux 作为 Session 管理的核心组件，利用 tmux 成熟的进程托管能力实现：

- **进程持久化**：CLI 进程运行在 tmux session 中，与 daemon 生命周期解耦
- **Daemon 重启恢复**：daemon 重启后自动重新发现并管理现有 tmux sessions
- **多客户端支持**：tmux 原生支持多个终端同时 attach 到同一 session

代价是增加了 tmux 作为外部依赖，但考虑到 tmux 在 Linux/macOS 上的普及程度和稳定性，这是一个合理的 trade-off。
