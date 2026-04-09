//! User entity - represents a buildbot user

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a user
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub uid: i32,
    /// User identifier (nickname) for display
    pub identifier: String,
    /// Username for authentication
    pub bb_username: Option<String>,
    /// Password for authentication
    pub bb_password: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
