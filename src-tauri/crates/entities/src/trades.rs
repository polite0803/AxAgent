use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 手动交易记录 — 用户自己录入买卖，非券商 API。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "trades")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 交易方向: "buy" | "sell"
    pub direction: String,
    /// 成交价
    pub price: f64,
    /// 股数
    pub quantity: i32,
    /// 交易日期 YYYY-MM-DD
    pub trade_date: String,
    /// 交易时间 HH:MM
    pub trade_time: String,
    /// 手续费（可选）
    pub fee: Option<f64>,
    /// 卖出时计算的已实现盈亏
    pub realized_pnl: Option<f64>,
    /// 所属策略: "trend" | "value" | "capital" | "reversion" | "watchlist"
    pub strategy: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
