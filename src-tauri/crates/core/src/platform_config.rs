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

    pub slack_enabled: bool,
    pub slack_bot_token: Option<String>,
    pub slack_signing_secret: Option<String>,
    pub slack_workspace_id: Option<String>,
    pub slack_app_token: Option<String>,

    pub whatsapp_enabled: bool,
    pub whatsapp_phone_number_id: Option<String>,
    pub whatsapp_access_token: Option<String>,
    pub whatsapp_business_account_id: Option<String>,

    pub wechat_enabled: bool,
    pub wechat_app_id: Option<String>,
    pub wechat_app_secret: Option<String>,
    pub wechat_token: Option<String>,
    pub wechat_encoding_aes_key: Option<String>,
    pub wechat_original_id: Option<String>,

    pub feishu_enabled: bool,
    pub feishu_app_id: Option<String>,
    pub feishu_app_secret: Option<String>,
    pub feishu_verification_token: Option<String>,
    pub feishu_encrypt_key: Option<String>,

    pub qq_enabled: bool,
    pub qq_bot_app_id: Option<String>,
    pub qq_bot_token: Option<String>,
    pub qq_bot_secret: Option<String>,

    pub dingtalk_enabled: bool,
    pub dingtalk_app_key: Option<String>,
    pub dingtalk_app_secret: Option<String>,
    pub dingtalk_robot_code: Option<String>,

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

            slack_enabled: false,
            slack_bot_token: None,
            slack_signing_secret: None,
            slack_workspace_id: None,
            slack_app_token: None,

            whatsapp_enabled: false,
            whatsapp_phone_number_id: None,
            whatsapp_access_token: None,
            whatsapp_business_account_id: None,

            wechat_enabled: false,
            wechat_app_id: None,
            wechat_app_secret: None,
            wechat_token: None,
            wechat_encoding_aes_key: None,
            wechat_original_id: None,

            feishu_enabled: false,
            feishu_app_id: None,
            feishu_app_secret: None,
            feishu_verification_token: None,
            feishu_encrypt_key: None,

            qq_enabled: false,
            qq_bot_app_id: None,
            qq_bot_token: None,
            qq_bot_secret: None,

            dingtalk_enabled: false,
            dingtalk_app_key: None,
            dingtalk_app_secret: None,
            dingtalk_robot_code: None,

            api_server_enabled: false,
            api_server_port: None,

            auto_sync_messages: true,
            max_history_per_session: 100,
        }
    }
}

impl PlatformConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.telegram_enabled && self.telegram_bot_token.is_none() {
            anyhow::bail!("Telegram bot token is required when Telegram is enabled");
        }

        if self.discord_enabled && self.discord_bot_token.is_none() {
            anyhow::bail!("Discord bot token is required when Discord is enabled");
        }

        if self.slack_enabled && self.slack_app_token.is_none() {
            anyhow::bail!("Slack app token is required when Slack Socket Mode is enabled");
        }

        if self.wechat_enabled && (self.wechat_app_id.is_none() || self.wechat_app_secret.is_none())
        {
            anyhow::bail!("WeChat app_id and app_secret are required when WeChat is enabled");
        }

        if self.feishu_enabled && (self.feishu_app_id.is_none() || self.feishu_app_secret.is_none())
        {
            anyhow::bail!("Feishu app_id and app_secret are required when Feishu is enabled");
        }

        if self.qq_enabled && (self.qq_bot_app_id.is_none() || self.qq_bot_token.is_none()) {
            anyhow::bail!("QQ bot_app_id and bot_token are required when QQ is enabled");
        }

        if self.dingtalk_enabled
            && (self.dingtalk_app_key.is_none() || self.dingtalk_app_secret.is_none())
        {
            anyhow::bail!("Dingtalk app_key and app_secret are required when Dingtalk is enabled");
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
