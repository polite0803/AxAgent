//! 工具执行器 —— 解析 ToolNodeConfig 后通过注入的回调调用 MCP 工具。
//!
//! 默认无回调时返回清晰的"需要注入"错误，避免静默失败。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
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
    callback: Arc<tokio::sync::Mutex<Option<ToolCallback>>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            callback: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
    /// 设置工具回调（Arc<WorkEngine> 下可安全调用）
    pub async fn set_callback(&self, cb: ToolCallback) {
        *self.callback.lock().await = Some(cb);
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
            return Err(NodeError::type_mismatch(
                "tool".to_string(),
                super::node_type_name(node).to_string(),
            ));
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
        let cb_guard = self.callback.lock().await;
        let output = if let Some(ref cb) = *cb_guard {
            cb(tool_node.config.tool_name.clone(), resolved_args.clone())
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::TOOL_CALL_FAILED,
                        format!("Tool call failed: {e}"),
                    )
                })?
        } else {
            drop(cb_guard);
            serde_json::json!({
                "status": "tool_not_configured",
                "tool_name": tool_node.config.tool_name,
                "resolved_arguments": resolved_args,
                "message": "工具执行器未注入 MCP 回调，通过 ToolExecutor::set_callback() 注入",
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
