//! Dispatcher module — GitHub Actions-style job dispatcher
//!
//! Key concepts:
//! - **Job**: a single `/.ci/<N>_<name>.py` script, treated as an independent execution unit
//! - **Runner**: agent that polls for and executes Jobs (persistent or ephemeral)
//! - **Script Scanner**: parses `.ci/` directory, validates Python dependencies
//! - **Sandbox**: Docker container execution with secret env var filtering
//! - **Matrix**: generates Cartesian-product job variants from `matrix.json`
//!
//! Execution flow:
//! 1. Repo cloned / updated via webhook or periodic scan
//! 2. Script scanner discovers `/.ci/*.py` files, sorts by number prefix
//! 3. Strict mode: AST parse imports → check against requirements.txt → reject bad deps
//! 4. Matrix expansion: `matrix.json` → generate one Job per combination
//! 5. Jobs enqueued with Pending status
//! 6. Runner polls `GET /api/v2/dispatcher/jobs/poll?labels=x,y` → gets next pending job
//! 7. Runner executes in Docker container with secret env vars filtered
//! 8. Runner posts result via `POST /api/v2/dispatcher/jobs/:id/complete`
//! 9. BotMaster updates job status → notifies consumers

pub mod job;
pub mod matrix;
pub mod repo;
pub mod runner;
pub mod sandbox;
pub mod script;

use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub use job::{Job, JobSource, JobStatus};
pub use matrix::{MatrixConfig, MatrixInclude};
pub use repo::{clone_repository, get_latest_revision, scan_ci_directory};
pub use runner::{Runner, RunnerCapabilities, RunnerType};
pub use sandbox::{
    check_docker, execute_container, filter_env, prepare_container_request, ContainerRequest,
    ContainerResult,
};
pub use script::{ImportMode, ScriptInfo, ScriptScanner};

/// In-memory job registry (mirrors DB state for fast access)
type JobRegistry = HashMap<String, Job>;

/// In-memory runner registry
type RunnerRegistry = HashMap<String, Runner>;

/// Shared dispatcher state
pub struct DispatcherState {
    /// All jobs keyed by job ID
    jobs: RwLock<JobRegistry>,
    /// All registered runners keyed by runner name
    runners: RwLock<RunnerRegistry>,
    /// Channel to notify BotMaster when a job completes
    completion_tx: mpsc::Sender<JobCompletedEvent>,
    /// Import validation mode
    import_mode: ImportMode,
}

#[derive(Debug, Clone)]
pub struct JobCompletedEvent {
    pub job_id: String,
    pub job_name: String,
    pub status: JobStatus,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub duration_secs: Option<f64>,
}

impl DispatcherState {
    pub fn new(completion_tx: mpsc::Sender<JobCompletedEvent>, import_mode: ImportMode) -> Self {
        Self {
            jobs: RwLock::new(JobRegistry::new()),
            runners: RwLock::new(RunnerRegistry::new()),
            completion_tx,
            import_mode,
        }
    }

    // ─── Job operations ────────────────────────────────────────────────────────

    /// Enqueue a new job
    pub async fn enqueue(&self, job: Job) {
        let id = job.id.clone();
        let mut jobs = self.jobs.write().await;
        jobs.insert(id, job);
    }

    /// Get a job by ID
    pub async fn get_job(&self, job_id: &str) -> Option<Job> {
        self.jobs.read().await.get(job_id).cloned()
    }

