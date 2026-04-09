//! Buildbot - Rust CI/CD Dispatcher (GitHub Actions Style)
//!
//! This crate provides a GitHub Actions-style CI dispatcher with:
//! - Job/Runner model for scalable build execution
//! - Docker sandbox for secure script isolation
//! - Python script scanning from `.ci/` directories
//! - Matrix build support
//! - GitHub webhook integration

pub mod api;
pub mod config;
pub mod db;
pub mod dispatcher;
pub mod error;
pub mod master;
pub mod www;

pub use error::BuildbotError;
