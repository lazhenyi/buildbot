//! Dispatcher Runner — agent that executes Jobs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Runner type determines lifecycle management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerType {
    /// Persistent runner — stays online, polls for jobs continuously
    Persistent,
    /// Ephemeral runner — one-shot, takes one job then exits
    Ephemeral,
}

impl Default for RunnerType {
    fn default() -> Self {
        RunnerType::Persistent
    }
}

impl From<&str> for RunnerType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "persistent" => RunnerType::Persistent,
            "ephemeral" => RunnerType::Ephemeral,
            _ => RunnerType::Persistent,
        }
    }
}

/// System capabilities reported by the runner
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerCapabilities {
    /// Available Docker images (e.g. ["python:3.11", "ubuntu:22.04"])
    pub images: Vec<String>,
    /// Available network modes ("full", "limited", "none")
    pub network: String,
    /// Max concurrent jobs this runner can handle
    pub max_jobs: usize,
    /// CPU count
    pub cpus: usize,
    /// Total memory in bytes
    pub memory_bytes: u64,
    /// Optional GPU info
    pub gpus: Vec<String>,
}

impl RunnerCapabilities {
    pub fn default_python() -> Self {
        Self {
            images: vec!["python:3.11".to_string()],
            network: "full".to_string(),
            max_jobs: 1,
            cpus: 1,
            memory_bytes: 2 * 1024 * 1024 * 1024,
            gpus: vec![],
        }
    }
}

/// A registered Runner (agent) that can execute Jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    /// Unique runner name (chosen by the runner, must be unique across master)
    pub name: String,
    /// Runner type
    pub runner_type: RunnerType,
    /// Arbitrary labels used for job-to-runner matching (e.g. ["python", "docker", "linux"])
    pub labels: Vec<String>,
    /// System capabilities
    pub capabilities: RunnerCapabilities,
    /// Last heartbeat timestamp
    pub last_heartbeat_at: DateTime<Utc>,
    /// When the runner first connected
    pub registered_at: DateTime<Utc>,
    /// Current running job IDs
    pub active_jobs: Vec<String>,
    /// Max concurrent jobs (from capabilities)
    pub max_jobs: usize,
    /// Is the runner currently connected / alive
    pub connected: bool,
    /// Current status description
    pub status: String,
}

impl Runner {
    /// Create a new runner with default capabilities
    pub fn new(name: String, runner_type: RunnerType, labels: Vec<String>) -> Self {
        let now = Utc::now();
        let capabilities = RunnerCapabilities::default_python();
        let max_jobs = capabilities.max_jobs;
        Self {
            name,
            runner_type,
            labels,
            capabilities,
            last_heartbeat_at: now,
            registered_at: now,
            active_jobs: Vec::new(),
            max_jobs,
            connected: true,
            status: "idle".to_string(),
        }
    }

    /// Create with explicit capabilities
    pub fn with_capabilities(
        name: String,
        runner_type: RunnerType,
        labels: Vec<String>,
        capabilities: RunnerCapabilities,
    ) -> Self {
        let max_jobs = capabilities.max_jobs;
        let now = Utc::now();
        Self {
            name,
            runner_type,
            labels,
            capabilities,
            last_heartbeat_at: now,
            registered_at: now,
            active_jobs: Vec::new(),
            max_jobs,
            connected: true,
            status: "idle".to_string(),
        }
    }

    /// Refresh heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat_at = Utc::now();
        self.connected = true;
    }

    /// Mark runner as disconnected
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.status = "offline".to_string();
    }

    /// Register a job as running on this runner
    pub fn assign_job(&mut self, job_id: &str) {
        self.active_jobs.retain(|j| j != job_id);
        self.active_jobs.push(job_id.to_string());
        self.status = format!("running {} job(s)", self.active_jobs.len());
    }

    /// Remove a job (completed or cancelled)
    pub fn release_job(&mut self, job_id: &str) {
        self.active_jobs.retain(|j| j != job_id);
        if self.active_jobs.is_empty() {
            self.status = "idle".to_string();
        } else {
            self.status = format!("running {} job(s)", self.active_jobs.len());
        }
    }

    /// Check if runner can accept more jobs
    pub fn can_accept_job(&self) -> bool {
        self.connected && self.active_jobs.len() < self.max_jobs
    }

    /// Check if runner matches all required labels
    pub fn matches_labels(&self, required_labels: &[String]) -> bool {
        required_labels.iter().all(|l| self.labels.contains(l))
    }

    /// Check if runner is considered stale (missed heartbeat for N seconds)
    pub fn is_stale(&self, timeout_secs: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_heartbeat_at)
            .num_seconds();
        elapsed >= timeout_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_assign_and_release() {
        let capabilities = RunnerCapabilities {
            images: vec!["python:3.11".to_string()],
            network: "full".to_string(),
            max_jobs: 3,
            cpus: 4,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            gpus: vec![],
        };
        let mut runner = Runner::with_capabilities(
            "test-runner".to_string(),
            RunnerType::Persistent,
            vec!["python".to_string(), "docker".to_string()],
            capabilities,
        );

        assert!(runner.can_accept_job());
        assert!(runner.matches_labels(&["python".to_string()]));
        assert!(!runner.matches_labels(&["gpu".to_string()]));

        runner.assign_job("job-1");
        assert_eq!(runner.active_jobs.len(), 1);
        assert!(runner.can_accept_job()); // max_jobs=3, so still room

        runner.assign_job("job-2");
        runner.release_job("job-1");
        assert_eq!(runner.active_jobs.len(), 1);
        assert!(runner.can_accept_job());

        runner.release_job("job-2");
        assert!(runner.active_jobs.is_empty());
        assert_eq!(runner.status, "idle");
    }

    #[test]
    fn test_ephemeral_max_jobs() {
        let capabilities = RunnerCapabilities {
            images: vec!["python:3.11".to_string()],
            network: "full".to_string(),
            max_jobs: 1,
            cpus: 4,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            gpus: vec![],
        };
        let mut runner = Runner::with_capabilities(
            "ephemeral-1".to_string(),
            RunnerType::Ephemeral,
            vec![],
            capabilities,
        );

        runner.assign_job("job-a");
        assert!(!runner.can_accept_job()); // max 1
    }
}
