// SPDX-License-Identifier: AGPL-3.0-only

//! MultiAgent 节点执行器 —— 委派任务给多 Agent 协作。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use std::collections::HashMap;

pub struct MultiAgentExecutor;

impl MultiAgentExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MultiAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for MultiAgentExecutor {
    fn node_type(&self) -> &'static str {
        "multiAgent"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::MultiAgent(mn) = node else {
            return Err(NodeError::type_mismatch(
                "multiAgent".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let task = &mn.config.task;
        let output_var = mn.config.output_var.clone();

        if task.is_empty() {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "no_task",
                    "task": "",
                }),
                output_var: Some(output_var),
                control: None,
            });
        }

        let mut result = HashMap::new();
        result.insert("task".to_string(), serde_json::Value::String(task.clone()));
        result.insert("role".to_string(), serde_json::Value::String(mn.config.role.clone().unwrap_or_default()));
        result.insert("mode".to_string(), serde_json::Value::String(mn.config.mode.clone()));
        result.insert("max_rounds".to_string(), serde_json::Value::Number(serde_json::Number::from(mn.config.max_rounds)));
        result.insert("status".to_string(), serde_json::Value::String("delegated".to_string()));

        Ok(NodeOutput {
            output: serde_json::Value::Object(result.into_iter().collect()),
            output_var: Some(output_var),
            control: None,
        })
    }
}
