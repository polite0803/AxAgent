//! Agent 执行器 —— 支持 inline role 和 agent_profile 两种模式，均自动使用系统默认模型。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::workflow_types::WorkflowNode;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub struct AgentExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
}

impl AgentExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self { db, master_key }
    }
}
impl Default for AgentExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for AgentExecutor {
    fn node_type(&self) -> &'static str {
        "agent"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Agent(an) = node else {
            return Err(NodeError::type_mismatch(
                "agent".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // 1. 加载 agent profile（若配置）
        use axagent_core::entity::agent_profiles;
        use sea_orm::EntityTrait;
        let profile = if let Some(ref pid) = an.config.agent_profile_id {
            agent_profiles::Entity::find_by_id(pid.as_str())
                .one(self.db.as_ref())
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        format!("Agent profile query failed: {e}"),
                    )
                })?
        } else {
            None
        };

        // 2. 解析默认 provider + key + model。
        //    模型优先级：context.__workflow_model__ > 系统默认 provider 第一个 enabled model
        let session_model = context
            .variables
            .get("__workflow_model__")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let (prov, key, default_model) = if let Some(ref p) = profile {
            let all = axagent_core::repo::provider::list_providers(&self.db)
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        format!("Provider query failed: {e}"),
                    )
                })?;
            if let Some(ref suggested_id) = p.suggested_provider_id {
                if let Some(sp) = all
                    .into_iter()
                    .find(|pr| pr.id == *suggested_id && pr.enabled)
                {
                    let key = sp.keys.iter().find(|k| k.enabled).cloned().ok_or_else(|| {
                        NodeError::exec_failed(
                            error_code::AGENT_PROFILE_NOT_FOUND,
                            "建议的 provider 无可用 key".to_string(),
                        )
                    })?;
                    let model = sp
                        .models
                        .iter()
                        .find(|m| m.enabled)
                        .map(|m| m.model_id.clone())
                        .ok_or_else(|| {
                            NodeError::exec_failed(
                                error_code::AGENT_PROFILE_NOT_FOUND,
                                "建议的 provider 无可用模型".to_string(),
                            )
                        })?;
                    Ok::<_, NodeError>((sp, key, model))
                } else {
                    axagent_core::repo::provider::resolve_default_provider(&self.db)
                        .await
                        .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
                }
            } else {
                axagent_core::repo::provider::resolve_default_provider(&self.db)
                    .await
                    .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
            }
        } else {
            axagent_core::repo::provider::resolve_default_provider(&self.db)
                .await
                .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
        }?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::AGENT_PROFILE_NOT_FOUND,
                    format!("API key decryption failed: {e}"),
                )
            })?;

        // 3. 创建 adapter
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
                    error_code::AGENT_PROFILE_NOT_FOUND,
                    format!("Unsupported provider: {:?}", prov.provider_type),
                ));
            },
        };

        // 4. 构建 prompt：profile 优先，inline role fallback
        let (role_desc, mut system_prompt) = if let Some(ref p) = profile {
            let role = an
                .config
                .agent_role_override
                .as_deref()
                .or(p.agent_role.as_deref())
                .unwrap_or("executor");
            (role.to_string(), p.system_prompt.clone())
        } else {
            let role = an
                .config
                .role
                .as_ref()
                .map(|r| format!("{:?}", r))
                .unwrap_or_else(|| "executor".to_string());
            (role, an.config.system_prompt.clone())
        };
        system_prompt = format!("你是 {role_desc}。\n{system_prompt}");
        if let Some(ctx_json) = {
            let mut map = serde_json::Map::new();
            for s in &an.config.context_sources {
                if let Some(v) = context.variables.get(s) {
                    map.insert(s.clone(), v.clone());
                }
            }
            if map.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(map))
            }
        } {
            system_prompt.push_str(&format!("\n\n上下文数据:\n{ctx_json}"));
        }
        let user_prompt = context
            .variables
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ];

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
        let model = session_model.unwrap_or(default_model);
        let model_for_output = model.clone();
        let request = ChatRequest {
            model,
            messages,
            stream: false,
            temperature: an.config.temperature.map(|t| t as f64),
            max_tokens: an.config.max_tokens,
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
                error_code::AGENT_PROFILE_NOT_FOUND,
                format!("Agent LLM call failed: {e}"),
            )
        })?;

        Ok(NodeOutput {
            output: serde_json::json!({
                "role": role_desc, "model": model_for_output,
                "content": response.content, "thinking": response.thinking,
                "usage": { "input_tokens": response.usage.prompt_tokens, "output_tokens": response.usage.completion_tokens },
                "node_id": node.base_id(),
            }),
            output_var: Some(an.config.output_var.clone()),
        })
    }
}
