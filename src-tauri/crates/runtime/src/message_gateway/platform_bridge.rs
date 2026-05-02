use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::message_gateway::platform_manager::{PlatformManager, PlatformMessageCallback};
use axagent_providers::registry::ProviderRegistry;
use axagent_providers::{resolve_base_url_for_type, ProviderRequestContext};

async fn persist_session_route(
    db: &DatabaseConnection,
    platform: &str,
    user_id: &str,
    agent_session_id: &str,
) -> anyhow::Result<()> {
    let mut routes = axagent_core::repo::platform_config::load_session_routes(db).await;
    let key = format!("{}_{}", platform, user_id);
    routes.insert(key, agent_session_id.to_string());
    axagent_core::repo::platform_config::save_session_routes(db, &routes)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

pub struct PlatformBridge {
    db: DatabaseConnection,
    master_key: [u8; 32],
    platform_manager: Arc<PlatformManager>,
    webhook_dispatcher: Option<Arc<crate::webhook_dispatcher::WebhookDispatcher>>,
}

impl PlatformBridge {
    pub fn new(
        db: DatabaseConnection,
        master_key: [u8; 32],
        platform_manager: Arc<PlatformManager>,
    ) -> Self {
        Self {
            db,
            master_key,
            platform_manager,
            webhook_dispatcher: None,
        }
    }

    /// 设置 Webhook 派发器，用于在收到平台消息时触发 webhook 事件
    pub fn set_webhook_dispatcher(
        &mut self,
        dispatcher: Arc<crate::webhook_dispatcher::WebhookDispatcher>,
    ) {
        self.webhook_dispatcher = Some(dispatcher);
    }

    async fn call_llm(
        &self,
        provider_id: &str,
        model_id: &str,
        messages: Vec<axagent_core::types::ChatMessage>,
    ) -> anyhow::Result<String> {
        use axagent_core::repo::provider;

        let provider_config = provider::get_provider(&self.db, provider_id).await?;

        let registry_key = format!("{:?}", provider_config.provider_type).to_lowercase();
        let registry = ProviderRegistry::create_default();
        let adapter = registry
            .get(&registry_key)
            .ok_or_else(|| anyhow::anyhow!("Provider adapter not found: {}", registry_key))?;

        let key_row = provider::get_active_key(&self.db, provider_id).await?;

        let api_key = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &self.master_key)?;

        let ctx = ProviderRequestContext {
            api_key,
            key_id: key_row.id.clone(),
            provider_id: provider_id.to_string(),
            base_url: Some(resolve_base_url_for_type(
                &provider_config.api_host,
                &provider_config.provider_type,
            )),
            api_path: provider_config.api_path.clone(),
            proxy_config: provider_config.proxy_config.clone(),
            custom_headers: provider_config
                .custom_headers
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };

        let request = axagent_core::types::ChatRequest {
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

        let response = adapter.chat(&ctx, request).await?;
        Ok(response.content)
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
            data.insert(
                "platform".to_string(),
                serde_json::Value::String(platform.to_string()),
            );
            data.insert(
                "user_id".to_string(),
                serde_json::Value::String(user_id.to_string()),
            );
            data.insert(
                "chat_id".to_string(),
                serde_json::Value::String(chat_id.to_string()),
            );
            data.insert(
                "text".to_string(),
                serde_json::Value::String(text.to_string()),
            );
            if let Some(uname) = username {
                data.insert(
                    "username".to_string(),
                    serde_json::Value::String(uname.to_string()),
                );
            }
            let _ = dispatcher
                .dispatch(
                    crate::webhook_subscription::WebhookEvent::MessageReceived,
                    data,
                )
                .await;
        }

        match self
            .process_incoming_message(platform, user_id, username, chat_id, text)
            .await
        {
            Ok(reply) => {
                // 派发 message_sent webhook 事件
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
                    if let Some(ref r) = reply {
                        data.insert("reply".to_string(), serde_json::Value::String(r.clone()));
                    }
                    let _ = dispatcher
                        .dispatch(crate::webhook_subscription::WebhookEvent::MessageSent, data)
                        .await;
                }
                reply
            },
            Err(e) => {
                tracing::error!("[PlatformBridge] process failed: {}", e);
                None
            },
        }
    }

    async fn save_cursor(&self, platform: &str, cursor: i64) {
        if let Err(e) =
            axagent_core::repo::platform_config::save_platform_cursor(&self.db, platform, cursor)
                .await
        {
            tracing::error!(
                "[PlatformBridge] cursor save failed for {}: {}",
                platform,
                e
            );
        }
    }
}

impl PlatformBridge {
    async fn process_incoming_message(
        &self,
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        _chat_id: &str,
        text: &str,
    ) -> anyhow::Result<Option<String>> {
        use axagent_core::repo::{conversation, message, settings};
        use axagent_core::types::MessageRole;

        let app_settings = settings::get_settings(&self.db).await?;
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
            .get_linked_agent_session(platform, user_id, Some(&self.db))
            .await;

        let conv = if let Some(ref existing_id) = existing_conv_id {
            match conversation::get_conversation(&self.db, existing_id).await {
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
                    conversation::create_conversation(
                        &self.db,
                        &conv_title,
                        model_id,
                        provider_id,
                        None,
                    )
                    .await?
                },
            }
        } else {
            // 没有已有会话，创建新对话
            conversation::create_conversation(&self.db, &conv_title, model_id, provider_id, None)
                .await?
        };

        message::create_message(&self.db, &conv.id, MessageRole::User, text, &[], None, 0).await?;

        conversation::increment_message_count(&self.db, &conv.id).await?;

        let system_prompt = format!(
            "You are AxAgent. The user is messaging from {} (username: {}). \
             Provide helpful, concise responses.",
            platform,
            username.unwrap_or("unknown")
        );

        let messages: Vec<axagent_core::types::ChatMessage> = vec![
            axagent_core::types::ChatMessage {
                role: "system".to_string(),
                content: axagent_core::types::ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
            },
            axagent_core::types::ChatMessage {
                role: "user".to_string(),
                content: axagent_core::types::ChatContent::Text(text.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let reply_content = self.call_llm(provider_id, model_id, messages).await?;

        message::create_message(
            &self.db,
            &conv.id,
            MessageRole::Assistant,
            &reply_content,
            &[],
            None,
            0,
        )
        .await?;

        conversation::increment_message_count(&self.db, &conv.id).await?;

        self.platform_manager
            .link_agent_session(platform, user_id, &conv.id)
            .await;

        // 持久化会话路由
        if let Err(e) = persist_session_route(&self.db, platform, user_id, &conv.id).await {
            tracing::warn!("[PlatformBridge] session route persist failed: {}", e);
        }

        tracing::info!(
            "[PlatformBridge] {} {}: handled, conv={}",
            platform,
            user_id,
            conv.id
        );

        Ok(Some(reply_content))
    }
}
