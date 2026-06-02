//! LLM 执行器 —— 解析系统默认 provider 和模型后调用 `adapter.chat()`。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::workflow_types::WorkflowNode;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub struct LlmExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
}

impl LlmExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self { db, master_key }
    }
}
impl Default for LlmExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
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
            .get("__workflow_model__")
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get("__workflow_provider_id__")
            .and_then(|v| v.as_str());
        let (prov, key, model) = axagent_core::repo::provider::resolve_model_for_node(
            &self.db,
            node_model,
            session_model,
            session_provider_id,
            None,
        )
        .await
        .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::API_KEY_DECRYPT_FAILED,
                    format!("API key decryption failed: {e}"),
                )
            })?;

        // 创建 adapter
        use axagent_core::types::ProviderType;
        use axagent_providers::{ProviderAdapter, resolve_base_url_for_type};
        let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
            ProviderType::OpenAI => Arc::new(axagent_providers::openai::OpenAIAdapter::new()),
            ProviderType::OpenAIResponses => {
                Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new())
            },
            ProviderType::Anthropic => {
                Arc::new(axagent_providers::anthropic::AnthropicAdapter::new())
            },
            ProviderType::Gemini => Arc::new(axagent_providers::gemini::GeminiAdapter::new()),
            ProviderType::Ollama => Arc::new(axagent_providers::ollama::OllamaAdapter::new()),
            _ => {
                return Err(NodeError::exec_failed(
                    error_code::API_KEY_DECRYPT_FAILED,
                    format!("Unsupported provider: {:?}", prov.provider_type),
                ));
            },
        };

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

        let base_url = resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let req_ctx = axagent_providers::ProviderRequestContext {
            provider_id: prov.id.clone(),
            api_key,
            key_id: key.id.clone(),
            base_url: Some(base_url),
            api_path: None,
            proxy_config: None,
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };
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
                error_code::API_KEY_DECRYPT_FAILED,
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
