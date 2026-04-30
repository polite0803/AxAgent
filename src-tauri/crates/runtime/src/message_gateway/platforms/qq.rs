use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct QQAdapter {
    connected: Arc<AtomicBool>,
    ws_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl QQAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            ws_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for QQAdapter {
    fn default() -> Self {
        Self::new()
    }
}

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
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match run_qq_gateway(&bot_app_id, &bot_token, &connected, &running).await {
                    Ok(()) => tracing::info!("QQ gateway disconnected"),
                    Err(e) => tracing::error!("QQ gateway error: {}", e),
                }

                if running.load(Ordering::SeqCst) {
                    connected.store(false, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
        });

        *self.ws_task.lock().await = Some(task);
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.ws_task.lock().await.take() {
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
        let bot_token = config.qq_bot_token.clone().unwrap_or_default();
        send_qq_message(&bot_token, chat_id, text).await
    }
}

async fn run_qq_gateway(
    bot_app_id: &str,
    bot_token: &str,
    connected: &Arc<AtomicBool>,
    running: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let gateway_url = get_qq_gateway_url(&client, bot_app_id, bot_token).await?;

    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&gateway_url)
        .await
        .map_err(|e| anyhow::anyhow!("QQ ws connect: {}", e))?;

    tracing::info!("QQ: WebSocket connected");

    let mut sequence: Option<u64> = None;
    let mut heartbeat_interval: u64 = 41250;
    let mut identified = false;
    let mut last_heartbeat_ack = true;

    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let timeout_dur = std::time::Duration::from_millis(heartbeat_interval + 10000);

        let msg = match tokio::time::timeout(timeout_dur, ws_stream.next()).await {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(e))) => {
                tracing::error!("QQ ws error: {}", e);
                break;
            }
            Ok(None) => break,
            Err(_) => {
                if !identified {
                    continue;
                }
                if !last_heartbeat_ack {
                    tracing::error!("QQ heartbeat not acknowledged, reconnecting");
                    break;
                }
                let hb = serde_json::json!({ "op": 1, "d": sequence });
                let hb_str = serde_json::to_string(&hb)?;
                if let Err(e) = ws_stream.send(Message::Text(hb_str.into())).await {
                    tracing::error!("QQ heartbeat send failed: {}", e);
                    break;
                }
                last_heartbeat_ack = false;
                continue;
            }
        };

        match msg {
            Message::Text(text) => {
                let payload: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("QQ: invalid JSON: {}", e);
                        continue;
                    }
                };

                let op = payload["op"].as_i64().unwrap_or(-1);
                sequence = payload["s"].as_u64().or(sequence);

                match op {
                    0 => {
                        let t = payload["t"].as_str().unwrap_or("");
                        let d = &payload["d"];

                        match t {
                            "READY" => {
                                connected.store(true, Ordering::SeqCst);
                                tracing::info!("QQ: ready");
                            }
                            "RESUMED" => {
                                connected.store(true, Ordering::SeqCst);
                                tracing::info!("QQ: session resumed");
                            }
                            "AT_MESSAGE_CREATE" | "MESSAGE_CREATE"
                            | "DIRECT_MESSAGE_CREATE" | "C2C_MESSAGE_CREATE"
                            | "GROUP_AT_MESSAGE_CREATE" => {
                                let author_id = d["author"]["id"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();
                                let author_username = d["author"]["username"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();
                                let content = d["content"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();
                                let channel_id = d["channel_id"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();
                                let guild_id = d["guild_id"].as_str().unwrap_or("").to_string();

                                if !content.is_empty() {
                                    tracing::info!(
                                        "QQ message: {} ({}) in {}: {}",
                                        author_username,
                                        author_id,
                                        channel_id,
                                        content
                                    );

                                    let cb =
                                        crate::message_gateway::platforms::get_message_callback();
                                    if let Some(cb) = cb {
                                        let bt = bot_token.to_string();
                                        let uid = author_id.clone();
                                        let uname = author_username.clone();
                                        let ch = channel_id.clone();
                                        let t = content.clone();
                                        let gid = guild_id.clone();
                                        tokio::spawn(async move {
                                            let reply = cb
                                                .on_message(
                                                    "qq",
                                                    &uid,
                                                    Some(&uname),
                                                    &ch,
                                                    &t,
                                                )
                                                .await;
                                            if let Some(reply_text) = reply {
                                                send_qq_reply(
                                                    &bt, &ch, &gid, &uid, &reply_text,
                                                )
                                                .await;
                                            }
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    7 => {
                        last_heartbeat_ack = false;
                    }
                    10 => {
                        heartbeat_interval = payload["d"]["heartbeat_interval"]
                            .as_u64()
                            .unwrap_or(41250);

                        if !identified {
                            // QQ bot intents: 1<<0 PUBLIC_GUILD_MESSAGES | 1<<12 GROUP_AT_MESSAGE |
                            // 1<<25 C2C_MESSAGE | 1<<30 DIRECT_MESSAGE
                            let intents = (1u64 << 0) | (1u64 << 12) | (1u64 << 25) | (1u64 << 30);
                            let identify = serde_json::json!({
                                "op": 2,
                                "d": {
                                    "token": format!("QQBot {}", bot_token),
                                    "intents": intents,
                                    "shard": [0, 1],
                                    "properties": {}
                                }
                            });
                            let id_str = serde_json::to_string(&identify)?;
                            ws_stream.send(Message::Text(id_str.into())).await?;
                            identified = true;
                            tracing::info!("QQ: identify sent");
                        }
                    }
                    11 => {
                        last_heartbeat_ack = true;
                    }
                    _ => {}
                }
            }
            Message::Close(_) => {
                tracing::info!("QQ ws close frame received");
                break;
            }
            _ => {}
        }
    }

    connected.store(false, Ordering::SeqCst);
    Ok(())
}

async fn get_qq_gateway_url(
    client: &reqwest::Client,
    bot_app_id: &str,
    bot_token: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://api.sgroup.qq.com/gateway/bot?appid={}",
        bot_app_id
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("QQBot {}", bot_token))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status_code = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("QQ gateway fetch failed ({}): {}", status_code, body);
    }

    let json: serde_json::Value = resp.json().await?;
    json["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No gateway URL in QQ response"))
}

async fn send_qq_message(
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

async fn send_qq_reply(
    bot_token: &str,
    channel_id: &str,
    guild_id: &str,
    user_id: &str,
    text: &str,
) {
    let url = if guild_id.is_empty() {
        format!(
            "https://api.sgroup.qq.com/v2/users/{}/messages",
            user_id
        )
    } else {
        format!(
            "https://api.sgroup.qq.com/v2/channels/{}/messages",
            channel_id
        )
    };
    let body = serde_json::json!({
        "content": text,
        "msg_type": 0,
    });

    let client = reqwest::Client::new();
    let _ = client
        .post(&url)
        .header("Authorization", format!("QQBot {}", bot_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
}
