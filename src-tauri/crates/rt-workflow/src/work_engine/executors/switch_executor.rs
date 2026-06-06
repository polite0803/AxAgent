use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct SwitchExecutor;

impl SwitchExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SwitchExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析点号分隔路径，从 ExecutionState.variables 提取目标值。
/// 空路径直接返回 None；segments 中间值非对象也返回 None。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }
    let mut current = context.variables.get(path)?.clone();
    for segment in path.split('.').skip(1) {
        current = current.get(segment)?.clone();
    }
    Some(current)
}

#[async_trait]
impl NodeExecutorTrait for SwitchExecutor {
    fn node_type(&self) -> &'static str {
        "switch"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Switch(n) = node else {
            return Err(NodeError::type_mismatch("switch", self.node_type()));
        };
        let c = &n.config;

        // 1. 取 input_var 的实际值
        let actual = resolve_var_path(&c.input_var, context);
        let actual_str = actual.as_ref().and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            serde_json::Value::Null => Some(String::new()),
            other => Some(other.to_string()),
        });

        // 2. 按 match_mode 匹配 case.value，确定 matched_label
        //    - "exact"（默认）：实际值 == case.value 字符串
        //    - "regex"：用 regex 匹配 case.value（需要 regex crate，harness 中已可用）
        //    - "contains"：实际值包含 case.value
        let matched_label: Option<String> = match actual_str.as_deref() {
            None => c.default_case.clone(),
            Some(needle) => {
                let mut found: Option<String> = None;
                for case in &c.cases {
                    let hit = match c.match_mode.as_str() {
                        "regex" => {
                            // 编译失败视为不匹配（不 panic 阻断流程）
                            match regex::Regex::new(&case.value) {
                                Ok(re) => re.is_match(needle),
                                Err(e) => {
                                    tracing::warn!(
                                        "[SwitchExecutor] case '{}' regex 编译失败: {e}",
                                        case.label
                                    );
                                    false
                                },
                            }
                        },
                        "contains" => needle.contains(&case.value),
                        _ => needle == case.value, // exact / 默认
                    };
                    if hit {
                        found = Some(case.label.clone());
                        break;
                    }
                }
                found.or_else(|| c.default_case.clone())
            },
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "input_var": c.input_var,
                "actual_value": actual,
                "matched_label": matched_label,
                "case_count": c.cases.len(),
                "match_mode": c.match_mode,
                "node_id": node.base_id(),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
