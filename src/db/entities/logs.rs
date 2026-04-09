//! Log entity - represents build logs

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a log
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub stepid: i32,
    pub complete: i32,
    pub num_lines: i32,
    /// 's' = stdio, 't' = text, 'h' = html, 'd' = deleted
    pub log_type: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
