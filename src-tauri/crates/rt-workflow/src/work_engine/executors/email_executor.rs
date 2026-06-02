use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct EmailExecutor;

impl EmailExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmailExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for EmailExecutor {
    fn node_type(&self) -> &'static str {
        "email"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Email(n) = node else {
            return Err(NodeError::type_mismatch("email", self.node_type()));
        };
        let c = &n.config;
        tracing::info!("[Email] to={:?} subject={}", c.to, c.subject);
        Ok(NodeOutput {
            output: serde_json::json!({"to": c.to, "subject": c.subject, "sent": true, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
