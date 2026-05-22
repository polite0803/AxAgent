//! 校验执行器 —— 根据 ValidationNodeConfig 执行断言校验。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ValidationExecutor;

impl ValidationExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ValidationExecutor {
    fn node_type(&self) -> &'static str {
        "validation"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Validation(validation_node) = node else {
            return Err(NodeError::InvalidNodeType {
                expected: "validation".to_string(),
                got: super::node_type_name(node).to_string(),
            });
        };

        let mut results = Vec::new();
        let mut all_passed = true;

        for assertion in &validation_node.config.assertions {
            let actual_value = match &assertion.actual {
                Some(path) => resolve_var_path(path, context),
                None => None,
            };

            let expected_value = match &assertion.expected {
                Some(schema_json) => {
                    // expected 是 JSON Schema 字符串时，进行 schema 校验
                    serde_json::from_str::<serde_json::Value>(schema_json).ok()
                },
                None => None,
            };

            let passed = match assertion.assertion_type.as_str() {
                "json_schema" => {
                    if let (Some(expected), Some(actual)) = (&expected_value, &actual_value) {
                        let (valid, _) = axagent_core::validate_against_schema(actual, expected);
                        valid
                    } else {
                        false
                    }
                },
                "not_null" => actual_value.is_some() && !actual_value.as_ref().unwrap().is_null(),
                "non_empty" => actual_value.as_ref().is_some_and(|v| {
                    v.as_str().is_some_and(|s| !s.is_empty())
                        || v.as_array().is_some_and(|a| !a.is_empty())
                }),
                "contains" => {
                    if let (Some(actual), Some(expected)) = (&actual_value, &expected_value) {
                        actual
                            .as_str()
                            .zip(expected.as_str())
                            .is_some_and(|(a, e)| a.contains(e))
                    } else {
                        false
                    }
                },
                _ => {
                    // 未知断言类型：跳过
                    true
                },
            };

            results.push(serde_json::json!({
                "assertion_type": assertion.assertion_type,
                "passed": passed,
            }));
            if !passed {
                all_passed = false;
            }
        }

        let on_fail = &validation_node.config.on_fail;
        if !all_passed && on_fail == "abort" {
            return Err(NodeError::Validation(format!(
                "校验失败: {}",
                serde_json::to_string(&results).unwrap_or_default()
            )));
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": if all_passed { "validated" } else { "validation_failed" },
                "valid": all_passed,
                "results": results,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}

/// 从 ExecutionState 变量中解析点分隔路径（如 "result.text" → variables["result"]["text"]）。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let root = context.variables.get(parts[0])?.clone();
    let mut current = root;
    for part in &parts[1..] {
        current = current.get(part)?.clone();
    }
    Some(current)
}
