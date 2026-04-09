//! Builder entity - represents a build builder configuration

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a builder in the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "builders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Builder's name
    pub name: String,
    /// Builder's description
    pub description: Option<String>,
    /// The format of builder description
    pub description_format: Option<String>,
    /// Builder description rendered as html
    pub description_html: Option<String>,
    /// Builder's project (foreign key)
    pub projectid: Option<i32>,
    /// SHA1 hash of name for unique index
    pub name_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
