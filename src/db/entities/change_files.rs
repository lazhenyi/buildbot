//! ChangeFile entity - files touched in changes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Files touched in changes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "change_files")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub changeid: i32,
    pub filename: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
