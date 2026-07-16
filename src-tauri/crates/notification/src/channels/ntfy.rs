// SPDX-License-Identifier: AGPL-3.0-only

//! ntfy 渠道（自托管 / 公共 ntfy.sh 推送）
//!
//! API 文档：https://docs.ntfy.sh/
//! POST https://ntfy.sh/{topic}
//! body: 纯文本（Headers: Title, Priority, Tags）
//!
//! ntfy 是轻量 pub-sub 通知服务，支持自托管或使用公共 ntfy.sh

use async_trait::async_trait;
use reqwest::{Client, header};

use axagent_harness::{AlertPayload, AlertSeverity, NotificationChannel, ReportPayload};

/// ntfy 渠道配置
#[derive(Debug, Clone)]
pub struct NtfyConfig {
    /// ntfy 服务器地址（默认 https://ntfy.sh，自托管填 https://your-host.com）
    pub server_url: String,
    /// topic 名称（订阅端需订阅同名 topic）
    pub topic: String,
    /// 认证 token（可选，自托管服务可能需要）
    pub token: Option<String>,
    /// 默认优先级（1-5，5 最高）
    pub default_priority: Option<u8>,
}

/// ntfy 推送渠道
pub struct NtfyChannel {
    config: NtfyConfig,
    client: Client,
}

impl NtfyChannel {
    pub fn new(config: NtfyConfig) -> Self {
        Self { config, client: Client::new() }
    }

    pub fn with_client(config: NtfyConfig, client: Client) -> Self {
        Self { config, client }
    }

    fn api_url(&self) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        format!("{}/{}", base, self.config.topic)
    }

    /// severity → ntfy priority（1=min, 5=max, 3=default）
    fn severity_to_priority(severity: AlertSeverity) -> u8 {
        match severity {
            AlertSeverity::Info => 1,
            AlertSeverity::Warning => 3,
            AlertSeverity::Error => 4,
            AlertSeverity::Critical => 5,
        }
    }

    fn severity_to_tag(severity: AlertSeverity) -> &'static str {
        match severity {
            AlertSeverity::Info => "information_source",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Error => "rotating_light",
            AlertSeverity::Critical => "rotating_light",
        }
    }

    async fn send(
        &self,
        title: &str,
        message: String,
        priority: u8,
        tags: Option<&str>,
    ) -> Result<String, String> {
        let mut req = self
            .client
            .post(self.api_url())
            .header("Title", title)
            .header("Priority", priority.to_string());
        if let Some(t) = tags {
            req = req.header("Tags", t);
        }
        if let Some(ref token) = self.config.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let resp = req.body(message).send().await.map_err(|e| format!("ntfy 请求失败: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("ntfy HTTP {status}: {body}"));
        }
        Ok(format!("ntfy-{}-{}", self.config.topic, title))
    }
}

#[async_trait]
impl NotificationChannel for NtfyChannel {
    fn name(&self) -> &str {
        "ntfy"
    }

    fn display_name(&self) -> &str {
        "ntfy"
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        let priority = self.config.default_priority.unwrap_or(3);
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
        self.send(&payload.title, msg, priority, None).await
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        let priority = NtfyChannel::severity_to_priority(payload.severity);
        let tag = NtfyChannel::severity_to_tag(payload.severity);
        let msg = if let Some(code) = &payload.stock_code {
            format!("{} ({})\n\n{}", payload.title, code, payload.body)
        } else {
            payload.body.clone()
        };
        self.send(&payload.title, msg, priority, Some(tag)).await
    }

    async fn is_ready(&self) -> bool {
        !self.config.topic.is_empty() && !self.config.server_url.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_cfg(topic: &str) -> NtfyConfig {
        NtfyConfig {
            server_url: "https://ntfy.sh".to_string(),
            topic: topic.to_string(),
            token: None,
            default_priority: None,
        }
    }

    #[test]
    fn test_name_and_display() {
        let ch = NtfyChannel::new(mk_cfg("t"));
        assert_eq!(ch.name(), "ntfy");
        assert_eq!(ch.display_name(), "ntfy");
    }

    #[test]
    fn test_api_url() {
        let ch = NtfyChannel::new(mk_cfg("mytopic"));
        assert_eq!(ch.api_url(), "https://ntfy.sh/mytopic");
    }

    #[test]
    fn test_api_url_trailing_slash() {
        let mut cfg = mk_cfg("t");
        cfg.server_url = "https://ntfy.sh/".to_string();
        let ch = NtfyChannel::new(cfg);
        assert_eq!(ch.api_url(), "https://ntfy.sh/t");
    }

    #[test]
    fn test_severity_to_priority() {
        assert_eq!(NtfyChannel::severity_to_priority(AlertSeverity::Info), 1);
        assert_eq!(NtfyChannel::severity_to_priority(AlertSeverity::Warning), 3);
        assert_eq!(NtfyChannel::severity_to_priority(AlertSeverity::Error), 4);
        assert_eq!(NtfyChannel::severity_to_priority(AlertSeverity::Critical), 5);
    }

    #[tokio::test]
    async fn test_is_ready() {
        assert!(NtfyChannel::new(mk_cfg("abc")).is_ready().await);
        let mut cfg = mk_cfg("");
        cfg.server_url = "".to_string();
        assert!(!NtfyChannel::new(cfg).is_ready().await);
    }

    #[tokio::test]
    async fn test_send_unreachable_returns_err() {
        let mut cfg = mk_cfg("abc");
        cfg.server_url = "http://127.0.0.1:1".to_string();
        let ch = NtfyChannel::new(cfg);
        let res = ch.send("t", "b".to_string(), 3, None).await;
        assert!(res.is_err());
    }
}
