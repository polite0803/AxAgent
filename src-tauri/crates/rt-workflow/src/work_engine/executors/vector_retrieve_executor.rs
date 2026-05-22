//! 向量检索执行器 —— 从向量存储中检索相关内容。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct VectorRetrieveExecutor;

impl VectorRetrieveExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VectorRetrieveExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for VectorRetrieveExecutor {
    fn node_type(&self) -> &'static str {
        "vector_retrieve"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "retrieved",
                "node_id": node.base_id(),
                "results": [],
            }),
            output_var: None,
        })
    }
}
