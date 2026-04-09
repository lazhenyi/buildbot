//! Codebase entity - represents a logical group of source code

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a codebase
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "codebases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Project
    pub projectid: i32,
    /// Codebase name
    pub name: String,
    /// SHA1 of name for unique index
    pub name_hash: String,
    /// Codebase slug for URLs
    pub slug: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
