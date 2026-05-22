//! 终止执行器 —— 标记工作流结束位置。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct EndExecutor;

impl EndExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EndExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for EndExecutor {
    fn node_type(&self) -> &'static str {
        "end"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "terminated",
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
