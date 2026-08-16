// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "trajectory_trajectories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    /// 结构化 agent 标识：记录该轨迹由哪个 Agent（AgentProfile 名称）执行。
    /// 进化系统据此精准聚合每个 Agent 的证据（v121 新增，可空列）。
    pub agent_name: Option<String>,
    pub topic: String,
    pub summary: String,
    pub outcome: String,
    pub duration_ms: i64,
    pub quality_overall: f64,
    pub quality_task_completion: f64,
    pub quality_tool_efficiency: f64,
    pub quality_reasoning_quality: f64,
    pub quality_user_satisfaction: f64,
    pub value_score: f64,
    pub patterns: String,
    pub created_at: String,
    pub replay_count: i32,
    pub last_replay_at: Option<String>,
    /// 失效标记（append-only 证据存储：0=有效，1=已失效）。
    /// 轨迹作为进化证据不可物理删除，仅可标记失效（v120 新增）。
    pub is_invalidated: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
