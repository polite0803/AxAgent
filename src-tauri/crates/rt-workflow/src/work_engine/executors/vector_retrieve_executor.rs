//! 向量检索执行器 —— 通过注入的回调查询向量存储。
//!
//! 默认无回调时返回解析后的 query 和配置，不报错但标记为未配置。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use std::pin::Pin;
use std::sync::Arc;

pub type VectorRetrieveCallback = Arc<
    dyn Fn(
            String,
            u32,
        ) -> Pin<
            Box<dyn std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send>,
        > + Send
        + Sync,
>;

pub struct VectorRetrieveExecutor {
    callback: Arc<tokio::sync::Mutex<Option<VectorRetrieveCallback>>>,
}

impl VectorRetrieveExecutor {
    pub fn new() -> Self {
        Self {
            callback: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
    pub async fn set_callback(&self, cb: VectorRetrieveCallback) {
        *self.callback.lock().await = Some(cb);
    }
}
impl Default for VectorRetrieveExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for VectorRetrieveExecutor {
    fn node_type(&self) -> &'static str {
        "vector_retrieve"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::VectorRetrieve(vr) = node else {
            return Err(NodeError::type_mismatch(
                "vector_retrieve".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let resolved_query = resolve_query_template(&vr.config.query, &context.variables);

        let cb_guard = self.callback.lock().await;
        let results = if let Some(ref cb) = *cb_guard {
            cb(resolved_query.clone(), vr.config.top_k)
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::VECTOR_RETRIEVE_FAILED,
                        format!("Vector retrieval failed: {e}"),
                    )
                })?
        } else {
            vec![
                serde_json::json!({"status": "not_configured", "message": "Vector retrieve callback not configured"}),
            ]
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "query": resolved_query, "knowledge_base_id": vr.config.knowledge_base_id,
                "top_k": vr.config.top_k, "results": results, "node_id": node.base_id(),
            }),
            output_var: Some(vr.config.output_var.clone()),
        })
    }
}

fn resolve_query_template(
    template: &str,
    vars: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        let owned = v.to_string();
        let replacement = v.as_str().unwrap_or(&owned);
        result = result.replace(&placeholder, replacement);
    }
    result
}
