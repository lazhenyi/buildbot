//! Repository operations - clone, scan, and create jobs

use std::path::Path;
use tokio::process::Command;
use std::collections::HashMap;

use crate::dispatcher::{DispatcherState, JobSource};
use crate::error::Result;

/// Clone a git repository to the specified directory
pub async fn clone_repository(
    repo_url: &str,
    branch: &str,
    workdir: &Path,
    credentials: Option<&str>,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = workdir.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| crate::error::BuildbotError::Master(format!("Failed to create workdir: {}", e)))?;
    }

    // Set GIT_TERMINAL_PROMPT=0 to prevent interactive prompts
    let mut envs = HashMap::new();
    envs.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());

    // Setup credentials if provided
    if let Some(creds) = credentials {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let git_creds_path = Path::new(&home).join(".git-credentials");
            let cred_url = make_credential_url(repo_url, creds);
            tokio::fs::write(&git_creds_path, &cred_url).await
                .map_err(|e| crate::error::BuildbotError::Master(format!("Failed to write git credentials: {}", e)))?;
            envs.insert("GIT_CONFIG_COUNT".to_string(), "1".to_string());
            envs.insert("GIT_CONFIG_KEY_0".to_string(), "credential.helper".to_string());
            envs.insert(
                "GIT_CONFIG_VALUE_0".to_string(),
                format!("store --file={}", git_creds_path.display()),
            );
        }
    }

    // Check if repo already exists
    let git_dir = workdir.join(".git");
    if git_dir.exists() {
        tracing::info!("Repository already exists at {:?}, updating...", workdir);

        let mut cmd = Command::new("git");
        cmd.arg("fetch").arg("origin").arg(branch);
        cmd.current_dir(workdir);
        for (k, v) in &envs {
            cmd.env(k, v);
        }

        let output = cmd.output().await
            .map_err(|e| crate::error::BuildbotError::Master(format!("git fetch failed: {}", e)))?;

        if !output.status.success() {
            tracing::warn!("git fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Checkout the branch
        let mut cmd = Command::new("git");
        cmd.arg("checkout").arg(branch);
        cmd.current_dir(workdir);
        let _ = cmd.output().await;

        // Pull latest
        let mut cmd = Command::new("git");
        cmd.arg("pull").arg("origin").arg(branch);
        cmd.current_dir(workdir);
        for (k, v) in &envs {
            cmd.env(k, v);
        }
        let _ = cmd.output().await;
    } else {
        tracing::info!("Cloning repository {} (branch: {}) to {:?}", repo_url, branch, workdir);

        let mut cmd = Command::new("git");
        cmd.arg("clone")
           .arg("--branch").arg(branch)
           .arg("--single-branch")
           .arg(repo_url)
           .arg(workdir);

        for (k, v) in &envs {
            cmd.env(k, v);
        }

        let output = cmd.output().await
            .map_err(|e| crate::error::BuildbotError::Master(format!("git clone failed: {}", e)))?;

        if !output.status.success() {
            return Err(crate::error::BuildbotError::Master(format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }

    Ok(())
}

/// Make credential URL from repository URL and credentials
fn make_credential_url(repo_url: &str, creds: &str) -> String {
    let host = repo_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("git@")
        .trim_start_matches("ssh://")
        .split('/').next().unwrap_or("github.com")
        .split(':').next().unwrap_or("github.com");

    if creds.contains(':') {
        let parts: Vec<&str> = creds.splitn(2, ':').collect();
        format!("https://{}:{}@{}", url_encode(parts[0]), url_encode(parts[1]), host)
    } else {
        format!("https://{}@{}", url_encode(creds), host)
    }
}

/// URL encode helper
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}

/// Get the latest revision from a cloned repository
pub async fn get_latest_revision(workdir: &Path) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("rev-parse").arg("HEAD");
    cmd.current_dir(workdir);

    let output = cmd.output().await
        .map_err(|e| crate::error::BuildbotError::Master(format!("git rev-parse failed: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(crate::error::BuildbotError::Master("Failed to get revision".to_string()))
    }
}

/// Scan CI directory and create jobs
pub async fn scan_ci_directory(
    dispatcher: &DispatcherState,
    repo_url: &str,
    branch: &str,
    _revision: Option<&str>,
    ci_dir: &Path,
    requirements_path: &Path,
    source: JobSource,
    extra_labels: Vec<String>,
) -> Result<usize> {
    // Read requirements.txt if it exists
    let requirements_content = if requirements_path.exists() {
        tokio::fs::read_to_string(requirements_path).await.unwrap_or_default()
    } else {
        String::new()
    };

    // Scan and enqueue
    let (enqueued, errors) = dispatcher.scan_and_enqueue(
        repo_url,
        branch,
        ci_dir,
        &requirements_content,
        source,
        extra_labels,
    ).await;

    if !errors.is_empty() {
        tracing::warn!("CI scan errors: {:?}", errors);
    }

    tracing::info!("Enqueued {} jobs from CI directory", enqueued);
    Ok(enqueued)
}
