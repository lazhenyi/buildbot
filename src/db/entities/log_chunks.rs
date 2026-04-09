//! LogChunk entity - represents a chunk of build log data

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a log chunk
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "logchunks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub logid: i32,
    /// 0-based line number range in this chunk (inclusive)
    pub first_line: i32,
    pub last_line: i32,
    /// Log contents, encoded in utf-8 or compressed
    pub content: Option<Vec<u8>>,
    /// 0 = none, 1 = gzip, 2 = bzip2, 3 = lz4, 4 = br, 5 = zstd
    pub compressed: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
