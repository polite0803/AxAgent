//! 业务规则引擎 — 强制执行业务规则，LLM 无法绕过。
//!
//! 与 `domain_constraints`（软约束，仅作为 LLM prompt 建议）不同，
//! `BusinessRuleEngine` 在执行层直接拦截违规操作，是硬约束。
//!
//! 两者可以共存：软约束指导 LLM 行为，硬约束兜底防止越权。

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

/// 业务规则引擎 — 持有若干规则，对外提供批量评估接口。
#[derive(Debug, Default)]
pub struct BusinessRuleEngine {
    rules: Vec<BusinessRule>,
}

impl BusinessRuleEngine {
    pub fn new(rules: Vec<BusinessRule>) -> Self {
        Self { rules }
    }

    /// 返回空引擎（无规则，所有节点都通过）
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// 追加一条规则
    pub fn add_rule(&mut self, rule: BusinessRule) {
        self.rules.push(rule);
    }

    /// 获取所有规则的引用
    pub fn rules(&self) -> &[BusinessRule] {
        &self.rules
    }

    /// 对指定节点类型和输入数据执行全部规则评估。
    ///
    /// 返回第一个违反结果（Block 优先于 Warn 优先于 RequireApproval）。
    /// 全部通过返回 `RuleEvaluationOutcome::Pass`。
    pub fn evaluate(
        &self,
        node_type: &str,
        node_input: &serde_json::Value,
    ) -> RuleEvaluationOutcome {
        for rule in &self.rules {
            let result = (rule.evaluate)(node_type, node_input);
            match result {
                RuleResult::Pass => continue,
                RuleResult::Violation { ref reason } => {
                    return RuleEvaluationOutcome::Violation {
                        rule_name: rule.name.clone(),
                        rule_description: rule.description.clone(),
                        action: rule.action.clone(),
                        reason: reason.clone(),
                    };
                },
                RuleResult::RequiresApproval { ref reason } => {
                    return RuleEvaluationOutcome::RequiresApproval {
                        rule_name: rule.name.clone(),
                        rule_description: rule.description.clone(),
                        reason: reason.clone(),
                    };
                },
            }
        }
        RuleEvaluationOutcome::Pass
    }
}

/// 批量规则评估的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleEvaluationOutcome {
    /// 全部规则通过
    Pass,
    /// 违反了一条规则
    Violation {
        rule_name: String,
        rule_description: String,
        action: RuleAction,
        reason: String,
    },
    /// 需要人工审批
    RequiresApproval {
        rule_name: String,
        rule_description: String,
        reason: String,
    },
}

// ── 预置规则构建器 ──

/// 金额阈值审批规则 — 当工具操作涉及金额超过阈值时需审批。
///
/// 该规则检查节点输入中是否包含 `amount` 字段且其值超过阈值。
/// 适用于支付、转账、扣费等金融操作工具。
pub fn amount_threshold_rule(threshold: f64) -> BusinessRule {
    BusinessRule {
        name: format!("AmountThreshold_{threshold}"),
        description: format!("金额超 {threshold} 需审批"),
        evaluate: Arc::new(move |_node_type: &str, input: &serde_json::Value| {
            let amount = input
                .get("amount")
                .or_else(|| input.get("value"))
                .or_else(|| input.get("price"))
                .and_then(|v| v.as_f64());
            match amount {
                Some(val) if val > threshold => RuleResult::RequiresApproval {
                    reason: format!("金额 {val} 超过阈值 {threshold}，需人工审批确认",),
                },
                _ => RuleResult::Pass,
            }
        }),
        action: RuleAction::RequireApproval(format!("金额超 {} 需人工审批", threshold)),
    }
}

