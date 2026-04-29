use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct FeishuAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl FeishuAdapter {
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
impl PlatformAdapter for FeishuAdapter {
    fn name(&self) -> &'static str {
        "feishu"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.feishu_enabled
            && config.feishu_app_id.is_some()
            && config.feishu_app_secret.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("Feishu is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let app_id = config.feishu_app_id.clone().unwrap_or_default();
        let app_secret = config.feishu_app_secret.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Fetch initial tenant_access_token
            match fetch_tenant_access_token(&client, &app_id, &app_secret).await {
                Ok(_token) => {
                    tracing::info!("Feishu: tenant_access_token obtained");
                    connected.store(true, Ordering::SeqCst);
                    // Store token for send_message use
                    // (in a production system this would be cached and refreshed)
                }
                Err(e) => {
                    tracing::error!("Feishu: failed to obtain tenant_access_token: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            }

            // Status monitoring loop
            // Feishu uses server-push (Event Subscription) for message receiving.
            // This adapter monitors API health and maintains the connection state.
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match verify_feishu_api_health(&client, &app_id, &app_secret).await {
                    Ok(()) => {
                        if !connected.load(Ordering::SeqCst) {
                            connected.store(true, Ordering::SeqCst);
                            tracing::info!("Feishu: connection restored");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Feishu: API health check failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("Feishu poll loop stopped");
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
        let app_id = config.feishu_app_id.clone().unwrap_or_default();
        let app_secret = config.feishu_app_secret.clone().unwrap_or_default();

        let client = reqwest::Client::new();
        let token = fetch_tenant_access_token(&client, &app_id, &app_secret).await?;

        send_feishu_text_message(&client, &token, chat_id, text).await
    }
}

impl Default for FeishuAdapter {
    fn default() -> Self {
        Self::new()
    }
}

async fn fetch_tenant_access_token(
    client: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
) -> anyhow::Result<String> {
    let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
    let body = serde_json::json!({
        "app_id": app_id,
        "app_secret": app_secret
    });

    let resp = client.post(url).json(&body).send().await?;
    let body: serde_json::Value = resp.json().await?;

    if let Some(code) = body.get("code").and_then(|v| v.as_i64()) {
        if code != 0 {
            let msg = body
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("Feishu API error: code={}, msg={}", code, msg);
        }
    }

    body.get("tenant_access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Feishu: tenant_access_token not found in response"))
}

async fn verify_feishu_api_health(
    client: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
) -> anyhow::Result<()> {
    let _token = fetch_tenant_access_token(client, app_id, app_secret).await?;
    let url = "https://open.feishu.cn/open-apis/auth/v3/app_access_token/internal";
    let body = serde_json::json!({
        "app_id": app_id,
        "app_secret": app_secret
    });

    let resp = client.post(url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    if let Some(code) = json.get("code").and_then(|v| v.as_i64()) {
        if code != 0 {
            anyhow::bail!("Feishu health check failed: code={}", code);
        }
    }
    Ok(())
}

async fn send_feishu_text_message(
    client: &reqwest::Client,
    token: &str,
    receive_id: &str,
    text: &str,
) -> anyhow::Result<()> {
    let url = "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id";
    let body = serde_json::json!({
        "receive_id": receive_id,
        "msg_type": "text",
        "content": serde_json::json!({
            "text": text
        }).to_string()
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    if let Some(code) = json.get("code").and_then(|v| v.as_i64()) {
        if code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("Feishu send message failed: code={}, msg={}", code, msg);
        }
    }
    Ok(())
}
