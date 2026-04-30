use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

            let mut access_token = match fetch_wechat_token(&client, &app_id, &app_secret).await {
                Some(t) => {
                    tracing::info!("WeChat: access_token obtained");
                    connected.store(true, Ordering::SeqCst);
                    t
                }
                None => {
                    tracing::error!("WeChat: failed to obtain access_token");
                    connected.store(false, Ordering::SeqCst);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut token_expiry = chrono::Utc::now().timestamp() + 7000;

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                let now = chrono::Utc::now().timestamp();
                if now > token_expiry {
                    match fetch_wechat_token(&client, &app_id, &app_secret).await {
                        Some(t) => {
                            access_token = t;
                            token_expiry = now + 7000;
                            connected.store(true, Ordering::SeqCst);
                            tracing::info!("WeChat: access_token refreshed");
                        }
                        None => {
                            tracing::error!("WeChat: token refresh failed");
                            connected.store(false, Ordering::SeqCst);
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                }

                if let Ok(msgs) = poll_wechat_cs_messages(&client, &access_token).await {
                    for (openid, text) in msgs {
                        tracing::info!("WeChat msg: {} from {}", text, openid);

                        if let Some(cb) = crate::message_gateway::platforms::get_message_callback()
                        {
                            let at = access_token.clone();
                            let oid = openid.clone();
                            let t = text.clone();
                            tokio::spawn(async move {
                                let reply = cb.on_message("wechat", &oid, None, &oid, &t).await;
                                if let Some(reply_text) = reply {
                                    let _ =
                                        send_wechat_custom_message(&at, &oid, &reply_text).await;
                                }
                            });
                        }
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
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

async fn fetch_wechat_token(
    client: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
) -> Option<String> {
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

async fn poll_wechat_cs_messages(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/message/custom/receive?access_token={}",
        access_token
    );

    let body = serde_json::json!({
        "action": "get_message",
        "limit": 10
    });

    let resp = client.post(&url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;

    let mut results = Vec::new();

    if let Some(records) = json["record_list"].as_array() {
        for record in records {
            let openid = record["openid"].as_str().unwrap_or("");
            let text = record["text"]["content"].as_str().unwrap_or("");
            if !openid.is_empty() && !text.is_empty() {
                results.push((openid.to_string(), text.to_string()));
            }
        }
    }

    Ok(results)
}

async fn send_wechat_custom_message(
    access_token: &str,
    open_id: &str,
    text: &str,
) -> anyhow::Result<()> {
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
