use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 股票管道执行记录
///
/// 每次 `run_stock_pipeline_inner` 调用对应一行，记录从发现到分析的完整执行轨迹。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "stock_pipeline_runs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 管道执行日期（YYYY-MM-DD）
    pub run_date: String,
    /// 时间旅行模式截止日（可选）
    pub as_of_date: Option<String>,
    /// running / completed / failed
    pub status: String,
    /// 候选股列表 JSON
    pub candidates_json: Option<String>,
    /// 新候选股分析摘要 JSON
    pub new_analyses_json: Option<String>,
    /// 持仓再评估摘要 JSON
    pub reassessed_json: Option<String>,
    /// 汇总报告 JSON
    pub summary_json: Option<String>,
    /// 失败原因（status=failed 时）
    pub error_message: Option<String>,
    /// 开始时间戳（ms）
    pub started_at: i64,
    /// 完成时间戳（ms，可空）
    pub completed_at: Option<i64>,
    /// 创建时间戳（ms）
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
