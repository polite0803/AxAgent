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
        let api_version = resolve_api_version(config);

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            match verify_whatsapp(&client, &phone_number_id, &access_token, &api_version).await {
                Ok(_) => {
                    tracing::info!("WhatsApp: phone number verified");
                    connected.store(true, Ordering::SeqCst);
                },
                Err(e) => {
                    tracing::warn!(
                        "WhatsApp: phone verification failed (webhook mode still active for messages): {}",
                        e
                    );
                    connected.store(true, Ordering::SeqCst);
                },
            }

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Err(e) =
                    verify_whatsapp(&client, &phone_number_id, &access_token, &api_version).await
                {
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
        config: &PlatformConfig,
        recipient: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let phone_number_id = config.whatsapp_phone_number_id.clone().unwrap_or_default();
        let access_token = config.whatsapp_access_token.clone().unwrap_or_default();
        let api_version = resolve_api_version(config);

        let url = format!(
            "https://graph.facebook.com/{}/{}/messages",
            api_version, phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "text",
            "text": {
                "preview_url": true,
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

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "WhatsApp API error ({}): {}",
                status.as_u16(),
                err_body
            ));
        }
        Ok(())
    }
}

/// 处理 WhatsApp Webhook 验证请求 (GET)
/// 返回对应的 challenge 字符串以完成 Meta 的 webhook 验证流程
pub fn verify_webhook_challenge(
    config: &PlatformConfig,
    mode: &str,
    token: &str,
    challenge: &str,
) -> Result<String, String> {
    let expected_token = config
        .whatsapp_webhook_verify_token
        .as_deref()
        .unwrap_or("");

    if mode != "subscribe" {
        return Err(format!("Invalid hub.mode: {}", mode));
    }

    if token != expected_token {
        return Err("Verify token mismatch".to_string());
    }

    Ok(challenge.to_string())
}

/// 处理 WhatsApp Webhook 通知 (POST) 中的消息事件
/// 解析 Meta 推送的 JSON payload，提取文本消息并调用回调处理
pub async fn handle_webhook_notification(
    config: &PlatformConfig,
    body: &serde_json::Value,
) -> Result<(), String> {
    let entries = body["entry"].as_array().ok_or("Missing entry array")?;

    for entry in entries {
        let changes = entry["changes"].as_array().ok_or("Missing changes array")?;
        for change in changes {
            let value = &change["value"];

            let messages = value["messages"].as_array();
            let contacts = value["contacts"].as_array();

            let Some(messages) = messages else { continue };

            for msg in messages {
                let msg_type = msg["type"].as_str().unwrap_or("");
                if msg_type != "text" {
                    continue;
                }

                let from = msg["from"].as_str().unwrap_or("");
                let text = msg["text"]["body"].as_str().unwrap_or("");

                if from.is_empty() || text.is_empty() {
                    continue;
                }

                let sender_name = contacts
                    .and_then(|c| c.first())
                    .and_then(|c| c["profile"]["name"].as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| from.to_string());

                tracing::info!("WhatsApp webhook: {} ({}) — {}", sender_name, from, text);

                if let Some(cb) = crate::message_gateway::platforms::get_message_callback() {
                    let access_token = config.whatsapp_access_token.clone().unwrap_or_default();
                    let phone_number_id =
                        config.whatsapp_phone_number_id.clone().unwrap_or_default();
                    let api_version = resolve_api_version(config);
                    let from_owned = from.to_string();
                    let text_owned = text.to_string();

                    let sender_owned = sender_name.clone();
                    tokio::spawn(async move {
                        let reply = cb
                            .on_message(
                                "whatsapp",
                                &from_owned,
                                Some(&sender_owned),
                                &from_owned,
                                &text_owned,
                            )
                            .await;

                        if let Some(reply_text) = reply {
                            let url = format!(
                                "https://graph.facebook.com/{}/{}/messages",
                                api_version, phone_number_id
                            );
                            let body = serde_json::json!({
                                "messaging_product": "whatsapp",
                                "to": from_owned,
                                "type": "text",
                                "text": { "preview_url": true, "body": reply_text }
                            });

                            let client = reqwest::Client::new();
                            let _ = client
                                .post(&url)
                                .header("Authorization", format!("Bearer {}", access_token))
                                .header("Content-Type", "application/json")
                                .json(&body)
                                .send()
                                .await;
                        }
                    });
                }
            }
        }
    }

    Ok(())
}

fn resolve_api_version(config: &PlatformConfig) -> String {
    config
        .whatsapp_api_version
        .clone()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "v18.0".to_string())
}

async fn verify_whatsapp(
    client: &reqwest::Client,
    phone_number_id: &str,
    access_token: &str,
    api_version: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://graph.facebook.com/{}/{}",
        api_version, phone_number_id
    );

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
