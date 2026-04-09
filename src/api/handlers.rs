//! API request handlers
//!
//! REST API handlers for the Dispatcher CI system.

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::Database;
use crate::dispatcher::runner::{Runner, RunnerCapabilities, RunnerType};
use crate::dispatcher::{job::JobStatus, DispatcherState};
use crate::www::server::SharedApiState;

/// Application state shared across handlers
pub struct ApiState {
    pub db: Arc<Database>,
    pub dispatcher: Arc<RwLock<DispatcherState>>,
}

// ─────────────────────────────────────────────────────────────
// Health check
// ─────────────────────────────────────────────────────────────

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "buildbot-dispatcher"
    }))
}

// ─────────────────────────────────────────────────────────────
// Webhook handlers
// ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct GitHubWebhookPayload {
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub after: Option<String>,
    pub repository: Option<GitHubRepository>,
    pub pusher: Option<GitHubPusher>,
    pub head_commit: Option<GitHubCommit>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubRepository {
    pub full_name: Option<String>,
    pub clone_url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubPusher {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubCommit {
    pub id: Option<String>,
    pub message: Option<String>,
    pub author: Option<GitHubAuthor>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubAuthor {
    pub name: Option<String>,
    pub email: Option<String>,
}

pub async fn handle_github_webhook(
    state: web::Data<SharedApiState>,
    payload: web::Json<GitHubWebhookPayload>,
) -> impl Responder {
    let repo = payload.repository.as_ref();
    let _pusher = payload.pusher.as_ref();
    let commit = payload.head_commit.as_ref();

    let repository = repo
        .and_then(|r| r.clone_url.as_ref())
        .or_else(|| repo.and_then(|r| r.full_name.as_ref()))
        .map(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();

    let revision = commit
        .and_then(|c| c.id.as_ref())
        .map(|s| s.as_str())
        .or_else(|| payload.after.as_deref());

    let branch = payload
        .git_ref
        .as_ref()
        .and_then(|r| r.strip_prefix("refs/heads/"))
        .map(String::from);

    tracing::info!(
        "GitHub webhook: repository={}, branch={:?}, revision={:?}",
        repository,
        branch,
        revision
    );

    // Dispatcher: clone repository and scan CI scripts
    if let (Some(clone_url), Some(branch_name)) = (Some(repository.as_str()), branch.as_ref()) {
        let state_clone = Arc::clone(&state);
        let clone_url = clone_url.to_string();
        let branch_name = branch_name.to_string();
        let revision = revision.map(|s| s.to_string());

        tokio::spawn(async move {
            use crate::dispatcher::{clone_repository, scan_ci_directory, JobSource};

            // Generate safe workdir name from repo URL
            let repo_hash = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                clone_url.hash(&mut hasher);
                format!("{:x}", hasher.finish())
            };

            // Use temp directory for dispatcher repos
            let workdir = std::env::temp_dir()
                .join("buildbot_dispatcher")
                .join(&repo_hash);

            tracing::info!(
                "Dispatcher: cloning {} (branch: {}) to {}",
                clone_url,
                branch_name,
                workdir.display()
            );

            // Clone or update repository
            match clone_repository(&clone_url, &branch_name, &workdir, None).await {
                Ok(()) => {
                    tracing::info!("Dispatcher: repository cloned successfully");

                    // Get latest revision if not provided
                    let rev = if let Some(r) = revision {
                        r
                    } else {
                        match crate::dispatcher::get_latest_revision(&workdir).await {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::error!("Failed to get revision: {}", e);
                                return;
                            }
                        }
                    };

                    // Scan CI directory
                    let ci_dir = workdir.join(".ci");
                    let requirements_path = workdir.join("requirements.txt");

                    let guard = state_clone.read().await;
                    let dispatcher = guard.dispatcher.read().await;

                    match scan_ci_directory(
                        &dispatcher,
                        &clone_url,
                        &branch_name,
                        Some(&rev),
                        &ci_dir,
                        &requirements_path,
                        JobSource::Webhook {
                            repository_id: 0,
                            branch: branch_name.clone(),
                        },
                        vec![],
                    )
                    .await
                    {
                        Ok(count) => {
                            tracing::info!("Dispatcher: enqueued {} jobs from CI directory", count);
                        }
                        Err(e) => {
                            tracing::error!("Dispatcher: failed to scan CI directory: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Dispatcher: failed to clone repository: {}", e);
                }
            }
        });
    }

    HttpResponse::Ok().json(json!({
        "status": "ok",
        "received": true,
        "message": "Webhook received, jobs will be enqueued",
    }))
}

// ─────────────────────────────────────────────────────────────
// Dispatcher API handlers
// ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PollQuery {
    /// Comma-separated list of required labels to filter jobs
    pub labels: Option<String>,
    /// Filter jobs by status (Pending, Running, Success, Failed, Cancelled, Lost)
    pub status: Option<String>,
    /// Name of the runner polling for jobs
    pub runner_name: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRunnerRequest {
    pub name: String,
    pub runner_type: String,
    pub labels: Vec<String>,
    pub capabilities: Option<RunnerCapabilities>,
    pub max_jobs: Option<i32>,
}

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct CompleteJobRequest {
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub duration_secs: Option<f64>,
}

/// Get dispatcher info
pub async fn get_dispatcher_info(state: web::Data<SharedApiState>) -> impl Responder {
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;
    let jobs = dispatcher.list_jobs(None).await;
    let runners = dispatcher.list_runners().await;

    HttpResponse::Ok().json(json!({
        "total_jobs": jobs.len(),
        "pending_jobs": jobs.iter().filter(|j| j.status == JobStatus::Pending).count(),
        "running_jobs": jobs.iter().filter(|j| j.status == JobStatus::Running).count(),
        "completed_jobs": jobs.iter().filter(|j| matches!(j.status, JobStatus::Success | JobStatus::Failed | JobStatus::Cancelled | JobStatus::Lost)).count(),
        "total_runners": runners.len(),
        "connected_runners": runners.iter().filter(|r| r.connected).count(),
    }))
}

/// Get all jobs
pub async fn get_jobs(
    state: web::Data<SharedApiState>,
    query: web::Query<PollQuery>,
) -> impl Responder {
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    let status_filter = query
        .status
        .as_ref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "pending" => Some(JobStatus::Pending),
            "running" => Some(JobStatus::Running),
            "success" => Some(JobStatus::Success),
            "failed" => Some(JobStatus::Failed),
            "cancelled" => Some(JobStatus::Cancelled),
            "lost" => Some(JobStatus::Lost),
            _ => None,
        });

    let mut jobs = dispatcher.list_jobs(status_filter).await;

    // Filter by labels if provided
    if let Some(ref labels_str) = query.labels {
        let required: Vec<String> = labels_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !required.is_empty() {
            jobs.retain(|j| required.iter().all(|l| j.labels.contains(l)));
        }
    }

    #[derive(serde::Serialize)]
    struct JobInfo {
        id: String,
        name: String,
        status: String,
        labels: Vec<String>,
        repository_url: String,
        branch: String,
        runner_name: Option<String>,
    }

    let job_infos: Vec<JobInfo> = jobs
        .iter()
        .map(|j| JobInfo {
            id: j.id.clone(),
            name: j.name.clone(),
            status: format!("{:?}", j.status),
            labels: j.labels.clone(),
            repository_url: j.repository_url.clone(),
            branch: j.branch.clone(),
            runner_name: j.runner_name.clone(),
        })
        .collect();

    HttpResponse::Ok().json(json!({
        "total": job_infos.len(),
        "jobs": job_infos,
    }))
}

/// Get a specific job
pub async fn get_job(state: web::Data<SharedApiState>, path: web::Path<String>) -> impl Responder {
    let job_id = path.into_inner();
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    if let Some(job) = dispatcher.get_job(&job_id).await {
        HttpResponse::Ok().json(json!({
            "id": job.id,
            "name": job.name,
            "status": format!("{:?}", job.status),
            "labels": job.labels,
            "repository_url": job.repository_url,
            "branch": job.branch,
            "revision": job.revision,
            "runner_name": job.runner_name,
            "script_path": job.script_path,
            "workdir": job.workdir,
            "exit_code": job.exit_code,
            "error_message": job.error_message,
        }))
    } else {
        HttpResponse::NotFound().json(json!({
            "error": "Job not found",
            "job_id": job_id,
        }))
    }
}

/// Cancel a job
pub async fn cancel_job(
    state: web::Data<SharedApiState>,
    path: web::Path<String>,
) -> impl Responder {
    let job_id = path.into_inner();
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    match dispatcher.cancel_job(&job_id).await {
        Some(job) => {
            tracing::info!("Job {} cancelled", job_id);
            HttpResponse::Ok().json(json!({
                "status": "ok",
                "job_id": job.id,
                "message": "Job cancelled",
            }))
        }
        None => HttpResponse::NotFound().json(json!({
            "error": "Job not found or cannot be cancelled",
            "job_id": job_id,
        })),
    }
}

/// Poll for a job (Runner API)
pub async fn poll_job(
    state: web::Data<SharedApiState>,
    query: web::Query<PollQuery>,
) -> impl Responder {
    let labels: Vec<String> = query
        .labels
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let runner_name = query
        .runner_name
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "runner".to_string());

    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    match dispatcher.poll_job(&runner_name, &labels).await {
        Some(job) => {
            tracing::info!("Runner '{}' polled job '{}'", runner_name, job.id);
            HttpResponse::Ok().json(json!({
                "job_id": job.id,
                "name": job.name,
                "labels": job.labels,
                "repository_url": job.repository_url,
                "branch": job.branch,
                "revision": job.revision,
                "script_path": job.script_path,
                "workdir": job.workdir,
                "env": job.env,
            }))
        }
        None => HttpResponse::Ok().json(json!({
            "message": "No pending jobs available",
        })),
    }
}

/// Complete a job (Runner API)
pub async fn complete_job(
    state: web::Data<SharedApiState>,
    path: web::Path<String>,
    body: web::Json<CompleteJobRequest>,
) -> impl Responder {
    let job_id = path.into_inner();
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    match dispatcher
        .complete_job(
            &job_id,
            body.exit_code,
            body.error_message.clone(),
            body.duration_secs,
        )
        .await
    {
        Some(job) => {
            tracing::info!(
                "Job {} completed with exit_code {:?}",
                job_id,
                body.exit_code
            );
            HttpResponse::Ok().json(json!({
                "status": "ok",
                "job_id": job.id,
                "final_status": format!("{:?}", job.status),
            }))
        }
        None => HttpResponse::NotFound().json(json!({
            "error": "Job not found",
            "job_id": job_id,
        })),
    }
}

/// Get all runners
pub async fn get_runners(state: web::Data<SharedApiState>) -> impl Responder {
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;
    let runners = dispatcher.list_runners().await;

    #[derive(serde::Serialize)]
    struct RunnerInfo {
        name: String,
        runner_type: String,
        labels: Vec<String>,
        connected: bool,
        status: String,
        max_jobs: i32,
        active_jobs: usize,
        last_heartbeat_at: i64,
    }

    let runner_infos: Vec<RunnerInfo> = runners
        .iter()
        .map(|r| RunnerInfo {
            name: r.name.clone(),
            runner_type: format!("{:?}", r.runner_type),
            labels: r.labels.clone(),
            connected: r.connected,
            status: r.status.clone(),
            max_jobs: r.max_jobs as i32,
            active_jobs: r.active_jobs.len(),
            last_heartbeat_at: r.last_heartbeat_at.timestamp(),
        })
        .collect();

    HttpResponse::Ok().json(json!({
        "total": runner_infos.len(),
        "runners": runner_infos,
    }))
}

/// Register a runner
pub async fn register_runner(
    state: web::Data<SharedApiState>,
    body: web::Json<RegisterRunnerRequest>,
) -> impl Responder {
    let runner_type = match body.runner_type.as_str() {
        "persistent" => RunnerType::Persistent,
        "ephemeral" => RunnerType::Ephemeral,
        _ => RunnerType::Ephemeral,
    };

    let runner = Runner::new(body.name.clone(), runner_type, body.labels.clone());

    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;
    dispatcher.register_runner(runner).await;

    tracing::info!("Runner '{}' registered", body.name);
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "name": body.name,
        "message": "Runner registered successfully",
    }))
}

/// Runner heartbeat
pub async fn runner_heartbeat(
    state: web::Data<SharedApiState>,
    body: web::Json<HeartbeatRequest>,
) -> impl Responder {
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;

    match dispatcher.runner_heartbeat(&body.name).await {
        Some(runner) => HttpResponse::Ok().json(json!({
            "status": "ok",
            "name": runner.name,
            "message": "Heartbeat received",
        })),
        None => HttpResponse::NotFound().json(json!({
            "error": "Runner not found",
            "name": body.name,
        })),
    }
}

/// Unregister a runner
pub async fn unregister_runner(
    state: web::Data<SharedApiState>,
    path: web::Path<String>,
) -> impl Responder {
    let name = path.into_inner();
    let guard = state.read().await;
    let dispatcher = guard.dispatcher.read().await;
    dispatcher.unregister_runner(&name).await;

    tracing::info!("Runner '{}' unregistered", name);
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "name": name,
        "message": "Runner unregistered",
    }))
}
