//! Platform Integration Module - Telegram and Discord bot integration
//!
//! This module provides infrastructure for integrating AxAgent with messaging platforms:
//! - Telegram bot support via Bot API
//! - Discord bot support via Discord API
//! - Unified message handling and routing
//! - Webhook-based event processing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_webhook_secret: Option<String>,
    pub discord_enabled: bool,
    pub discord_bot_token: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub auto_sync_messages: bool,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_webhook_secret: None,
            discord_enabled: false,
            discord_bot_token: None,
            discord_webhook_url: None,
            auto_sync_messages: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum PlatformMessage {
    #[serde(rename = "telegram")]
    Telegram(TelegramMessage),
    #[serde(rename = "discord")]
    Discord(DiscordMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat_id: i64,
    pub text: String,
    pub from_user_id: Option<i64>,
    pub username: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub message_id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub content: String,
    pub author_id: String,
    pub author_username: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub platform: MessagePlatform,
    pub chat_id: String,
    pub content: String,
    pub parse_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagePlatform {
    Telegram,
    Discord,
}

impl MessagePlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessagePlatform::Telegram => "telegram",
            MessagePlatform::Discord => "discord",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSession {
    pub session_id: String,
    pub platform: MessagePlatform,
    pub user_id: String,
    pub username: Option<String>,
    pub is_active: bool,
    pub last_activity: i64,
}

pub struct PlatformIntegrationService {
    config: RwLock<PlatformConfig>,
    sessions: RwLock<HashMap<String, PlatformSession>>,
    message_handlers: Vec<Box<dyn PlatformMessageHandler>>,
}

impl PlatformIntegrationService {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(PlatformConfig::default()),
            sessions: RwLock::new(HashMap::new()),
            message_handlers: Vec::new(),
        }
    }

    pub async fn update_config(&self, config: PlatformConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    pub async fn get_config(&self) -> PlatformConfig {
        self.config.read().await.clone()
    }

    pub async fn register_handler(&mut self, handler: Box<dyn PlatformMessageHandler>) {
        self.message_handlers.push(handler);
    }

    pub async fn process_message(&self, message: PlatformMessage) -> Option<OutgoingMessage> {
        for handler in &self.message_handlers {
            if let Some(response) = handler.handle(&message).await {
                return Some(response);
            }
        }
        None
    }

    pub async fn create_session(&self, platform: MessagePlatform, user_id: String, username: Option<String>) -> String {
        let session_id = format!("{}_{}_{}", platform.as_str(), user_id, chrono::Utc::now().timestamp_millis());
        let session = PlatformSession {
            session_id: session_id.clone(),
            platform,
            user_id,
            username,
            is_active: true,
            last_activity: chrono::Utc::now().timestamp_millis(),
        };
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        session_id
    }

    pub async fn get_active_sessions(&self) -> Vec<PlatformSession> {
        let sessions = self.sessions.read().await;
        sessions.values().filter(|s| s.is_active).cloned().collect()
    }

    pub async fn deactivate_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = false;
        }
    }
}

impl Default for PlatformIntegrationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
pub trait PlatformMessageHandler: Send + Sync {
    async fn handle(&self, message: &PlatformMessage) -> Option<OutgoingMessage>;
    fn can_handle(&self, platform: &str) -> bool;
}

pub struct TelegramHandler {
    bot_token: String,
}

impl TelegramHandler {
    pub fn new(bot_token: String) -> Self {
        Self { bot_token }
    }

    pub async fn send_message(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
        });

        let client = reqwest::Client::new();
        client.post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl PlatformMessageHandler for TelegramHandler {
    async fn handle(&self, message: &PlatformMessage) -> Option<OutgoingMessage> {
        match message {
            PlatformMessage::Telegram(tg_msg) => {
                let response = format!("Received: {}", tg_msg.text);
                Some(OutgoingMessage {
                    platform: MessagePlatform::Telegram,
                    chat_id: tg_msg.chat_id.to_string(),
                    content: response,
                    parse_mode: Some("Markdown".to_string()),
                })
            }
            PlatformMessage::Discord(_) => None,
        }
    }

    fn can_handle(&self, platform: &str) -> bool {
        platform == "telegram"
    }
}

pub struct DiscordHandler {
    #[allow(dead_code)]
    bot_token: String,
    webhook_url: Option<String>,
}

impl DiscordHandler {
    pub fn new(bot_token: String, webhook_url: Option<String>) -> Self {
        Self { bot_token, webhook_url }
    }

    pub async fn send_message(&self, _channel_id: &str, content: &str) -> anyhow::Result<()> {
        if let Some(ref webhook_url) = self.webhook_url {
            let body = serde_json::json!({
                "content": content,
            });

            let client = reqwest::Client::new();
            client.post(webhook_url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl PlatformMessageHandler for DiscordHandler {
    async fn handle(&self, message: &PlatformMessage) -> Option<OutgoingMessage> {
        match message {
            PlatformMessage::Discord(dc_msg) => {
                let response = format!("Received: {}", dc_msg.content);
                Some(OutgoingMessage {
                    platform: MessagePlatform::Discord,
                    chat_id: dc_msg.channel_id.clone(),
                    content: response,
                    parse_mode: None,
                })
            }
            PlatformMessage::Telegram(_) => None,
        }
    }

    fn can_handle(&self, platform: &str) -> bool {
        platform == "discord"
    }
}

pub mod routes {
    use super::*;
    use axum::{
        extract::Path,
        http::StatusCode,
        response::Json,
        routing::post,
        Router,
    };

    pub async fn telegram_webhook(
        Path(_token): Path<String>,
        Json(payload): Json<TelegramMessage>,
    ) -> StatusCode {
        tracing::info!("Telegram webhook received: {:?}", payload);
        StatusCode::OK
    }

    pub async fn discord_webhook(
        Json(payload): Json<DiscordMessage>,
    ) -> StatusCode {
        tracing::info!("Discord webhook received: {:?}", payload);
        StatusCode::OK
    }

    pub fn create_router() -> Router {
        Router::new()
            .route("/telegram/:token", post(telegram_webhook))
            .route("/discord/webhook", post(discord_webhook))
    }
}
