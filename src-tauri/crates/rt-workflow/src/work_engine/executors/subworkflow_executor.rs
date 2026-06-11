// SPDX-License-Identifier: AGPL-3.0-only

//! 子工作流执行器 —— 通过引擎内递归执行运行嵌套工作流。
//!
//! 从 ExecutionState.callbacks.subworkflow 获取引擎回调，
//! 直接调用 WorkEngine.run_workflow() 执行子工作流，产生独立 ExecutionState，
//! 支持 parent_execution_id 关联和子执行记录追踪。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_core::workflow_types::{SubWorkflowNode, WorkflowNode};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// 子工作流引擎回调 — 接收 (sub_workflow_id, parent_execution_id, input)，
/// 返回 (child_execution_id, output)。内部由 WorkEngine.run_workflow 实现。
pub type SubWorkflowCallback = Arc<
    dyn Fn(
            String,
            String,
            HashMap<String, Value>,
        )
            -> Pin<Box<dyn std::future::Future<Output = Result<(String, Value), String>> + Send>>
        + Send
        + Sync,
>;

#[derive(Debug, Clone)]
pub struct SubWorkflowExecutorConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub cache_enabled: bool,
    pub cache_ttl_secs: u64,
}
impl Default for SubWorkflowExecutorConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300,
            max_retries: 3,
            cache_enabled: true,
            cache_ttl_secs: 300,
        }
    }
}

#[derive(Clone)]
pub struct SubWorkflowExecutor {
    config: SubWorkflowExecutorConfig,
}

impl SubWorkflowExecutor {
    pub fn new() -> Self {
        Self::with_config(SubWorkflowExecutorConfig::default())
    }
    pub fn with_config(config: SubWorkflowExecutorConfig) -> Self {
        Self { config }
    }

    fn map_inputs(
        node: &SubWorkflowNode,
        context: &ExecutionState,
    ) -> Result<HashMap<String, Value>, NodeError> {
        let mut mapped = HashMap::new();
        for (target_var, source_var) in &node.config.input_mapping {
            let value = context.variables.get(source_var).cloned().ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_FAILED,
                    format!("Variable '{}' not found", source_var),
                )
            })?;
            mapped.insert(target_var.clone(), value);
        }
        Ok(mapped)
    }

    async fn execute_with_retry(
        cb: &SubWorkflowCallback,
        sub_workflow_id: &str,
        parent_execution_id: &str,
        input: HashMap<String, Value>,
        max_retries: u32,
    ) -> Result<(String, Value), NodeError> {
        let mut last_error = None;
        for attempt in 1..=max_retries + 1 {
            match cb(sub_workflow_id.to_string(), parent_execution_id.to_string(), input.clone())
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt <= max_retries {
                        tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                    }
                },
            }
        }
        Err(last_error
            .map(|e| NodeError::exec_failed(error_code::SUBWORKFLOW_FAILED, e))
            .unwrap_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_FAILED,
                    "Sub-workflow execution failed".to_string(),
                )
            }))
    }
}

impl Default for SubWorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for SubWorkflowExecutor {
    fn node_type(&self) -> &'static str {
        "subWorkflow"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let sub_node = match node {
            WorkflowNode::SubWorkflow(s) => s,
            _ => {
                return Err(NodeError::type_mismatch(
                    "subWorkflow".to_string(),
                    super::node_type_name(node).to_string(),
                ));
            },
        };

        let cb = context
            .callbacks
            .as_ref()
            .and_then(|cbs| cbs.subworkflow.clone())
            .ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_NOT_CONFIGURED,
                    "Sub-workflow engine callback not configured".to_string(),
                )
            })?;

        let mapped_input = Self::map_inputs(sub_node, context)?;

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let result = tokio::time::timeout(
            timeout,
            Self::execute_with_retry(
                &cb,
                &sub_node.config.sub_workflow_id,
                &context.execution_id,
                mapped_input,
                self.config.max_retries,
            ),
        )
        .await
        .map_err(|_| {
            NodeError::timed_out(
                error_code::SUBWORKFLOW_FAILED,
                format!("Sub-workflow timeout({}s)", self.config.timeout_secs),
            )
        })??;

        let (child_execution_id, output) = result;
        let child_eid_value = serde_json::Value::String(child_execution_id.clone());

        let mut enriched_output = if output.is_object() {
            let mut obj = output.as_object().cloned().unwrap_or_default();
            obj.insert("_child_execution_id".to_string(), child_eid_value.clone());
            serde_json::Value::Object(obj)
        } else {
            serde_json::json!({
                "result": output,
                "_child_execution_id": child_eid_value,
            })
        };

        if context.dry_run
            && let Some(obj) = enriched_output.as_object_mut()
        {
            obj.insert("status".to_string(), serde_json::Value::String("dry_run".to_string()));
            obj.insert(
                "sub_workflow_id".to_string(),
                serde_json::Value::String(sub_node.config.sub_workflow_id.clone()),
            );
            obj.insert(
                "message".to_string(),
                serde_json::Value::String("Sub-workflow dry run completed".to_string()),
            );
        }

        Ok(NodeOutput {
            output: enriched_output,
            output_var: Some(sub_node.config.output_var.clone()),
        })
    }
}
