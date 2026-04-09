# Buildbot Dispatcher

![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-active-green.svg)

A self-hosted, GitHub Actions-style CI system written in Rust. Dispatcher manages a pool of Runner agents that execute CI jobs in isolated Docker containers. Designed to be fast, resource-efficient, and easy to deploy on a single server or a small cluster.

---

## Table of Contents

- [Why Buildbot Dispatcher?](#why-buildbot-dispatcher)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CI Script Structure](#ci-script-structure)
- [API Reference](#api-reference)
- [Architecture](#architecture)
- [Development](#development)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

---

## Why Buildbot Dispatcher?

| | Buildbot Dispatcher | GitHub Actions | Jenkins | Buildbot Classic |
|---|---|---|---|---|
| **Hosting** | Self-hosted | SaaS only | Self-hosted | Self-hosted |
| **Setup complexity** | Low | N/A | Medium | High |
| **Runner model** | Job/Runner | Job/Runner | Master/Worker | Master/Worker/Builder |
| **Isolation** | Docker | Docker | Docker/SSH | PB |
| **Language** | Rust | Ruby/Node | Java | Python |
| **Config format** | YAML | YAML | Jenkinsfile | Python/TAC |
| **Web UI** | Built-in | GitHub | Built-in | Built-in |
| **Memory footprint** | ~20MB binary | N/A | ~500MB JVM | ~200MB Python |

Buildbot Dispatcher gives you the GitHub Actions runner model with self-hosting control. If you need GitHub Actions but cannot use github.com or have compliance requirements, Buildbot Dispatcher fills that gap. Compared to Jenkins it requires no JVM and has a fraction of the memory footprint. Compared to the original Buildbot it eliminates the Python TAC configuration model in favor of a simple YAML file and a clean REST API.

---

## Requirements

- **Rust 1.85+** (or `rustup default stable`)
- **SQLite** (default) or **PostgreSQL 14+**
- **Docker 20.10+** (for container sandbox execution)
- **Git** (for repository cloning)
- **OpenSSL** / `libssl-dev` (for TLS/HTTP client)

---

## Quick Start

### Install from source

```bash
git clone https://github.com/your-org/buildbot-dispatcher.git
cd buildbot-dispatcher
cargo build --release
./target/release/buildbot master --basedir /tmp/buildbot
```

### Install via Docker

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

### First steps

1. Create `master.cfg` in your basedir (see [Configuration](#configuration)).
2. Visit `http://localhost:8010` for the web UI.
3. Register a runner: `POST /api/v1/dispatcher/runners/register`
4. Push to your GitHub repo — the webhook will trigger CI jobs.

---

## Configuration

Create `master.cfg` in your basedir:

```yaml
# ─── Master ────────────────────────────────────────────────────────────────
master:
  # Human-readable name shown in the web UI
  name: "my-ci"
  # URL the web UI is accessible at
  web_url: "http://localhost:8010"
  # Directory where repositories are cloned for CI scanning
  dispatcher_workdir: "/app/repos"
  # Runner heartbeat timeout in seconds. Runners not seen within this window
  # are marked as disconnected and their pending jobs are re-dispatched.
  runner_timeout_secs: 300

# ─── Database ──────────────────────────────────────────────────────────────
database:
  # SQLite (default — no extra software needed):
  url: "sqlite:///app/data/buildbot.db"
  # PostgreSQL (uncomment to use):
  # url: "postgres://buildbot:password@localhost:5432/buildbot"

# ─── Web Interface ─────────────────────────────────────────────────────────
www:
  # Internal API port (not directly user-facing)
  port: 9990
  # Web UI port
  web_port: 8010
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info` | Tracing log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `BUILDBOT_BASEDIR` | `.` | Base working directory |
| `DATABASE_URL` | from master.cfg | Overrides `database.url` |

---

## CI Script Structure

Place Python CI scripts inside the `.ci/` directory of your repository. Scripts are discovered automatically and executed in sort-key order:

```
repo/
├── .ci/
│   ├── 01_checkout.py    # Runs first (lowest sort key)
│   ├── 02_build.py       # Runs second
│   └── 03_test.py        # Runs third
├── matrix.json           # Optional: generates job variants
└── requirements.txt      # Optional: Python dependency allowlist
```

### CI Script example

```python
#!/usr/bin/env python3
"""Build step — runs on every push"""

import subprocess
import os

def main():
    # CI environment variables:
    #   BUILDBOT_REPOSITORY  — clone URL
    #   BUILDBOT_BRANCH      — branch name
    #   BUILDBOT_REVISION    — commit SHA
    #   BUILDBOT_JOB_NAME    — script filename without sort key prefix

    revision = os.environ.get("BUILDBOT_REVISION", "")[:8]
    print(f"Building {revision} on branch {os.environ.get('BUILDBOT_BRANCH')}")

    subprocess.run(["cargo", "build", "--release"], check=True)
    subprocess.run(["cargo", "test"], check=True)

if __name__ == "__main__":
    main()
```

### Matrix builds

Add `matrix.json` to generate Cartesian-product job variants:

```json
{
  "include": [
    { "os": "ubuntu-latest", "python": "3.11" },
    { "os": "ubuntu-latest", "python": "3.12" },
    { "os": "windows-latest", "python": "3.11" }
  ]
}
```

Each matrix entry generates a separate CI job with its own `OS`, `PYTHON` environment variables.

---

## API Reference

All endpoints are under `/api/v1/`. Responses are JSON.

### Health

```
GET /api/v1/health
```

Returns `{ "status": "ok", "service": "buildbot-dispatcher" }`.

### Webhooks

```
POST /api/v1/hooks/github
```

Receives GitHub push/pull request events. Clones the repository, scans `.ci/` directory, and enqueues jobs automatically.

### Dispatcher

```
GET /api/v1/dispatcher
```

Returns dispatcher summary: job counts by status, runner counts.

```
GET /api/v1/dispatcher/jobs?status=<status>&labels=<labels>
```

List all jobs. Optional query parameters:

- `status` — filter by `Pending`, `Running`, `Success`, `Failed`, `Cancelled`, `Lost`
- `labels` — comma-separated labels; returns only jobs that have all listed labels

```
GET /api/v1/dispatcher/jobs/{job_id}
```

Get details for a single job including `exit_code`, `error_message`, and timestamps.

```
POST /api/v1/dispatcher/jobs/{job_id}/cancel
```

Cancel a pending or running job.

```
POST /api/v1/dispatcher/jobs/{job_id}/complete
```

Mark a job as complete. Request body:

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

Runner polls for the next pending job matching the given labels. Returns the job payload or `{ "message": "No pending jobs available" }`.

```
POST /api/v1/dispatcher/runners/register
```

Register a new runner. Request body:

```json
{
  "name": "runner-01",
  "runner_type": "persistent",
  "labels": ["ubuntu", "docker", "linux"],
  "max_jobs": 2
}
```

`runner_type` must be `persistent` or `ephemeral`.

```
POST /api/v1/dispatcher/runners/heartbeat
```

Send a heartbeat to keep the runner connected. Request body:

```json
{ "name": "runner-01" }
```

```
DELETE /api/v1/dispatcher/runners/{name}
```

Unregister a runner and release its active jobs.

```
GET /api/v1/dispatcher/runners
```

List all registered runners with their status, labels, and active job counts.

---

## Architecture

```
                    ┌──────────────────────────────────────────────┐
                    │           Buildbot Dispatcher Master          │
                    │                                              │
GitHub ───────────►│  Web Server (Actix-web)                       │
Webhook             │    POST /api/v1/hooks/github                  │
                    │    GET  /api/v1/dispatcher/jobs               │
                    │                                              │
                    │  Dispatcher State (in-memory)                 │
                    │    Job queue  ──► pending / running / done    │
                    │    Runner registry ──► connected / stale     │
                    │                                              │
                    │  Database (SQLite / PostgreSQL via SeaORM)   │
                    │    Migrations: core + dispatcher tables       │
                    └──────────────┬───────────────────────────────┘
                                   │ Docker socket
                                   ▼
                         ┌──────────────────┐
                         │  Docker Runtime  │
                         │  Job execution  │
                         │  in containers   │
                         └──────────────────┘
```

### Components

| Component | File | Responsibility |
|---|---|---|
| `dispatcher/mod.rs` | In-memory state | Job/Runner registry, dispatch logic |
| `dispatcher/job.rs` | Domain model | Job lifecycle (pending → running → done) |
| `dispatcher/runner.rs` | Domain model | Runner registration, heartbeat, stale detection |
| `dispatcher/sandbox.rs` | Docker execution | Container creation, secret filtering |
| `dispatcher/script.rs` | CI scanner | `.ci/*.py` discovery, dependency validation |
| `dispatcher/matrix.rs` | Matrix expansion | `matrix.json` → Cartesian product jobs |
| `api/handlers.rs` | HTTP layer | REST API request handlers |
| `db/` | Persistence | SeaORM entities and migrations |

---

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --check
```

### Running locally with SQLite

```bash
export RUST_LOG=debug
cargo run -- master --basedir /tmp/buildbot --config master.cfg
```

### Running with PostgreSQL

```bash
export DATABASE_URL=postgres://buildbot:password@localhost:5432/buildbot
cargo run -- master --basedir /tmp/buildbot --config master.cfg
```

---

## Contributing

Contributions are welcome. Please read the [Pull Request Checklist](.github/PULL_REQUEST_TEMPLATE.md) before submitting.

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes and run tests: `cargo test`
4. Run clippy and fix warnings: `cargo clippy -- -D warnings`
5. Format code: `cargo fmt`
6. Push and open a Pull Request

---

## Security

- **Secret filtering**: Environment variables containing `SECRET`, `TOKEN`, `PASSWORD`, `PRIVATE_KEY`, `CREDENTIALS`, or `AUTH` are stripped before jobs execute in Docker containers.
- **Runner isolation**: Each job runs in its own Docker container. Containers are removed after the job completes.
- **Webhook signature**: GitHub webhook payloads should be verified using a `X-Hub-Signature-256` header in production deployments.
- **Database**: Use PostgreSQL with TLS in production. Restrict database file permissions for SQLite deployments.

If you discover a security vulnerability, please report it via the repository's Security tab or contact the maintainers directly. Do not open public issues for security problems.

---

## License

MIT License. See [LICENSE](LICENSE) for the full text.
