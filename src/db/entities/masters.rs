//! Master entity - represents a Buildbot master instance

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a Buildbot master in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "masters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Master's name (generally in the form hostname:basedir)
    pub name: String,
    /// SHA1 hash of name for unique index
    pub name_hash: String,
    /// True if this master is running
    pub active: i32,
    /// Updated periodically by a running master
    pub last_active: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
