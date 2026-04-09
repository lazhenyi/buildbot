//! Master Service - main service for Dispatcher-based CI system

pub mod config;
pub mod service;

pub use config::MasterConfig;
pub use service::MasterService;
