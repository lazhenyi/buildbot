//! Integration tests for Buildbot Dispatcher
//!
//! Run with: cargo test

use buildbot::config::loader::{ConfigLoader, YamlConfig};
use buildbot::db::Database;
use buildbot::dispatcher::script::ImportMode;
use buildbot::dispatcher::{DispatcherState, Job, JobSource, JobStatus, Runner, RunnerType};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────
// Database tests
// ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_database_connection() {
    let db = Database::new("sqlite::memory:").await;
    assert!(db.is_ok());

    let db = db.unwrap();
    let alive = db.ping().await;
    assert!(alive.is_ok());
    assert!(alive.unwrap());
}

#[tokio::test]
async fn test_database_migrations_run() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("buildbot_test_{}.db", std::process::id()));
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let db = Database::new(&db_url).await.unwrap();
    let result = db.run_migrations().await;
    assert!(result.is_ok(), "Database migrations should succeed");

    let alive = db.ping().await;
    assert!(alive.is_ok());

    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_dispatcher_job_status_update() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("buildbot_disp_job_test_{}.db", std::process::id()));
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let db = Arc::new(Database::new(&db_url).await.expect("create temp DB"));
    db.run_migrations().await.expect("run migrations");

    let result = db
        .update_job_status("test-job-001", "Success", Some(0), None)
        .await;
    assert!(
        result.is_ok(),
        "update_job_status should succeed: {:?}",
        result
    );

    let _ = std::fs::remove_file(&db_path);
}

// ─────────────────────────────────────────────────────────────
// Dispatcher state tests
// ─────────────────────────────────────────────────────────────

fn make_test_job(id: &str, name: &str, labels: Vec<String>, status: JobStatus) -> Job {
    let now = Utc::now();
    Job {
        id: id.to_string(),
        name: name.to_string(),
        sort_key: 1,
        status,
        labels,
        source: JobSource::Webhook {
            repository_id: 0,
            branch: "main".to_string(),
        },
        repository_url: "https://github.com/test/repo".to_string(),
        branch: "main".to_string(),
        revision: Some("abc123".to_string()),
        runner_name: None,
        env: HashMap::new(),
        exit_code: None,
        error_message: None,
        created_at: now,
        updated_at: now,
        started_at: None,
        finished_at: None,
        script_path: ".ci/01_test.py".to_string(),
        workdir: "/tmp/work".to_string(),
    }
}

fn make_dispatcher() -> DispatcherState {
    let (tx, _rx) = tokio::sync::mpsc::channel(100);
    DispatcherState::new(tx, ImportMode::AllowAll)
}

#[tokio::test]
async fn test_dispatcher_enqueuing_job() {
    let dispatcher = make_dispatcher();
    let job = make_test_job(
        "job-enqueue-1",
        "enqueue-test",
        vec!["test".to_string()],
        JobStatus::Pending,
    );

    dispatcher.enqueue(job).await;
    let jobs = dispatcher.list_jobs(None).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, "job-enqueue-1");
    assert!(matches!(jobs[0].status, JobStatus::Pending));
}

#[tokio::test]
async fn test_dispatcher_poll_job_assigns_to_runner() {
    let dispatcher = make_dispatcher();
    let job = make_test_job(
        "job-poll-1",
        "poll-test",
        vec!["linux".to_string()],
        JobStatus::Pending,
    );

    dispatcher.enqueue(job).await;

    let polled = dispatcher
        .poll_job("linux-runner-1", &["linux".to_string()])
        .await;
    assert!(
        polled.is_some(),
        "Should return a pending job with matching labels"
    );

    let polled_job = polled.unwrap();
    assert_eq!(polled_job.id, "job-poll-1");
    assert_eq!(polled_job.runner_name.as_deref(), Some("linux-runner-1"));
}

#[tokio::test]
async fn test_dispatcher_poll_job_no_matching_labels() {
    let dispatcher = make_dispatcher();
    let job = make_test_job(
        "job-poll-2",
        "poll-test-2",
        vec!["windows".to_string()],
        JobStatus::Pending,
    );

    dispatcher.enqueue(job).await;

    let polled = dispatcher
        .poll_job("linux-runner", &["linux".to_string()])
        .await;
    assert!(
        polled.is_none(),
        "Should not return job with non-matching labels"
    );
}

#[tokio::test]
async fn test_dispatcher_complete_job_updates_status() {
    let dispatcher = make_dispatcher();
    let job = make_test_job(
        "job-complete-1",
        "complete-test",
        vec![],
        JobStatus::Running,
    );

    dispatcher.enqueue(job).await;

    let completed = dispatcher
        .complete_job("job-complete-1", Some(0), None, Some(30.0))
        .await;
    assert!(completed.is_some());

    let jobs = dispatcher.list_jobs(None).await;
    assert!(jobs[0].exit_code.is_some());
    assert_eq!(jobs[0].exit_code.unwrap(), 0);
}

