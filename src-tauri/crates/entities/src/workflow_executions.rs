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
    // 实际库列是 INT4（INTEGER）。迁移 DDL 虽声明 BIGINT，但当前运行库该列
    // 实际为 INTEGER，用 i64 会在 PG 上触发 INT8→INT4 解码失败。按"实体适配
    // 库列"纪律改用 i32（INT4）。若日后重建库使列变回 BIGINT，需同步改回 i64。
    pub total_time_ms: Option<i32>,
    /// 序列化后的 ExecutionStateSnapshot，用于崩溃后恢复
    pub execution_state_json: Option<String>,
    /// 暂停时间戳（毫秒），用于超时判断
    pub paused_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
