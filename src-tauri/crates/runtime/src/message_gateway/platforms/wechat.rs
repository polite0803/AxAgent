use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct WeChatAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl WeChatAdapter {
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
impl PlatformAdapter for WeChatAdapter {
    fn name(&self) -> &'static str {
        "wechat"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.wechat_enabled
            && config.wechat_app_id.is_some()
            && config.wechat_app_secret.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("WeChat is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let app_id = config.wechat_app_id.clone().unwrap_or_default();
        let app_secret = config.wechat_app_secret.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();
            connected.store(true, Ordering::SeqCst);

            // Step 1: Fetch access_token
            let access_token = match fetch_wechat_token(&client, &app_id, &app_secret).await {
                Some(token) => token,
                None => {
                    tracing::error!("WeChat: failed to obtain access_token");
                    connected.store(false, Ordering::SeqCst);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            tracing::info!("WeChat: access_token obtained");

            // Step 2: Long-poll for messages using custom menu / callback wait
            // WeChat Official Accounts use server-push (callback URL), so this adapter
            // polls a lightweight status endpoint. For full message receiving, a public
            // callback URL is needed. This adapter provides the send capability and
            // status monitoring.
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match check_wechat_api_status(&client, &access_token).await {
                    Ok(()) => {
                        if !connected.load(Ordering::SeqCst) {
                            connected.store(true, Ordering::SeqCst);
                            tracing::info!("WeChat: connection restored");
                        }
                    }
                    Err(e) => {
                        tracing::error!("WeChat: API check failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("WeChat poll loop stopped");
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
        open_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let app_id = config.wechat_app_id.clone().unwrap_or_default();
        let app_secret = config.wechat_app_secret.clone().unwrap_or_default();

        let access_token = fetch_wechat_token(&reqwest::Client::new(), &app_id, &app_secret)
            .await
            .ok_or_else(|| anyhow::anyhow!("WeChat: failed to fetch access_token"))?;

        send_wechat_custom_message(&access_token, open_id, text).await
    }
}

impl Default for WeChatAdapter {
    fn default() -> Self {
        Self::new()
    }
}

async fn fetch_wechat_token(client: &reqwest::Client, app_id: &str, app_secret: &str) -> Option<String> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
        app_id, app_secret
    );
    let resp = client.get(&url).send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

async fn check_wechat_api_status(client: &reqwest::Client, access_token: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/getcallbackip?access_token={}",
        access_token
    );
    let resp = client.get(&url).send().await?;
    let body: serde_json::Value = resp.json().await?;
    if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 {
            anyhow::bail!("WeChat API error: errcode={}, errmsg={:?}", errcode, body.get("errmsg"));
        }
    }
    Ok(())
}

async fn send_wechat_custom_message(access_token: &str, open_id: &str, text: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/message/custom/send?access_token={}",
        access_token
    );
    let body = serde_json::json!({
        "touser": open_id,
        "msgtype": "text",
        "text": {
            "content": text
        }
    });

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body).send().await?;
    let resp_body: serde_json::Value = resp.json().await?;

    if let Some(errcode) = resp_body.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 {
            anyhow::bail!(
                "WeChat send failed: errcode={}, errmsg={:?}",
                errcode,
                resp_body.get("errmsg")
            );
        }
    }
    Ok(())
}
