//! Patch entity - represents a patch for SourceStamps

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a patch for SourceStamps
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "patches")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Number of directory levels to strip off (patch -pN)
    pub patchlevel: i32,
    /// Base64-encoded version of the patch file
    pub patch_base64: String,
    /// Patch author, if known
    pub patch_author: String,
    /// Patch comment
    pub patch_comment: String,
    /// Subdirectory in which the patch should be applied
    pub subdir: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
