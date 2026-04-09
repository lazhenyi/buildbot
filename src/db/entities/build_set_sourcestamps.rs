//! BuildSetSourceStamp entity - many-to-many relationship between buildsets and sourcestamps

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BuildSet SourceStamps - many-to-many relationship
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buildset_sourcestamps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buildsetid: i32,
    pub sourcestampid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
