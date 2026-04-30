use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct WhatsAppAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl WhatsAppAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            poll_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for WhatsAppAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for WhatsAppAdapter {
    fn name(&self) -> &'static str {
        "whatsapp"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.whatsapp_enabled
            && config.whatsapp_phone_number_id.is_some()
            && config.whatsapp_access_token.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("WhatsApp is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let phone_number_id = config.whatsapp_phone_number_id.clone().unwrap_or_default();
        let access_token = config.whatsapp_access_token.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            match verify_whatsapp(&client, &phone_number_id, &access_token).await {
                Ok(_) => {
                    tracing::info!("WhatsApp: phone number verified");
                    connected.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    tracing::warn!(
                        "WhatsApp: phone verification failed (webhook-only for messages): {}",
                        e
                    );
                    connected.store(true, Ordering::SeqCst);
                }
            }

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Err(e) = verify_whatsapp(&client, &phone_number_id, &access_token).await {
                    tracing::warn!("WhatsApp: health check failed: {}", e);
                    connected.store(false, Ordering::SeqCst);
                } else if !connected.load(Ordering::SeqCst) {
                    connected.store(true, Ordering::SeqCst);
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("WhatsApp health check loop stopped");
        });

        *self.poll_task.lock().await = Some(task);
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.poll_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn send_message(
        &self,
        _config: &PlatformConfig,
        recipient: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let phone_number_id = _config.whatsapp_phone_number_id.clone().unwrap_or_default();
        let access_token = _config.whatsapp_access_token.clone().unwrap_or_default();

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "text",
            "text": {
                "body": text
            }
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("WhatsApp API error: {}", resp.status()));
        }
        Ok(())
    }
}

async fn verify_whatsapp(
    client: &reqwest::Client,
    phone_number_id: &str,
    access_token: &str,
) -> anyhow::Result<()> {
    let url = format!("https://graph.facebook.com/v18.0/{}", phone_number_id);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("WhatsApp verify failed ({}): {}", status.as_u16(), body);
    }

    Ok(())
}
