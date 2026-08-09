// SPDX-License-Identifier: AGPL-3.0-only
//! 节点质量反馈实体（通用版）
//!
//! 用于持久化存储前端检测到的节点数据质量问题，
//! 作为自我进化引擎的输入数据。
//!
//! 支持的节点类型：
//! - analyst: 分析师节点（a-market-analyst, a-fundamentals 等）
//! - debate: 辩论节点（bull-r2, bear-r2 等）
//! - decision: 决策节点（research-manager, reflection 等）
//! - tool: 工具/算法节点（t-scoring, t-valuation, t-risk 等）
//! - valuation: 估值节点
//! - risk: 风险节点

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 节点类型枚举
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// 分析师节点
    Analyst,
    /// 辩论节点
    Debate,
    /// 决策节点
    Decision,
    /// 工具/算法节点
    Tool,
    /// 估值节点
    Valuation,
    /// 风险节点
    Risk,
    /// 其他
    Other,
}

impl NodeType {
    pub fn as_str(&self) -> &str {
        match self {
            NodeType::Analyst => "analyst",
            NodeType::Debate => "debate",
            NodeType::Decision => "decision",
            NodeType::Tool => "tool",
            NodeType::Valuation => "valuation",
            NodeType::Risk => "risk",
            NodeType::Other => "other",
        }
    }
}

/// 节点质量反馈 Entity（通用版）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "analyst_feedbacks")]
pub struct Model {
    /// 反馈 ID
    #[sea_orm(primary_key)]
    pub id: String,
    /// 节点类型 (analyst/debate/decision/tool/valuation/risk/other)
    pub node_type: String,
    /// 节点 ID (e.g., "a-market-analyst", "bull-r2", "research-manager")
    pub node_id: String,
    /// 报告 ID
    pub report_id: String,
    /// 股票代码
    pub stock_code: String,
    /// 执行 ID (workflow run id)
    pub execution_id: String,
    /// 质量评分 (0-100)
    pub quality_score: i32,
    /// 评分等级 (A/B/C/D/F)
    pub grade: String,
    /// 检测到的问题数量
    pub issue_count: i32,
    /// 检测到的警告数量
    pub warning_count: i32,
    /// 检测到的良好项数量
    pub good_count: i32,
    /// 详细检查结果 (JSON)
    pub checks_json: String,
    /// 通用质量指标 (JSON) - 存储节点特有的质量指标
    /// 分析师: {bull_score, bear_score, confidence, score_consistent, direction_consistent}
    /// 辩论: {stance, strength_score, confidence, logic_consistent}
    /// 决策: {action, confidence, risk_assessed, criteria_met}
    /// 工具: {accuracy, completeness, credibility, data_freshness}
    pub quality_metrics_json: String,
    /// 是否已被进化引擎消费
    pub consumed: bool,
    /// 是否触发了进化
    pub evolution_triggered: bool,
    /// 创建时间
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
