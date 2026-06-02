use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct WebhookSendExecutor;

impl WebhookSendExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebhookSendExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for WebhookSendExecutor {
    fn node_type(&self) -> &'static str {
        "webhookSend"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::WebhookSend(n) = node else {
            return Err(NodeError::type_mismatch("webhookSend", self.node_type()));
        };
        let c = &n.config;
        Ok(NodeOutput {
            output: serde_json::json!({"url": c.url, "method": c.method, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