    /// Get all jobs, optionally filtered by status
    pub async fn list_jobs(&self, status_filter: Option<JobStatus>) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        match status_filter {
            Some(s) => jobs.values().filter(|j| j.status == s).cloned().collect(),
            None => jobs.values().cloned().collect(),
        }
    }

    /// Mark a job as dispatched (assigned to a runner)
    pub async fn dispatch_job(&self, job_id: &str, runner_name: &str) -> Option<Job> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.dispatch(runner_name.to_string());
            return Some(job.clone());
        }
        None
    }

    /// Mark a job as running
    pub async fn start_job(&self, job_id: &str) -> Option<Job> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.start();
            return Some(job.clone());
        }
        None
    }

    /// Complete a job and emit completion event
    pub async fn complete_job(
        &self,
        job_id: &str,
        exit_code: Option<i32>,
        error: Option<String>,
        duration_secs: Option<f64>,
    ) -> Option<Job> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            let _status = job.status.clone();
            let job_name = job.name.clone();
            let _repo_url = job.repository_url.clone();

            if let Some(code) = exit_code {
                if code == 0 {
                    job.succeed(code);
                } else {
                    job.fail(exit_code, error.clone());
                }
            } else {
                job.fail(exit_code, error.clone());
            }

            let completed_job = job.clone();

            // Emit completion event to BotMaster
            let evt = JobCompletedEvent {
                job_id: job_id.to_string(),
                job_name,
                status: completed_job.status.clone(),
                exit_code,
                error_message: error,
                duration_secs,
            };
            let _ = self.completion_tx.send(evt).await;

            return Some(completed_job);
        }
        None
    }

    /// Cancel a job
    pub async fn cancel_job(&self, job_id: &str) -> Option<Job> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            if job.can_cancel() {
                job.cancel();
                return Some(job.clone());
            }
        }
        None
    }

    /// Get the next pending job that matches the given labels.
    /// Returns the job with the smallest (sort_key, filename).
    pub async fn poll_job(&self, runner_name: &str, required_labels: &[String]) -> Option<Job> {
        let mut jobs = self.jobs.write().await;

        // Find the first pending job matching labels, sorted by sort_key
        let best_id: Option<String> = jobs
            .iter()
            .filter(|(_, j)| j.status == JobStatus::Pending && j.matches_labels(required_labels))
            .map(|(id, j)| (j.sort_key, id.clone(), j.clone()))
            .collect::<Vec<_>>()
            .into_iter()
            .min_by_key(|(sort_key, _, _)| *sort_key)
            .map(|(_, id, _)| id);

        if let Some(id) = best_id {
            let job = jobs.get_mut(&id)?;
            job.dispatch(runner_name.to_string());
            return Some(job.clone());
        }

        None
    }

    /// Scan repository CI scripts and enqueue all valid jobs.
    /// Returns the number of jobs enqueued and validation errors.
    pub async fn scan_and_enqueue(
        &self,
        repo_url: &str,
        branch: &str,
        ci_dir: &std::path::Path,
        requirements_txt: &str,
        source: JobSource,
        extra_labels: Vec<String>,
    ) -> (usize, Vec<ScriptValidationError>) {
        let scanner = ScriptScanner::new(self.import_mode, requirements_txt);
        let scripts = scanner.scan_directory(ci_dir);

        let mut errors = Vec::new();
        let mut enqueued = 0;

        for script in scripts {
            if script.deps_valid {
                let labels = self.infer_labels(&script, extra_labels.clone());
                let job = Job::new(
                    script.job_name.clone(),
                    script.path.clone(),
                    repo_url.to_string(),
                    branch.to_string(),
                    labels,
                    source.clone(),
                );
                self.enqueue(job).await;
                enqueued += 1;
            } else {
                errors.push(ScriptValidationError {
                    script_path: script.path.clone(),
                    script_name: script.filename.clone(),
                    unapproved_imports: script.unapproved_imports,
                });
            }
        }

        (enqueued, errors)
    }

    /// Infer labels from script filename and content
    fn infer_labels(&self, script: &ScriptInfo, mut labels: Vec<String>) -> Vec<String> {
        let filename_lower = script.filename.to_lowercase();
        let name_lower = script.job_name.to_lowercase();

        // Auto-detect labels from name
        if (name_lower.contains("docker") || name_lower.contains("container"))
            && !labels.contains(&"docker".to_string())
        {
            labels.push("docker".to_string());
        }
        if (name_lower.contains("gpu") || name_lower.contains("cuda"))
            && !labels.contains(&"gpu".to_string())
        {
            labels.push("gpu".to_string());
        }
        if name_lower.contains("windows") || filename_lower.contains("win") {
            if !labels.contains(&"windows".to_string()) {
                labels.push("windows".to_string());
            }
        } else {
            if !labels.contains(&"linux".to_string()) {
                labels.push("linux".to_string());
            }
        }
        if !labels.contains(&"python".to_string()) {
            labels.push("python".to_string());
        }

        labels
    }

    // ─── Runner operations ────────────────────────────────────────────────────

    /// Register a new runner
    pub async fn register_runner(&self, runner: Runner) {
        let mut runners = self.runners.write().await;
        tracing::info!(
            "Dispatcher: registered runner '{}' (type={:?}, labels={:?})",
            runner.name,
            runner.runner_type,
            runner.labels
        );
        runners.insert(runner.name.clone(), runner);
    }

    /// Remove a runner
    pub async fn unregister_runner(&self, name: &str) {
        let mut runners = self.runners.write().await;
        tracing::info!("Dispatcher: unregistered runner '{}'", name);
        runners.remove(name);
    }

    /// Get runner by name
    pub async fn get_runner(&self, name: &str) -> Option<Runner> {
        self.runners.read().await.get(name).cloned()
    }

    /// List all runners
    pub async fn list_runners(&self) -> Vec<Runner> {
        self.runners.read().await.values().cloned().collect()
    }

    /// Refresh runner heartbeat
    pub async fn runner_heartbeat(&self, name: &str) -> Option<Runner> {
        let mut runners = self.runners.write().await;
        if let Some(r) = runners.get_mut(name) {
            r.heartbeat();
            return Some(r.clone());
        }
        None
    }

    /// Mark stale runners as disconnected
    pub async fn cleanup_stale_runners(&self, timeout_secs: i64) -> Vec<String> {
        let mut runners = self.runners.write().await;
        let mut disconnected = Vec::new();
        for (name, runner) in runners.iter_mut() {
            if runner.is_stale(timeout_secs) {
                tracing::warn!(
                    "Dispatcher: runner '{}' stale (last seen {}s ago), marking offline",
                    name,
                    timeout_secs
                );
                runner.disconnect();
                disconnected.push(name.clone());
            }
        }
        disconnected
    }

    /// Find the best runner for a set of required labels
    pub async fn find_runner(&self, labels: &[String], _max_jobs: usize) -> Option<Runner> {
        let runners = self.runners.read().await;
        runners
            .values()
            .filter(|r| r.connected && r.can_accept_job() && r.matches_labels(labels))
            .min_by_key(|r| r.active_jobs.len())
            .cloned()
    }
}

/// Script validation error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptValidationError {
    pub script_path: String,
    pub script_name: String,
    pub unapproved_imports: Vec<String>,
}

// ─── Helper impl for Job label matching ────────────────────────────────────

impl Job {
    fn matches_labels(&self, required: &[String]) -> bool {
        required.iter().all(|l| self.labels.contains(l))
    }
}
