//! ChangeProperty entity - properties for changes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Properties for changes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "change_properties")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub changeid: i32,
    pub property_name: String,
    /// JSON-encoded tuple of (value, source)
    pub property_value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
