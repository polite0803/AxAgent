use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub entity_type: String,
    pub rule_type: RuleType,
    pub condition: serde_json::Value,
    pub severity: RuleSeverity,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum RuleType {
    #[default]
    Validation,
    Computation,
    BusinessLogic,
    Compliance,
    Trigger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum RuleSeverity {
    Info,
    #[default]
    Warning,
    Error,
    Critical,
}

impl RuleDefinition {
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        rule_type: RuleType,
    ) -> Self {
        Self {
            id: 0,
            name: name.into(),
            description: String::new(),
            entity_type: entity_type.into(),
            rule_type,
            condition: serde_json::Value::Null,
            severity: RuleSeverity::default(),
            is_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub rule_id: Option<i64>,
    pub severity: RuleSeverity,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            rule_id: None,
            severity: RuleSeverity::Error,
        }
    }
}

// ── OPC 业务规则集（融合自 opc-dao/rules.rs） ──────────────────

use axagent_harness::business_rules::{BusinessRule, RuleAction, RuleResult};
use axagent_harness::workflow_types::NodeKind;
use std::sync::Arc;

/// 创建一个金额阈值审批规则
fn make_amount_rule(name: &str, desc: &str, threshold: f64) -> BusinessRule {
    BusinessRule {
        name: name.to_string(),
        description: desc.to_string(),
        evaluate: Arc::new(move |_node_type: &NodeKind, input: &serde_json::Value| {
            if let Some(amount) = input.get("amount").and_then(|v| v.as_f64()) {
                if amount > threshold {
                    return RuleResult::RequiresApproval {
                        reason: format!("金额 ¥{amount:.2} 超过阈值 ¥{threshold:.2}，需要人工审批"),
                    };
                }
            }
            RuleResult::Pass
        }),
        action: RuleAction::RequireApproval(format!("金额超过 ¥{threshold:.2} 需要审批")),
    }
}

/// 返回完整的 OPC 业务规则集合
pub fn opc_business_rules() -> Vec<BusinessRule> {
    vec![
        make_amount_rule("invoice_high_value", "单笔发票金额超过 ¥10,000 需审批", 10_000.0),
        make_amount_rule("customer_total_revenue", "客户累计收款超过 ¥100,000 需审批", 100_000.0),
    ]
}
