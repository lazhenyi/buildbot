//! Dispatcher Job — atomic execution unit, mirrors a CI Job

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Job execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Waiting to be dispatched to a runner
    Pending,
    /// Sent to a runner, runner has not yet acknowledged
    Dispatched,
    /// Runner has picked up the job and is executing
    Running,
    /// Completed successfully
    Success,
    /// Completed with a non-zero exit code
    Failed,
    /// Cancelled by user or system
    Cancelled,
    /// Runner disappeared without completing (treated as failed)
    Lost,
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Pending
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Dispatched => write!(f, "dispatched"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Success => write!(f, "success"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
            JobStatus::Lost => write!(f, "lost"),
        }
    }
}

impl From<&str> for JobStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => JobStatus::Pending,
            "dispatched" => JobStatus::Dispatched,
            "running" => JobStatus::Running,
            "success" => JobStatus::Success,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            "lost" => JobStatus::Lost,
            _ => JobStatus::Pending,
        }
    }
}

/// Who requested this job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JobSource {
    /// Triggered by a repository webhook (includes branch/ref info)
    Webhook { repository_id: i64, branch: String },
    /// Triggered by a periodic scheduler
    Periodic { scheduler_name: String },
    /// Manually forced via API
    Force { triggered_by: String },
}

/// A dispatched Job — corresponds to a single `/.ci/<N>_<name>.py` script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Human-readable job name (derived from script filename, e.g. "01_build.py" → "build")
    pub name: String,
    /// Numeric sort key extracted from script filename (e.g. "01" → 1)
    pub sort_key: i32,
    /// Job status
    pub status: JobStatus,
    /// Labels / tags used for runner matching (e.g. ["python", "docker"])
    pub labels: Vec<String>,
    /// Source that triggered this job
    pub source: JobSource,
    /// Repository URL this job belongs to
    pub repository_url: String,
    /// Git branch or ref
    pub branch: String,
    /// Optional commit SHA
    pub revision: Option<String>,
    /// Runner that this job is assigned to (None if pending)
    pub runner_name: Option<String>,
    /// Arbitrary key-value metadata (e.g. matrix vars, env overrides)
    pub env: std::collections::HashMap<String, String>,
    /// Exit code once finished (None if still running)
    pub exit_code: Option<i32>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
    /// When the job was dispatched to a runner
    pub started_at: Option<DateTime<Utc>>,
    /// When the job finished
    pub finished_at: Option<DateTime<Utc>>,
    /// Absolute path to the script file inside the repository clone
    pub script_path: String,
    /// Working directory for the container
    pub workdir: String,
}

impl Job {
    /// Create a new pending job from a script path.
    /// sort_key is extracted from numeric prefix; 9999 if no prefix.
    pub fn new(
        name: String,
        script_path: String,
        repository_url: String,
        branch: String,
        labels: Vec<String>,
        source: JobSource,
    ) -> Self {
        let sort_key = Self::extract_sort_key(&name);
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            sort_key,
            status: JobStatus::Pending,
            labels,
            source,
            repository_url,
            branch,
            revision: None,
            runner_name: None,
            env: std::collections::HashMap::new(),
            exit_code: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            script_path,
            workdir: String::new(),
        }
    }

    /// Extract numeric sort key from filename like "01_build.py" → 1.
    /// Returns 9999 if no numeric prefix.
    fn extract_sort_key(name: &str) -> i32 {
        let stem = std::path::Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name);
        let leading: String = stem
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .filter(|c| c.is_ascii_digit())
            .collect();
        leading.parse::<i32>().unwrap_or(9999)
    }

    /// Transition to dispatched state
    pub fn dispatch(&mut self, runner_name: String) {
        self.status = JobStatus::Dispatched;
        self.runner_name = Some(runner_name);
        self.updated_at = Utc::now();
    }

    /// Transition to running state
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to success state
    pub fn succeed(&mut self, exit_code: i32) {
        self.status = JobStatus::Success;
        self.exit_code = Some(exit_code);
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to failed state
    pub fn fail(&mut self, exit_code: Option<i32>, error: Option<String>) {
        self.status = JobStatus::Failed;
        self.exit_code = exit_code;
        self.error_message = error;
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to cancelled state
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to lost state (runner died)
    pub fn mark_lost(&mut self, error: Option<String>) {
        self.status = JobStatus::Lost;
        self.error_message = error.or_else(|| Some("Runner disappeared".to_string()));
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if the job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Success | JobStatus::Failed | JobStatus::Cancelled | JobStatus::Lost
        )
    }

    /// Check if the job can be cancelled
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Pending | JobStatus::Dispatched | JobStatus::Running
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_key_extraction() {
        assert_eq!(Job::extract_sort_key("01_build"), 1);
        assert_eq!(Job::extract_sort_key("10_test.py"), 10);
        assert_eq!(Job::extract_sort_key("100_deploy.py"), 100);
        assert_eq!(Job::extract_sort_key("build"), 9999);
        assert_eq!(Job::extract_sort_key("_test"), 9999);
        assert_eq!(Job::extract_sort_key(""), 9999);
    }

    #[test]
    fn test_job_state_machine() {
        let source = JobSource::Force {
            triggered_by: "admin".to_string(),
        };
        let mut job = Job::new(
            "01_test".to_string(),
            ".ci/01_test.py".to_string(),
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec!["python".to_string()],
            source,
        );

        assert_eq!(job.status, JobStatus::Pending);
        assert!(!job.is_terminal());

        job.dispatch("runner-1".to_string());
        assert_eq!(job.status, JobStatus::Dispatched);

        job.start();
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());

        job.succeed(0);
        assert_eq!(job.status, JobStatus::Success);
        assert_eq!(job.exit_code, Some(0));
        assert!(job.is_terminal());
        assert!(job.finished_at.is_some());
    }

    #[test]
    fn test_job_cancel() {
        let source = JobSource::Periodic {
            scheduler_name: "nightly".to_string(),
        };
        let mut job = Job::new(
            "02_build".to_string(),
            ".ci/02_build.py".to_string(),
            "https://github.com/example/repo".to_string(),
            "develop".to_string(),
            vec![],
            source,
        );

        assert!(job.can_cancel());
        job.cancel();
        assert!(job.is_terminal());
        assert!(!job.can_cancel()); // terminal jobs cannot be cancelled again
    }
}
