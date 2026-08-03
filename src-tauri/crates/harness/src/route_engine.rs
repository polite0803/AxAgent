// SPDX-License-Identifier: AGPL-3.0-only

//! 动态路由引擎契约 — 定义会话级路由决策和 Hard Gate 机制
//!
//! 本模块为 OPC 行业工作流提供动态路由能力：
//! - RouteEngine: 路由引擎 trait，决定工作流执行路径
//! - RouteDecision: 路由决策 DTO
//! - HardGate: 强制门控机制，确保关键步骤通过验收
//! - RouteStrategy: 路由策略枚举

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 路由策略 ─────────────────────────────────────────

/// 路由策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteStrategy {
    /// 基于固定规则的路由（静态配置）
    RuleBased,
    /// 基于历史数据学习的路由（自适应）
    LearningBased,
    /// 混合策略（规则优先，学习辅助）
    Hybrid,
}

impl RouteStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::RuleBased => "rule_based",
            Self::LearningBased => "learning_based",
            Self::Hybrid => "hybrid",
        }
    }
}

// ── 路由决策 ─────────────────────────────────────────

/// 路由决策类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteDecisionType {
    /// 继续执行当前路径
    Continue,
    /// 跳转到指定节点
    Redirect { target_node_id: String },
    /// 跳过当前节点
    Skip,
    /// 回退到备用路径
    Fallback { fallback_path_id: String },
    /// 暂停等待外部输入
    Pause { reason: String },
    /// 终止工作流
    Terminate { reason: String },
}

/// 路由决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// 决策 ID
    pub id: String,
    /// 决策类型
    pub decision_type: RouteDecisionType,
    /// 决策原因
    pub reason: String,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 决策上下文
    pub context: serde_json::Value,
    /// 决策时间戳
    pub decided_at: u64,
}

// ── Hard Gate ─────────────────────────────────────────

/// Hard Gate 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardGateStatus {
    /// 已激活（需要验收）
    Active,
    /// 已通过验收
    Passed,
    /// 已拒绝（需要修改）
    Rejected { reason: String },
    /// 已超时
    Timeout,
}

/// Hard Gate 验收标准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardGateCriteria {
    /// 标准 ID
    pub id: String,
    /// 标准描述
    pub description: String,
    /// 验证类型（auto_verify / manual_verify / llm_judge）
    pub verification_type: String,
    /// 最低通过分数（0.0 - 1.0）
    pub min_score: f64,
    /// 权重
    pub weight: f64,
}

/// Hard Gate 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardGate {
    /// Gate ID
    pub id: String,
    /// Gate 名称
    pub name: String,
    /// 关联的节点 ID
    pub node_id: String,
    /// Gate 状态
    pub status: HardGateStatus,
    /// 验收标准列表
    pub criteria: Vec<HardGateCriteria>,
    /// 是否为阻塞性 Gate（通过前不能继续）
    pub blocking: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 通过后的回调动作
    pub on_passed_action: Option<String>,
}

impl HardGate {
    /// 计算综合得分
    pub fn calculate_score(&self, scores: &HashMap<String, f64>) -> f64 {
        let weighted_sum: f64 = self
            .criteria
            .iter()
            .map(|c| {
                let score = scores.get(&c.id).copied().unwrap_or(0.0);
                score * c.weight
            })
            .sum();
        let total_weight: f64 = self.criteria.iter().map(|c| c.weight).sum();
        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// 检查是否通过
    pub fn is_passed(&self, scores: &HashMap<String, f64>) -> bool {
        let score = self.calculate_score(scores);
        let min_score = self.criteria.iter().map(|c| c.min_score).fold(0.5_f64, f64::max);
        score >= min_score
    }
}

// ── 路由上下文 ─────────────────────────────────────────

/// 路由上下文（用于路由决策的输入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteContext {
    /// 会话 ID
    pub session_id: String,
    /// 行业 ID
    pub industry_id: String,
    /// 当前节点 ID
    pub current_node_id: String,
    /// 工作流 ID
    pub workflow_id: String,
    /// 历史执行结果
    pub execution_history: Vec<NodeExecutionResult>,
    /// 当前状态
    pub state: serde_json::Value,
    /// 路由策略
    pub strategy: RouteStrategy,
}

/// 节点执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    /// 节点 ID
    pub node_id: String,
    /// 是否成功
    pub success: bool,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 输出摘要
    pub output_summary: String,
    /// 质量评分（0.0 - 1.0）
    pub quality_score: f64,
}

// ── RouteEngine trait ─────────────────────────────────

/// 路由引擎契约
///
/// 负责在工作流执行过程中做出路由决策，
/// 支持动态调整执行路径以适应不同的场景和条件。
#[async_trait]
pub trait RouteEngine: Send + Sync {
    /// 做出路由决策
    async fn decide_route(&self, context: &RouteContext) -> Result<RouteDecision, String>;

    /// 评估 Hard Gate
    async fn evaluate_gate(
        &self,
        gate: &HardGate,
        context: &RouteContext,
    ) -> Result<HardGateStatus, String>;

    /// 注册路由规则
    async fn register_rule(&self, rule: RouteRule) -> Result<(), String>;

    /// 获取当前策略
    fn current_strategy(&self) -> RouteStrategy;
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 触发条件（JSON 表达式）
    pub condition: String,
    /// 决策类型
    pub decision_type: RouteDecisionType,
    /// 优先级（数字越大优先级越高）
    pub priority: i32,
    /// 是否启用
    pub enabled: bool,
}

/// No-op 实现（用于测试或未配置路由时）
pub struct NoopRouteEngine {
    strategy: RouteStrategy,
}

impl NoopRouteEngine {
    pub fn new(strategy: RouteStrategy) -> Self {
        Self { strategy }
    }
}

impl Default for NoopRouteEngine {
    fn default() -> Self {
        Self::new(RouteStrategy::RuleBased)
    }
}

#[async_trait]
impl RouteEngine for NoopRouteEngine {
    async fn decide_route(&self, _context: &RouteContext) -> Result<RouteDecision, String> {
        Ok(RouteDecision {
            id: format!("route-decision-{}", uuid::Uuid::new_v4()),
            decision_type: RouteDecisionType::Continue,
            reason: "默认路由决策：继续执行".to_string(),
            confidence: 1.0,
            context: serde_json::Value::Null,
            decided_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    async fn evaluate_gate(
        &self,
        _gate: &HardGate,
        _context: &RouteContext,
    ) -> Result<HardGateStatus, String> {
        // No-op：默认通过所有 Gate
        Ok(HardGateStatus::Passed)
    }

    async fn register_rule(&self, _rule: RouteRule) -> Result<(), String> {
        Ok(())
    }

    fn current_strategy(&self) -> RouteStrategy {
        self.strategy.clone()
    }
}

// HashMap 需要这个导入
use std::collections::HashMap;
