use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DataTransformerExecutor;

impl DataTransformerExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeExecutorTrait for DataTransformerExecutor {
    fn node_type(&self) -> &'static str {
        "dataTransformer"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::DataTransformer(n) = node else {
            return Err(NodeError::type_mismatch("dataTransformer", self.node_type()));
        };
        let c = &n.config;
        Ok(NodeOutput {
            output: serde_json::json!({"expression": c.expression, "input_var": c.input_var, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
