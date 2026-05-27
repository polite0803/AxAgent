//! 延迟执行器 —— 根据 DelayNodeConfig 等待指定时长，支持取消检查。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DelayExecutor;

impl DelayExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DelayExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for DelayExecutor {
    fn node_type(&self) -> &'static str {
        "delay"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Delay(delay_node) = node else {
            return Err(NodeError::type_mismatch(
                "delay".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let seconds = delay_node.config.seconds;
        let cancel_token = context.cancel_token.clone();
        let sleep_future = tokio::time::sleep(std::time::Duration::from_secs(seconds));

        tokio::select! {
            _ = sleep_future => {},
            _ = async {
                if let Some(token) = cancel_token.as_ref() {
                    token.cancelled().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                return Err(NodeError::exec_failed(
                    "CANCELLED",
                    "Delay node cancelled".to_string(),
                ));
            },
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "delayed",
                "delay_type": delay_node.config.delay_type,
                "seconds": seconds,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
