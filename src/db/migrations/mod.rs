//! Database migrations module
//!
//! Uses SeaORM's migration framework for versioned schema management.
//!
//! Migrations are defined in:
//! - `m20250101_init`: Core tables (masters, workers, builders, builds, steps, logs, changes)
//! - `m20250102_secondary`: Secondary tables (schedulers, codebases, users, test results)
//! - `m20250103_dispatcher`: Dispatcher tables (dispatcher_jobs, dispatcher_runners) + indexes

pub mod initializer;
pub mod m20250101_init;
pub mod m20250102_secondary;
pub mod m20250103_dispatcher;

pub use initializer::init_database;
