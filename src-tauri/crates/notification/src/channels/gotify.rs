// SPDX-License-Identifier: AGPL-3.0-only

//! Gotify 渠道（自托管 Gotify 推送）
//!
//! API 文档：https://gotify.net/docs/
//! POST https://{host}/message?token={token}
//! body: { title, message, priority }
//!
//! Gotify 是自托管推送服务，priority 范围 0-10

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

use axagent_harness::{AlertPayload, AlertSeverity, NotificationChannel, ReportPayload};

/// Gotify 渠道配置
#[derive(Debug, Clone)]
pub struct GotifyConfig {
    /// Gotify 服务器地址（如 https://gotify.example.com）
    pub server_url: String,
    /// 应用 token（在 Gotify 创建应用时获取）
    pub token: String,
    /// 默认优先级（0-10，默认 5）
    pub default_priority: Option<u8>,
}

/// Gotify 推送渠道
pub struct GotifyChannel {
    config: GotifyConfig,
    client: Client,
}

#[derive(Serialize)]
struct GotifyRequest<'a> {
    title: &'a str,
    message: String,
    priority: u8,
}

impl GotifyChannel {
    pub fn new(config: GotifyConfig) -> Self {
        Self { config, client: Client::new() }
    }

    pub fn with_client(config: GotifyConfig, client: Client) -> Self {
        Self { config, client }
    }

    fn api_url(&self) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        format!("{}/message?token={}", base, self.config.token)
    }

    /// severity → gotify priority（0-10）
    fn severity_to_priority(severity: AlertSeverity) -> u8 {
        match severity {
            AlertSeverity::Info => 2,
            AlertSeverity::Warning => 5,
            AlertSeverity::Error => 8,
            AlertSeverity::Critical => 10,
        }
    }

    async fn send(&self, title: &str, message: String, priority: u8) -> Result<String, String> {
        let req = GotifyRequest { title, message, priority };
        let resp = self
            .client
            .post(self.api_url())
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Gotify 请求失败: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Gotify HTTP {status}: {body}"));
        }
        Ok(format!("gotify-{}", title))
    }
}

#[async_trait]
impl NotificationChannel for GotifyChannel {
    fn name(&self) -> &str {
        "gotify"
    }

    fn display_name(&self) -> &str {
        "Gotify"
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        let priority = self.config.default_priority.unwrap_or(5);
        let mut msg = payload.body_md.clone();
        if !payload.stocks.is_empty() {
            msg.push_str("\n\n股票摘要:\n");
            for s in &payload.stocks {
                msg.push_str(&format!(
                    "- {}({}): {} 评分:{}\n",
                    s.stock_name, s.stock_code, s.action, s.score
                ));
            }
        }
        self.send(&payload.title, msg, priority).await
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        let priority = GotifyChannel::severity_to_priority(payload.severity);
        let msg = if let Some(code) = &payload.stock_code {
            format!("{} ({})\n\n{}", payload.title, code, payload.body)
        } else {
            payload.body.clone()
        };
        self.send(&payload.title, msg, priority).await
    }

    async fn is_ready(&self) -> bool {
        !self.config.server_url.is_empty() && !self.config.token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_cfg(token: &str) -> GotifyConfig {
        GotifyConfig {
            server_url: "https://gotify.example.com".to_string(),
            token: token.to_string(),
            default_priority: None,
        }
    }

    #[test]
    fn test_name_and_display() {
        let ch = GotifyChannel::new(mk_cfg("t"));
        assert_eq!(ch.name(), "gotify");
        assert_eq!(ch.display_name(), "Gotify");
    }

    #[test]
    fn test_api_url() {
        let ch = GotifyChannel::new(mk_cfg("AbCd123"));
        assert_eq!(ch.api_url(), "https://gotify.example.com/message?token=AbCd123");
    }

    #[test]
    fn test_api_url_trailing_slash() {
        let mut cfg = mk_cfg("t");
        cfg.server_url = "https://gotify.example.com/".to_string();
        let ch = GotifyChannel::new(cfg);
        assert_eq!(ch.api_url(), "https://gotify.example.com/message?token=t");
    }

    #[test]
    fn test_severity_to_priority() {
        assert_eq!(GotifyChannel::severity_to_priority(AlertSeverity::Info), 2);
        assert_eq!(GotifyChannel::severity_to_priority(AlertSeverity::Warning), 5);
        assert_eq!(GotifyChannel::severity_to_priority(AlertSeverity::Error), 8);
        assert_eq!(GotifyChannel::severity_to_priority(AlertSeverity::Critical), 10);
    }

    #[tokio::test]
    async fn test_is_ready() {
        assert!(GotifyChannel::new(mk_cfg("abc")).is_ready().await);
        let mut cfg = mk_cfg("");
        cfg.server_url = "".to_string();
        assert!(!GotifyChannel::new(cfg).is_ready().await);
    }

    #[tokio::test]
    async fn test_send_unreachable_returns_err() {
        let mut cfg = mk_cfg("abc");
        cfg.server_url = "http://127.0.0.1:1".to_string();
        let ch = GotifyChannel::new(cfg);
        let res = ch.send("t", "b".to_string(), 5).await;
        assert!(res.is_err());
    }
}
