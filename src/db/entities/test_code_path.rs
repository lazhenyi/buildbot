//! TestCodePath entity - represents test code paths

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents test code paths
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "test_code_paths")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub builderid: i32,
    pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
