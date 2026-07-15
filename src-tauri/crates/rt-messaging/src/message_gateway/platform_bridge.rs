// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use crate::message_gateway::platform_manager::{PlatformManager, PlatformMessageCallback};
use axagent_harness::build_provider_request_context;
use axagent_harness::repositories::{
    CreateConversationInput, CreateMessageInput, conversation_repository, message_repository,
    platform_config_repository, provider_repository, settings_repository,
};

async fn persist_session_route(
    platform: &str,
    user_id: &str,
    agent_session_id: &str,
) -> anyhow::Result<()> {
    let mut routes = platform_config_repository().load_session_routes().await;
    let key = format!("{}_{}", platform, user_id);
    routes.insert(key, agent_session_id.to_string());
    platform_config_repository()
        .save_session_routes(&routes)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

pub struct PlatformBridge {
    master_key: [u8; 32],
    platform_manager: Arc<PlatformManager>,
    webhook_dispatcher: Option<Arc<dyn crate::webhook_subscription::WebhookDispatch>>,
    /// 由 Harness 注入的 Provider 注册表（不为空时跳过本地 create_default）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl PlatformBridge {
    pub fn new(
        _db: sea_orm::DatabaseConnection,
        master_key: [u8; 32],
        platform_manager: Arc<PlatformManager>,
    ) -> Self {
        Self { master_key, platform_manager, webhook_dispatcher: None, provider_registry: None }
    }

    /// 设置 Webhook 派发器，用于在收到平台消息时触发 webhook 事件
    pub fn set_webhook_dispatcher(
        &mut self,
        dispatcher: Arc<dyn crate::webhook_subscription::WebhookDispatch>,
    ) {
        self.webhook_dispatcher = Some(dispatcher);
    }

    async fn call_llm(
        &self,
        provider_id: &str,
        model_id: &str,
        messages: Vec<axagent_harness::types::ChatMessage>,
    ) -> anyhow::Result<String> {
        let provider_config = provider_repository()
            .get_provider(provider_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let registry_key = axagent_harness::types::provider_model::provider_registry_key(
            &provider_config.provider_type,
        );
        let registry = self.provider_registry.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "PlatformBridge 未注入 ProviderRegistry（请调用 HasProviderRegistry::set_provider_registry）"
            )
        })?;
        let adapter = registry
            .get(registry_key)
            .ok_or_else(|| anyhow::anyhow!("Provider adapter not found: {}", registry_key))?;

        let key_row = provider_repository()
            .get_active_key(provider_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let api_key =
            axagent_crypto::crypto::decrypt_key(&key_row.key_encrypted, &self.master_key)?;

        let ctx = build_provider_request_context(&provider_config, &key_row, api_key);

        let request = axagent_harness::types::ChatRequest {
            model: model_id.to_string(),
            messages,
            stream: false,
            temperature: None,
            top_p: None,
            max_tokens: Some(4096),
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

        let response = adapter.chat(&ctx, request.into()).await?;
        Ok(response.content)
    }
}

impl axagent_harness::HasProviderRegistry for PlatformBridge {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
    }
}

#[async_trait::async_trait]
impl PlatformMessageCallback for PlatformBridge {
    async fn on_message(
        &self,
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        chat_id: &str,
        text: &str,
    ) -> Option<String> {
        // 派发 message_received webhook 事件
        if let Some(ref dispatcher) = self.webhook_dispatcher {
            let mut data = std::collections::HashMap::new();
            data.insert("platform".to_string(), serde_json::Value::String(platform.to_string()));
            data.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
            data.insert("chat_id".to_string(), serde_json::Value::String(chat_id.to_string()));
            data.insert("text".to_string(), serde_json::Value::String(text.to_string()));
            if let Some(uname) = username {
                data.insert("username".to_string(), serde_json::Value::String(uname.to_string()));
            }
            let _ = dispatcher
                .dispatch(crate::webhook_subscription::WebhookEvent::MessageReceived, data)
                .await;
        }

        match self.route_incoming_message(platform, user_id, username, chat_id, text).await {
            Ok(reply) => {
                let processed = reply.map(|r| {
                    let (cleaned, attachments) =
                        crate::message_gateway::media_types::process_media_attachments(&r);
                    if !attachments.is_empty() {
                        tracing::info!(
                            "[PlatformBridge] detected {} media attachment(s) for {}",
                            attachments.len(),
                            platform
                        );
                        for att in &attachments {
                            tracing::info!(
                                "[PlatformBridge] media: {} type={} mode={}",
                                att.path,
                                att.media_type.as_str(),
                                att.delivery_mode.as_str()
                            );
                        }
                    }
                    (cleaned, attachments)
                });

                if let Some(ref dispatcher) = self.webhook_dispatcher {
                    let mut data = std::collections::HashMap::new();
                    data.insert(
                        "platform".to_string(),
                        serde_json::Value::String(platform.to_string()),
                    );
                    data.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_id.to_string()),
                    );
                    if let Some((ref r, ref atts)) = processed {
                        data.insert("reply".to_string(), serde_json::Value::String(r.clone()));
                        if !atts.is_empty() {
                            data.insert(
                                "media_attachments".to_string(),
                                serde_json::to_value(atts).unwrap_or(serde_json::Value::Null),
                            );
                        }
                    }
                    let _ = dispatcher
                        .dispatch(crate::webhook_subscription::WebhookEvent::MessageSent, data)
                        .await;
                }
                processed.map(|(r, _)| r)
            },
            Err(e) => {
                tracing::error!("[PlatformBridge] process failed: {}", e);
                None
            },
        }
    }

    async fn save_cursor(&self, platform: &str, cursor: i64) {
        if let Err(e) = platform_config_repository().save_platform_cursor(platform, cursor).await {
            tracing::error!("[PlatformBridge] cursor save failed for {}: {}", platform, e);
        }
    }
}

