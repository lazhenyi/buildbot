//! BuildRequestClaim entity - tracks which master has claimed a build request

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Build request claims - tracks which master has claimed a build request
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buildrequest_claims")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brid: i32,
    pub masterid: i32,
    pub claimed_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
