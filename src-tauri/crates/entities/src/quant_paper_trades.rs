use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 纸面成交记录（回测撮合产出）
///
/// 与 `trades` 表的区别：
/// - `trades`: 用户手动录入的真实交易
/// - `quant_paper_trades`: 策略回测期间的模拟成交
///
/// 字段对应 `quant::Trade`：
/// - code / side / quantity / price / amount
/// - commission / stamp_tax / slippage（成本明细）
/// - realized_pnl（仅卖出成交填入）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quant_paper_trades")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联 quant_runs.id
    pub run_id: String,
    pub code: String,
    /// "long" | "short" | "flat"
    pub side: String,
    /// 成交股数
    pub quantity: i64,
    /// 成交价
    pub price: f64,
    /// 成交金额
    pub amount: f64,
    /// 佣金
    pub commission: f64,
    /// 印花税
    pub stamp_tax: f64,
    /// 滑点损失
    pub slippage: f64,
    /// YYYY-MM-DD
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// 已实现盈亏（仅平仓成交）
    pub realized_pnl: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
