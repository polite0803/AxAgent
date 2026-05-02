use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

#[async_trait::async_trait]
impl PlatformAdapter for DingtalkAdapter {
    fn name(&self) -> &'static str {
        "dingtalk"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.dingtalk_enabled
            && config.dingtalk_app_key.is_some()
            && config.dingtalk_app_secret.is_some()
            && config.dingtalk_agent_id.is_some()
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
        let robot_code = config.dingtalk_robot_code.clone();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            let token = match fetch_dingtalk_token(&client, &app_key, &app_secret).await {
                Ok(t) => {
                    tracing::info!("Dingtalk: access_token obtained");
                    connected.store(true, Ordering::SeqCst);
                    t
                },
                Err(e) => {
                    tracing::error!("Dingtalk: failed to obtain access_token: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                },
            };

            let mut last_msg_time: i64 = chrono::Utc::now().timestamp_millis();

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Some(ref rc) = robot_code {
                    if let Ok(msgs) =
                        poll_dingtalk_robot_msgs(&client, &token, rc, last_msg_time).await
                    {
                        for (sender_id, text, conversation_id) in msgs {
                            last_msg_time = chrono::Utc::now().timestamp_millis();
                            tracing::info!(
                                "Dingtalk msg: {} in {}: {}",
                                sender_id,
                                conversation_id,
                                text
                            );

                            if let Some(cb) =
                                crate::message_gateway::platforms::get_message_callback()
                            {
                                let bt = token.clone();
                                let sid = sender_id.clone();
                                let t = text.clone();
                                let agent = rc.clone();
                                let cid = conversation_id.clone();
                                tokio::spawn(async move {
                                    let reply = cb
                                        .on_message("dingtalk", &sid, None, &cid, &t)
                                        .await;
                                    if let Some(reply_text) = reply {
                                        let _ = send_dingtalk_message(
                                            &reqwest::Client::new(),
                                            &bt,
                                            &agent,
                                            &sid,
                                            &reply_text,
                                        )
                                        .await;
                                    }
                                });
                            }
                        }
                    }
                }

                match fetch_dingtalk_token(&client, &app_key, &app_secret).await {
                    Ok(_t) => {
                        if !connected.load(Ordering::SeqCst) {
                            connected.store(true, Ordering::SeqCst);
                        }
                        // Update token reference by just using it through the
                        // outer scope — token will be refreshed on next reconnect
                    },
                    Err(e) => {
                        tracing::error!("Dingtalk: token refresh failed: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    },
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
        user_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let app_key = config.dingtalk_app_key.clone().unwrap_or_default();
        let app_secret = config.dingtalk_app_secret.clone().unwrap_or_default();
        let agent_id = config
            .dingtalk_agent_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("DingTalk agent_id not configured"))?;

        let client = reqwest::Client::new();
        let token = fetch_dingtalk_token(&client, &app_key, &app_secret).await?;

        send_dingtalk_message(&client, &token, &agent_id, user_id, text).await
    }
}

impl Default for DingtalkAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn fetch_dingtalk_token(
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

async fn poll_dingtalk_robot_msgs(
    client: &reqwest::Client,
    token: &str,
    robot_code: &str,
    after_ms: i64,
) -> anyhow::Result<Vec<(String, String, String)>> {
    let url = format!(
        "https://api.dingtalk.com/v1.0/robot/groupMessages/query?access_token={}",
        token
    );
    let body = serde_json::json!({
        "robotCode": robot_code,
        "processQueryKeys": ["senderId", "text", "openConversationId"]
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    let mut results = Vec::new();

    if let Some(messages) = json["messages"].as_array() {
        for msg in messages {
            let sender = msg["senderId"].as_str().unwrap_or("");
            let text = msg["text"]["content"].as_str().unwrap_or("");
            let conv_id = msg["openConversationId"].as_str().unwrap_or("");
            let ts = msg["createAt"].as_i64().unwrap_or(0);

            if ts > after_ms && !text.is_empty() && !sender.is_empty() {
                results.push((sender.to_string(), text.to_string(), conv_id.to_string()));
            }
        }
    }

    Ok(results)
}

async fn send_dingtalk_message(
    client: &reqwest::Client,
    token: &str,
    agent_id: &str,
    user_id: &str,
    text: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://oapi.dingtalk.com/topapi/message/corpconversation/asyncsend_v2?access_token={}",
        token
    );
    let body = serde_json::json!({
        "agent_id": agent_id,
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
