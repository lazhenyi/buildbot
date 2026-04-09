//! ConfiguredWorker entity - links workers to builder/master pairs

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Link workers to all builder/master pairs for which they are configured
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "configured_workers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buildermasterid: i32,
    pub workerid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
