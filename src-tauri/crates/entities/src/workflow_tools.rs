// SPDX-License-Identifier: AGPL-3.0-only
//! 工作流运行时工具实体 —— 动态发现/生成工具的持久化。
//!
//! 独立于 `workflow_templates.tool_defs`（模板内置、随版本快照走），
//! 支持运行时发现工具写回、跨工作流复用、使用统计。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_tools")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 归属工作流模板 ID（与 workflow_templates.id 关联）
    pub workflow_id: String,
    /// 工具名（运行时注册名，同一工作流内唯一）
    pub tool_name: String,
    /// 工具类型: rhai_script | workflow_dag | llm_function
    pub tool_type: String,
    pub description: Option<String>,
    /// 实现体：Rhai 源码 / DAG JSON / LLM 函数定义
    pub code: Option<String>,
    /// 输入 JSON Schema
    pub input_schema: Option<String>,
    /// 来源标记: runtime_discovery | ai_generated | evolution | manual
    pub source: String,
    /// 状态: pending | active | disabled（运行时只注册 active）
    pub status: String,
    pub usage_count: i32,
    pub success_rate: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
