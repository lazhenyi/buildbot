//! CodebaseCommit entity - represents a commit in a codebase

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a codebase commit
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "codebase_commits")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Codebase
    pub codebaseid: i32,
    /// Author's name
    pub author: String,
    /// Committer's name
    pub committer: Option<String>,
    /// Commit comment
    pub comments: String,
    /// Timestamp of the revision
    pub when_timestamp: i64,
    /// Revision identifier
    pub revision: String,
    /// Parent commit
    pub parent_commitid: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
