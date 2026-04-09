//! Worker module
//!
//! This module provides worker-related functionality for the master side.

pub mod worker;
pub mod connection;

pub use worker::Worker;
pub use connection::WorkerConnection;
