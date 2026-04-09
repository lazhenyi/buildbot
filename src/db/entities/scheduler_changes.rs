//! SchedulerChange entity - classified changes that have not yet been processed

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents classified changes that have not yet been processed
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduler_changes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub schedulerid: i32,
    pub changeid: i32,
    /// True (nonzero) if this change is important to this scheduler
    pub important: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
