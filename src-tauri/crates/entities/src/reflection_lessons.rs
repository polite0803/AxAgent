use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// F1 借鉴：反思教训规则化表
///
/// 借鉴 TradingAgents 反思 → 规则提取机制。每次反思完成后，
/// 提取 lesson_summary 为可重用的规则，下次决策可以查询。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reflection_lessons")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// ≤200 字符规则描述
    pub lesson_summary: String,
    /// 规则触发条件（如"分批建仓节奏 ≤3 天"）
    pub rule_pattern: Option<String>,
    /// 来源反思行 ID
    pub source_reflection_id: Option<String>,
    /// 适用 ticker（None=通用规则）
    pub stock_code: Option<String>,
    /// JSON 数组：适用场景标签（如 ["短线", "高估值"]）
    pub applicable_scenarios: Option<String>,
    /// 已应用次数
    pub times_applied: i32,
    /// 应用后成功次数
    pub success_count: i32,
    /// 规则置信度 0-1
    pub confidence: f64,
    /// active / deprecated
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
