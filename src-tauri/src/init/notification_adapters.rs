// SPDX-License-Identifier: AGPL-3.0-only

//! IM 平台适配器：把上游 `PlatformAdapter`（rt-messaging）包装为 `NotificationChannel`
//!
//! 上游 `rt-messaging` crate 已提供 8 个 IM 平台适配器
//! （telegram/discord/feishu/dingtalk/slack/wechat/qq/whatsapp），
//! 实现 `PlatformAdapter` trait（含 `send_message` 能力）。
//!
//! 本模块的 `PlatformNotificationChannel` 将其适配为出站推送渠道，
//! 注册到 `NotificationDispatcher` 后可按路由分发报告/告警。
//!
//! # 为什么在 wiring 层
//!
//! `notification` crate 是 implementor，`rt-messaging` 是 hybrid。
//! 根据架构铁律，implementor 不能依赖 hybrid，因此适配工作必须在 wiring 层完成。

use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::platform_config::PlatformConfig;
use axagent_harness::{AlertPayload, AlertSeverity, NotificationChannel, ReportPayload};
use axagent_rt_messaging::message_gateway::platforms::PlatformAdapter;

/// IM 平台推送适配器
///
/// 把上游 `PlatformAdapter` 的 `send_message` 能力适配为 `NotificationChannel`。
pub struct PlatformNotificationChannel {
    /// 平台标识（如 "telegram"、"feishu"）
    platform_name: String,
    /// 显示名称（如 "Telegram"、"飞书"）
    display_name: String,
    /// 上游平台适配器
    adapter: Arc<dyn PlatformAdapter>,
    /// 平台配置快照
    config: PlatformConfig,
    /// 推送目标 chat_id（如 Telegram chat_id、飞书 open_id）
    chat_id: String,
}

impl PlatformNotificationChannel {
    pub fn new(
        platform_name: impl Into<String>,
        display_name: impl Into<String>,
        adapter: Arc<dyn PlatformAdapter>,
        config: PlatformConfig,
        chat_id: impl Into<String>,
    ) -> Self {
        Self {
            platform_name: platform_name.into(),
            display_name: display_name.into(),
            adapter,
            config,
            chat_id: chat_id.into(),
        }
    }

    /// 格式化报告为 IM 消息文本
    fn format_report(payload: &ReportPayload) -> String {
        let mut text = format!("📊 *{}*\n\n", payload.title);
        if !payload.stocks.is_empty() {
            text.push_str("股票摘要:\n");
            for s in &payload.stocks {
                text.push_str(&format!(
                    "• {}({}): {} 评分:{} 置信度:{:.2}\n",
                    s.stock_name, s.stock_code, s.action, s.score, s.confidence
                ));
            }
            text.push('\n');
        }
        text.push_str(&payload.body_md);
        text
    }

    /// 格式化告警为 IM 消息文本
    fn format_alert(payload: &AlertPayload) -> (String, String) {
        let severity_tag = match payload.severity {
            AlertSeverity::Info => "ℹ️ [INFO]",
            AlertSeverity::Warning => "⚠️ [WARN]",
            AlertSeverity::Error => "🔴 [ERROR]",
            AlertSeverity::Critical => "🚨 [CRITICAL]",
        };
        let title = format!("{severity_tag} {}", payload.title);
        let body = if let Some(code) = &payload.stock_code {
            format!("{} ({})\n\n{}", payload.title, code, payload.body)
        } else {
            payload.body.clone()
        };
        (title, body)
    }
}

#[async_trait]
impl NotificationChannel for PlatformNotificationChannel {
    fn name(&self) -> &str {
        &self.platform_name
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        let text = Self::format_report(payload);
        self.adapter
            .send_message(&self.config, &self.chat_id, &text, Some("Markdown"))
            .await
            .map_err(|e| format!("{} 推送报告失败: {e}", self.platform_name))?;
        Ok(format!("{}-report-{}", self.platform_name, payload.title))
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        let (title, body) = Self::format_alert(payload);
        let text = format!("{title}\n\n{body}");
        self.adapter
            .send_message(&self.config, &self.chat_id, &text, Some("Markdown"))
            .await
            .map_err(|e| format!("{} 推送告警失败: {e}", self.platform_name))?;
        Ok(format!("{}-alert-{}", self.platform_name, payload.title))
    }

