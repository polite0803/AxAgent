use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_webhook_url: Option<String>,
    pub telegram_webhook_secret: Option<String>,
    pub telegram_allowed_users: Option<Vec<i64>>,

    pub discord_enabled: bool,
    pub discord_bot_token: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub discord_allowed_channels: Option<Vec<String>>,

    pub api_server_enabled: bool,
    pub api_server_port: Option<u16>,

    pub auto_sync_messages: bool,
    pub max_history_per_session: usize,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_webhook_url: None,
            telegram_webhook_secret: None,
            telegram_allowed_users: None,

            discord_enabled: false,
            discord_bot_token: None,
            discord_webhook_url: None,
            discord_allowed_channels: None,

            api_server_enabled: false,
            api_server_port: None,

            auto_sync_messages: true,
            max_history_per_session: 100,
        }
    }
}

impl PlatformConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.telegram_enabled {
            if self.telegram_bot_token.is_none() {
                anyhow::bail!("Telegram bot token is required when Telegram is enabled");
            }
        }

        if self.discord_enabled {
            if self.discord_bot_token.is_none() {
                anyhow::bail!("Discord bot token is required when Discord is enabled");
            }
        }

        if self.api_server_enabled {
            let port = self.api_server_port.unwrap_or(8080);
            if port == 0 {
                anyhow::bail!("API server port must be non-zero");
            }
        }

        Ok(())
    }
}
