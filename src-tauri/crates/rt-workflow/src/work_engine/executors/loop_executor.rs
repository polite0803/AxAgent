//! 循环执行器 —— 根据 LoopNodeConfig 初始化和迭代循环变量。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

pub struct LoopExecutor;
impl LoopExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for LoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for LoopExecutor {
    fn node_type(&self) -> &'static str {
        "loop"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Loop(ln) = node else {
            return Err(NodeError::type_mismatch(
                "loop".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let items: Vec<serde_json::Value> = ln
            .config
            .items_var
            .as_ref()
            .and_then(|v| context.variables.get(v))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "loop_initiated",
                "loop_type": format!("{:?}", ln.config.loop_type),
                "item_count": items.len(),
                "max_iterations": ln.config.max_iterations,
                "iteratee_var": ln.config.iteratee_var,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
