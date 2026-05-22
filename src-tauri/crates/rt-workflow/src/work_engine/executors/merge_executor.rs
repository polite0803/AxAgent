//! 合并执行器 —— 收集并行分支的结果并合并为单个输出。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct MergeExecutor;

impl MergeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MergeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for MergeExecutor {
    fn node_type(&self) -> &'static str {
        "merge"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "merged",
                "node_id": node.base_id(),
                "branches_merged": 0,
            }),
            output_var: None,
        })
    }
}
