use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct SlackAdapter {
    connected: Arc<AtomicBool>,
    ws_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl SlackAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            ws_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for SlackAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for SlackAdapter {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.slack_enabled && config.slack_app_token.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("Slack is not enabled or missing app token");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let app_token = config
            .slack_app_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Slack app token missing"))?;
        let bot_token = config.slack_bot_token.clone().unwrap_or_default();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match run_slack_socket_mode(&app_token, &bot_token, &connected, &running).await {
                    Ok(()) => tracing::info!("Slack Socket Mode disconnected"),
                    Err(e) => tracing::error!("Slack Socket Mode error: {}", e),
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
        _config: &PlatformConfig,
        channel_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let bot_token = _config.slack_bot_token.clone().unwrap_or_default();
        let url = "https://slack.com/api/chat.postMessage";
        let body = serde_json::json!({
            "channel": channel_id,
            "text": text,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {}", bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Slack API error: {}", resp.status()));
        }
        Ok(())
    }
}

async fn run_slack_socket_mode(
    app_token: &str,
    bot_token: &str,
    connected: &Arc<AtomicBool>,
    running: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let ws_url = get_slack_socket_url(app_token).await?;
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| anyhow::anyhow!("Slack ws connect: {}", e))?;

    connected.store(true, Ordering::SeqCst);

    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let msg = match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            ws_stream.next(),
        )
        .await
        {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(e))) => {
                tracing::error!("Slack ws error: {}", e);
                break;
            }
            Ok(None) => break,
            Err(_) => continue,
        };

        match msg {
            Message::Text(text) => {
                let payload: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Slack: invalid JSON: {}", e);
                        continue;
                    }
                };

                let envelope_id = payload["envelope_id"].as_str().unwrap_or("");
                let msg_type = payload["type"].as_str().unwrap_or("");

                match msg_type {
                    "hello" => {
                        tracing::info!("Slack Socket Mode connected");
                    }
                    "disconnect" => {
                        tracing::info!("Slack Socket Mode: server requested disconnect");
                        break;
                    }
                    "events_api" => {
                        let ack = serde_json::json!({
                            "envelope_id": envelope_id
                        });
                        let ack_str = serde_json::to_string(&ack)?;
                        let _ = ws_stream.send(Message::Text(ack_str.into())).await;

                        let event = &payload["payload"]["event"];
                        if event["type"] == "message" && event.get("bot_id").is_none() {
                            let user = event["user"].as_str().unwrap_or("");
                            let channel = event["channel"].as_str().unwrap_or("");
                            let text = event["text"].as_str().unwrap_or("");

                            if !text.is_empty() {
                                let cb =
                                    crate::message_gateway::platforms::get_message_callback();
                                if let Some(cb) = cb {
                                    let bt = bot_token.to_string();
                                    let uid = user.to_string();
                                    let ch = channel.to_string();
                                    let t = text.to_string();
                                    tokio::spawn(async move {
                                        let reply = cb
                                            .on_message(
                                                "slack", &uid, None, &ch, &t,
                                            )
                                            .await;
                                        if let Some(reply_text) = reply {
                                            let client = reqwest::Client::new();
                                            let body = serde_json::json!({
                                                "channel": ch,
                                                "text": &reply_text,
                                            });
                                            let _ = client
                                                .post("https://slack.com/api/chat.postMessage")
                                                .header(
                                                    "Authorization",
                                                    format!("Bearer {}", bt),
                                                )
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
                    _ => {}
                }
            }
            Message::Close(_) => {
                tracing::info!("Slack ws close frame received");
                break;
            }
            _ => {}
        }
    }

    connected.store(false, Ordering::SeqCst);
    Ok(())
}

async fn get_slack_socket_url(app_token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://slack.com/api/apps.connections.open")
        .header("Authorization", format!("Bearer {}", app_token))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        anyhow::bail!("Slack apps.connections.open failed: {}", error);
    }

    body["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No url in Slack response"))
}
