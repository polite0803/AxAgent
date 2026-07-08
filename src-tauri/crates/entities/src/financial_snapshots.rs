use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 每日估值快照(R3-C)
///
/// 每只股票每个交易日 EOD 写入一行,保存 (PE, PB, PS, PCF, EV/EBITDA, ROE, gross_margin,
/// debt_ratio, revenue_yoy, profit_yoy) 等估值与基本面字段,用于"估值带"分位计算。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "financial_snapshots")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub stock_code: String,
    /// YYYY-MM-DD
    pub snapshot_date: String,
    pub pe_ttm: Option<f64>,
    pub pb: Option<f64>,
    pub ps_ttm: Option<f64>,
    pub pcf: Option<f64>,
    pub ev_ebitda: Option<f64>,
    pub roe: Option<f64>,
    pub gross_margin: Option<f64>,
    pub debt_ratio: Option<f64>,
    pub revenue_yoy: Option<f64>,
    pub profit_yoy: Option<f64>,
    pub source: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
