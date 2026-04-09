# Buildbot Dispatcher

![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)
![许可协议](https://img.shields.io/badge/license-MIT-blue.svg)
![状态](https://img.shields.io/badge/status-活跃-green.svg)

基于 Rust 实现的自托管 CI 系统，采用 Job/Runner 架构，Runner 在隔离的 Docker 容器中执行 CI 任务。设计目标是快速、资源占用低、一键部署，适合单台服务器或小规模集群场景。

如果你是 GitHub Actions 用户，但因合规要求或基础设施限制无法使用 github.com，Buildbot Dispatcher 可以填补这一空白。它提供与 GitHub Actions 相同的 Runner 模型，但完全自主托管，数据不出本环境。

---

## 目录

- [为什么选择 Buildbot Dispatcher？](#为什么选择-buildbot-dispatcher)
- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [CI 脚本结构](#ci-脚本结构)
- [API 接口文档](#api-接口文档)
- [系统架构](#系统架构)
- [开发指南](#开发指南)
- [贡献指南](#贡献指南)
- [安全说明](#安全说明)
- [许可证](#许可证)

---

## 为什么选择 Buildbot Dispatcher？

| | Buildbot Dispatcher | GitHub Actions | Jenkins | Buildbot 经典版 |
|---|---|---|---|---|
| **托管方式** | 自托管 | 仅 SaaS | 自托管 | 自托管 |
| **上手难度** | 低 | 不适用 | 中 | 高 |
| **执行模型** | Job/Runner | Job/Runner | Master/Worker | Master/Worker/Builder |
| **隔离方式** | Docker | Docker | Docker/SSH | PB |
| **实现语言** | Rust | Ruby/Node | Java | Python |
| **配置格式** | YAML | YAML | Jenkinsfile | Python/TAC |
| **内存占用** | ~20MB 二进制文件 | 不适用 | ~500MB JVM | ~200MB Python |

---

## 环境要求

- **Rust 1.85+**（`rustup default stable`）
- **SQLite**（默认，无需额外软件）或 **PostgreSQL 14+**
- **Docker 20.10+**（容器沙箱执行）
- **Git**（仓库克隆）
- **OpenSSL / libssl-dev**（TLS/HTTP 客户端）

---

## 快速开始

### 从源码编译安装

```bash
git clone https://github.com/your-org/buildbot-dispatcher.git
cd buildbot-dispatcher
cargo build --release
./target/release/buildbot master --basedir /tmp/buildbot
```

### Docker 部署

```bash
docker pull ghcr.io/your-org/buildbot-dispatcher:latest
docker run -d \
  --name buildbot \
  -p 8010:8010 \
  -p 9990:9990 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v ./data:/app/data \
  ghcr.io/your-org/buildbot-dispatcher:latest
```

### 启动后操作

1. 在 basedir 创建 `master.cfg`（见[配置说明](#配置说明)）
2. 访问 `http://localhost:8010` 打开 Web UI
3. 注册 Runner：`POST /api/v1/dispatcher/runners/register`
4. 向 GitHub 仓库推送代码，Webhook 自动触发 CI 任务

---

## 配置说明

在 basedir 下创建 `master.cfg`：

```yaml
# ─── Master ─────────────────────────────────────────────────────────────────
master:
  # Web UI 中显示的名称
  name: "my-ci"
  # Web UI 的访问地址
  web_url: "http://localhost:8010"
  # 克隆仓库的目录，CI 脚本扫描在此目录下进行
  dispatcher_workdir: "/app/repos"
  # Runner 心跳超时（秒），超过此时间未响应的 Runner 标记为断开，
  # 其挂起的任务会被重新分配。
  runner_timeout_secs: 300

# ─── 数据库 ───────────────────────────────────────────────────────────────
database:
  # SQLite（默认，无需安装额外软件）：
  url: "sqlite:///app/data/buildbot.db"
  # PostgreSQL（取消注释即可使用）：
  # url: "postgres://buildbot:password@localhost:5432/buildbot"

# ─── Web 接口 ─────────────────────────────────────────────────────────────
www:
  # 内部 API 端口（不直接暴露给用户）
  port: 9990
  # Web UI 端口
  web_port: 8010
```

### 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `RUST_LOG` | `info` | 日志级别（`error`、`warn`、`info`、`debug`、`trace`） |
| `BUILDBOT_BASEDIR` | `.` | 工作根目录 |
| `DATABASE_URL` | 来自 master.cfg | 优先级高于 `database.url` |

---

## CI 脚本结构

在仓库的 `.ci/` 目录下放置 Python CI 脚本。系统会自动发现并按编号前缀顺序执行：

```
repo/
├── .ci/
│   ├── 01_checkout.py    # 最先执行（编号最小）
│   ├── 02_build.py        # 第二执行
│   └── 03_test.py        # 第三执行
├── matrix.json            # 可选：生成任务变体
└── requirements.txt        # 可选：Python 依赖白名单
```

### 脚本示例

```python
#!/usr/bin/env python3
"""构建步骤 — 每次推送时执行"""

import subprocess
import os

def main():
    # CI 环境变量：
    #   BUILDBOT_REPOSITORY  — 克隆 URL
    #   BUILDBOT_BRANCH      — 分支名
    #   BUILDBOT_REVISION    — 提交 SHA
    #   BUILDBOT_JOB_NAME    — 脚本名（不含编号前缀）

    revision = os.environ.get("BUILDBOT_REVISION", "")[:8]
    print(f"构建 {revision}，分支 {os.environ.get('BUILDBOT_BRANCH')}")

    subprocess.run(["cargo", "build", "--release"], check=True)
    subprocess.run(["cargo", "test"], check=True)

if __name__ == "__main__":
    main()
```

### Matrix 构建

在仓库根目录添加 `matrix.json` 以生成笛卡尔积任务：

```json
{
  "include": [
    { "os": "ubuntu-latest", "python": "3.11" },
    { "os": "ubuntu-latest", "python": "3.12" },
    { "os": "windows-latest", "python": "3.11" }
  ]
}
```

每个矩阵组合会生成一个独立的任务，环境变量中会包含对应的 `OS` 和 `PYTHON` 值。

---

## API 接口文档

所有接口前缀为 `/api/v1/`，返回 JSON 格式。

### 健康检查

```
GET /api/v1/health
```

返回：`{ "status": "ok", "service": "buildbot-dispatcher" }`

### Webhook

```
POST /api/v1/hooks/github
```

接收 GitHub push/pull request 事件，自动克隆仓库、扫描 `.ci/` 目录并入队 CI 任务。

### 调度器

```
GET /api/v1/dispatcher
```

返回调度器摘要：各状态任务数量、Runner 数量。

```
GET /api/v1/dispatcher/jobs?status=<status>&labels=<labels>
```

列出所有任务。可选查询参数：

- `status` — 按状态过滤，取值：`Pending`、`Running`、`Success`、`Failed`、`Cancelled`、`Lost`
- `labels` — 逗号分隔的标签列表，只返回包含所有指定标签的任务

```
GET /api/v1/dispatcher/jobs/{job_id}
```

获取单个任务详情，包含 `exit_code`、`error_message` 和各时间戳。

```
POST /api/v1/dispatcher/jobs/{job_id}/cancel
```

取消处于 pending 或 running 状态的任务。

```
POST /api/v1/dispatcher/jobs/{job_id}/complete
```

标记任务完成。请求体：

```json
{
  "exit_code": 0,
  "error_message": null,
  "duration_secs": 42.5
}
```

### Runner API

```
GET /api/v1/dispatcher/jobs/poll?runner_name=<name>&labels=<labels>
```

Runner 轮询获取下一个匹配标签的 pending 任务。返回任务负载或 `{ "message": "No pending jobs available" }`。

```
POST /api/v1/dispatcher/runners/register
```

注册新 Runner。请求体：

```json
{
  "name": "runner-01",
  "runner_type": "persistent",
  "labels": ["ubuntu", "docker", "linux"],
  "max_jobs": 2
}
```

`runner_type` 必填，可选 `persistent`（持久化）或 `ephemeral`（临时）。

```
POST /api/v1/dispatcher/runners/heartbeat
```

发送心跳保活。请求体：

```json
{ "name": "runner-01" }
```

```
DELETE /api/v1/dispatcher/runners/{name}
```

注销 Runner，释放其所有活跃任务。

```
GET /api/v1/dispatcher/runners
```

列出所有已注册的 Runner，包含状态、标签和活跃任务数。

---

## 系统架构

```
                    ┌──────────────────────────────────────────┐
                    │          Buildbot Dispatcher Master        │
                    │                                             │
GitHub ───────────►│  Web Server (Actix-web)                      │
Webhook             │    POST /api/v1/hooks/github                │
                    │    GET  /api/v1/dispatcher/jobs             │
                    │                                             │
                    │  Dispatcher State (内存)                     │
                    │    任务队列  ──► pending / running / done   │
                    │    Runner 注册表 ──► connected / stale      │
                    │                                             │
                    │  数据库 (SQLite / PostgreSQL + SeaORM)       │
                    │    Migrations: core + dispatcher 表         │
                    └──────────────┬──────────────────────────────┘
                                   │ Docker socket
                                   ▼
                         ┌──────────────────┐
                         │   Docker 运行时   │
                         │  容器中执行任务   │
                         │  自动清理容器     │
                         └──────────────────┘
```

### 核心模块

| 模块 | 文件 | 职责 |
|---|---|---|
| `dispatcher/mod.rs` | 内存状态 | 任务/Runner 注册、分发逻辑 |
| `dispatcher/job.rs` | 领域模型 | 任务生命周期（pending → running → done） |
| `dispatcher/runner.rs` | 领域模型 | Runner 注册、心跳、失联检测 |
| `dispatcher/sandbox.rs` | Docker 执行 | 容器创建、敏感环境变量过滤 |
| `dispatcher/script.rs` | CI 扫描器 | `.ci/*.py` 发现、依赖验证 |
| `dispatcher/matrix.rs` | Matrix 扩展 | `matrix.json` → 笛卡尔积任务生成 |
| `api/handlers.rs` | HTTP 层 | REST API 请求处理 |
| `db/` | 持久层 | SeaORM 实体定义与数据库迁移 |

---

## 开发指南

```bash
# 编译
cargo build

# 运行测试
cargo test

# 代码检查
cargo clippy --all-targets -- -D warnings

# 格式检查
cargo fmt --check
```

### 使用 SQLite 本地运行

```bash
export RUST_LOG=debug
cargo run -- master --basedir /tmp/buildbot --config master.cfg
```

### 使用 PostgreSQL 本地运行

```bash
export DATABASE_URL=postgres://buildbot:password@localhost:5432/buildbot
cargo run -- master --basedir /tmp/buildbot --config master.cfg
```

---

## 贡献指南

欢迎提交贡献。提交前请阅读 [.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md) 中的检查清单。

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feat/my-feature`
3. 编写代码并运行测试：`cargo test`
4. 运行 clippy 并修复警告：`cargo clippy -- -D warnings`
5. 格式化代码：`cargo fmt`
6. Push 并提交 Pull Request

---

## 安全说明

- **敏感变量过滤**：包含 `SECRET`、`TOKEN`、`PASSWORD`、`PRIVATE_KEY`、`CREDENTIALS`、`AUTH` 的环境变量在进入 Docker 容器前会被自动清除。
- **Runner 隔离**：每个任务在其独立的 Docker 容器中执行，任务完成后容器自动销毁。
- **Webhook 签名**：生产环境应验证 GitHub Webhook 的 `X-Hub-Signature-256` 请求头。
- **数据库**：生产环境建议使用启用了 TLS 的 PostgreSQL；SQLite 部署时注意限制数据库文件的操作系统权限。

如发现安全漏洞，请通过仓库的 Security 标签页或直接联系维护者报告。请勿在公开 Issue 中提交安全问题。

---

## 许可证

MIT License，详见 [LICENSE](LICENSE)。
