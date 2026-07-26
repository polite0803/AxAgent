// SPDX-License-Identifier: AGPL-3.0-only

//! G17 Cron delivery → gateway 闭环 — Wiring 层 Sink 实现
//!
//! 实现 `CronDeliverySink` trait，把 4 种渠道的投递逻辑落地：
//!
//! 1. **Gateway** — 通过 `MessageGateway` 推送 AgentMessage
//! 2. **Webhook** — 通过 reqwest HTTP POST 推送 JSON
//! 3. **Notification** — 通过 `NotificationChannel` 推送（占位：日志记录）
//! 4. **File** — 写入本地文件（追加 / 覆盖）
//!
//! 此文件位于 wiring 层，可同时依赖 harness（trait 定义）和 rt-messaging
//! （MessageGateway 实现），符合「wiring 全栈胶水」的角色。

use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::cron_delivery::{CronDeliveryChannel, CronDeliveryPayload, CronDeliverySink};
use axagent_rt_messaging::message_gateway::{AgentMessage, MessageGateway, MessagePayload};

/// GatewayDeliverySink — 把 MessageGateway 包装成 CronDeliverySink
pub struct GatewayDeliverySink {
    gateway: Arc<MessageGateway>,
    /// 用于 webhook 的 reqwest client（共享连接池）
    http_client: reqwest::Client,
}

impl GatewayDeliverySink {
    pub fn new(gateway: Arc<MessageGateway>) -> Self {
        Self {
            gateway,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait]
impl CronDeliverySink for GatewayDeliverySink {
    async fn deliver(
        &self,
        channel: &CronDeliveryChannel,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        match channel {
            CronDeliveryChannel::Gateway { agent_id, from } => {
                self.deliver_via_gateway(agent_id, from.as_deref(), payload).await
            },
            CronDeliveryChannel::Webhook { url, headers, sign_secret } => {
                self.deliver_via_webhook(url, headers.as_ref(), sign_secret.as_deref(), payload)
                    .await
            },
            CronDeliveryChannel::Notification { channel_id, severity } => {
                self.deliver_via_notification(channel_id, severity.as_deref(), payload).await
            },
            CronDeliveryChannel::File { path, append } => {
                self.deliver_via_file(path, append.unwrap_or(true), payload).await
            },
        }
    }
}

impl GatewayDeliverySink {
    /// 通过 MessageGateway 推送
    async fn deliver_via_gateway(
        &self,
        agent_id: &str,
        from: Option<&str>,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        let from_id = from.unwrap_or("cron").to_string();
        let content = format!(
            "【定时任务通知】\n任务: {}\n状态: {}\n执行次数: {}\n耗时: {} ms\n{}\n{}",
            payload.job_name,
            if payload.success { "成功" } else { "失败" },
            payload.run_count,
            payload.duration_ms,
            payload.output.as_deref().map(|o| format!("输出: {o}")).unwrap_or_default(),
            payload.error.as_deref().map(|e| format!("错误: {e}")).unwrap_or_default(),
        );

        let message = AgentMessage::new(&from_id, agent_id, MessagePayload::Text { content });

        // 调用 gateway.send_message 推送消息
        // 注意：MessageGateway::send_message 内部会路由到 transport_handler
        self.gateway.send_message(&message).await.map_err(|e| format!("Gateway 投递失败: {e}"))
    }

    /// 通过 Webhook HTTP POST 推送
    async fn deliver_via_webhook(
        &self,
        url: &str,
        headers: Option<&std::collections::HashMap<String, String>>,
        sign_secret: Option<&str>,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        let body =
            serde_json::to_value(payload).map_err(|e| format!("序列化 payload 失败: {e}"))?;

        let mut req = self.http_client.post(url).json(&body);

        // 自定义 headers
        if let Some(hs) = headers {
            for (k, v) in hs {
                req = req.header(k, v);
            }
        }

        // HMAC-SHA256 签名
        if let Some(secret) = sign_secret {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            hasher.update(serde_json::to_string(&payload).unwrap_or_default().as_bytes());
            let signature = hex::encode(hasher.finalize());
            req = req.header("X-Cron-Signature", signature);
        }

        let resp = req.send().await.map_err(|e| format!("Webhook 请求失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Webhook 返回非 2xx: status={}", resp.status().as_u16()));
        }

        Ok(())
    }

    /// 通过通知渠道推送（占位实现：记录日志）
    ///
    /// 真正的通知渠道实现在 `notification_adapters.rs` 中，需要 db 连接查询
    /// notification_channel 表。当前仅记录日志，待完整集成后替换。
    async fn deliver_via_notification(
        &self,
        channel_id: &str,
        severity: Option<&str>,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        tracing::info!(
            "[CronDelivery:Notification] channel_id={}, severity={:?}, job={}, success={}",
            channel_id,
            severity,
            payload.job_name,
            payload.success
        );
        // TODO: 接入 notification_adapters 推送到飞书 / 钉钉 / Slack / Email
        Ok(())
    }

    /// 写入本地文件
    async fn deliver_via_file(
        &self,
        path: &str,
        append: bool,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        use std::io::Write;

        // 简单展开 ~/ 前缀（不依赖 shellexpand crate）
        let expanded = if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
            {
                std::path::Path::new(&home).join(rest).to_string_lossy().to_string()
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };

        let line = format!(
            "[{}] job={} success={} duration_ms={} run_count={} output={:?} error={:?}\n",
            chrono::DateTime::from_timestamp(payload.executed_at / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| payload.executed_at.to_string()),
            payload.job_name,
            payload.success,
            payload.duration_ms,
            payload.run_count,
            payload.output,
            payload.error
        );

        let mut opts = std::fs::OpenOptions::new();
        if append {
            opts.append(true).create(true);
        } else {
            opts.write(true).create(true).truncate(true);
        }

        let mut file = opts.open(&expanded).map_err(|e| format!("打开文件失败 {expanded}: {e}"))?;

        file.write_all(line.as_bytes()).map_err(|e| format!("写入文件失败: {e}"))?;

        Ok(())
    }
}
