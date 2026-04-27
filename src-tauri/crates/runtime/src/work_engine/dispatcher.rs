use std::collections::HashMap;

use axagent_core::workflow_types::WorkflowNode;

use super::executors::{AtomicSkillExecutor, LlmExecutor, SubWorkflowExecutor};
use super::node_executor_trait::{NodeError, NodeExecutorTrait};
use super::{ExecutionState, NodeOutput};

pub struct NodeDispatcher {
    executors: HashMap<&'static str, Box<dyn NodeExecutorTrait>>,
}

impl Default for NodeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeDispatcher {
    pub fn new() -> Self {
        let mut dispatcher = Self {
            executors: HashMap::new(),
        };
        dispatcher.register(AtomicSkillExecutor::new());
        dispatcher.register(LlmExecutor::new());
        dispatcher.register(SubWorkflowExecutor::new());
        dispatcher
    }

    pub fn register<E: NodeExecutorTrait + 'static>(&mut self, executor: E) {
        self.executors
            .insert(executor.node_type(), Box::new(executor));
    }

    pub async fn dispatch(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let node_type = match node {
            WorkflowNode::AtomicSkill(_) => "atomic_skill",
            WorkflowNode::Agent(_) => "agent",
            WorkflowNode::Llm(_) => "llm",
            WorkflowNode::SubWorkflow(_) => "sub_workflow",
            _ => {
                return Err(NodeError::UnsupportedNodeType(format!(
                    "Node type {:?} is handled by the DAG engine directly",
                    node
                )));
            }
        };

        let executor = self.executors.get(node_type).ok_or_else(|| {
            NodeError::UnsupportedNodeType(format!("No executor registered for {}", node_type))
        })?;

        executor.execute(node, context).await
    }

    pub fn get_executor(&self, node_type: &str) -> Option<&dyn NodeExecutorTrait> {
        self.executors.get(node_type).map(|e| e.as_ref())
    }

    pub fn registered_types(&self) -> Vec<&'static str> {
        self.executors.keys().copied().collect()
    }
}
