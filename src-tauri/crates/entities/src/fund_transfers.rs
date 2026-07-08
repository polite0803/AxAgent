use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 资金流水 — 银证转账出入金记录。
/// 用于计算真实收益率（结合 trades 表和资金流水计算资金加权收益率）。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fund_transfers")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// "deposit" | "withdrawal"
    pub transfer_type: String,
    /// 金额（正数）
    pub amount: f64,
    /// 银证转账日期 YYYY-MM-DD
    pub transfer_date: String,
    /// 转账手续费（可选）
    pub fee: Option<f64>,
    pub notes: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
