//! 决策事后验证表（V55 新增）
//!
//! 每次 `recommend_stocks()` 生成的 reco pick 在 T+5/T+20/T+60 实际价格可获取时，
//! 写入一条验证记录。用于：
//! - 测算 hit_rate（建议正确率）
//! - 测算假阳性率（建议买入但实际下跌）
//! - 9 因子 IC（信息系数）重标定
//! - portfolio-mgr.rhai 因子权重回测验证
//!
//! 数据流：
//! ```
//! reco_picks ──触发──> run_decision_backtest 命令
//!                     ↓
//!              拉取 T+N 实际价格（待接入行情 API）
//!                     ↓
//!              写 decision_validations
//!                     ↓
//!              聚合 hit_rate / 9 因子 IC
//! ```

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "decision_validations")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联的 reco_picks.id
    pub pick_id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 风格: "trend" | "value" | "capital" | "reversion" | "watchlist"
    pub style: String,
    /// 周期: "short" | "mid" | "long"
    pub period: String,
    /// 验证窗口天数 (5 / 20 / 60)
    pub t_plus_n: i32,
    /// 决策生成时间（ISO 8601）
    pub generated_at: String,
    /// 验证完成时间（ISO 8601）
    pub validated_at: String,
    /// 决策时的价格
    pub entry_price: f64,
    /// 目标位
    pub target_price: f64,
    /// 止损
    pub stop_loss: f64,
    /// 建议仓位（%）
    pub position_pct: f64,
    /// 置信度 0-100
    pub confidence: i32,
    /// 推断的动作（"buy" | "sell" | "hold"），由 target_price vs price 推断
    pub inferred_action: String,
    /// T+N 当日实际收盘价
    pub t_plus_n_price: Option<f64>,
    /// 期间最高价
    pub max_price: Option<f64>,
    /// 期间最低价
    pub min_price: Option<f64>,
    /// 期间最大涨幅（%）
    pub max_return_pct: Option<f64>,
    /// 期间最大回撤（%）
    pub max_drawdown_pct: Option<f64>,
    /// T+N 收益（%）
    pub final_return_pct: Option<f64>,
    /// 是否触及止损（0/1，INTEGER 列）
    pub hit_stop_loss: Option<i32>,
    /// 是否触及目标（0/1，INTEGER 列）
    pub hit_target: Option<i32>,
    /// 综合命中判定：buy 且 final_return_pct>0 → hit；buy 且 final_return_pct<-5% → false_hit
    /// 写入规则: "hit" | "miss" | "false_hit" | "partial" | "insufficient"
    pub hit_outcome: Option<String>,
    /// 9 因子快照（JSON object，每个因子 0-1），供 IC 重标定
    /// 格式: {"f1_technical": 0.8, "f2_consensus": 0.6, ...}
    pub factor_snapshot: Option<String>,
    /// 验证数据源说明（如 "akshare_daily" | "manual" | "fallback_seed"）
    pub data_source: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
