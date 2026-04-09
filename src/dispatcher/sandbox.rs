//! Container sandbox — executes Jobs inside Docker containers
//! with full network access and secret env var filtering.

use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

/// Patterns that identify secret environment variables
const SECRET_PATTERNS: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "_KEY",
    "PRIVATE",
    "CREDENTIAL",
    "AUTH",
    "API_KEY",
    "GITHUB_TOKEN",
    "GIT_TOKEN",
    "AWS_SECRET",
    "AWS_ACCESS",
    "STRIPE",
    "SENTRY_DSN",
    "DATABASE_URL",
    "POSTGRES_PASSWORD",
    "MYSQL_PASSWORD",
    "REDIS_PASSWORD",
    "JWT",
    "BEARER",
];

/// A prepared container execution request
#[derive(Debug, Clone)]
pub struct ContainerRequest {
    /// Docker image to use
    pub image: String,
    /// Script path inside the container workspace
    pub script_path: String,
    /// Working directory (repo root)
    pub workdir: String,
    /// Environment variables (secrets already stripped)
    pub env: HashMap<String, String>,
    /// Job ID for logging
    pub job_id: String,
}

/// Result of a container execution
#[derive(Debug, Clone)]
pub struct ContainerResult {
    /// Job ID
    pub job_id: String,
    /// Exit code
    pub exit_code: i32,
    /// Combined stdout + stderr
    pub output: String,
    /// Duration in seconds
    pub duration_secs: f64,
}

/// Filter environment variables, removing secrets
pub fn filter_env(env: &HashMap<String, String>) -> HashMap<String, String> {
    let mut filtered = HashMap::new();
    for (key, value) in env {
        let upper = key.to_uppercase();
        let is_secret = SECRET_PATTERNS.iter().any(|p| upper.contains(p))
            || upper == "HOME"
            || upper == "USER"
            || upper == "PATH";
        if is_secret {
            filtered.insert(key.clone(), "[FILTERED]".to_string());
        } else {
            filtered.insert(key.clone(), value.clone());
        }
    }
    filtered
}

/// Build a blocking docker run command for a job.
fn build_docker_args(req: &ContainerRequest) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--network=host".to_string(),
        "-w".to_string(),
        req.workdir.clone(),
        "-i".to_string(),
        "-v".to_string(),
        format!("{}:/workspace", req.workdir),
        req.image.clone(),
        "python".to_string(),
        req.script_path.clone(),
    ];
    for (k, v) in &req.env {
        args.push(format!("BB_{}={}", k, v));
    }
    args
}

/// Run docker in a blocking thread pool task, return (exit_code, output).
fn run_docker_blocking(req: ContainerRequest) -> (i32, String) {
    let args = build_docker_args(&req);
    let output = std::process::Command::new("docker")
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let code = out.status.code().unwrap_or(-1);
            (code, combined)
        }
        Err(e) => (-1, format!("docker execution failed: {}", e)),
    }
}

/// Execute a container job (blocking, runs in thread pool).
/// Returns ContainerResult.
pub async fn execute_container(req: ContainerRequest) -> ContainerResult {
    let start = std::time::Instant::now();
    let job_id = req.job_id.clone();
    let image = req.image.clone();
    let script_path = req.script_path.clone();

    tracing::info!(
        "[Job {}] Starting container {} with script {}",
        job_id,
        image,
        script_path
    );

    let (exit_code, output) = tokio::task::spawn_blocking({
        let req = req.clone();
        move || run_docker_blocking(req)
    })
    .await
    .unwrap_or((-1, "task join failed".to_string()));

    let duration = start.elapsed().as_secs_f64();
    tracing::info!(
        "[Job {}] Container finished with exit code {} in {:.1}s",
        job_id,
        exit_code,
        duration
    );

    ContainerResult {
        job_id,
        exit_code,
        output,
        duration_secs: duration,
    }
}

/// Validate that Docker is available and running
pub async fn check_docker() -> Result<bool, String> {
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .output()
        .await
        .map_err(|e| format!("docker not available: {}", e))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!("Docker available: version {}", version);
        Ok(true)
    } else {
        Err("docker daemon not running".to_string())
    }
}

/// Prepare a container request from a job and repo workspace
pub fn prepare_container_request(
    job_id: &str,
    script_path: &str,
    _workdir: &Path,
    raw_env: &HashMap<String, String>,
) -> ContainerRequest {
    let filtered_env = filter_env(raw_env);
    ContainerRequest {
        image: "python:3.11".to_string(),
        script_path: script_path.to_string(),
        workdir: "/workspace".to_string(),
        env: filtered_env,
        job_id: job_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_env() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "secret123".to_string());
        env.insert("AWS_SECRET_KEY".to_string(), "s3cr3t".to_string());
        env.insert("MY_VAR".to_string(), "safe".to_string());
        env.insert("BUILDER_TOKEN".to_string(), "token123".to_string());

        let filtered = filter_env(&env);
        assert_eq!(filtered.get("GITHUB_TOKEN").unwrap(), "[FILTERED]");
        assert_eq!(filtered.get("AWS_SECRET_KEY").unwrap(), "[FILTERED]");
        assert_eq!(filtered.get("MY_VAR").unwrap(), "safe");
        assert_eq!(filtered.get("BUILDER_TOKEN").unwrap(), "[FILTERED]");
    }

    #[test]
    fn test_secret_patterns() {
        let patterns: &[&str] = SECRET_PATTERNS;
        assert!(patterns.contains(&"TOKEN"));
        assert!(patterns.contains(&"SECRET"));
        assert!(patterns.contains(&"PASSWORD"));
        assert!(patterns.contains(&"_KEY"));
    }

    #[test]
    fn test_build_docker_args() {
        let req = ContainerRequest {
            image: "python:3.11".to_string(),
            script_path: "build.py".to_string(),
            workdir: "/workspace".to_string(),
            env: HashMap::new(),
            job_id: "test".to_string(),
        };
        let args = build_docker_args(&req);
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"--network=host".to_string()));
        assert!(args.contains(&"python:3.11".to_string()));
    }
}
