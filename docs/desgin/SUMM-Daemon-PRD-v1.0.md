# SUMM Daemon - 产品需求文档 (PRD)

**版本:** 1.0  
**日期:** 2025年2月  
**项目定位:** CLI 进程管理守护服务

---

## 1. 项目概述

### 1.1 项目定位

SUMM Daemon 是一个常驻后台服务，负责管理 CLI 进程。它提供命令行接口，支持启动、停止、查询 Session，以及向运行中的 Session 注入消息，实现多 Agent 协同工作。



### 1.2 核心能力

1. **进程管理**：启动、停止、查询 CLI 进程
2. **初始化管理**：支持从压缩包或目录初始化 Session 工作环境
3. **消息注入**：向运行中的 Session 注入消息，以支持 Agent 间协同

---

## 2. 系统架构

待补充

---

## 3. 配置管理

### 3.1 配置文件结构

```
~/.summ-daemon/
├── config.json              # Daemon 全局配置
├── sessions/                # Session 运行时数据
│   ├── session_001/         # Session 工作目录（从 --init 初始化）
│   │   └── meta.json        # Session 元信息
│   └── session_002/
│       └── ...
└── logs/                    # 日志目录
```

### 3.2 全局配置文件 (config.json)

```json
{
  "sessions_dir": "~/.summ-daemon/sessions",
  "logs_dir": "~/.summ-daemon/logs"
}
```

---

## 4. 命令行接口

### 4.1 命令概览

| 命令 | 说明 |
|-----|------|
| `summ start` | 启动新 Session |
| `summ stop` | 停止 Session |
| `summ list` | 列出所有 Session |
| `summ status` | 查询 Session 状态 |
| `summ attach` | 连接到 Session 终端 |
| `summ inject` | 向 Session 注入消息 |
| `summ daemon start` | 启动守护进程 |
| `summ daemon stop` | 停止守护进程 |
| `summ daemon status` | 查看守护进程状态 |

### 4.2 命令详细说明

#### 4.2.1 summ start

启动新的 CLI Session。

```bash
summ start --cli <command> --init <path> [--name <session-name>]
```

**参数：**

| 参数 | 必填 | 说明 |
|-----|------|------|
| --cli | 是 | CLI 启动命令（如 `claude-code`、`aider` 等） |
| --init | 是 | 初始化资源路径，支持压缩包（.zip/.tar.gz）或目录 |
| --name | 否 | 自定义 Session 名称，默认自动生成 |

**初始化行为：**

1. 生成唯一 Session ID
2. 在 `~/.summ-daemon/sessions/` 下创建以 Session ID 命名的目录
3. 如果 `--init` 是压缩包：解压到该目录
4. 如果 `--init` 是目录：复制该目录内容到新目录
5. 在该目录下执行 `--cli` 指定的启动命令
6. 记录 Session 元信息

**输出：**

```json
{
  "session_id": "session_001",
  "name": "my-session",
  "cli": "claude-code",
  "workdir": "/home/user/.summ-daemon/sessions/session_001",
  "status": "running",
  "pid": 12345,
  "created_at": "2025-02-01T10:00:00Z"
}
```

#### 4.2.2 summ stop

停止指定 Session。

```bash
summ stop <session-id>
```

**参数：**

| 参数 | 必填 | 说明 |
|-----|------|------|
| session-id | 是 | Session ID 或名称 |

**输出：**

```json
{
  "session_id": "session_001",
  "status": "stopped"
}
```

#### 4.2.3 summ list

列出所有 Session。

```bash
summ list [--status <status>]
```

**参数：**

| 参数 | 必填 | 说明 |
|-----|------|------|
| --status | 否 | 过滤状态：running / idle / stopped |

**输出：**

```json
{
  "sessions": [
    {
      "session_id": "session_001",
      "name": "dev-session",
      "cli": "claude-code",
      "workdir": "/home/user/.summ-daemon/sessions/session_001",
      "status": "running",
      "created_at": "2025-02-01T10:00:00Z"
    },
    {
      "session_id": "session_002",
      "name": "test-session",
      "cli": "aider",
      "workdir": "/home/user/.summ-daemon/sessions/session_002",
      "status": "idle",
      "created_at": "2025-02-01T11:00:00Z"
    }
  ]
}
```

#### 4.2.4 summ status

查询单个 Session 的详细状态。

```bash
summ status <session-id>
```

**输出：**

```json
{
  "session_id": "session_001",
  "name": "dev-session",
  "cli": "claude-code",
  "workdir": "/home/user/.summ-daemon/sessions/session_001",
  "status": "running",
  "pid": 12345,
  "created_at": "2025-02-01T10:00:00Z",
  "last_activity": "2025-02-01T12:30:00Z"
}
```

#### 4.2.5 summ attach

连接到 Session 的终端，进行交互式操作。

```bash
summ attach <session-id>
```

**行为：**
- 连接到指定 Session 的 stdin/stdout
- 支持实时交互
- Ctrl+D 或特定快捷键退出（不终止 Session）

#### 4.2.6 summ inject

向运行中的 Session 注入消息。

```bash
summ inject <session-id> --message <message>
summ inject <session-id> --file <file-path>
```

**参数：**

| 参数 | 必填 | 说明 |
|-----|------|------|
| session-id | 是 | Session ID 或名称 |
| --message | 二选一 | 注入的消息内容 |
| --file | 二选一 | 从文件读取消息内容 |

**输出：**

```json
{
  "session_id": "session_001",
  "injected": true,
  "message_length": 128
}
```

**使用场景：**
- SUMM 主代理通知子 Session 任务变更
- 传递上游 Session 的输出结果
- 系统级通知或指令

#### 4.2.7 summ daemon start/stop/status

管理守护进程本身。

```bash
summ daemon start [--port <port>]
summ daemon stop
summ daemon status
```

---

## 5. Session 数据模型

### 5.1 Session 元信息 (meta.json)

```json
{
  "session_id": "session_001",
  "name": "dev-session",
  "cli": "claude-code",
  "workdir": "/home/user/.summ-daemon/sessions/session_001",
  "init_source": "/path/to/init.zip",
  "status": "running",
  "pid": 12345,
  "created_at": "2025-02-01T10:00:00Z",
  "last_activity": "2025-02-01T12:30:00Z"
}
```

### 5.2 状态定义

| 状态 | 说明 |
|-----|------|
| running | Session 正在执行任务 |
| idle | Session 空闲，等待新任务 |
| stopped | Session 已停止 |

---

## 6. 守护进程管理

### 6.1 启动行为

1. 读取全局配置
3. 开始监听命令

### 6.2 进程监控

- 监控所有子进程（CLI）状态
- 检测进程异常退出，更新 Session 状态
- 定期清理已结束的 Session 数据（可配置保留时长）

### 6.3 日志管理

```
~/.summ-daemon/logs/
├── daemon.log         # 守护进程日志
├── session_001.log    # 各 Session 日志
└── session_002.log
```

---

## 7. 错误处理

### 7.1 错误码

| 错误码 | 说明 |
|-------|------|
| E001 | 初始化资源不存在或无法访问 |
| E002 | Session 不存在 |
| E003 | Session 已停止，无法操作 |
| E004 | 压缩包解压失败 |
| E005 | 进程启动失败 |
| E006 | 消息注入失败 |
| E007 | 守护进程未运行 |
| E008 | CLI 命令无效或不存在 |

### 7.2 错误输出格式

```json
{
  "error": true,
  "code": "E002",
  "message": "Session not found: session_999"
}
```

