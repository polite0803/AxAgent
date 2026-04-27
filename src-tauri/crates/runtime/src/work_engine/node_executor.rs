use axagent_core::workflow_types::WorkflowNode;

use super::dispatcher::NodeDispatcher;
use super::execution_state::ExecutionState;
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub use super::node_executor_trait::NodeError as NodeExecutorError;

pub struct NodeExecutor {
    dispatcher: NodeDispatcher,
}

impl Default for NodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeExecutor {
    pub fn new() -> Self {
        Self {
            dispatcher: NodeDispatcher::new(),
        }
    }

    pub async fn execute_node(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        self.dispatcher.dispatch(node, context).await
    }

    pub fn register<E: NodeExecutorTrait + 'static>(&mut self, executor: E) {
        self.dispatcher.register(executor);
    }

    pub fn registered_types(&self) -> Vec<&'static str> {
        self.dispatcher.registered_types()
    }
}
