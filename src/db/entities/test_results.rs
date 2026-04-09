//! Test result entities

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a test result set
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "test_result_sets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub builderid: i32,
    pub buildid: i32,
    pub stepid: i32,
    /// Description of test data source
    pub description: Option<String>,
    pub category: String,
    pub value_unit: String,
    /// Number of passed tests
    pub tests_passed: Option<i32>,
    /// Number of failed tests
    pub tests_failed: Option<i32>,
    /// True when all test results have been generated
    pub complete: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
