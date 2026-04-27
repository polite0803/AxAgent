use async_trait::async_trait;
use axagent_core::workflow_types::{LLMNode, WorkflowNode};

use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait};
use crate::work_engine::{ExecutionState, NodeOutput};

pub struct LlmExecutor;

impl LlmExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for LlmExecutor {
    fn node_type(&self) -> &'static str {
        "llm"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        match node {
            WorkflowNode::Llm(llm_node) => Self::execute_llm(llm_node, context).await,
            _ => Err(NodeError::UnsupportedNodeType(format!(
                "Expected LLM node, got {:?}",
                node
            ))),
        }
    }
}

impl LlmExecutor {
    async fn execute_llm(
        node: &LLMNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "model": node.config.model,
                "prompt": node.config.prompt,
            }),
            output_var: None,
        })
    }
}
