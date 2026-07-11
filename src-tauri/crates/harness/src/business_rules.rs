// SPDX-License-Identifier: AGPL-3.0-only

//! 业务规则类型定义 — 纯契约层，零实现逻辑。
//!
//! 具体实现（BusinessRuleEngine、工厂函数、测试）已迁移至 `axagent_rt_workflow::business_rules`。

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 核心类型 ──

/// 规则评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleResult {
    /// 规则通过，无违规
    Pass,
    /// 违反规则，附带原因
    Violation { reason: String },
    /// 需要人工审批，附带原因
    RequiresApproval { reason: String },
}

/// 违反规则时的行为
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// 阻断操作，返回错误
    Block(String),
    /// 警告但继续执行
    Warn(String),
    /// 需要人工审批
    RequireApproval(String),
}

/// 一条业务规则
#[allow(clippy::type_complexity)]
pub struct BusinessRule {
    pub name: String,
    pub description: String,
    /// 规则评估函数：输入 (node_type, 节点输入数据) → 是否违反
    pub evaluate: Arc<dyn Fn(&str, &serde_json::Value) -> RuleResult + Send + Sync>,
    /// 违反时的行为
    pub action: RuleAction,
}

impl std::fmt::Debug for BusinessRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusinessRule")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("action", &self.action)
            .finish()
    }
}

/// 批量规则评估的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleEvaluationOutcome {
    /// 全部规则通过
    Pass,
    /// 违反了一条规则
    Violation { rule_name: String, rule_description: String, action: RuleAction, reason: String },
    /// 需要人工审批
    RequiresApproval { rule_name: String, rule_description: String, reason: String },
}

/// 业务规则评估器 trait — 由 rt-workflow 中的 BusinessRuleEngine 实现。
///
/// 拦截器等组件通过此 trait 解耦，无需直接依赖具体实现类型。
pub trait BusinessRuleEvaluator: std::fmt::Debug + Send + Sync {
    fn evaluate(&self, node_type: &str, node_input: &serde_json::Value) -> RuleEvaluationOutcome;
}
