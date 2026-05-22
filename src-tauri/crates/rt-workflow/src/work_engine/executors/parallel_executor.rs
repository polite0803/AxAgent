//! 并行执行器 —— 读取分支配置，返回分支列表供 DAG 引擎调度。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

pub struct ParallelExecutor;
impl ParallelExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ParallelExecutor {
    fn node_type(&self) -> &'static str {
        "parallel"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Parallel(pn) = node else {
            return Err(NodeError::type_mismatch(
                "parallel".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let branches: Vec<String> = pn.config.branches.iter().map(|b| b.id.clone()).collect();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "parallel_initiated",
                "branch_count": branches.len(),
                "branches": branches,
                "wait_for_all": pn.config.wait_for_all,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
