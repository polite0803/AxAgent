use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct LlmClassifierExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
}

impl LlmClassifierExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self { db, master_key }
    }
}

impl Default for LlmClassifierExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for LlmClassifierExecutor {
    fn node_type(&self) -> &'static str {
        "llmClassifier"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::LlmClassifier(n) = node else {
            return Err(NodeError::type_mismatch(
                "llmClassifier".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let c = &n.config;

        let input_text = if c.input_var.is_empty() {
            context
                .variables
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            context
                .variables
                .get(&c.input_var)
                .map(|v| v.to_string())
                .unwrap_or_default()
        };

        if input_text.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "LlmClassifier: input_var 指向的变量为空或不存在".to_string(),
            ));
        }

        let categories_list = c
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| format!("{}. {}", i + 1, cat))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "你是一个文本分类器。请根据以下分类规则，将输入文本归入最匹配的类别。\n\n\
             ## 分类规则\n{prompt}\n\n\
             ## 可选类别\n{categories_list}\n\n\
             ## 输入文本\n{input_text}\n\n\
             请只输出最匹配的类别名称，不要包含任何其他内容。",
            prompt = c.prompt,
            categories_list = categories_list,
            input_text = input_text,
        );

        let node_model = c.model.as_deref().filter(|m| !m.is_empty());
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

        if context.dry_run {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "category": c.categories.first().cloned().unwrap_or_default(),
                    "model": model,
                    "dry_run": true,
                    "node_id": node.base_id(),
                }),
                output_var: if c.output_var.is_empty() {
                    None
                } else {
                    Some(c.output_var.clone())
                },
            });
        }

        use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ProviderType};
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
                    error_code::PROVIDER_QUERY_FAILED,
                    format!("Unsupported provider: {:?}", prov.provider_type),
                ));
            },
        };

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

        let request = ChatRequest {
            model: model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: Some(0.0),
            max_tokens: Some(64),
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
                error_code::PROVIDER_QUERY_FAILED,
                format!("LLM classifier call failed: {e}"),
            )
        })?;

        let raw_category = response.content.trim().to_string();

        let matched = c
            .categories
            .iter()
            .find(|cat| cat.to_lowercase() == raw_category.to_lowercase())
            .cloned()
            .unwrap_or_else(|| {
                c.categories
                    .iter()
                    .find(|cat| raw_category.to_lowercase().contains(&cat.to_lowercase()))
                    .cloned()
                    .unwrap_or(raw_category)
            });

        Ok(NodeOutput {
            output: serde_json::json!({
                "category": matched,
                "model": model,
                "provider": prov.id,
                "input_var": c.input_var,
                "node_id": node.base_id(),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
