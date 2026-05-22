//! 工具执行器 —— 解析 ToolNodeConfig 后通过注入的回调调用 MCP 工具。
//!
//! 默认无回调时返回清晰的"需要注入"错误，避免静默失败。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use std::pin::Pin;
use std::sync::Arc;

pub type ToolCallback = Arc<
    dyn Fn(
            String,
            serde_json::Value,
        )
            -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;

pub struct ToolExecutor {
    callback: Option<ToolCallback>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self { callback: None }
    }
    pub fn with_callback(mut self, cb: ToolCallback) -> Self {
        self.callback = Some(cb);
        self
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

        // 解析输入映射
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

        // 调用工具回调（若已注入）
        let output = if let Some(ref cb) = self.callback {
            cb(tool_node.config.tool_name.clone(), resolved_args.clone())
                .await
                .map_err(|e| NodeError::ExecutionFailed(format!("工具调用失败: {e}")))?
        } else {
            serde_json::json!({
                "status": "tool_not_configured",
                "tool_name": tool_node.config.tool_name,
                "resolved_arguments": resolved_args,
                "message": "工具执行器未注入 MCP 回调，通过 register_executor 注入 ToolExecutor::with_callback()",
                "node_id": node.base_id(),
            })
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "tool_name": tool_node.config.tool_name,
                "result": output,
                "node_id": node.base_id(),
            }),
            output_var: Some(tool_node.config.output_var.clone()),
        })
    }
}

fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let root = context.variables.get(parts[0])?.clone();
    let mut current = root;
    for part in &parts[1..] {
        current = current.get(part)?.clone();
    }
    Some(current)
}
