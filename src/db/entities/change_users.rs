//! ChangeUser entity - users associated with changes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Users associated with this change
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "change_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub changeid: i32,
    pub uid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
