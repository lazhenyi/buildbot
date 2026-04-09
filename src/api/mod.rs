//! API module
//!
//! REST API endpoints for Buildbot using Actix-web.

pub mod routes;
pub mod handlers;

pub use routes::configure_routes;