#[tokio::test]
async fn test_dispatcher_cancel_pending_job() {
    let dispatcher = make_dispatcher();
    let job = make_test_job("job-cancel-1", "cancel-test", vec![], JobStatus::Pending);

    dispatcher.enqueue(job).await;
    let cancelled = dispatcher.cancel_job("job-cancel-1").await;
    assert!(cancelled.is_some());
    assert!(matches!(cancelled.unwrap().status, JobStatus::Cancelled));
}

#[tokio::test]
async fn test_dispatcher_get_job() {
    let dispatcher = make_dispatcher();
    let job = make_test_job("job-get-1", "get-test", vec![], JobStatus::Pending);

    dispatcher.enqueue(job).await;

    let found = dispatcher.get_job("job-get-1").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "get-test");

    let not_found = dispatcher.get_job("nonexistent").await;
    assert!(not_found.is_none());
}

// ─────────────────────────────────────────────────────────────
// Runner tests
// ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_runner_registration() {
    let dispatcher = make_dispatcher();
    let runner = Runner::new(
        "test-runner-1".to_string(),
        RunnerType::Persistent,
        vec!["linux".to_string(), "docker".to_string()],
    );

    dispatcher.register_runner(runner).await;
    let runners = dispatcher.list_runners().await;
    assert_eq!(runners.len(), 1);
    assert_eq!(runners[0].name, "test-runner-1");
    assert!(runners[0].connected);
}

#[tokio::test]
async fn test_runner_heartbeat() {
    let dispatcher = make_dispatcher();
    let runner = Runner::new(
        "heartbeat-runner".to_string(),
        RunnerType::Ephemeral,
        vec![],
    );
    dispatcher.register_runner(runner).await;

    let updated = dispatcher.runner_heartbeat("heartbeat-runner").await;
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().name, "heartbeat-runner");
}

#[tokio::test]
async fn test_runner_heartbeat_not_found() {
    let dispatcher = make_dispatcher();
    let updated = dispatcher.runner_heartbeat("nonexistent-runner").await;
    assert!(updated.is_none());
}

#[tokio::test]
async fn test_runner_unregistration() {
    let dispatcher = make_dispatcher();
    let runner = Runner::new("unreg-runner".to_string(), RunnerType::Persistent, vec![]);
    dispatcher.register_runner(runner).await;
    assert_eq!(dispatcher.list_runners().await.len(), 1);

    dispatcher.unregister_runner("unreg-runner").await;
    assert!(dispatcher.list_runners().await.is_empty());
}

// ─────────────────────────────────────────────────────────────
// Config loading tests
// ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_yaml_config_minimal() {
    let yaml = r#"
"#;

    let config: YamlConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.master.is_none());
    assert!(config.database.is_none());
    assert!(config.www.is_none());
}

#[tokio::test]
async fn test_yaml_config_full() {
    let yaml = r#"
master:
  name: "dispatcher-1"
  web_url: "http://localhost:8080"
  strict_python_deps: true
  dispatcher_workdir: "/var/lib/buildbot/repos"
  runner_timeout_secs: 300
database:
  url: "sqlite:dispatcher.db"
www:
  port: 9000
  web_port: 9001
"#;

    let config: YamlConfig = serde_yaml::from_str(yaml).unwrap();

    let master = config.master.as_ref().unwrap();
    assert_eq!(master.name, "dispatcher-1");
    assert!(master.strict_python_deps.unwrap_or(false));

    let db_section = config.database.as_ref().unwrap();
    assert_eq!(db_section.url.as_deref(), Some("sqlite:dispatcher.db"));

    let www = config.www.as_ref().unwrap();
    assert_eq!(www.port, 9000);
    assert_eq!(www.web_port.unwrap_or(9001), 9001);
}

#[tokio::test]
async fn test_config_loader_reads_file() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join(format!("buildbot_config_test_{}.yaml", std::process::id()));
    let config_content = r#"
master:
  name: "test-dispatcher"
  web_url: "http://localhost:9999"
database:
  url: "sqlite:test.db"
"#;

    std::fs::write(&config_path, config_content).unwrap();

    let loader = ConfigLoader::new();
    let result = loader.load_from_file(&config_path).await;
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.master.as_ref().unwrap().name, "test-dispatcher");

    let _ = std::fs::remove_file(&config_path);
}
