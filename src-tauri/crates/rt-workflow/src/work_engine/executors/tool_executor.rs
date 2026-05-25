//! 工具执行器 —— 解析 ToolNodeConfig 后通过注入的回调调用 MCP 工具。
//!
//! 默认无回调时返回清晰的"需要注入"错误，避免静默失败。
//!
//! 回调查找优先级：
//!   1. `context.callbacks.tool_handlers` 按 tool_name 精确匹配（多路注册）
//!   2. `context.callbacks.tool_fallback` 旧版全局回调（兼容）

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

        let tool_name = &tool_node.config.tool_name;

        // 查找回调：优先多路注册 → fallback → 未配置
        let cb: Option<ToolCallback> = context
            .callbacks
            .as_ref()
            .and_then(|cbs| cbs.tool_handlers.get(tool_name).cloned())
            .or_else(|| {
                context
                    .callbacks
                    .as_ref()
                    .and_then(|cbs| cbs.tool_fallback.clone())
            });

        let output = if let Some(ref cb) = cb {
            cb(tool_name.clone(), resolved_args.clone())
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::TOOL_CALL_FAILED,
                        format!("Tool call failed: {e}"),
                    )
                })?
        } else {
            serde_json::json!({
                "status": "tool_not_configured",
                "tool_name": tool_name,
                "resolved_arguments": resolved_args,
                "message": "工具未注册，请通过 WorkEngine::register_tool_handler() 注册",
                "node_id": node.base_id(),
            })
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "tool_name": tool_name,
                "result": output,
                "node_id": node.base_id(),
            }),
            output_var: Some(tool_node.config.output_var.clone()),
        })
    }
}

fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    // 尝试按节点输出路径解析：root 为节点 ID，后续为嵌套字段
    if let Some(root) = context.variables.get(parts[0]) {
        let mut current = root.clone();
        for part in &parts[1..] {
            current = current.get(part)?.clone();
        }
        return Some(current);
    }
    // fallback：root 不是节点 ID，将整个 path 作为模板变量名直查
    context.variables.get(path).cloned()
}
