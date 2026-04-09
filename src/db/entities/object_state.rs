//! ObjectState entity - stores key/value pairs for objects

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Stores key/value pairs for objects
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "object_state")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub objectid: i32,
    /// Name for this value (local to the object)
    pub name: String,
    /// Value as JSON string
    pub value_json: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
