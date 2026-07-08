use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 信号历史（按回测 run 归档）
///
/// 用于：
/// - 调试策略：查看哪些 bar 触发了信号 / 原因
/// - 复盘：对比实际持仓与策略信号
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quant_signals")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联 quant_runs.id
    pub run_id: String,
    pub code: String,
    /// "buy" | "sell" | "hold"
    pub action: String,
    /// 0..1
    pub strength: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// 平仓原因（仅 sell）：take_profit / stop_loss / signal_reverse / risk_control / end_of_backtest / manual
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub close_reason: Option<String>,
    /// YYYY-MM-DD
    pub timestamp: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
