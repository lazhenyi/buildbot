//! Worker entity - represents a build worker/agent

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a worker in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "workers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    /// Worker info as JSON
    pub info: Json,
    pub paused: i32,
    pub pause_reason: Option<String>,
    pub graceful: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
