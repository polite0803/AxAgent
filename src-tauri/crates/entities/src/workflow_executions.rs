// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub input_params: Option<String>,
    pub output_result: Option<String>,
    pub node_executions: Option<String>,
    // DDL 为 BIGINT（v100_consolidated.rs），必须用 i64；旧库该列实际也是
    // BIGINT，用 i32 会导致 PG 上 INT8→INT4 解码失败。
    pub total_time_ms: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
