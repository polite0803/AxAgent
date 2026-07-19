// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流反思历史持久化 entity（v103 migration 创建）。
//!
//! 反思结果由 `WorkflowReflectorImpl` 在每次 `reflect()` / `reflect_node()` 调用后落库，
//! 用于跨会话查询 / 模式聚合 / 进化决策。`WorkflowOptimizer` / `WorkflowEvolver` 通过
//! `TrajectoryStorage::get_workflow_reflections()` 读取历史反思驱动优化。

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_workflow_reflections")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 工作流 ID（用于按模板聚合历史反思）。
    pub workflow_id: String,
    /// 执行 ID（与 `Reflection.task_id` 一致，单次执行的唯一标识）。
    pub execution_id: String,
    /// 工作流模板 ID（可选，部分执行没有模板上下文）。
    pub template_id: Option<String>,
    /// 质量分（0-10），来自 `Reflection.quality_score`。
    pub quality_score: i32,
    /// 总结性描述，来自 `Reflection.overall_summary`。
    pub summary: String,
    /// JSON 序列化的 `Vec<String>`，来自 `Reflection.error_patterns`。
    pub error_patterns_json: String,
    /// JSON 序列化的 `Vec<String>`，来自 `Reflection.reusable_patterns`。
    pub reusable_patterns_json: String,
    /// JSON 序列化的 `WorkflowReflectionMetadata`，来自 `Reflection.metadata`。
    pub metadata_json: String,
    /// 反思时间戳（RFC3339），来自 `Reflection.timestamp`。
    pub timestamp: String,
    /// 入库时间戳（RFC3339），由存储层在落库时设置。
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
