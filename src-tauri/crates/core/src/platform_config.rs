//! Platform configuration types
//!
//! DTO 由 `axagent-harness` 提供，本模块 re-export 并附加校验逻辑。

pub use axagent_harness::platform_config::PlatformConfig;

/// 校验扩展 trait — 因 PlatformConfig 定义在 harness 中，不能做 inherent impl
pub trait PlatformConfigExt {
    fn validate(&self) -> anyhow::Result<()>;
}

impl PlatformConfigExt for PlatformConfig {
    fn validate(&self) -> anyhow::Result<()> {
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
            && (self.dingtalk_app_key.is_none()
                || self.dingtalk_app_secret.is_none()
                || self.dingtalk_agent_id.is_none())
        {
            anyhow::bail!(
                "Dingtalk app_key, app_secret, and agent_id are required when Dingtalk is enabled"
            );
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
