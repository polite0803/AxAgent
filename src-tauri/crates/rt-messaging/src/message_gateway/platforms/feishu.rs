use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

            let token = match fetch_feishu_token(&client, &app_id, &app_secret).await {
                Ok(t) => {
                    tracing::info!("Feishu: tenant_access_token obtained");
                    connected.store(true, Ordering::SeqCst);
                    t
                },
                Err(e) => {
                    tracing::error!("Feishu: failed to obtain token: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                },
            };

            let mut seen_ids: HashSet<String> = HashSet::new();

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match poll_feishu_messages(&client, &token, &mut seen_ids).await {
                    Ok(msgs) => {
                        for (user_id, text, chat_id, msg_id) in msgs {
                            tracing::info!("Feishu msg [{}]: {} (from {})", msg_id, text, user_id);

                            if let Some(cb) =
                                crate::message_gateway::platforms::get_message_callback()
                            {
                                let bt = token.clone();
                                let uid = user_id.clone();
                                let t = text.clone();
                                let ch = chat_id.clone();
                                tokio::spawn(async move {
                                    let reply = cb.on_message("feishu", &uid, None, &ch, &t).await;
                                    if let Some(reply_text) = reply {
                                        let _ = send_feishu_text_message(
                                            &reqwest::Client::new(),
                                            &bt,
                                            &uid,
                                            &reply_text,
                                        )
                                        .await;
                                    }
                                });
                            }
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Feishu: message poll failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        match fetch_feishu_token(&client, &app_id, &app_secret).await {
                            Ok(_) => {
                                connected.store(true, Ordering::SeqCst);
                            },
                            Err(e2) => {
                                tracing::error!("Feishu: token refresh failed: {}", e2);
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            },
                        }
                    },
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
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
        let token = fetch_feishu_token(&client, &app_id, &app_secret).await?;

        send_feishu_text_message(&client, &token, chat_id, text).await
    }
}

impl Default for FeishuAdapter {
    fn default() -> Self {
        Self::new()
    }
}

async fn fetch_feishu_token(
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

async fn poll_feishu_messages(
    client: &reqwest::Client,
    token: &str,
    seen_ids: &mut HashSet<String>,
) -> anyhow::Result<Vec<(String, String, String, String)>> {
    let url = "https://open.feishu.cn/open-apis/im/v1/messages\
         ?receive_id_type=tenant\
         &page_size=20\
         &sort_type=ByCreateTimeDesc"
        .to_string();

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    if let Some(code) = json.get("code").and_then(|v| v.as_i64()) {
        if code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("Feishu list messages failed: code={}, msg={}", code, msg);
        }
    }

    let mut results = Vec::new();

    if let Some(items) = json["data"]["items"].as_array() {
        for item in items {
            let msg_id = item["message_id"].as_str().unwrap_or("");
            let msg_type = item["msg_type"].as_str().unwrap_or("");
            let chat_id = item["chat_id"].as_str().unwrap_or("");
            let sender_id = item["sender"]["id"].as_str().unwrap_or("sender");

            if seen_ids.contains(msg_id) {
                continue;
            }
            seen_ids.insert(msg_id.to_string());

            if seen_ids.len() > 5000 {
                let to_remove: Vec<String> = seen_ids.iter().take(2000).cloned().collect();
                for old in to_remove {
                    seen_ids.remove(&old);
                }
            }

            let content = &item["body"]["content"];
            let text = match msg_type {
                "text" => content["text"].as_str().unwrap_or("").to_string(),
                "post" => content.as_str().unwrap_or("").to_string(),
                _ => continue,
            };

            if !text.is_empty() {
                results.push((
                    sender_id.to_string(),
                    text,
                    chat_id.to_string(),
                    msg_id.to_string(),
                ));
            }
        }
    }

    Ok(results)
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
