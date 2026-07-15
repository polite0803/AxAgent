// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_skills")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub content: String,
    pub category: String,
    pub tags: String,
    pub scenarios: String,
    pub parameters: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    /// 连续失败次数：Failure 累加，Success/Partial 清零
    #[sea_orm(default_value = 0)]
    pub consecutive_failures: i32,
    /// 最近一次失败时间（ISO8601 字符串），NULL 表示从未失败
    pub last_failure_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
