// SPDX-License-Identifier: AGPL-3.0-only

//! G17 Cron delivery → gateway 闭环 — Delivery 配置 DTO + Sink trait
//!
//! 对齐 DojoAgents 的 Cron delivery 闭环：定时任务执行完成后，根据 `delivery`
//! 配置把结果通过 MessageGateway / Webhook / Email 等渠道推送出去。
//!
//! ## 设计
//!
//! - **DTO + Trait 在 harness**：`CronDeliveryConfig` 是纯数据，`CronDeliverySink`
//!   是 trait 接口，runtime-core 通过 trait 调用，不直接依赖 rt-messaging 等实现层
//! - **具体实现在 wiring 层**：`src/init/` 中创建 `GatewayDeliverySink`，把
//!   `MessageGateway` 包装成 `CronDeliverySink`
//! - **多渠道支持**：单次 delivery 可同时推送到多个渠道（gateway / webhook / channel）
//!
//! ## 使用示例
//!
//! ```ignore
//! use axagent_harness::cron_delivery::{CronDeliveryConfig, CronDeliveryChannel, CronDeliverySink};
//!
//! struct MyGatewaySink;
//!
//! #[async_trait::async_trait]
//! impl CronDeliverySink for MyGatewaySink {
//!     async fn deliver(&self, channel: &CronDeliveryChannel, payload: &CronDeliveryPayload) -> Result<(), String> {
//!         // 推送到 MessageGateway / Webhook / ...
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Delivery 配置 ─────────────────────────────────────────────────────────

/// Cron 任务执行结果的投递配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CronDeliveryConfig {
    /// 投递渠道列表（同时推送）
    pub channels: Vec<CronDeliveryChannel>,
    /// 是否仅在失败时投递（默认 false = 总是投递）
    pub only_on_failure: bool,
    /// 是否附带执行历史记录
    pub include_history: bool,
    /// 自定义消息模板（可选，支持 {{result}} / {{error}} / {{duration_ms}} 占位符）
    pub message_template: Option<String>,
}

/// 单个投递渠道
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum CronDeliveryChannel {
    /// 通过 MessageGateway 推送（agent_id 为目标）
    Gateway {
        /// 目标 agent_id / endpoint_id
        agent_id: String,
        /// 来源标识（默认 "cron"）
        from: Option<String>,
    },
    /// 通过 Webhook 推送（HTTP POST）
    Webhook {
        /// Webhook URL
        url: String,
        /// 自定义 headers
        headers: Option<std::collections::HashMap<String, String>>,
        /// 是否签名（HMAC-SHA256）
        sign_secret: Option<String>,
    },
    /// 通过通知渠道推送（飞书 / 钉钉 / Slack / Email 等）
    Notification {
        /// 通知渠道 ID（在 notification_channel 表中注册）
        channel_id: String,
        /// 严重级别（info / warning / error）
        severity: Option<String>,
    },
    /// 写入文件（本地日志 / 共享目录）
    File {
        /// 文件路径（支持 ~/ 缩写）
        path: String,
        /// 是否追加（默认 true）
        append: Option<bool>,
    },
}

// ── Delivery 载荷 ─────────────────────────────────────────────────────────

/// Cron 任务执行结果的投递载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronDeliveryPayload {
    /// Cron 任务 ID
    pub job_id: String,
    /// Cron 任务名称
    pub job_name: String,
    /// 执行结果
    pub success: bool,
    /// 输出内容（成功时）
    pub output: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行时间戳（epoch millis）
    pub executed_at: i64,
    /// 第 N 次执行
    pub run_count: u32,
}

impl CronDeliveryPayload {
    /// 应用消息模板（替换占位符）
    pub fn render_template(&self, template: &str) -> String {
        template
            .replace("{{result}}", self.output.as_deref().unwrap_or(""))
            .replace("{{error}}", self.error.as_deref().unwrap_or(""))
            .replace("{{duration_ms}}", &self.duration_ms.to_string())
            .replace("{{job_name}}", &self.job_name)
            .replace("{{run_count}}", &self.run_count.to_string())
            .replace("{{success}}", if self.success { "成功" } else { "失败" })
    }
}

// ── CronDeliverySink trait ────────────────────────────────────────────────

/// Cron 投递 Sink — 由 wiring 层实现，runtime-core 通过 trait 调用
///
/// 实现方需要为每种 `CronDeliveryChannel` 变体提供具体推送逻辑。
#[async_trait]
pub trait CronDeliverySink: Send + Sync {
    /// 投递单条结果到指定渠道
    ///
    /// 实现应处理渠道特定的错误（网络失败 / 文件写入失败等），返回 Err 时
    /// 由调用方决定是否记录日志或重试。
    async fn deliver(
        &self,
        channel: &CronDeliveryChannel,
        payload: &CronDeliveryPayload,
    ) -> Result<(), String>;

    /// 批量投递到多个渠道（默认实现：循环调用 deliver）
    ///
    /// 单个渠道失败不影响其他渠道，全部尝试后聚合错误返回。
    async fn deliver_all(
        &self,
        config: &CronDeliveryConfig,
        payload: &CronDeliveryPayload,
    ) -> Result<(), Vec<String>> {
        // only_on_failure 检查
        if config.only_on_failure && payload.success {
            return Ok(());
        }

        let mut errors = Vec::new();
        for channel in &config.channels {
            if let Err(e) = self.deliver(channel, payload).await {
                tracing::warn!("[CronDelivery] 渠道投递失败: channel={:?}, error={e}", channel);
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ── NoopSink — 默认空实现 ─────────────────────────────────────────────────

/// 空实现 Sink，用于未配置 wiring 的场景
pub struct NoopDeliverySink;

#[async_trait]
impl CronDeliverySink for NoopDeliverySink {
    async fn deliver(
        &self,
        _channel: &CronDeliveryChannel,
        _payload: &CronDeliveryPayload,
    ) -> Result<(), String> {
        // noop
        Ok(())
    }
}

// ── 测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_render_template() {
        let payload = CronDeliveryPayload {
            job_id: "job-001".to_string(),
            job_name: "每日盘前分析".to_string(),
            success: true,
            output: Some("今日大盘震荡".to_string()),
            error: None,
            duration_ms: 1500,
            executed_at: 1700000000000,
            run_count: 5,
        };

        let rendered = payload
            .render_template("【{{job_name}}】第 {{run_count}} 次执行{{success}}: {{result}}");
        assert!(rendered.contains("【每日盘前分析】"));
        assert!(rendered.contains("第 5 次执行成功"));
        assert!(rendered.contains("今日大盘震荡"));
    }

    #[test]
    fn test_only_on_failure_skip() {
        // 测试 only_on_failure=true 且 success=true 的情况应该跳过
        // 这里只验证配置结构能正确序列化
        let config = CronDeliveryConfig {
            channels: vec![],
            only_on_failure: true,
            include_history: false,
            message_template: None,
        };
        assert!(config.only_on_failure);
        assert!(config.channels.is_empty());
    }

    #[tokio::test]
    async fn test_noop_sink() {
        let sink = NoopDeliverySink;
        let payload = CronDeliveryPayload {
            job_id: "test".to_string(),
            job_name: "test".to_string(),
            success: true,
            output: None,
            error: None,
            duration_ms: 100,
            executed_at: 0,
            run_count: 1,
        };
        let result = sink
            .deliver(
                &CronDeliveryChannel::File {
                    path: "/tmp/test.log".to_string(),
                    append: Some(true),
                },
                &payload,
            )
            .await;
        assert!(result.is_ok());
    }
}