impl PlatformBridge {
    /// 公开入口：处理来自任意平台的入站消息，调用 LLM 并返回回复文本
    pub async fn route_incoming_message(
        &self,
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        _chat_id: &str,
        text: &str,
    ) -> anyhow::Result<Option<String>> {
        use axagent_harness::types::MessageRole;
        use axagent_kit::slash_command::apply_slash_command_to_input;

        let preprocessed = apply_slash_command_to_input(text);
        let effective_text = &preprocessed.modified_text;

        let app_settings =
            settings_repository().get_settings().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        let provider_id = app_settings
            .default_provider_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No default provider configured"))?;
        let model_id = app_settings
            .default_model_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No default model configured"))?;

        // 尝试复用已有对话：查找已关联的 agent_session
        let conv_title = format!("[{}] {}", platform, username.unwrap_or(user_id));
        let existing_conv_id = self
            .platform_manager
            .get_linked_agent_session(platform, user_id, None::<&sea_orm::DatabaseConnection>)
            .await;

        let conv = if let Some(ref existing_id) = existing_conv_id {
            match conversation_repository().get_conversation(existing_id).await {
                Ok(c) => {
                    tracing::info!(
                        "[PlatformBridge] reusing existing conversation {} for {} {}",
                        c.id,
                        platform,
                        user_id
                    );
                    c
                },
                Err(_) => {
                    // 对话已删除或不存在，创建新对话
                    conversation_repository()
                        .create_conversation(CreateConversationInput {
                            title: conv_title,
                            model_id: model_id.to_string(),
                            provider_id: provider_id.to_string(),
                            system_prompt: None,
                        })
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                },
            }
        } else {
            // 没有已有会话，创建新对话
            conversation_repository()
                .create_conversation(CreateConversationInput {
                    title: conv_title,
                    model_id: model_id.to_string(),
                    provider_id: provider_id.to_string(),
                    system_prompt: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?
        };

        message_repository()
            .create_message(CreateMessageInput {
                conversation_id: conv.id.clone(),
                role: MessageRole::User,
                content: effective_text.to_string(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        conversation_repository()
            .increment_message_count(&conv.id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let safe_username: String = username
            .unwrap_or("unknown")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .take(32)
            .collect();
        let mut system_prompt = format!(
            "You are AxAgent. The user is messaging from {} (username: {}). \
             Provide helpful, concise responses.",
            platform, safe_username
        );
        if let Some(ref personality_msg) = preprocessed.personality_prompt {
            system_prompt.push_str(&format!("\n\n{}", personality_msg));
        }

        let messages: Vec<axagent_harness::types::ChatMessage> = vec![
            axagent_harness::types::ChatMessage {
                role: "system".to_string(),
                content: axagent_harness::types::ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            axagent_harness::types::ChatMessage {
                role: "user".to_string(),
                content: axagent_harness::types::ChatContent::Text(effective_text.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ];

        let reply_content = self.call_llm(provider_id, model_id, messages).await?;

        message_repository()
            .create_message(CreateMessageInput {
                conversation_id: conv.id.clone(),
                role: MessageRole::Assistant,
                content: reply_content.clone(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        conversation_repository()
            .increment_message_count(&conv.id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        self.platform_manager.link_agent_session(platform, user_id, &conv.id).await;

        // 持久化会话路由
        if let Err(e) = persist_session_route(platform, user_id, &conv.id).await {
            tracing::warn!("[PlatformBridge] session route persist failed: {}", e);
        }

        tracing::info!("[PlatformBridge] {} {}: handled, conv={}", platform, user_id, conv.id);

        Ok(Some(reply_content))
    }
}
