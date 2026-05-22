//! 触发器执行器 —— 工作流的入口节点，标记触发已激活。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct TriggerExecutor;

impl TriggerExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TriggerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for TriggerExecutor {
    fn node_type(&self) -> &'static str {
        "trigger"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "triggered",
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
