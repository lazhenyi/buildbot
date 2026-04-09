//! Project entity - represents a build project

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a project
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Project name
    pub name: String,
    /// SHA1 of name for unique index
    pub name_hash: String,
    /// Project slug for URLs
    pub slug: String,
    /// Project description
    pub description: Option<String>,
    /// Format of project description
    pub description_format: Option<String>,
    /// Description rendered as HTML
    pub description_html: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
