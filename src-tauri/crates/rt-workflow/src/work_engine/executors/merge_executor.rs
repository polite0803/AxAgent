//! 合并执行器 —— 从上下文变量中收集并行分支的输出并合并。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::{MergeStrategy, WorkflowNode};

pub struct MergeExecutor;
impl MergeExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MergeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn merge_strategy_to_string(strategy: &MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::All => "all",
        MergeStrategy::Any => "any",
        MergeStrategy::Race => "race",
        MergeStrategy::Majority => "majority",
    }
}

#[async_trait]
impl NodeExecutorTrait for MergeExecutor {
    fn node_type(&self) -> &'static str {
        "merge"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Merge(mn) = node else {
            return Err(NodeError::type_mismatch(
                "merge".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let collected: serde_json::Value =
            mn.config
                .inputs
                .iter()
                .fold(serde_json::json!({}), |mut acc, input_var| {
                    if let Some(val) = context.variables.get(input_var) {
                        acc[input_var] = val.clone();
                    }
                    acc
                });
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "merged",
                "merge_type": merge_strategy_to_string(&mn.config.merge_type),
                "merge_strategy": mn.config.merge_type,
                "collected_inputs": collected,
                "input_count": mn.config.inputs.len(),
                "auto_inputs_from_branches": mn.config.auto_inputs_from_branches,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
