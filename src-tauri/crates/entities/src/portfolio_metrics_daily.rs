use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 组合监控：每日 EOD 写入一行组合快照。
///
/// 数据来源：刷新时（手动或 cron 触发）汇总 `trades` + 行情价，
/// 配合沪深 300 同期收益算 beta / max_dd / sharpe_30d。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_metrics_daily")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 快照日期 (YYYY-MM-DD)
    pub snapshot_date: String,
    /// 持仓总市值
    pub total_market_value: f64,
    /// 现金占比 (0-1)
    pub cash_pct: f64,
    /// 总未实现盈亏（元）
    pub total_pnl: f64,
    /// 总未实现盈亏百分比
    pub total_pnl_pct: f64,
    /// 自有持仓起的滚动 max drawdown
    pub max_drawdown_pct: f64,
    /// 相对沪深 300 的 beta（None 表示样本不足）
    pub beta: Option<f64>,
    /// 30 日滚动 sharpe（年化）
    pub sharpe_30d: Option<f64>,
    /// 平均两两相关性 (0-1)
    pub correlation_avg: Option<f64>,
    /// 单股最大占比 (0-1)
    pub top_concentration_pct: f64,
    /// 行业暴露 JSON: { "科技": 0.18, "消费": 0.12, ... }
    pub sector_exposure_json: String,
    /// 压测结果 JSON: { "m10": {...}, "m20": {...}, "blackSwan": {...} }
    pub stress_test_json: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
