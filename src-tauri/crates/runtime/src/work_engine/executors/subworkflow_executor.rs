use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axagent_core::workflow_types::{SubWorkflowNode, WorkflowNode};
use itertools::Itertools;
use serde_json::Value;

use crate::work_engine::cache_layer::{CacheLayer, InMemoryCache};
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use crate::work_engine::ExecutionState;

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
    cache: Arc<InMemoryCache>,
    config: SubWorkflowExecutorConfig,
}

impl SubWorkflowExecutor {
    pub fn new() -> Self {
        Self::with_config(SubWorkflowExecutorConfig::default())
    }

    pub fn with_config(config: SubWorkflowExecutorConfig) -> Self {
        Self {
            cache: Arc::new(InMemoryCache::new(config.cache_ttl_secs)),
            config,
        }
    }

    fn compute_cache_key(&self, node: &SubWorkflowNode, context: &ExecutionState) -> String {
        let input_vars = context
            .variables
            .keys()
            .sorted()
            .map(|k| {
                format!(
                    "{}={}",
                    k,
                    context
                        .variables
                        .get(k)
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                )
            })
            .join(";");
        format!(
            "subworkflow:{}[{}]",
            node.config.sub_workflow_id, input_vars
        )
    }

    async fn execute_subworkflow_internal(
        &self,
        node: &SubWorkflowNode,
        context: &ExecutionState,
    ) -> Result<Value, NodeError> {
        let mapped_input = self.map_inputs(node, context)?;
        let cache_key = self.compute_cache_key(node, context);

        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(&cache_key).await {
                return serde_json::from_slice(&cached).map_err(|e| {
                    NodeError::ExecutionFailed(format!(
                        "Failed to deserialize cached result: {}",
                        e
                    ))
                });
            }
        }

        let result = self.execute_with_retry(node, mapped_input).await?;

        if self.config.cache_enabled {
            let serialized = serde_json::to_vec(&result).map_err(|e| {
                NodeError::ExecutionFailed(format!("Failed to serialize result: {}", e))
            })?;
            let _ = self
                .cache
                .set(&cache_key, &serialized, self.config.cache_ttl_secs)
                .await;
        }

        Ok(result)
    }

    fn map_inputs(
        &self,
        node: &SubWorkflowNode,
        context: &ExecutionState,
    ) -> Result<HashMap<String, Value>, NodeError> {
        let mut mapped = HashMap::new();
        for (target_var, source_var) in &node.config.input_mapping {
            let value = context.variables.get(source_var).cloned().ok_or_else(|| {
                NodeError::ExecutionFailed(format!(
                    "Input mapping: variable '{}' not found",
                    source_var
                ))
            })?;
            mapped.insert(target_var.clone(), value);
        }
        Ok(mapped)
    }

    fn map_output(
        &self,
        result: &Value,
        output_var: &str,
        context: &mut ExecutionState,
    ) -> Result<(), NodeError> {
        context
            .variables
            .insert(output_var.to_string(), result.clone());
        Ok(())
    }

    async fn execute_with_retry(
        &self,
        node: &SubWorkflowNode,
        _input: HashMap<String, Value>,
    ) -> Result<Value, NodeError> {
        let mut last_error = None;
        let mut attempts = 0;

        while attempts <= self.config.max_retries {
            attempts += 1;
            match self.execute_single_attempt(node).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempts <= self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            NodeError::ExecutionFailed("SubWorkflow execution failed after retries".to_string())
        }))
    }

    async fn execute_single_attempt(&self, node: &SubWorkflowNode) -> Result<Value, NodeError> {
        Ok(serde_json::json!({
            "status": "simulated",
            "sub_workflow_id": node.config.sub_workflow_id,
            "message": "SubWorkflow execution simulated - integrate with workflow engine"
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
        "sub_workflow"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let subworkflow_node = match node {
            WorkflowNode::SubWorkflow(s) => s,
            _ => {
                return Err(NodeError::UnsupportedNodeType(format!(
                    "Expected SubWorkflow node, got {:?}",
                    node
                )));
            }
        };

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let result = tokio::time::timeout(
            timeout,
            self.execute_subworkflow_internal(subworkflow_node, context),
        )
        .await
        .map_err(|_| {
            NodeError::Timeout(format!(
                "SubWorkflow execution timed out after {} seconds",
                self.config.timeout_secs
            ))
        })??;

        let output_var = &subworkflow_node.config.output_var;
        let mut ctx = context.clone();
        self.map_output(&result, output_var, &mut ctx)?;

        Ok(NodeOutput {
            output: result,
            output_var: Some(output_var.to_string()),
        })
    }
}
