//! CodebaseBranch entity - represents a branch in a codebase

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a codebase branch
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "codebase_branches")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Codebase
    pub codebaseid: i32,
    /// Branch name
    pub name: String,
    /// SHA1 of name for unique index
    pub name_hash: String,
    /// Most recent commit
    pub commitid: Option<i32>,
    /// Timestamp when branch was last updated
    pub last_timestamp: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
