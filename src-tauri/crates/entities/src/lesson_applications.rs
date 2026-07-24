//! P2-F15 切入点 3：lesson 应用追踪闭环
//!
//! 记录每次决策分析（`stock_analyses`）引用了哪些 lesson（`reflection_lessons`），
//! 以及后续 T+N 验证完成后的实际 outcome。`run_lesson_validation` 据此精确计算
//! `times_applied` 和 `success_count`，反哺 `reflection_lessons.confidence`。
//!
//! ## 闭环流程
//!
//! ```text
//! 1. fetch_stock_lessons 注入 lesson 到分析上下文
//!    → 同步写入 lesson_applications(lesson_id, analysis_id, applied_at)
//! 2. run_decision_backtest 完成 T+N 验证
//!    → 反推 stock_analyses.outcome (win/loss)
//!    → 更新 lesson_applications.outcome_at_validation
//! 3. run_lesson_validation 定时任务
//!    → SELECT COUNT(*) FROM lesson_applications WHERE lesson_id=? → times_applied
//!    → SELECT COUNT(*) FROM lesson_applications WHERE lesson_id=? AND outcome='win' → success_count
//!    → build_lesson_validation 计算调整后的 confidence
//!    → 回写 reflection_lessons
//! ```

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lesson_applications")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 被引用的 lesson ID → reflection_lessons.id
    pub lesson_id: String,
    /// 引用该 lesson 的决策分析 ID → stock_analyses.id
    pub analysis_id: String,
    /// 决策分析的股票代码（冗余字段，便于按股票维度查询）
    pub stock_code: String,
    /// 注入时间（ISO 8601）
    pub applied_at: String,
    /// T+N 验证完成后该 analysis 的 outcome：`win` / `loss` / NULL=未验证
    pub outcome_at_validation: Option<String>,
    /// 验证来源：`t_plus_5` / `t_plus_20` / `t_plus_60` / `manual`
    pub validation_source: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
