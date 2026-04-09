//! Dispatcher Jobs entity - stores dispatcher job state

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a dispatcher job in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatcher_jobs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Unique job UUID (string)
    pub job_id: String,
    /// Job name
    pub name: String,
    /// Numeric sort key
    pub sort_key: i32,
    /// Job status (string)
    pub status: String,
    /// JSON array of labels
    pub labels: String, // JSON
    /// Job source type (webhook/periodic/force)
    pub source_type: String,
    /// Source JSON data
    pub source_json: String, // JSON
    /// Repository URL
    pub repository_url: String,
    /// Git branch
    pub branch: String,
    /// Git revision (optional)
    pub revision: Option<String>,
    /// Runner name (assigned, optional)
    pub runner_name: Option<String>,
    /// JSON environment variables
    pub env_json: String, // JSON
    /// Exit code (optional)
    pub exit_code: Option<i32>,
    /// Error message (optional)
    pub error_message: Option<String>,
    /// Script path inside repository
    pub script_path: String,
    /// Working directory
    pub workdir: String,
    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,
    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
    /// Start timestamp (optional, Unix timestamp)
    pub started_at: Option<i64>,
    /// Finish timestamp (optional, Unix timestamp)
    pub finished_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
