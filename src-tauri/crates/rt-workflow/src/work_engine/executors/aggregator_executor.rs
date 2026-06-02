use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct AggregatorExecutor;

impl AggregatorExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeExecutorTrait for AggregatorExecutor {
    fn node_type(&self) -> &'static str {
        "aggregator"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Aggregator(n) = node else {
            return Err(NodeError::type_mismatch("aggregator", self.node_type()));
        };
        let c = &n.config;
        let mut collected = Vec::new();
        for src in &c.input_sources {
            if let Some(val) = ctx.variables.get(src.as_str()) {
                collected.push(val.clone());
            }
        }
        Ok(NodeOutput {
            output: serde_json::json!({"strategy": c.strategy, "count": collected.len(), "data": collected, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
