//! BuildProperty entity - key-value pairs associated with a build

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Build properties - key-value pairs associated with a build
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "build_properties")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buildid: i32,
    pub name: String,
    /// JSON encoded value
    pub value: String,
    pub source: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
