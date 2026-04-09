//! ChangeSource entity - represents a source of code changes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a changesource
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "changesources")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Name for this changesource
    pub name: String,
    /// SHA1 hash of name for unique index
    pub name_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
