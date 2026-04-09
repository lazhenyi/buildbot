//! ChangeSourceMaster entity - links changesources to the master where they are running

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Links changesources to the master where they are running
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "changesource_masters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub changesourceid: i32,
    pub masterid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
