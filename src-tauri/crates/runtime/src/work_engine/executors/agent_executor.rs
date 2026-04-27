use async_trait::async_trait;
use axagent_core::workflow_types::{AgentNode, WorkflowNode};

use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait};
use crate::work_engine::{ExecutionState, NodeOutput};

pub struct AgentExecutor;

impl AgentExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for AgentExecutor {
    fn node_type(&self) -> &'static str {
        "agent"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        match node {
            WorkflowNode::Agent(agent_node) => Self::execute_agent(agent_node, context).await,
            _ => Err(NodeError::UnsupportedNodeType(format!(
                "Expected Agent node, got {:?}",
                node
            ))),
        }
    }
}

impl AgentExecutor {
    async fn execute_agent(
        node: &AgentNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "role": node.config.role.as_str(),
                "system_prompt": node.config.system_prompt,
                "output_var": node.config.output_var,
            }),
            output_var: Some(node.config.output_var.clone()),
        })
    }
}
