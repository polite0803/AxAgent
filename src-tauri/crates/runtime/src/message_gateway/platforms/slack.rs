use crate::message_gateway::platform_config::PlatformConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub signing_secret: String,
    pub workspace_id: String,
}

pub struct SlackAdapter {
    connected: Arc<RwLock<bool>>,
    client: reqwest::Client,
}

impl Default for SlackAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SlackAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(RwLock::new(false)),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl super::PlatformAdapter for SlackAdapter {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.slack_enabled
    }

    async fn start(&self, _config: &PlatformConfig) -> anyhow::Result<()> {
        *self.connected.write().await = true;
        tracing::info!("Slack adapter started");
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        *self.connected.write().await = false;
        tracing::info!("Slack adapter stopped");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn send_message(
        &self,
        _config: &PlatformConfig,
        channel_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let slack_config = SlackConfig {
            bot_token: _config.slack_bot_token.clone().unwrap_or_default(),
            signing_secret: _config.slack_signing_secret.clone().unwrap_or_default(),
            workspace_id: _config.slack_workspace_id.clone().unwrap_or_default(),
        };
        let url = "https://slack.com/api/chat.postMessage";
        let body = serde_json::json!({
            "channel": channel_id,
            "text": text,
        });
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", slack_config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Slack API error: {}", response.status()));
        }
        tracing::debug!("Message sent to Slack channel {}", channel_id);
        Ok(())
    }
}
