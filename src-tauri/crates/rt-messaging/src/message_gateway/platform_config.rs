// SPDX-License-Identifier: AGPL-3.0-only

//! Platform configuration types
//!
//! DTO 由 `axagent-harness` 提供，本模块 re-export。

pub use axagent_harness::platform_config::PlatformConfig;

/// 校验扩展 trait
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        // 全平台关闭的默认配置应通过校验
        assert!(PlatformConfig::default().validate().is_ok());
    }

    #[test]
    fn telegram_enabled_requires_token() {
        let mut c = PlatformConfig { telegram_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.telegram_bot_token = Some("bot-token".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn discord_enabled_requires_token() {
        let mut c = PlatformConfig { discord_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.discord_bot_token = Some("t".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn slack_enabled_requires_app_token() {
        let mut c = PlatformConfig { slack_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.slack_app_token = Some("xapp-1".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn wechat_enabled_requires_app_id_and_secret() {
        let mut c = PlatformConfig { wechat_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.wechat_app_id = Some("id".into());
        // 仅有 app_id 仍不足
        assert!(c.validate().is_err());
        c.wechat_app_secret = Some("secret".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn feishu_enabled_requires_app_id_and_secret() {
        let mut c = PlatformConfig { feishu_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.feishu_app_id = Some("id".into());
        assert!(c.validate().is_err());
        c.feishu_app_secret = Some("secret".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn qq_enabled_requires_app_id_and_token() {
        let mut c = PlatformConfig { qq_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.qq_bot_app_id = Some("id".into());
        assert!(c.validate().is_err());
        c.qq_bot_token = Some("tok".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn dingtalk_enabled_requires_key_secret_agent() {
        let mut c = PlatformConfig { dingtalk_enabled: true, ..Default::default() };
        assert!(c.validate().is_err());
        c.dingtalk_app_key = Some("k".into());
        assert!(c.validate().is_err());
        c.dingtalk_app_secret = Some("s".into());
        assert!(c.validate().is_err());
        c.dingtalk_agent_id = Some("a".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn api_server_rejects_zero_port() {
        let mut c = PlatformConfig { api_server_enabled: true, ..Default::default() };
        // 默认端口回退到 8080 → ok
        assert!(c.validate().is_ok());
        c.api_server_port = Some(0);
        assert!(c.validate().is_err());
        c.api_server_port = Some(9000);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn disabled_platform_ignores_missing_credentials() {
        // 平台关闭时即使缺凭证也应通过
        let c = PlatformConfig {
            telegram_enabled: false,
            telegram_bot_token: None,
            dingtalk_enabled: false,
            ..Default::default()
        };
        assert!(c.validate().is_ok());
    }
}
