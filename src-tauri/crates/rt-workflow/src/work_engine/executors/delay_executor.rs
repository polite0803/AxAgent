//! 延迟执行器 —— 根据 DelayNodeConfig 等待指定时长。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DelayExecutor;

impl DelayExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DelayExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for DelayExecutor {
    fn node_type(&self) -> &'static str {
        "delay"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Delay(delay_node) = node else {
            return Err(NodeError::InvalidNodeType {
                expected: "delay".to_string(),
                got: super::node_type_name(node).to_string(),
            });
        };

        let seconds = delay_node.config.seconds;
        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "delayed",
                "delay_type": delay_node.config.delay_type,
                "seconds": seconds,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
