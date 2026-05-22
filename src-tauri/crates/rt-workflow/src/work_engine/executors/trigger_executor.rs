//! 触发器执行器 —— 解析触发配置（manual/schedule/webhook/event）并初始化工作流入口变量。

use async_trait::async_trait;
use axagent_core::workflow_types::{TriggerType, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct TriggerExecutor;

impl TriggerExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for TriggerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for TriggerExecutor {
    fn node_type(&self) -> &'static str {
        "trigger"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Trigger(trigger_node) = node else {
            return Err(NodeError::type_mismatch(
                "trigger".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let trigger_type = match trigger_node.config.trigger_type {
            TriggerType::Manual => "manual",
            TriggerType::Schedule => "schedule",
            TriggerType::Webhook => "webhook",
            TriggerType::Event => "event",
        };
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "triggered",
                "trigger_type": trigger_type,
                "config": trigger_node.config.config,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
