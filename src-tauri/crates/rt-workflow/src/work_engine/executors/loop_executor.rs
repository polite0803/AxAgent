//! 循环执行器 —— 返回循环配置信息。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct LoopExecutor;

impl LoopExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for LoopExecutor {
    fn node_type(&self) -> &'static str {
        "loop"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "loop_start",
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
