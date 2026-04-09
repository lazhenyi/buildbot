//! Web server for Buildbot UI

use actix_files::Files;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::path::PathBuf;
use std::sync::Arc;

/// Web server configuration
#[derive(Clone)]
pub struct WebServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Base directory
    pub basedir: PathBuf,
    /// Static files directory
    pub static_dir: PathBuf,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8010,
            basedir: PathBuf::from("."),
            static_dir: PathBuf::from("www/static"),
        }
    }
}

/// Shared API state for web handlers
pub type SharedApiState = Arc<tokio::sync::RwLock<crate::api::handlers::ApiState>>;

/// Buildbot web server
pub struct WebServer {
    config: WebServerConfig,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl WebServer {
    pub fn new(config: WebServerConfig) -> Self {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        Self {
            config,
            shutdown_tx,
        }
    }

    pub fn shutdown_handle(&self) -> tokio::sync::watch::Sender<bool> {
        self.shutdown_tx.clone()
    }

    pub fn spawn(self, api_state: SharedApiState) -> tokio::task::JoinHandle<std::io::Result<()>> {
        let host = self.config.host.clone();
        let port = self.config.port;
        let static_dir = self.config.static_dir.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            tracing::info!("Starting web server on {}:{}", host, port);

            let server = HttpServer::new(move || {
                let app = App::new()
                    .app_data(web::Data::new(Arc::clone(&api_state)))
                    .wrap(middleware::Logger::default())
                    .configure(crate::api::routes::configure_routes)
                    // UI pages
                    .route("/", web::get().to(index_handler))
                    .route("/dashboard", web::get().to(dashboard_handler))
                    .route("/jobs", web::get().to(jobs_handler))
                    .route("/jobs/{id}", web::get().to(job_detail_handler))
                    .route("/runners", web::get().to(runners_handler));

                if static_dir.is_dir() {
                    app.service(Files::new("/static", &static_dir).show_files_listing())
                } else {
                    tracing::warn!("Static directory '{}' does not exist", static_dir.display());
                    app
                }
            })
            .bind(format!("{}:{}", host, port))?
            .disable_signals();

            tokio::select! {
                result = server.run() => result,
                _ = shutdown_rx.changed() => {
                    tracing::info!("Web server received shutdown signal");
                    Ok(())
                }
            }
        })
    }
}

async fn index_handler() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../www/index.html").to_string())
}

async fn dashboard_handler() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../www/dashboard.html").to_string())
}

async fn jobs_handler() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../www/jobs.html").to_string())
}

async fn job_detail_handler(path: web::Path<String>) -> HttpResponse {
    let job_id = path.into_inner();
    let html = include_str!("../../www/job_detail.html").replace("{{JOB_ID}}", &job_id);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn runners_handler() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../www/runners.html").to_string())
}
