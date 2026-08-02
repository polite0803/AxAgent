// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域规则 — 预置业务规则集
//!
//! 提供 OPC 模块内置的业务规则，用于工作流节点拦截器。
//! 规则通过闭包实现评估逻辑，注册到 BusinessRuleEngine。

use axagent_harness::business_rules::{BusinessRule, RuleAction, RuleResult};
use std::sync::Arc;

/// 创建一个金额阈值审批规则
fn make_amount_rule(name: &str, desc: &str, threshold: f64) -> BusinessRule {
    BusinessRule {
        name: name.to_string(),
        description: desc.to_string(),
        evaluate: Arc::new(move |_node_type: &str, input: &serde_json::Value| {
            if let Some(amount) = input.get("amount").and_then(|v| v.as_f64())
                && amount > threshold
            {
                return RuleResult::RequiresApproval {
                    reason: format!("金额 ¥{amount:.2} 超过阈值 ¥{threshold:.2}，需要人工审批"),
                };
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
