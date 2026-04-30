use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct DiscordAdapter {
    connected: Arc<AtomicBool>,
    ws_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl DiscordAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            ws_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for DiscordAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &'static str {
        "discord"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.discord_enabled && config.discord_bot_token.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("Discord is not enabled or missing bot token");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let bot_token = config
            .discord_bot_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Discord bot token missing"))?;
        let allowed_channels = config.discord_allowed_channels.clone();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match run_discord_gateway(&bot_token, &allowed_channels, &connected, &running).await
                {
                    Ok(()) => tracing::info!("Discord gateway disconnected"),
                    Err(e) => tracing::error!("Discord gateway error: {}", e),
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
        _channel_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let webhook_url = config
            .discord_webhook_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Discord webhook URL missing"))?;

        let body = serde_json::json!({ "content": text });
        let client = reqwest::Client::new();
        let resp = client.post(webhook_url).json(&body).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "Discord webhook failed: {}",
                resp.text().await.unwrap_or_default()
            );
        }
        Ok(())
    }
}

async fn run_discord_gateway(
    bot_token: &str,
    allowed_channels: &Option<Vec<String>>,
    connected: &Arc<AtomicBool>,
    running: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let gateway_url = get_discord_gateway_url(bot_token).await?;
    let ws_url = format!("{}?v=10&encoding=json", gateway_url);

    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| anyhow::anyhow!("Discord ws connect: {}", e))?;

    connected.store(true, Ordering::SeqCst);

    let mut sequence: Option<i64> = None;
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
                tracing::error!("Discord ws error: {}", e);
                break;
            }
            Ok(None) => break,
            Err(_) => {
                if !identified {
                    continue;
                }
                if !last_heartbeat_ack {
                    tracing::error!("Discord heartbeat not acknowledged, reconnecting");
                    break;
                }
                let hb = serde_json::json!({ "op": 1, "d": sequence });
                let hb_str = serde_json::to_string(&hb)?;
                if let Err(e) = ws_stream.send(Message::Text(hb_str.into())).await {
                    tracing::error!("Discord heartbeat send failed: {}", e);
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
                        tracing::warn!("Discord: invalid JSON: {}", e);
                        continue;
                    }
                };

                let op = payload["op"].as_i64().unwrap_or(-1);
                sequence = payload["s"].as_i64().or(sequence);

                match op {
                    0 => {
                        let t = payload["t"].as_str().unwrap_or("");
                        handle_dispatch(t, &payload["d"], allowed_channels, bot_token).await;
                    }
                    7 => {
                        last_heartbeat_ack = false;
                    }
                    10 => {
                        heartbeat_interval =
                            payload["d"]["heartbeat_interval"].as_u64().unwrap_or(41250);

                        let identify = serde_json::json!({
                            "op": 2,
                            "d": {
                                "token": bot_token,
                                "intents": 512,
                                "properties": {
                                    "os": "linux",
                                    "browser": "axagent",
                                    "device": "axagent"
                                }
                            }
                        });
                        let id_str = serde_json::to_string(&identify)?;
                        ws_stream.send(Message::Text(id_str.into())).await?;
                        identified = true;
                    }
                    11 => {
                        last_heartbeat_ack = true;
                    }
                    _ => {}
                }
            }
            Message::Close(_) => {
                tracing::info!("Discord ws close frame received");
                break;
            }
            _ => {}
        }
    }

    connected.store(false, Ordering::SeqCst);
    Ok(())
}

async fn handle_dispatch(
    event_type: &str,
    data: &serde_json::Value,
    allowed_channels: &Option<Vec<String>>,
    bot_token: &str,
) {
    match event_type {
        "MESSAGE_CREATE" => {
            let channel_id = data["channel_id"].as_str().unwrap_or("").to_string();
            if let Some(ref channels) = allowed_channels {
                if !channels.contains(&channel_id) {
                    return;
                }
            }

            let author_id = data["author"]["id"].as_str().unwrap_or("").to_string();
            let author_username = data["author"]["username"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let content = data["content"].as_str().unwrap_or("").to_string();

            if author_id.is_empty() || content.is_empty() {
                return;
            }

            if let Some(bot) = data["author"]["bot"].as_bool() {
                if bot {
                    return;
                }
            }

            tracing::info!(
                "Discord message: {}#{} in {}: {}",
                author_username,
                author_id,
                channel_id,
                content
            );

            if let Some(cb) = crate::message_gateway::platforms::get_message_callback() {
                let bot = bot_token.to_string();
                let ch = channel_id.clone();
                tokio::spawn(async move {
                    let reply = cb
                        .on_message("discord", &author_id, Some(&author_username), &ch, &content)
                        .await;
                    if let Some(reply_text) = reply {
                        let client = reqwest::Client::new();
                        let url = format!("https://discord.com/api/v10/channels/{}/messages", ch);
                        let body = serde_json::json!({ "content": &reply_text[..2000.min(reply_text.len())] });
                        let _ = client
                            .post(&url)
                            .header("Authorization", format!("Bot {}", bot))
                            .json(&body)
                            .send()
                            .await;
                    }
                });
            }
        }
        "READY" => {
            let username = data["user"]["username"].as_str().unwrap_or("unknown");
            tracing::info!("Discord bot connected as {}", username);
        }
        _ => {}
    }
}

async fn get_discord_gateway_url(bot_token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://discord.com/api/v10/gateway/bot")
        .header("Authorization", format!("Bot {}", bot_token))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to get Discord gateway URL: {}", resp.status());
    }

    let json: serde_json::Value = resp.json().await?;
    json["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No gateway URL in Discord response"))
}