    async fn is_ready(&self) -> bool {
        self.adapter.is_connected().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axagent_harness::ReportStockSummary;
    use chrono::Utc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// 测试用的 mock PlatformAdapter
    struct MockPlatformAdapter {
        connected: AtomicBool,
        last_message: tokio::sync::Mutex<Option<String>>,
    }

    impl MockPlatformAdapter {
        fn new(connected: bool) -> Self {
            Self {
                connected: AtomicBool::new(connected),
                last_message: tokio::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl PlatformAdapter for MockPlatformAdapter {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn is_enabled(&self, _config: &PlatformConfig) -> bool {
            true
        }

        async fn start(&self, _config: &PlatformConfig) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn send_message(
            &self,
            _config: &PlatformConfig,
            _chat_id: &str,
            text: &str,
            _parse_mode: Option<&str>,
        ) -> anyhow::Result<()> {
            *self.last_message.lock().await = Some(text.to_string());
            Ok(())
        }
    }

    fn make_report() -> ReportPayload {
        ReportPayload {
            title: "测试报告".to_string(),
            body_md: "正文内容".to_string(),
            body_html: None,
            stocks: vec![ReportStockSummary {
                stock_code: "600519".to_string(),
                stock_name: "贵州茅台".to_string(),
                action: "增持".to_string(),
                score: 85,
                confidence: 0.82,
            }],
            generated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_name_and_display() {
        let adapter = Arc::new(MockPlatformAdapter::new(true));
        let ch = PlatformNotificationChannel::new(
            "telegram",
            "Telegram",
            adapter,
            PlatformConfig::default(),
            "123456",
        );
        assert_eq!(ch.name(), "telegram");
        assert_eq!(ch.display_name(), "Telegram");
    }

    #[tokio::test]
    async fn test_send_report_success() {
        let adapter = Arc::new(MockPlatformAdapter::new(true));
        let ch = PlatformNotificationChannel::new(
            "telegram",
            "Telegram",
            adapter.clone(),
            PlatformConfig::default(),
            "123456",
        );
        let result = ch.send_report(&make_report()).await;
        assert!(result.is_ok());
        // 验证消息已发送到 mock
        let last = adapter.last_message.lock().await;
        assert!(last.is_some());
        assert!(last.as_ref().unwrap().contains("测试报告"));
        assert!(last.as_ref().unwrap().contains("贵州茅台"));
    }

    #[tokio::test]
    async fn test_send_alert_success() {
        let adapter = Arc::new(MockPlatformAdapter::new(true));
        let ch = PlatformNotificationChannel::new(
            "feishu",
            "飞书",
            adapter.clone(),
            PlatformConfig::default(),
            "open_id_xxx",
        );
        let payload = AlertPayload {
            severity: AlertSeverity::Critical,
            title: "价格突破".to_string(),
            body: "触及止损线".to_string(),
            stock_code: Some("600519".to_string()),
            generated_at: Utc::now(),
        };
        let result = ch.send_alert(&payload).await;
        assert!(result.is_ok());
        let last = adapter.last_message.lock().await;
        assert!(last.as_ref().unwrap().contains("[CRITICAL]"));
        assert!(last.as_ref().unwrap().contains("价格突破"));
    }

    #[tokio::test]
    async fn test_is_ready_connected() {
        let adapter = Arc::new(MockPlatformAdapter::new(true));
        let ch =
            PlatformNotificationChannel::new("t", "T", adapter, PlatformConfig::default(), "c");
        assert!(ch.is_ready().await);
    }

    #[tokio::test]
    async fn test_is_ready_disconnected() {
        let adapter = Arc::new(MockPlatformAdapter::new(false));
        let ch =
            PlatformNotificationChannel::new("t", "T", adapter, PlatformConfig::default(), "c");
        assert!(!ch.is_ready().await);
    }

    #[test]
    fn test_format_report_includes_stocks() {
        let report = make_report();
        let text = PlatformNotificationChannel::format_report(&report);
        assert!(text.contains("测试报告"));
        assert!(text.contains("贵州茅台"));
        assert!(text.contains("600519"));
        assert!(text.contains("增持"));
    }

    #[test]
    fn test_format_alert_critical() {
        let payload = AlertPayload {
            severity: AlertSeverity::Critical,
            title: "测试".to_string(),
            body: "内容".to_string(),
            stock_code: Some("000001".to_string()),
            generated_at: Utc::now(),
        };
        let (title, body) = PlatformNotificationChannel::format_alert(&payload);
        assert!(title.contains("[CRITICAL]"));
        assert!(body.contains("000001"));
    }

    #[test]
    fn test_format_alert_info() {
        let payload = AlertPayload {
            severity: AlertSeverity::Info,
            title: "提示".to_string(),
            body: "内容".to_string(),
            stock_code: None,
            generated_at: Utc::now(),
        };
        let (title, _body) = PlatformNotificationChannel::format_alert(&payload);
        assert!(title.contains("[INFO]"));
    }
}
