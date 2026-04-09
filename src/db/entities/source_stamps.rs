//! SourceStamp entity - identifies a particular instance of source code

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a sourcestamp - identifies a particular instance of source code
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sourcestamps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Hash of branch, revision, patchid, repository, codebase, and project
    pub ss_hash: String,
    /// The branch to check out (NULL means main branch)
    pub branch: Option<String>,
    /// The revision to check out, or latest if NULL
    pub revision: Option<String>,
    /// The patch to apply
    pub patchid: Option<i32>,
    /// The repository from which this source should be checked out
    pub repository: String,
    /// Codebase - logical name to specify what is in the repository
    pub codebase: String,
    /// The project this source code represents
    pub project: String,
    /// The time this sourcestamp was first seen
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
