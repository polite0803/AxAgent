use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 组合监控：两两相关性快照（按日聚合，每对持仓一行）。
///
/// 数据来源：刷新时拉每只持仓近 N 日 K 线收盘价，算 Pearson 相关系数后写入。
/// N≤20 时一次写入 N×(N-1)/2 行；N>20 时退化为只算与最大持仓的相关性。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_correlation_snapshot")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 快照日期 (YYYY-MM-DD)
    pub snapshot_date: String,
    /// 回看窗口（交易日）
    pub lookback_days: i32,
    pub code_a: String,
    pub code_b: String,
    /// Pearson 相关系数 (-1, 1)
    pub correlation: f64,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
