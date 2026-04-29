use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct DingtalkAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl DingtalkAdapter {
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
impl PlatformAdapter for DingtalkAdapter {
    fn name(&self) -> &'static str {
        "dingtalk"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.dingtalk_enabled
            && config.dingtalk_app_key.is_some()
            && config.dingtalk_app_secret.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("Dingtalk is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let app_key = config.dingtalk_app_key.clone().unwrap_or_default();
        let app_secret = config.dingtalk_app_secret.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Fetch and verify access_token
            match fetch_dingtalk_token(&client, &app_key, &app_secret).await {
                Ok(_token) => {
                    tracing::info!("Dingtalk: access_token obtained");
                    connected.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    tracing::error!("Dingtalk: failed to obtain access_token: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            }

            // Status monitoring loop
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match verify_dingtalk_health(&client, &app_key, &app_secret).await {
                    Ok(()) => {
                        if !connected.load(Ordering::SeqCst) {
                            connected.store(true, Ordering::SeqCst);
                            tracing::info!("Dingtalk: connection restored");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Dingtalk: health check failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("Dingtalk poll loop stopped");
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
        let app_key = config.dingtalk_app_key.clone().unwrap_or_default();
        let app_secret = config.dingtalk_app_secret.clone().unwrap_or_default();

        let client = reqwest::Client::new();
        let token = fetch_dingtalk_token(&client, &app_key, &app_secret).await?;

        send_dingtalk_message(&client, &token, chat_id, text).await
    }
}

impl Default for DingtalkAdapter {
    fn default() -> Self {
        Self::new()
    }
}

async fn fetch_dingtalk_token(
    client: &reqwest::Client,
    app_key: &str,
    app_secret: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://oapi.dingtalk.com/gettoken?appkey={}&appsecret={}",
        app_key, app_secret
    );
    let resp = client.get(&url).send().await?;
    let body: serde_json::Value = resp.json().await?;

    if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 {
            let errmsg = body
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!(
                "Dingtalk token error: errcode={}, errmsg={}",
                errcode,
                errmsg
            );
        }
    }

    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Dingtalk: access_token not found in response"))
}

async fn verify_dingtalk_health(
    client: &reqwest::Client,
    app_key: &str,
    app_secret: &str,
) -> anyhow::Result<()> {
    let token = fetch_dingtalk_token(client, app_key, app_secret).await?;
    // Use a lightweight API call to verify the token works
    let url = format!(
        "https://oapi.dingtalk.com/user/get?access_token={}&userid=manager",
        token
    );
    let resp = client.get(&url).send().await?;
    let body: serde_json::Value = resp.json().await?;

    // errcode 0 (success) or 40021 (user not found) both mean the token is valid
    if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 && errcode != 40021 {
            let errmsg = body
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!(
                "Dingtalk health check failed: errcode={}, errmsg={}",
                errcode,
                errmsg
            );
        }
    }
    Ok(())
}

async fn send_dingtalk_message(
    client: &reqwest::Client,
    token: &str,
    user_id: &str,
    text: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://oapi.dingtalk.com/topapi/message/corpconversation/asyncsend_v2?access_token={}",
        token
    );
    let body = serde_json::json!({
        "agent_id": user_id,
        "userid_list": user_id,
        "msg": {
            "msgtype": "text",
            "text": {
                "content": text
            }
        }
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    if let Some(errcode) = json.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 {
            let errmsg = json
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!(
                "Dingtalk send failed: errcode={}, errmsg={}",
                errcode,
                errmsg
            );
        }
    }
    Ok(())
}
