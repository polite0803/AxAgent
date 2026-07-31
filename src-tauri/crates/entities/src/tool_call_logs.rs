// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tool_call_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub conversation_id: Option<String>,
    pub trajectory_id: Option<String>,
    pub step_index: i32,
    pub tool_name: String,
    #[sea_orm(column_type = "Text")]
    pub arguments: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub result: Option<String>,
    pub success: i32,
    pub duration_ms: u64,
    pub related_source_id: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
