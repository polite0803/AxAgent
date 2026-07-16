// SPDX-License-Identifier: AGPL-3.0-only

//! Server酱渠道（微信推送）
//!
//! API 文档：https://sct.ftqq.com/
//! POST https://sctapi.ftqq.com/{key}.send
//! body: { title, desp }
//!
//! title 最多 32 字符，desp 支持 Markdown

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

use axagent_harness::{AlertPayload, AlertSeverity, NotificationChannel, ReportPayload};

/// Server酱渠道配置
#[derive(Debug, Clone)]
pub struct ServerChanConfig {
    /// SendKey（用户在 sct.ftqq.com 获取）
    pub key: String,
    /// 自定义 API 地址前缀（默认 https://sctapi.ftqq.com，可用于代理）
    pub api_base: Option<String>,
}

/// Server酱推送渠道
pub struct ServerChanChannel {
    config: ServerChanConfig,
    client: Client,
}

#[derive(Serialize)]
struct ServerChanRequest<'a> {
    title: &'a str,
    desp: String,
}

impl ServerChanChannel {
    pub fn new(config: ServerChanConfig) -> Self {
        Self { config, client: Client::new() }
    }

    pub fn with_client(config: ServerChanConfig, client: Client) -> Self {
        Self { config, client }
    }

    fn api_url(&self) -> String {
        let base = self.config.api_base.as_deref().unwrap_or("https://sctapi.ftqq.com");
        format!("{}/{}.send", base, self.config.key)
    }

    async fn send(&self, title: &str, desp: String) -> Result<String, String> {
        // Server酱 title 最多 32 字符
        let truncated_title = if title.chars().count() > 32 {
            let mut end = 0;
            for (i, _) in title.char_indices().take(29) {
                end = i;
            }
            format!("{}...", &title[..=end])
        } else {
            title.to_string()
        };
        let req = ServerChanRequest { title: &truncated_title, desp };
        let resp = self
            .client
            .post(self.api_url())
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Server酱请求失败: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Server酱 HTTP {status}: {body}"));
        }
        Ok(format!("serverchan-{}", truncated_title))
    }
}

#[async_trait]
impl NotificationChannel for ServerChanChannel {
    fn name(&self) -> &str {
        "serverchan"
    }

    fn display_name(&self) -> &str {
        "Server酱"
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        // Server酱 desp 支持 Markdown，用 body_md
        let desp = if payload.stocks.is_empty() {
            payload.body_md.clone()
        } else {
            let mut md = payload.body_md.clone();
            md.push_str("\n\n## 股票摘要\n");
            for s in &payload.stocks {
                md.push_str(&format!(
                    "- **{}**({}): {}  评分:{}  置信度:{:.2}\n",
                    s.stock_name, s.stock_code, s.action, s.score, s.confidence
                ));
            }
            md
        };
        self.send(&payload.title, desp).await
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        let severity_tag = match payload.severity {
            AlertSeverity::Info => "[INFO]",
            AlertSeverity::Warning => "[WARN]",
            AlertSeverity::Error => "[ERROR]",
            AlertSeverity::Critical => "[CRITICAL]",
        };
        let title = format!("{severity_tag} {}", payload.title);
        let desp = if let Some(code) = &payload.stock_code {
            format!("**{}** ({})\n\n{}", payload.title, code, payload.body)
        } else {
            payload.body.clone()
        };
        self.send(&title, desp).await
    }

    async fn is_ready(&self) -> bool {
        !self.config.key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_cfg(key: &str) -> ServerChanConfig {
        ServerChanConfig { key: key.to_string(), api_base: None }
    }

    #[test]
    fn test_name_and_display() {
        let ch = ServerChanChannel::new(mk_cfg("k"));
        assert_eq!(ch.name(), "serverchan");
        assert_eq!(ch.display_name(), "Server酱");
    }

    #[test]
    fn test_api_url_default() {
        let ch = ServerChanChannel::new(mk_cfg("SCT123"));
        assert_eq!(ch.api_url(), "https://sctapi.ftqq.com/SCT123.send");
    }

    #[test]
    fn test_api_url_custom_base() {
        let mut cfg = mk_cfg("k");
        cfg.api_base = Some("https://proxy.example.com".to_string());
        let ch = ServerChanChannel::new(cfg);
        assert_eq!(ch.api_url(), "https://proxy.example.com/k.send");
    }

    #[tokio::test]
    async fn test_is_ready() {
        assert!(ServerChanChannel::new(mk_cfg("abc")).is_ready().await);
        assert!(!ServerChanChannel::new(mk_cfg("")).is_ready().await);
    }

    #[tokio::test]
    async fn test_send_unreachable_returns_err() {
        let mut cfg = mk_cfg("abc");
        cfg.api_base = Some("http://127.0.0.1:1".to_string());
        let ch = ServerChanChannel::new(cfg);
        let res = ch.send("t", "b".to_string()).await;
        assert!(res.is_err());
    }
}
