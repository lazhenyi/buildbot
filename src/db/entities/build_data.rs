//! BuildData entity - transient build state

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Build data - transient build state
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "build_data")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buildid: i32,
    pub name: String,
    pub value: Vec<u8>,
    pub length: i32,
    pub source: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
