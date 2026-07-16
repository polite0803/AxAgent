// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-notification — 出站推送通知实现层
//!
//! 借鉴 daily_stock_analysis 项目的多渠道推送能力，
//! 在 `axagent-stock-analysis::notification_channel` 契约之上实现：
//!
//! - [`dispatcher::NotificationDispatcher`]：核心分发器，负责
//!   渠道注册、路由配置、策略应用（去重 / 冷却 / 静默 / 级别过滤）
//!   以及并发分发 + 结果汇总。
//! - [`dispatcher::LogChannel`]：内置日志渠道（测试 / 兜底用）。
//! - [`channels`]：DSA 特有的 4 种单向推送渠道
//!   （PushPlus / Server酱 / ntfy / Gotify）。
//!
//! # Crate 归属
//!
//! 角色：**implementor**（依赖 `axagent-harness`）。
//! 不依赖 `entities` / `dao` / `kit` 等其他 implementor。
//!
//! # 与上游渠道的关系（避免重复）
//!
//! 上游已提供以下渠道能力，**本 crate 不重复实现**：
//! - `rt-workflow::EmailExecutor`：SMTP 邮件（基于 lettre）
//! - `rt-workflow::WebhookSendExecutor`：通用 Webhook（POST/PUT/PATCH/GET/DELETE + 凭证注入）
//! - `rt-workflow::NotificationExecutor`：Slack/WeCom/DingTalk/Feishu incoming webhook
//! - `rt-messaging::PlatformAdapter` × 8：Telegram/Discord/WeChat/Feishu/QQ/DingTalk/Slack/WhatsApp 双向 IM
//!
//! 由于本 crate 是 implementor、`rt-messaging` 是 hybrid，implementor 不能依赖 hybrid，
//! 因此 IM 平台适配为 `NotificationChannel` 的工作在 wiring 层（`src/init/`）完成。

pub mod channels;
pub mod dispatcher;

pub use dispatcher::{LogChannel, NotificationDispatcher};

// DSA 特有渠道 re-export
pub use channels::{
    GotifyChannel, GotifyConfig, NtfyChannel, NtfyConfig, PushPlusChannel, PushPlusConfig,
    ServerChanChannel, ServerChanConfig,
};

// 从 harness re-export 推送相关契约，方便调用方一站式 import
pub use axagent_harness::{
    AlertPayload, AlertSeverity, NotificationChannel, NotificationDispatchResult,
    NotificationDispatchSummary, NotificationPolicy, NotificationRoute, ReportPayload,
    ReportStockSummary, RouteConfig,
};
