//! Dispatcher Master Service - main entry point for Dispatcher-based CI system

use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

use super::config::MasterConfig;
use crate::db::Database;
use crate::dispatcher::{script::ImportMode, DispatcherState};
use crate::error::Result;
use crate::www::server::{SharedApiState, WebServer, WebServerConfig};

/// The main Master service that coordinates the Dispatcher CI system
pub struct MasterService {
    /// Master configuration
    config: MasterConfig,
    /// Database connection
    db: Arc<Database>,
    /// Shutdown signal receiver
    shutdown_rx: broadcast::Receiver<()>,
    /// Web server background task handle
    web_server_handle: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
    /// Web server shutdown trigger
    web_shutdown_tx: watch::Sender<bool>,
    /// Dispatcher state (GitHub Actions-style job dispatcher)
    dispatcher: Arc<RwLock<DispatcherState>>,
    /// Runner cleanup task handle
    runner_cleanup_handle: Option<tokio::task::JoinHandle<()>>,
    /// Job completion event receiver
    job_completion_rx: Option<tokio::sync::mpsc::Receiver<crate::dispatcher::JobCompletedEvent>>,
}

impl MasterService {
    /// Create a new MasterService
    pub fn new(config: MasterConfig, db: Database) -> Self {
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (web_shutdown_tx, _) = watch::channel(false);
        let db = Arc::new(db);
        let (dispatcher_tx, dispatcher_rx) = tokio::sync::mpsc::channel(100);
        let import_mode = if config.strict_python_deps {
            ImportMode::Strict
        } else {
            ImportMode::AllowAll
        };
        let dispatcher = Arc::new(RwLock::new(DispatcherState::new(
            dispatcher_tx,
            import_mode,
        )));
        Self {
            config,
            db: Arc::clone(&db),
            shutdown_rx,
            web_server_handle: None,
            web_shutdown_tx,
            dispatcher,
            runner_cleanup_handle: None,
            job_completion_rx: Some(dispatcher_rx),
        }
    }

    /// Start the master service
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Buildbot Dispatcher: {}", self.config.name);

        // Run database migrations
        tracing::info!("Running database migrations...");
        self.db
            .run_migrations()
            .await
            .map_err(|e| crate::error::BuildbotError::Database(e.to_string()))?;

        // Create API state for web handlers
        let api_state: SharedApiState =
            Arc::new(tokio::sync::RwLock::new(crate::api::handlers::ApiState {
                db: Arc::clone(&self.db),
                dispatcher: Arc::clone(&self.dispatcher),
            }));

        // Start web server in background
        tracing::info!("Starting web server on 0.0.0.0:{}...", self.config.web_port);
        let web_config = WebServerConfig {
            host: "0.0.0.0".to_string(),
            port: self.config.web_port,
            basedir: self.config.basedir.clone(),
            static_dir: self.config.basedir.join("www/static"),
        };
        let web_server = WebServer::new(web_config);
        let handle = web_server.spawn(Arc::clone(&api_state));
        self.web_server_handle = Some(handle);
        tracing::info!(
            "Web server started on http://0.0.0.0:{}",
            self.config.web_port
        );

        // Start runner cleanup task
        tracing::info!("Starting runner cleanup task...");
        let dispatcher = Arc::clone(&self.dispatcher);
        let runner_timeout = self.config.runner_timeout_secs;
        let cleanup_interval = std::time::Duration::from_secs(60); // Check every 60 seconds
        let cleanup_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let dispatcher = dispatcher.read().await;
                let disconnected = dispatcher.cleanup_stale_runners(runner_timeout).await;
                if !disconnected.is_empty() {
                    tracing::info!(
                        "Marked {} runners as disconnected: {:?}",
                        disconnected.len(),
                        disconnected
                    );
                }
            }
        });
        self.runner_cleanup_handle = Some(cleanup_handle);

        // Start job completion handler
        tracing::info!("Starting job completion handler...");
        let mut completion_rx = self
            .job_completion_rx
            .take()
            .expect("job completion rx already taken");
        let db = Arc::clone(&self.db);
        let callback = self.config.build_complete_callback.clone();
        let completion_handle = tokio::spawn(async move {
            while let Some(event) = completion_rx.recv().await {
                tracing::info!(
                    "Job completed: {} (status: {:?}, exit_code: {:?})",
                    event.job_name,
                    event.status,
                    event.exit_code
                );

                // Update job status in database
                if let Err(e) = db
                    .update_job_status(
                        &event.job_id,
                        &event.status.to_string(),
                        event.exit_code,
                        event.error_message.clone(),
                    )
                    .await
                {
                    tracing::error!("Failed to update job status: {}", e);
                }

                // Call external callback if configured
                if let Some(ref callback_url) = callback {
                    let payload = serde_json::json!({
                        "job_id": event.job_id,
                        "job_name": event.job_name,
                        "status": event.status.to_string(),
                        "exit_code": event.exit_code,
                        "error_message": event.error_message,
                        "duration_secs": event.duration_secs,
                    });

                    match reqwest::Client::new()
                        .post(callback_url)
                        .json(&payload)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            tracing::info!("Job completion callback sent: {}", resp.status());
                        }
                        Err(e) => {
                            tracing::error!("Failed to send job completion callback: {}", e);
                        }
                    }
                }
            }
        });
        // Store the handle so it doesn't get dropped
        tokio::spawn(async move {
            completion_handle.await.ok();
        });

        tracing::info!("Buildbot Dispatcher started successfully");
        Ok(())
    }

    /// Stop the master service
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping Buildbot Dispatcher...");

        // Stop runner cleanup task
        if let Some(handle) = self.runner_cleanup_handle.take() {
            handle.abort();
            tracing::info!("Runner cleanup task stopped");
        }

        // Signal web server to stop
        let _ = self.web_shutdown_tx.send(true);

        // Wait for web server to stop
        if let Some(handle) = self.web_server_handle.take() {
            let _ = handle.await;
        }

        tracing::info!("Buildbot Dispatcher stopped");
        Ok(())
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&mut self) {
        let _ = self.shutdown_rx.recv().await;
    }

    /// Get the database reference
    pub fn db(&self) -> Arc<Database> {
        Arc::clone(&self.db)
    }

    /// Get the dispatcher reference
    pub fn dispatcher(&self) -> Arc<RwLock<DispatcherState>> {
        Arc::clone(&self.dispatcher)
    }
}
