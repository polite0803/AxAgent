// SPDX-License-Identifier: AGPL-3.0-only

//! 业务岗位实体 —— 对应现实业务中的岗位（CEO / CTO / 产品经理 / 财务总监 等）。
//! 与 `agent_roles`（抽象执行器类型）区别：business_role 表达「在组织里担什么责」，
//! agent_role 表达「怎么干活」。一个 business_role 可下属多个 agency_experts。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "business_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// 岗位核心职责（JSON 数组，元素为职责描述字符串）
    #[sea_orm(column_name = "responsibilities")]
    pub responsibilities: Option<String>,
    /// 决策权限边界（JSON 对象，例如 {"max_budget": 100000, "scopes": ["tech","hr"]}）
    #[sea_orm(column_name = "decision_authority")]
    pub decision_authority: Option<String>,
    /// 汇报对象（business_roles.id 自引用，None 表示顶层）
    #[sea_orm(column_name = "reports_to")]
    pub reports_to: Option<String>,
    /// 下属专家 ID 列表（JSON 数组，元素为 agency_experts.id）
    #[sea_orm(column_name = "managed_expert_ids")]
    pub managed_expert_ids: Option<String>,
    /// 准入条件（JSON 数组，例如 ["PMP 认证", "5 年管理经验"]）
    #[sea_orm(column_name = "required_certifications")]
    pub required_certifications: Option<String>,
    /// 激活业务域（JSON 数组，与 agency_experts.active_domains 同语义）
    #[sea_orm(column_name = "active_domains")]
    pub active_domains: Option<String>,
    /// 岗位系统提示词（描述该岗位的身份/视角/行为约束）
    #[sea_orm(column_name = "system_prompt")]
    pub system_prompt: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub source: String,
    pub sort_order: i32,
    pub is_enabled: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
