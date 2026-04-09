//! Dispatcher Runners entity - stores runner registration and state

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a registered runner in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatcher_runners")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Unique runner name
    pub name: String,
    /// Runner type (persistent/ephemeral)
    pub runner_type: String,
    /// JSON array of labels
    pub labels: String, // JSON
    /// JSON capabilities
    pub capabilities_json: String, // JSON
    /// Last heartbeat timestamp (Unix timestamp)
    pub last_heartbeat_at: i64,
    /// Registration timestamp (Unix timestamp)
    pub registered_at: i64,
    /// JSON array of active job IDs
    pub active_jobs_json: String, // JSON
    /// Max concurrent jobs
    pub max_jobs: i32,
    /// Whether runner is connected
    pub connected: bool,
    /// Runner status description
    pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
