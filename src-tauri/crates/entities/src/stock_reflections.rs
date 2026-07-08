use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "stock_reflections")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 原始分析的 ID（关联 stock_analyses.id）
    pub original_analysis_id: String,
    /// as-of 时间（原始分析日期，YYYY-MM-DD）
    pub as_of_date: String,
    /// 后见信息时间（校验/反思触发日期，YYYY-MM-DD）
    pub hindsight_date: String,
    /// 反思触发时的置信度阈值
    pub min_confidence_threshold: i32,
    /// 反思深度：light | deep（deep 会详述 reasoning chain）
    pub reflection_depth: String,
    /// 实际走势描述，如 "30天跌-8.3% → 失败"
    pub actual_outcome: String,
    // ── v008 升级（借鉴 TradingAgents 反思机制）──
    // 4 个结构化 outcome 变量:C3 借鉴。让 LLM 反思时直接引用"持仓 30 天跌
    // 8%,相对沪深 300 超额 -2.1%"这样的硬数字,避免 LLM 脑补。
    /// 原始收益率(%)。None=回退到 actual_outcome 自然语言。
    pub raw_return: Option<f64>,
    /// 相对基准的超额收益(%)。None=未算 alpha。
    pub alpha_return: Option<f64>,
    /// 实际持有天数。None=未到反思点(pending row)。
    pub holding_days: Option<i32>,
    /// 基准名称,如"沪深300"/"中证500"。None=未指定。
    pub benchmark_name: Option<String>,
    // 3 个 C2 借鉴短文本输出。
    /// 反思判定:correct / partial / wrong 三选一
    pub verdict: Option<String>,
    /// 反思中引用的关键 alpha/信号
    pub alpha_cited: Option<String>,
    /// ≤200 字符、≤2 句简短总结(C1 强制短文本)
    pub lesson_summary: Option<String>,
    /// 反思摘要：错因
    pub what_went_wrong: Option<String>,
    /// 反思摘要：被忽视的信号（JSON 数组字符串）
    pub missed_signals: Option<String>,
    /// 反思摘要：改进建议
    pub fix_for_future: Option<String>,
    /// 反思 agent 输出的参数调整建议（params_suggestion JSON 数组字符串）
    pub parameter_suggestions_json: Option<String>,
    /// portfolio-manager 完整输出 JSON
    pub decision_json: Option<String>,
    /// 工作流完整结果（用于追溯）
    pub blackboard_snapshot: Option<String>,
    /// 所用 LLM 的版本标识
    pub model_version: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
