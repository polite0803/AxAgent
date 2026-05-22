//! 工具执行器 —— 调用 MCP 工具或内置函数。
//!
//! 仅处理 `WorkflowNode::Tool`。通过 `ToolNodeConfig` 配置工具名称和参数映射，
//! 从执行上下文变量中解析输入参数后返回工具调用的配置信息。
//! 后续可通过 `register_executor` 注入真实的 MCP 客户端回调实现实际工具调用。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ToolExecutor;

impl ToolExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ToolExecutor {
    fn node_type(&self) -> &'static str {
        "tool"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Tool(tool_node) = node else {
            return Err(NodeError::InvalidNodeType {
                expected: "tool".to_string(),
                got: super::node_type_name(node).to_string(),
            });
        };

        // 解析输入映射，从上下文变量中提取参数
        let resolved_args: serde_json::Value =
            tool_node
                .config
                .input_mapping
                .iter()
                .fold(serde_json::json!({}), |mut acc, (k, v)| {
                    let resolved = resolve_var_path(v, context);
                    acc[k] = resolved.unwrap_or(serde_json::Value::Null);
                    acc
                });

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "tool_dispatched",
                "tool_name": tool_node.config.tool_name,
                "arguments": resolved_args,
                "node_id": node.base_id(),
            }),
            output_var: Some(tool_node.config.output_var.clone()),
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
