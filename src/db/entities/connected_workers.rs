//! ConnectedWorker entity - tracks workers connected to masters

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Link workers to the masters they are currently connected to
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "connected_workers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub masterid: i32,
    pub workerid: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
