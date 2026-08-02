// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 工作项实体（Self-Run 状态机持久层）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_work_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联的 rt-workflow 运行 id（可选，对接 DAG 引擎）
    pub run_id: Option<String>,
    /// 阶段（QUEUED / IN_PROGRESS / WAITING_FOR_CHILDREN / BLOCKED / REVIEW / APPROVED / DONE / FAILED / CANCELLED）
    pub phase: String,
    /// 标题
    #[sea_orm(column_type = "Text")]
    pub title: String,
    /// 负责角色 id（opc-xxx 或 agent_role id）
    pub owner_role_id: Option<String>,
    /// 依赖项 json 数组：["item-a", "item-b"]
    #[sea_orm(column_type = "Text")]
    pub deps_json: String,
    /// 被指派的 agent id（可选，空 = 未指派）
    pub assignee_agent_id: Option<String>,
    /// 管理模式（execute/delegate/review/integrate/rework）
    pub management_mode: Option<String>,
    /// 负责经理角色 id
    pub manager_role_id: Option<String>,
    /// 最近一次错误信息
    #[sea_orm(column_type = "Text")]
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
