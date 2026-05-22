//! 并行执行器 —— 标记并行分支起始点。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ParallelExecutor;

impl ParallelExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ParallelExecutor {
    fn node_type(&self) -> &'static str {
        "parallel"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "parallel_branch_start",
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
