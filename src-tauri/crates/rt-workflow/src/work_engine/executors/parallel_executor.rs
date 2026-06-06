use async_trait::async_trait;
use axagent_core::workflow_types::{MergeStrategy, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

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

/// 把 auto_input_from_parent + wait_for_all 翻译为 engine 调度时需要的信息，
/// 并把当前 context 的 variables 拷贝成可被下游分支读取的"父输入"。
#[async_trait]
impl NodeExecutorTrait for ParallelExecutor {
    fn node_type(&self) -> &'static str {
        "parallel"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Parallel(n) = node else {
            return Err(NodeError::type_mismatch("parallel", self.node_type()));
        };
        let c = &n.config;

        if c.branches.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Parallel node has no branches".to_string(),
            ));
        }

        // 收集每个 branch 的入口数据：auto_input_from_parent=true 时继承 context.variables
        // 快照，否则要求显式 input_var。
        let mut branch_inputs = serde_json::Map::new();
        for branch in &c.branches {
            let value = if c.auto_input_from_parent {
                serde_json::to_value(&context.variables).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            branch_inputs.insert(branch.id.clone(), value);
        }

        let aggregation = c.aggregation.clone().unwrap_or_default();
        let merge_label = match aggregation {
            MergeStrategy::All => "all",
            MergeStrategy::Any => "any",
            MergeStrategy::Race => "race",
            MergeStrategy::Majority => "majority",
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "branch_count": c.branches.len(),
                "wait_for_all": c.wait_for_all,
                "timeout": c.timeout,
                "aggregation": merge_label,
                "auto_input_from_parent": c.auto_input_from_parent,
                "branch_inputs": branch_inputs,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
