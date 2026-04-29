use crate::message_gateway::platform_config::PlatformConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub phone_number_id: String,
    pub access_token: String,
    pub business_account_id: String,
}

pub struct WhatsAppAdapter {
    connected: Arc<RwLock<bool>>,
    client: reqwest::Client,
}

impl Default for WhatsAppAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WhatsAppAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(RwLock::new(false)),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl super::PlatformAdapter for WhatsAppAdapter {
    fn name(&self) -> &'static str {
        "whatsapp"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.whatsapp_enabled
    }

    async fn start(&self, _config: &PlatformConfig) -> anyhow::Result<()> {
        *self.connected.write().await = true;
        tracing::info!("WhatsApp adapter started");
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        *self.connected.write().await = false;
        tracing::info!("WhatsApp adapter stopped");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn send_message(
        &self,
        _config: &PlatformConfig,
        recipient: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let wa_config = WhatsAppConfig {
            phone_number_id: _config.whatsapp_phone_number_id.clone().unwrap_or_default(),
            access_token: _config.whatsapp_access_token.clone().unwrap_or_default(),
            business_account_id: _config.whatsapp_business_account_id.clone().unwrap_or_default(),
        };
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            wa_config.phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "text",
            "text": {
                "body": text
            }
        });
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", wa_config.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("WhatsApp API error: {}", response.status()));
        }
        tracing::debug!("Message sent to WhatsApp recipient {}", recipient);
        Ok(())
    }
}
