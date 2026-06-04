//! LLM 执行器 —— 解析系统默认 provider 和模型后调用 `adapter.chat()`。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::workflow_types::WorkflowNode;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use axagent_harness::build_provider_request_context;

pub struct LlmExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    /// 由 Harness 注入的 ProviderRegistry（运行时按 provider 类型查找 adapter）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl LlmExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            provider_registry: None,
        }
    }
}

impl axagent_harness::HasProviderRegistry for LlmExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
    }
}
impl Default for LlmExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
            provider_registry: None,
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for LlmExecutor {
    fn node_type(&self) -> &'static str {
        "llm"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Llm(llm_node) = node else {
            return Err(NodeError::type_mismatch(
                "llm".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // 解析 provider + key + model。
        // 优先级：节点 config.model > 会话 __workflow_model__/__workflow_provider_id__ > 项目默认
        let node_model = if !llm_node.config.model.is_empty() {
            Some(llm_node.config.model.as_str())
        } else {
            None
        };
        let session_model = context
            .variables
            .get(super::WORKFLOW_MODEL_VAR)
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get(super::WORKFLOW_PROVIDER_ID_VAR)
            .and_then(|v| v.as_str());
        let (prov, key, model, adapter, api_key) = super::resolve_provider_and_adapter(
            &self.db,
            &self.master_key,
            self.provider_registry.as_ref(),
            node_model,
            session_model,
            session_provider_id,
            None,
            "LlmExecutor",
        )
        .await?;

        // 构建 messages
        let mut messages: Vec<ChatMessage> = llm_node
            .config
            .messages
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| serde_json::from_value(m).ok())
            .collect();
        if messages.is_empty() {
            let ctx_text = context
                .variables
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n");
            messages = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(llm_node.config.prompt.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(ctx_text),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ];
        }

        let req_ctx = build_provider_request_context(&prov, &key, api_key);
        let model_for_output = model.clone();

        if context.dry_run {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "content": "[DRY RUN] LLM 模拟输出", "model": model_for_output,
                    "usage": {"input_tokens":0,"output_tokens":0},
                    "dry_run": true, "node_id": node.base_id(),
                }),
                output_var: None,
            });
        }

        let request = ChatRequest {
            model,
            messages,
            stream: false,
            temperature: llm_node.config.temperature.map(|t| t as f64),
            max_tokens: llm_node.config.max_tokens,
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = adapter.chat(&req_ctx, request).await.map_err(|e| {
            NodeError::exec_failed(
                error_code::UNSUPPORTED_PROVIDER,
                format!("LLM call failed: {e}"),
            )
        })?;

        Ok(NodeOutput {
            output: serde_json::json!({
                "model": model_for_output, "provider": prov.id,
                "content": response.content, "thinking": response.thinking,
                "usage": { "input_tokens": response.usage.prompt_tokens, "output_tokens": response.usage.completion_tokens },
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
