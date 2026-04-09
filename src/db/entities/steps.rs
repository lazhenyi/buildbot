//! Step entity - represents a build step

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a build step
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "steps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub number: i32,
    pub name: String,
    pub buildid: i32,
    pub started_at: Option<i64>,
    pub locks_acquired_at: Option<i64>,
    pub complete_at: Option<i64>,
    pub state_string: String,
    pub results: Option<i32>,
    pub urls_json: String,
    pub hidden: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
