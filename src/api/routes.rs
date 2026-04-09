//! API routes configuration

use super::handlers;
use actix_web::web;

/// Configure API routes with shared application state
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // Health check
            .route("/health", web::get().to(handlers::health_check))
            // Webhooks
            .route(
                "/hooks/github",
                web::post().to(handlers::handle_github_webhook),
            )
            // Dispatcher
            .route("/dispatcher", web::get().to(handlers::get_dispatcher_info))
            .route("/dispatcher/jobs", web::get().to(handlers::get_jobs))
            .route("/dispatcher/jobs/poll", web::get().to(handlers::poll_job))
            .route(
                "/dispatcher/jobs/{job_id}",
                web::get().to(handlers::get_job),
            )
            .route(
                "/dispatcher/jobs/{job_id}/cancel",
                web::post().to(handlers::cancel_job),
            )
            .route(
                "/dispatcher/jobs/{job_id}/complete",
                web::post().to(handlers::complete_job),
            )
            .route("/dispatcher/runners", web::get().to(handlers::get_runners))
            .route(
                "/dispatcher/runners/register",
                web::post().to(handlers::register_runner),
            )
            .route(
                "/dispatcher/runners/heartbeat",
                web::post().to(handlers::runner_heartbeat),
            )
            .route(
                "/dispatcher/runners/{name}",
                web::delete().to(handlers::unregister_runner),
            ),
    );
}