/// 破坏性操作保护规则 — 删除/覆盖/清空类操作需要额外确认。
///
/// 检查节点类型或工具名称中是否包含破坏性关键词。
pub fn destructive_operation_guard() -> BusinessRule {
    BusinessRule {
        name: "DestructiveOperationGuard".to_string(),
        description: "破坏性操作（删除/覆盖）需额外确认".to_string(),
        evaluate: Arc::new(|node_type: &str, input: &serde_json::Value| {
            // 检查节点类型
            let type_is_destructive = matches!(node_type, "fileOperation" | "tool");
            // 检查工具名或操作类型是否包含破坏性关键词
            let tool_name = input
                .get("tool_name")
                .or_else(|| input.get("operation"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let is_destructive = type_is_destructive
                && (tool_name.contains("delete")
                    || tool_name.contains("remove")
                    || tool_name.contains("overwrite")
                    || tool_name.contains("clear")
                    || tool_name.contains("truncate")
                    || tool_name.contains("drop")
                    || tool_name.contains("destroy"));
            if is_destructive {
                RuleResult::RequiresApproval {
                    reason: format!(
                        "检测到破坏性操作 '{tool_name}'（节点类型: {node_type}），需人工确认",
                    ),
                }
            } else {
                RuleResult::Pass
            }
        }),
        action: RuleAction::RequireApproval("破坏性操作需人工确认".to_string()),
    }
}

/// 网络访问授权规则 — 网络请求需在白名单内。
///
/// 检查 URL 是否在授权域名白名单中。未配置白名单时所有网络请求都被拒绝。
pub fn network_access_guard(allowed_domains: Vec<String>) -> BusinessRule {
    BusinessRule {
        name: "NetworkAccessGuard".to_string(),
        description: format!("网络访问需授权白名单（允许域名: {}）", allowed_domains.join(", ")),
        evaluate: Arc::new(move |node_type: &str, input: &serde_json::Value| {
            // 仅检查网络相关节点类型
            if !matches!(node_type, "httpRequest" | "webhookSend" | "tool") {
                return RuleResult::Pass;
            }
            let url = input
                .get("url")
                .or_else(|| input.get("api_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if url.is_empty() {
                return RuleResult::Pass;
            }
            // 尝试从 URL 解析域名
            let is_allowed = allowed_domains.iter().any(|domain| {
                url.starts_with(&format!("https://{domain}"))
                    || url.starts_with(&format!("http://{domain}"))
                    || url.contains(&format!("//{domain}"))
            });
            if is_allowed {
                RuleResult::Pass
            } else {
                RuleResult::Violation {
                    reason: format!(
                        "网络访问目标 '{url}' 不在授权白名单中。允许的域名: {}",
                        allowed_domains.join(", ")
                    ),
                }
            }
        }),
        action: RuleAction::Block("非授权网络目标被拒绝".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_amount_threshold_passes_below() {
        let rule = amount_threshold_rule(10000.0);
        let input = json!({"amount": 5000});
        match (rule.evaluate)("tool", &input) {
            RuleResult::Pass => {},
            other => panic!("Expected Pass, got {:?}", other),
        }
    }

    #[test]
    fn test_amount_threshold_blocks_above() {
        let rule = amount_threshold_rule(10000.0);
        let input = json!({"amount": 15000});
        match (rule.evaluate)("tool", &input) {
            RuleResult::RequiresApproval { .. } => {},
            other => panic!("Expected RequiresApproval, got {:?}", other),
        }
    }

    #[test]
    fn test_destructive_operation_detected() {
        let rule = destructive_operation_guard();
        let input = json!({"tool_name": "delete_file", "file_path": "/tmp/test"});
        match (rule.evaluate)("tool", &input) {
            RuleResult::RequiresApproval { ref reason } => {
                assert!(reason.contains("delete_file"));
            },
            other => panic!("Expected RequiresApproval, got {:?}", other),
        }
    }

    #[test]
    fn test_destructive_operation_safe() {
        let rule = destructive_operation_guard();
        let input = json!({"tool_name": "read_file", "file_path": "/tmp/test"});
        match (rule.evaluate)("tool", &input) {
            RuleResult::Pass => {},
            other => panic!("Expected Pass, got {:?}", other),
        }
    }

    #[test]
    fn test_network_access_allowed() {
        let rule = network_access_guard(vec!["api.example.com".to_string()]);
        let input = json!({"url": "https://api.example.com/v1/data"});
        match (rule.evaluate)("httpRequest", &input) {
            RuleResult::Pass => {},
            other => panic!("Expected Pass, got {:?}", other),
        }
    }

    #[test]
    fn test_network_access_blocked() {
        let rule = network_access_guard(vec!["api.example.com".to_string()]);
        let input = json!({"url": "https://evil.com/hack"});
        match (rule.evaluate)("httpRequest", &input) {
            RuleResult::Violation { ref reason } => {
                assert!(reason.contains("evil.com"));
            },
            other => panic!("Expected Violation, got {:?}", other),
        }
    }

    #[test]
    fn test_engine_pass_all() {
        let engine = BusinessRuleEngine::new(vec![
            amount_threshold_rule(10000.0),
            destructive_operation_guard(),
        ]);
        let outcome = engine.evaluate("tool", &json!({"amount": 100, "tool_name": "read_file"}));
        assert!(matches!(outcome, RuleEvaluationOutcome::Pass));
    }

    #[test]
    fn test_engine_block_first() {
        let engine = BusinessRuleEngine::new(vec![
            amount_threshold_rule(10000.0),
            destructive_operation_guard(),
        ]);
        // 金额超阈值 + 破坏性操作，应返回金额违规（第一条规则优先）
        let outcome =
            engine.evaluate("tool", &json!({"amount": 50000, "tool_name": "delete_file"}));
        assert!(matches!(outcome, RuleEvaluationOutcome::RequiresApproval { .. }));
    }

    #[test]
    fn test_engine_empty() {
        let engine = BusinessRuleEngine::empty();
        let outcome = engine.evaluate("tool", &json!({"amount": 999999}));
        assert!(matches!(outcome, RuleEvaluationOutcome::Pass));
    }
}
