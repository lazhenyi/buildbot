//! API module
//!
//! REST API endpoints for Buildbot using Actix-web.

pub mod handlers;
pub mod routes;

pub use routes::configure_routes;
