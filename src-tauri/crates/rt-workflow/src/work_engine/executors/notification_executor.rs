// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct NotificationExecutor;

impl NotificationExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotificationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for NotificationExecutor {
    fn node_type(&self) -> &'static str {
        "notification"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Notification(n) = node else {
            return Err(NodeError::type_mismatch("notification", self.node_type()));
        };
        let c = &n.config;
        tracing::info!("[Notification] channel={} message={}", c.channel, c.message);
        Ok(NodeOutput {
            output: serde_json::json!({"sent": true, "channel": c.channel, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
