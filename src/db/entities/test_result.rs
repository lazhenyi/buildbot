//! TestResult entity - represents a single test result

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a test result
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "test_results")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub builderid: i32,
    pub test_result_setid: i32,
    pub test_nameid: Option<i32>,
    pub test_code_pathid: Option<i32>,
    /// Code line that the test originated from
    pub line: Option<i32>,
    /// Duration of test execution
    pub duration_ns: Option<i64>,
    /// Result string
    pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
