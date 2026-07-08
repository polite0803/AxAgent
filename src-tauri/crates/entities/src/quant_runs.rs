use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 回测运行记录
///
/// - status: pending → running → completed / failed
/// - result_json: 完成后填入 BacktestResult 序列化（用于前端展示 + 重算指标）
/// - walk_forward_*: WalkForwardReport 关键字段冗余（便于快速查询）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quant_runs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联 quant_strategies.id
    pub strategy_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub initial_cash: f64,
    /// BacktestConfig JSON
    pub config_json: String,
    /// "pending" | "running" | "completed" | "failed"
    pub status: String,
    /// BacktestResult JSON（status=completed 时填入）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result_json: Option<String>,
    /// Walk-Forward 启用
    pub walk_forward_enabled: bool,
    /// Walk-Forward fold 数
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub walk_forward_folds: Option<i32>,
    /// 整体过拟合告警
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub walk_forward_overfit_warning: Option<bool>,
    /// 参数稳定度 0..1
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub walk_forward_stability_score: Option<f64>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_message: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
