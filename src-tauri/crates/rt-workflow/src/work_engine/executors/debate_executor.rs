//! 辩论容器执行器 —— 读取 debater_steps 配置，返回辩手列表供 DAG 引擎调度。
//!
//! 与 ParallelExecutor 类似，DebateExecutor 作为容器节点只负责返回子节点信息。
//! 实际的多轮辩论编排由 DAG 引擎根据 debater_steps 和 max_rounds 驱动。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

pub struct DebateExecutor;
impl DebateExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for DebateExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for DebateExecutor {
    fn node_type(&self) -> &'static str {
        "debate"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Debate(dn) = node else {
            return Err(NodeError::type_mismatch(
                "debate".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let debater_count = dn.config.debater_steps.len();
        let max_rounds = dn.config.max_rounds;

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "debate_initiated",
                "debater_count": debater_count,
                "debater_steps": dn.config.debater_steps,
                "max_rounds": max_rounds,
                "topic_var": dn.config.topic_var,
                "convergence_prompt": dn.config.convergence_prompt,
                "node_id": node.base_id(),
            }),
            output_var: Some(dn.config.output_var.clone()),
        })
    }
}
