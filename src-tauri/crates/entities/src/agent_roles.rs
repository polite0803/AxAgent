// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[sea_orm(column_name = "system_prompt")]
    pub system_prompt: String,
    #[sea_orm(column_name = "default_tools")]
    pub default_tools: Option<String>,
    #[sea_orm(column_name = "active_domains")]
    pub active_domains: Option<String>,
    pub max_concurrent: i32,
    pub timeout_seconds: i64,
    pub source: String,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
    /// 岗位核心职责（JSON 数组，元素为职责描述字符串）。v218 由 business_roles 合并而来。
    pub responsibilities: Option<String>,
    /// 决策权限边界（JSON 对象，例如 {"max_budget": 100000, "scopes": ["tech","hr"]}）
    #[sea_orm(column_name = "decision_authority")]
    pub decision_authority: Option<String>,
    /// 汇报对象（agent_roles.id 自引用，None 表示顶层）
    #[sea_orm(column_name = "reports_to")]
    pub reports_to: Option<String>,
    /// 下属专家 ID 列表（JSON 数组，元素为 agency_experts.id）
    #[sea_orm(column_name = "managed_expert_ids")]
    pub managed_expert_ids: Option<String>,
    /// 准入条件（JSON 数组，例如 ["PMP 认证", "5 年管理经验"]）
    #[sea_orm(column_name = "required_certifications")]
    pub required_certifications: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    #[sea_orm(column_name = "is_enabled")]
    pub is_enabled: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
