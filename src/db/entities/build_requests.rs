//! BuildRequest entity - represents a request for a build to be performed

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a build request
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buildrequests")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buildsetid: i32,
    pub builderid: i32,
    pub priority: i32,
    /// If this is zero, then the build is still pending
    pub complete: i32,
    /// Results is only valid when complete == 1
    /// 0 = SUCCESS, 1 = WARNINGS, etc.
    pub results: Option<i32>,
    /// Time the buildrequest was created
    pub submitted_at: i64,
    /// Time the buildrequest was completed, or NULL
    pub complete_at: Option<i64>,
    /// Boolean indicating whether there is a step blocking
    pub waited_for: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
