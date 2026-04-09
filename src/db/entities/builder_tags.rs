//! BuilderTag entity - many-to-many relationship between builders and tags

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Builder-Tag relationship (many-to-many)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "builders_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub builderid: i32,
    pub tagid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
