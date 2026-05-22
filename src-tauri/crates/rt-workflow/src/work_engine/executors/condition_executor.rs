//! 条件执行器 —— 根据 ConditionNodeConfig 评估条件表达式。

use async_trait::async_trait;
use axagent_core::workflow_types::{CompareOperator, LogicalOperator, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ConditionExecutor;

impl ConditionExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConditionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ConditionExecutor {
    fn node_type(&self) -> &'static str {
        "condition"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Condition(condition_node) = node else {
            return Err(NodeError::InvalidNodeType {
                expected: "condition".to_string(),
                got: super::node_type_name(node).to_string(),
            });
        };

        let mut results = Vec::new();

        for condition in &condition_node.config.conditions {
            let actual = resolve_var_path(&condition.var_path, context);
            let passed = evaluate_single(&condition.operator, &actual, &condition.value);
            results.push(passed);
        }

        let overall = match condition_node.config.logical_op {
            LogicalOperator::And => results.iter().all(|&r| r),
            LogicalOperator::Or => results.iter().any(|&r| r),
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "evaluated",
                "result": overall,
                "conditions_checked": results.len(),
                "passed_count": results.iter().filter(|&&r| r).count(),
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}

/// 评估单个条件比较。
fn evaluate_single(
    op: &CompareOperator,
    actual: &Option<serde_json::Value>,
    expected: &serde_json::Value,
) -> bool {
    let Some(actual) = actual else {
        return matches!(op, CompareOperator::Ne | CompareOperator::IsEmpty);
    };

    match op {
        CompareOperator::Eq => actual == expected,
        CompareOperator::Ne => actual != expected,
        CompareOperator::Gt => compare_values(actual, expected) == std::cmp::Ordering::Greater,
        CompareOperator::Lt => compare_values(actual, expected) == std::cmp::Ordering::Less,
        CompareOperator::Gte => {
            !matches!(compare_values(actual, expected), std::cmp::Ordering::Less)
        },
        CompareOperator::Lte => {
            !matches!(compare_values(actual, expected), std::cmp::Ordering::Greater)
        },
        CompareOperator::Contains => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.contains(e)),
        CompareOperator::NotContains => actual
            .as_str()
            .zip(expected.as_str())
            .is_none_or(|(a, e)| !a.contains(e)),
        CompareOperator::StartsWith => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.starts_with(e)),
        CompareOperator::EndsWith => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.ends_with(e)),
        CompareOperator::RegexMatch => {
            actual
                .as_str()
                .zip(expected.as_str())
                .is_some_and(|(a, pat)| {
                    // 简单子串匹配作为 regex 的降级实现。
                    // 完整正则支持需要引入 regex crate。
                    a.contains(pat)
                })
        },
        CompareOperator::IsEmpty => {
            actual.is_null() || actual.as_str().is_some_and(|s| s.is_empty())
        },
        CompareOperator::IsNotEmpty => {
            !actual.is_null() && actual.as_str().is_none_or(|s| !s.is_empty())
        },
    }
}

/// 比较两个 JSON 值（数值按 f64，其他按字符串）。
fn compare_values(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(a_num), Some(b_num)) => a_num
            .partial_cmp(&b_num)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

/// 从 ExecutionState 变量中解析点分隔路径。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let root = context.variables.get(parts[0])?.clone();
    let mut current = root;
    for part in &parts[1..] {
        current = current.get(part)?.clone();
    }
    Some(current)
}
