//! Build entity - represents a single build execution

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a build in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "builds")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub number: i32,
    pub builderid: i32,
    pub buildrequestid: i32,
    /// Worker which performed this build (nullable for worker-free builds)
    pub workerid: Option<i32>,
    /// Master which controlled this build
    pub masterid: i32,
    /// Start/complete times
    pub started_at: i64,
    pub complete_at: Option<i64>,
    /// Total duration that completed steps spent waiting for locks
    pub locks_duration_s: i32,
    pub state_string: String,
    pub results: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
