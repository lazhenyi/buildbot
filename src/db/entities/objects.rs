//! Object entity - for tracking object state

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents an object that needs to maintain state
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "objects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Object's user-given name
    pub name: String,
    /// Object's class name
    pub class_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
