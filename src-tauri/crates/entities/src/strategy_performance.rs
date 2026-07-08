use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 复盘→进化：每条 strategy 在某只股票某段持仓后的实际表现。
///
/// 数据来源：复盘 cron 跑完后，对 stock_analyses 中 status=completed 且 outcome
/// 已经被 init/services.rs 写回 win/loss 的行，按 (strategy_id, period, stock_code) 拆分写入。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "strategy_performance")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// strategy 标识："trend" | "value" | "capital" | "reversion" | "watchlist"
    pub strategy_id: String,
    /// 持仓周期："short" | "mid" | "long"
    pub period: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 决策时间（ms）
    pub decision_at: i64,
    /// 退出时间（ms）
    pub exit_at: i64,
    pub holding_days: i32,
    /// 实际收益百分比（已含手续费），如 -3.2 表示 -3.2%
    pub return_pct: f64,
    /// 决策是否正确：买入/增持类 → exit > entry；卖出/减持类 → exit < entry
    pub was_correct: i32,
    /// 决策时的 LLM 置信度（0-100）
    pub decision_confidence: i32,
    /// 各 horizon 的盈亏序列 JSON（1d/3d/5d/10d/20d）
    pub horizon_pnl_json: Option<String>,
    /// 公式 vs LLM 决策一致性分数（0-100，Phase 3 新增）
    pub agreement_score: Option<i32>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
