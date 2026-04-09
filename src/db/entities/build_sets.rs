//! BuildSet entity - represents a set of BuildRequests that share the same cause

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a buildset - a collection of build requests
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buildsets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// A simple external identifier to track down this buildset
    pub external_idstring: Option<String>,
    /// A short string giving the reason the buildset was created
    pub reason: String,
    pub submitted_at: i64,
    /// If this is zero, then the build set is still pending
    pub complete: i32,
    pub complete_at: Option<i64>,
    /// Results is only valid when complete == 1
    pub results: Option<i32>,
    /// Optional parent build
    pub parent_buildid: Option<i32>,
    /// Text describing the relationship with the build
    pub parent_relationship: Option<String>,
    /// Optional rebuilt build id
    pub rebuilt_buildid: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
