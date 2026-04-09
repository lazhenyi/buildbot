//! UserInfo entity - additional user attributes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User info - additional user attributes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub uid: i32,
    /// Type of user attribute (e.g., 'git')
    pub attr_type: String,
    /// Data for the attribute
    pub attr_data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
