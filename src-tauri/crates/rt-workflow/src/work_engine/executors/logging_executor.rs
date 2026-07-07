// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct LoggingExecutor;

impl LoggingExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for LoggingExecutor {
    fn node_type(&self) -> &'static str {
        "logging"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Logging(n) = node else {
            return Err(NodeError::type_mismatch("logging", self.node_type()));
        };
        let c = &n.config;
        match c.level.as_str() {
            "debug" => tracing::debug!("{}", c.message),
            "warn" => tracing::warn!("{}", c.message),
            "error" => tracing::error!("{}", c.message),
            _ => tracing::info!("{}", c.message),
        }
        Ok(NodeOutput {
            output: serde_json::json!({"level": c.level, "logged": true, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
