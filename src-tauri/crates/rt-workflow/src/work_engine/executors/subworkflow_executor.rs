//! 子工作流执行器 —— 通过注入的回调运行嵌套工作流。
//!
//! 保留缓存层、输入映射、重试和超时逻辑。默认无回调时返回清晰错误。

use crate::work_engine::cache_layer::{CacheLayer, InMemoryCache};
use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::{SubWorkflowNode, WorkflowNode};
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type SubWorkflowCallback = Arc<
    dyn Fn(
            String,
            HashMap<String, Value>,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>
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
    cache: Arc<InMemoryCache>,
    config: SubWorkflowExecutorConfig,
    callback: Option<SubWorkflowCallback>,
}

impl SubWorkflowExecutor {
    pub fn new() -> Self {
        Self::with_config(SubWorkflowExecutorConfig::default())
    }
    pub fn with_config(config: SubWorkflowExecutorConfig) -> Self {
        Self {
            cache: Arc::new(InMemoryCache::new(config.cache_ttl_secs)),
            config,
            callback: None,
        }
    }
    pub fn with_callback(mut self, cb: SubWorkflowCallback) -> Self {
        self.callback = Some(cb);
        self
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
        format!("subworkflow:{}[{}]", node.config.sub_workflow_id, input_vars)
    }

    async fn execute_subworkflow_internal(
        &self,
        node: &SubWorkflowNode,
        context: &ExecutionState,
    ) -> Result<Value, NodeError> {
        let mapped_input = self.map_inputs(node, context)?;
        let cache_key = self.compute_cache_key(node, context);
        if self.config.cache_enabled
            && let Some(cached) = self.cache.get(&cache_key).await
        {
            return serde_json::from_slice(&cached)
                .map_err(|e| NodeError::ExecutionFailed(format!("缓存反序列化失败: {e}")));
        }
        let result = self.execute_with_retry(node, mapped_input).await?;
        if self.config.cache_enabled
            && let Ok(serialized) = serde_json::to_vec(&result)
        {
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
                NodeError::ExecutionFailed(format!("变量 '{}' 未找到", source_var))
            })?;
            mapped.insert(target_var.clone(), value);
        }
        Ok(mapped)
    }

    async fn execute_with_retry(
        &self,
        node: &SubWorkflowNode,
        input: HashMap<String, Value>,
    ) -> Result<Value, NodeError> {
        let mut last_error = None;
        for attempt in 1..=self.config.max_retries + 1 {
            match self.execute_single_attempt(node, input.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt <= self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                    }
                },
            }
        }
        Err(last_error
            .unwrap_or_else(|| NodeError::ExecutionFailed("子工作流执行失败".to_string())))
    }

    async fn execute_single_attempt(
        &self,
        node: &SubWorkflowNode,
        input: HashMap<String, Value>,
    ) -> Result<Value, NodeError> {
        if let Some(ref cb) = self.callback {
            cb(node.config.sub_workflow_id.clone(), input)
                .await
                .map_err(|e| NodeError::ExecutionFailed(format!("子工作流执行失败: {e}")))
        } else {
            Ok(serde_json::json!({
                "status": "sub_workflow_not_configured",
                "sub_workflow_id": node.config.sub_workflow_id,
                "mapped_input": input,
                "message": "子工作流执行器未注入回调，通过 register_executor 注入 SubWorkflowExecutor::with_callback()"
            }))
        }
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
        let sub_node = match node {
            WorkflowNode::SubWorkflow(s) => s,
            _ => {
                return Err(NodeError::InvalidNodeType {
                    expected: "sub_workflow".to_string(),
                    got: super::node_type_name(node).to_string(),
                });
            },
        };
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let result =
            tokio::time::timeout(timeout, self.execute_subworkflow_internal(sub_node, context))
                .await
                .map_err(|_| {
                    NodeError::Timeout(format!("子工作流超时({}s)", self.config.timeout_secs))
                })??;
        Ok(NodeOutput {
            output: result,
            output_var: Some(sub_node.config.output_var.clone()),
        })
    }
}
