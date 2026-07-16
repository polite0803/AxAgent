// SPDX-License-Identifier: AGPL-3.0-only

//! PushPlus 渠道（微信公众号推送）
//!
//! API 文档：https://www.pushplus.plus/doc/
//! POST https://www.pushplus.plus/send
//! body: { token, title, content, template }
//!
//! template 默认 "html"，report 用 "html"（支持 body_html），alert 用 "txt"

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

use axagent_harness::{AlertPayload, AlertSeverity, NotificationChannel, ReportPayload};

/// PushPlus 渠道配置
#[derive(Debug, Clone)]
pub struct PushPlusConfig {
    /// PushPlus token（用户在 pushplus.plus 获取）
    pub token: String,
    /// 自定义 API 地址（默认 https://www.pushplus.plus/send，可用于代理）
    pub api_url: Option<String>,
    /// 默认 topic（群组推送时使用，可选）
    pub topic: Option<String>,
}

/// PushPlus 推送渠道
pub struct PushPlusChannel {
    config: PushPlusConfig,
    client: Client,
}

#[derive(Serialize)]
struct PushPlusRequest<'a> {
    token: &'a str,
    title: &'a str,
    content: String,
    template: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct PushPlusResponse {
    code: i32,
    msg: String,
}

impl PushPlusChannel {
    pub fn new(config: PushPlusConfig) -> Self {
        Self { config, client: Client::new() }
    }

    /// 用自定义 HTTP 客户端构造（测试 / 共享连接池时用）
    pub fn with_client(config: PushPlusConfig, client: Client) -> Self {
        Self { config, client }
    }

    fn api_url(&self) -> &str {
        self.config.api_url.as_deref().unwrap_or("https://www.pushplus.plus/send")
    }

    async fn send(&self, title: &str, content: String, template: &str) -> Result<String, String> {
        let req = PushPlusRequest {
            token: &self.config.token,
            title,
            content,
            template,
            topic: self.config.topic.as_deref(),
        };
        let resp = self
            .client
            .post(self.api_url())
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("PushPlus 请求失败: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("PushPlus HTTP {status}: {body}"));
        }
        let pr: PushPlusResponse =
            resp.json().await.map_err(|e| format!("PushPlus 响应解析失败: {e}"))?;
        if pr.code != 200 {
            return Err(format!("PushPlus 业务错误 code={}: {}", pr.code, pr.msg));
        }
        Ok(format!("pushplus-{}", pr.msg))
    }
}

#[async_trait]
impl NotificationChannel for PushPlusChannel {
    fn name(&self) -> &str {
        "pushplus"
    }

    fn display_name(&self) -> &str {
        "PushPlus"
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        let content = payload.body_html.clone().unwrap_or_else(|| payload.body_md.clone());
        self.send(&payload.title, content, "html").await
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        let severity_tag = match payload.severity {
            AlertSeverity::Info => "[INFO]",
            AlertSeverity::Warning => "[WARN]",
            AlertSeverity::Error => "[ERROR]",
            AlertSeverity::Critical => "[CRITICAL]",
        };
        let title = format!("{severity_tag} {}", payload.title);
        let body = if let Some(code) = &payload.stock_code {
            format!("{} ({})\n\n{}", payload.title, code, payload.body)
        } else {
            payload.body.clone()
        };
        self.send(&title, body, "txt").await
    }

    async fn is_ready(&self) -> bool {
        !self.config.token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_config(token: &str) -> PushPlusConfig {
        PushPlusConfig { token: token.to_string(), api_url: None, topic: None }
    }

    #[test]
    fn test_name_and_display() {
        let ch = PushPlusChannel::new(mk_config("tok"));
        assert_eq!(ch.name(), "pushplus");
        assert_eq!(ch.display_name(), "PushPlus");
    }

    #[test]
    fn test_api_url_default() {
        let ch = PushPlusChannel::new(mk_config("t"));
        assert_eq!(ch.api_url(), "https://www.pushplus.plus/send");
    }

    #[test]
    fn test_api_url_custom() {
        let mut cfg = mk_config("t");
        cfg.api_url = Some("https://proxy.example.com/send".to_string());
        let ch = PushPlusChannel::new(cfg);
        assert_eq!(ch.api_url(), "https://proxy.example.com/send");
    }

    #[tokio::test]
    async fn test_is_ready_non_empty_token() {
        let ch = PushPlusChannel::new(mk_config("abc"));
        assert!(ch.is_ready().await);
    }

    #[tokio::test]
    async fn test_is_ready_empty_token() {
        let ch = PushPlusChannel::new(mk_config(""));
        assert!(!ch.is_ready().await);
    }

    #[tokio::test]
    async fn test_send_unreachable_returns_err() {
        let mut cfg = mk_config("abc");
        cfg.api_url = Some("http://127.0.0.1:1/send".to_string());
        let ch = PushPlusChannel::new(cfg);
        let res = ch.send("t", "b".to_string(), "txt").await;
        assert!(res.is_err());
    }
}
