use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct TelegramAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl TelegramAdapter {
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
impl PlatformAdapter for TelegramAdapter {
    fn name(&self) -> &'static str {
        "telegram"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.telegram_enabled && config.telegram_bot_token.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("Telegram is not enabled or missing bot token");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let bot_token = config
            .telegram_bot_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Telegram bot token missing"))?;
        let allowed_users = config.telegram_allowed_users.clone();

        let connected = self.connected.clone();
        let running = self.running.clone();

        self.running.store(true, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let webhook_url = std::env::var("TELEGRAM_WEBHOOK_URL").ok();
            if let Some(ref url) = webhook_url {
                set_telegram_webhook(&bot_token, url).await;
            } else {
                delete_telegram_webhook(&bot_token).await;
            }

            let client = reqwest::Client::new();
            let mut last_update_id: i64 = 0;
            connected.store(true, Ordering::SeqCst);

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match fetch_updates(&client, &bot_token, last_update_id).await {
                    Ok(updates) => {
                        for update in updates {
                            if let Some(msg) = update.message {
                                last_update_id = last_update_id.max(update.update_id);

                                if let Some(ref allowed) = allowed_users {
                                    if let Some(ref user) = msg.from {
                                        if !allowed.contains(&user.id) {
                                            continue;
                                        }
                                    }
                                }

                                let username = msg.from.as_ref().and_then(|u| u.username.clone());
                                let user_id = msg
                                    .from
                                    .as_ref()
                                    .map(|u| u.id.to_string())
                                    .unwrap_or_default();
                                let text = msg.text.clone().unwrap_or_default();
                                let chat_id = msg.chat.id;

                                if !text.is_empty() {
                                    let response = handle_telegram_command(&text, &username, &user_id);
                                    if let Some(resp) = response {
                                        send_telegram_message(
                                            &client, &bot_token, chat_id, &resp,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Telegram poll error: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        connected.store(true, Ordering::SeqCst);
                    }
                }

                if running.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }

            connected.store(false, Ordering::SeqCst);
            tracing::info!("Telegram poll loop stopped");
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
        parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let bot_token = config
            .telegram_bot_token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Telegram bot token missing"))?;
        let chat_id: i64 = chat_id
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid chat_id format"))?;

        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });
        if let Some(mode) = parse_mode {
            body["parse_mode"] = serde_json::Value::String(mode.to_string());
        }

        let client = reqwest::Client::new();
        let resp = client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Telegram sendMessage failed: {}", error_text);
        }
        Ok(())
    }
}

impl Default for TelegramAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_telegram_command(text: &str, username: &Option<String>, user_id: &str) -> Option<String> {
    let name = username.as_deref().unwrap_or(user_id);

    match text.trim() {
        "/start" => Some(format!(
            "Hello {}! I'm AxAgent, connected via Telegram. How can I help you today?",
            name
        )),
        "/help" => Some(
            "Available commands:\n\
            /start - Start the bot\n\
            /help - Show this help\n\
            /status - Check bot status\n\
            /ping - Ping the bot"
                .to_string(),
        ),
        "/ping" => Some("Pong!".to_string()),
        "/status" => Some("AxAgent bot is running and connected via Telegram.".to_string()),
        _ => {
            if text.starts_with('/') {
                None
            } else {
                Some(format!(
                    "I received: \"{}\". Processing...",
                    text
                ))
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramApiMessage>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct TelegramApiMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    date: i64,
    text: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TelegramUser {
    id: i64,
    username: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TelegramApiResponse {
    ok: bool,
    result: Option<serde_json::Value>,
}

async fn fetch_updates(
    client: &reqwest::Client,
    bot_token: &str,
    offset: i64,
) -> anyhow::Result<Vec<TelegramUpdate>> {
    let url = format!(
        "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30",
        bot_token,
        offset + 1
    );
    let resp = client.get(&url).send().await?;
    let response: TelegramApiResponse = resp.json().await?;

    if !response.ok {
        anyhow::bail!("Telegram API returned ok=false");
    }

    let result = response.result.unwrap_or_default();
    let updates: Vec<TelegramUpdate> = serde_json::from_value(result)
        .map_err(|e| anyhow::anyhow!("Failed to parse Telegram updates: {}", e))?;

    Ok(updates)
}

async fn set_telegram_webhook(bot_token: &str, webhook_url: &str) {
    let url = format!(
        "https://api.telegram.org/bot{}/setWebhook?url={}",
        bot_token, webhook_url
    );
    if let Ok(resp) = reqwest::Client::new().get(&url).send().await {
        tracing::info!("Telegram webhook set: {}", resp.status());
    }
}

async fn delete_telegram_webhook(bot_token: &str) {
    let url = format!(
        "https://api.telegram.org/bot{}/deleteWebhook",
        bot_token
    );
    if let Ok(resp) = reqwest::Client::new().get(&url).send().await {
        tracing::info!("Telegram webhook deleted: {}", resp.status());
    }
}

async fn send_telegram_message(client: &reqwest::Client, bot_token: &str, chat_id: i64, text: &str) {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
    });
    if let Err(e) = client.post(&url).json(&body).send().await {
        tracing::error!("Failed to send Telegram message: {}", e);
    }
}
