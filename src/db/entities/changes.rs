//! Change entity - represents a change to the source code

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a change to the source code
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "changes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub changeid: i32,
    /// Author's name (usually an email address)
    pub author: String,
    /// Committer's name
    pub committer: Option<String>,
    /// Commit comment
    pub comments: String,
    /// The branch where this change occurred (NULL means main branch)
    pub branch: Option<String>,
    /// Revision identifier for this change
    pub revision: Option<String>,
    pub revlink: Option<String>,
    /// Timestamp of the change
    pub when_timestamp: i64,
    /// An arbitrary string used for filtering changes
    pub category: Option<String>,
    /// Repository
    pub repository: String,
    /// Codebase
    pub codebase: String,
    /// Project
    pub project: String,
    /// The sourcestamp this change brought the codebase to
    pub sourcestampid: i32,
    /// The parent of the change
    pub parent_changeids: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
