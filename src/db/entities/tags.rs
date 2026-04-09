//! Tag entity - represents a builder tag

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a tag
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Tag's name
    pub name: String,
    /// SHA1 of name for unique index
    pub name_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
