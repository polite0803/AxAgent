// SPDX-License-Identifier: AGPL-3.0-only

//! 终止执行器 —— 标记工作流结束位置，可选提取上游节点输出作为最终结果。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct EndExecutor;

impl EndExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EndExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for EndExecutor {
    fn node_type(&self) -> &'static str {
        "end"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::End(end_node) = node else {
            return Err(NodeError::type_mismatch(
                "end".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // 若配置了 output_var，从 context.variables 中提取对应节点输出
        let output = if let Some(ref var) = end_node.config.output_var {
            let extracted = context.variables.get(var).cloned();
            serde_json::json!({
                "status": "terminated",
                "node_id": node.base_id(),
                "output": extracted.unwrap_or(serde_json::Value::Null),
                "source": var,
            })
        } else {
            serde_json::json!({
                "status": "terminated",
                "node_id": node.base_id(),
            })
        };

        Ok(NodeOutput {
            output,
            output_var: end_node.config.output_var.clone(),
        })
    }
}
