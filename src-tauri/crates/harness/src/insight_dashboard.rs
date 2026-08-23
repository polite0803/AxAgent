// SPDX-License-Identifier: AGPL-3.0-only

//! 洞察面板数据模型 (P2-17)
//!
//! 成本、用量统计和洞察相关 DTO

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 成本统计
// ---------------------------------------------------------------------------

/// 成本报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostReport {
    /// 报告 ID
    pub id: String,
    /// 时间范围
    pub period: ReportPeriod,
    /// 总成本（美元）
    pub total_cost_usd: f64,
    /// 按模型分布
    pub cost_by_model: HashMap<String, f64>,
    /// 按提供商分布
    pub cost_by_provider: HashMap<String, f64>,
    /// 按任务类型分布
    pub cost_by_task_type: HashMap<String, f64>,
    /// Token 统计
    pub token_stats: TokenUsageStats,
    /// 平均成本趋势
    pub avg_cost_trend: Vec<CostDataPoint>,
    /// 成本效率指标
    pub efficiency_metrics: EfficiencyMetrics,
}

/// 报告周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Custom,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub avg_tokens_per_request: f64,
    pub input_token_cost_usd: f64,
    pub output_token_cost_usd: f64,
}

/// 成本数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostDataPoint {
    pub timestamp: String,
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// 效率指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EfficiencyMetrics {
    /// 每 1000 token 成本
    pub cost_per_1k_tokens: f64,
    /// 成功请求平均成本
    pub avg_cost_per_request: f64,
    /// 失败请求成本占比
    pub failed_cost_ratio: f64,
    /// 成本效率评分（0-1）
    pub efficiency_score: f64,
}

// ---------------------------------------------------------------------------
// 用量统计
// ---------------------------------------------------------------------------

/// 用量报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    /// 报告 ID
    pub id: String,
    /// 时间范围
    pub period: ReportPeriod,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 按模型分布
    pub requests_by_model: HashMap<String, u64>,
    /// 按任务类型分布
    pub requests_by_task_type: HashMap<String, u64>,
    /// 按时间分布
    pub requests_by_hour: HashMap<u8, u64>,
    /// 使用的技能列表
    pub used_skills: Vec<SkillUsage>,
    /// 活跃会话数
    pub active_sessions: u64,
}

/// 技能使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUsage {
    pub skill_id: String,
    pub skill_name: String,
    pub usage_count: u64,
    pub avg_execution_time_ms: f64,
    pub success_rate: f64,
}

// ---------------------------------------------------------------------------
// 洞察建议
// ---------------------------------------------------------------------------

/// 洞察建议
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightRecommendation {
    pub id: String,
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub severity: InsightSeverity,
    pub related_metrics: Vec<String>,
    pub suggested_actions: Vec<String>,
    pub generated_at: String,
}

/// 建议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationType {
    CostOptimization,
    PerformanceImprovement,
    QualityImprovement,
    SecurityEnhancement,
    FeatureSuggestion,
}

/// 洞察严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}
