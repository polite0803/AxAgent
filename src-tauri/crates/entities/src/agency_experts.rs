// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agency_experts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    #[sea_orm(column_name = "system_prompt")]
    pub system_prompt: String,
    pub color: Option<String>,
    #[sea_orm(column_name = "source_dir")]
    pub source_dir: String,
    pub is_enabled: i32,
    pub imported_at: i64,
    #[sea_orm(column_name = "recommended_workflows")]
    pub recommended_workflows: Option<String>,
    #[sea_orm(column_name = "recommended_tools")]
    pub recommended_tools: Option<String>,
    #[sea_orm(column_name = "active_domains")]
    pub active_domains: Option<String>,
    /// 资历等级：junior / mid / senior / expert
    #[sea_orm(column_name = "seniority")]
    pub seniority: Option<String>,
    /// 擅长细分领域（JSON 数组）
    #[sea_orm(column_name = "specialties")]
    pub specialties: Option<String>,
    /// 归属角色（agent_roles.id）
    #[sea_orm(column_name = "parent_role_id")]
    pub parent_role_id: Option<String>,
    /// 历史成功率（0.0 ~ 1.0）
    #[sea_orm(column_name = "success_rate")]
    pub success_rate: Option<f64>,
    /// 平均执行延迟（毫秒）
    #[sea_orm(column_name = "avg_latency_ms")]
    pub avg_latency_ms: Option<i64>,
    /// 平均 token 成本
    #[sea_orm(column_name = "avg_token_cost")]
    pub avg_token_cost: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
