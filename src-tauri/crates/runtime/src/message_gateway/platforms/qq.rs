use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct QQAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl QQAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            poll_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

use std::sync::Arc;

#[async_trait::async_trait]
impl PlatformAdapter for QQAdapter {
    fn name(&self) -> &'static str {
        "qq"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.qq_enabled
            && config.qq_bot_app_id.is_some()
            && config.qq_bot_token.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("QQ is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let bot_app_id = config.qq_bot_app_id.clone().unwrap_or_default();
        let bot_token = config.qq_bot_token.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Verify bot credentials by checking bot info
            match verify_qq_bot(&client, &bot_app_id, &bot_token).await {
                Ok(_) => {
                    tracing::info!("QQ: bot verified successfully");
                    connected.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    tracing::error!("QQ: failed to verify bot: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            }

            // Status monitoring loop
            // QQ bot uses WebSocket (WebHook) for full message receiving.
            // This adapter provides HTTP API-based sending and gateway health monitoring.
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match check_qq_gateway_health(&client, &bot_app_id, &bot_token).await {
                    Ok(()) => {
                        if !connected.load(Ordering::SeqCst) {
                            connected.store(true, Ordering::SeqCst);
                            tracing::info!("QQ: connection restored");
                        }
                    }
                    Err(e) => {
                        tracing::error!("QQ: gateway health check failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("QQ poll loop stopped");
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
        chat_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let bot_app_id = config.qq_bot_app_id.clone().unwrap_or_default();
        let bot_token = config.qq_bot_token.clone().unwrap_or_default();

        send_qq_message(&bot_app_id, &bot_token, chat_id, text).await
    }
}

impl Default for QQAdapter {
    fn default() -> Self {
        Self::new()
    }
}

async fn verify_qq_bot(
    client: &reqwest::Client,
    _bot_app_id: &str,
    bot_token: &str,
) -> anyhow::Result<()> {
    let url = "https://api.sgroup.qq.com/v2/users/me";
    let resp = client
        .get(url)
        .header("Authorization", format!("QQBot {}", bot_token))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("QQ bot verification failed ({}): {}", status.as_u16(), body);
    }
    Ok(())
}

async fn check_qq_gateway_health(
    client: &reqwest::Client,
    bot_app_id: &str,
    bot_token: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://api.sgroup.qq.com/gateway/bot?appid={}",
        bot_app_id
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("QQBot {}", bot_token))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("QQ gateway health check failed ({}): {}", status.as_u16(), body);
    }
    Ok(())
}

async fn send_qq_message(
    _bot_app_id: &str,
    bot_token: &str,
    channel_id: &str,
    text: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://api.sgroup.qq.com/v2/channels/{}/messages",
        channel_id
    );
    let body = serde_json::json!({
        "content": text,
        "msg_type": 0,
        "markdown": {
            "content": text
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("QQBot {}", bot_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let resp_body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "QQ send message failed ({}): {}",
            status.as_u16(),
            resp_body
        );
    }
    Ok(())
}
