//! 兜底执行器 —— 处理尚未实现专用执行器的节点类型。
//!
//! 返回模拟 JSON 而非 UnsupportedNodeType 错误，确保工作流不会因不支持的节点类型而崩溃。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct FallbackExecutor;

impl FallbackExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FallbackExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for FallbackExecutor {
    fn node_type(&self) -> &'static str {
        "fallback"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let type_name = super::node_type_name(node);
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "degraded",
                "node_type": type_name,
                "node_id": node.base_id(),
                "message": "此节点类型尚未实现专用执行器，使用兜底模拟输出"
            }),
            output_var: None,
        })
    }
}
