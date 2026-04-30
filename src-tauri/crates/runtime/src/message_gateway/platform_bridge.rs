use std::sync::Arc;

use sea_orm::DatabaseConnection;

use axagent_providers::registry::ProviderRegistry;
use axagent_providers::{resolve_base_url_for_type, ProviderRequestContext};
use crate::message_gateway::platform_manager::{PlatformManager, PlatformMessageCallback};

pub struct PlatformBridge {
    db: DatabaseConnection,
    master_key: [u8; 32],
    platform_manager: Arc<PlatformManager>,
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
        }
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

        let key_row = provider::get_active_key(&self.db, provider_id)
            .await?;

        let api_key =
            axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &self.master_key)?;

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
        match self
            .process_incoming_message(platform, user_id, username, chat_id, text)
            .await
        {
            Ok(reply) => reply,
            Err(e) => {
                tracing::error!("[PlatformBridge] process failed: {}", e);
                None
            }
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

        let settings = settings::get_settings(&self.db).await?;
        let provider_id = settings
            .default_provider_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No default provider configured"))?;
        let model_id = settings
            .default_model_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No default model configured"))?;

        let conv_title = format!(
            "[{}] {}",
            platform,
            username.unwrap_or(user_id)
        );

        let conv = conversation::create_conversation(
            &self.db,
            &conv_title,
            model_id,
            provider_id,
            None,
        )
        .await?;

        message::create_message(
            &self.db,
            &conv.id,
            MessageRole::User,
            text,
            &[],
            None,
            0,
        )
        .await?;

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

        tracing::info!(
            "[PlatformBridge] {} {}: handled, conv={}",
            platform,
            user_id,
            conv.id
        );

        Ok(Some(reply_content))
    }
}
