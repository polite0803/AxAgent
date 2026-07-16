// SPDX-License-Identifier: AGPL-3.0-only

//! 出站推送渠道实现（借鉴 daily_stock_analysis 多渠道推送）
//!
//! 本模块提供 4 种 DSA 特有的单向推送渠道，均实现
//! [`axagent_harness::NotificationChannel`] trait：
//!
//! | 渠道 | 文件 | 用途 |
//! |------|------|------|
//! | PushPlus | [`pushplus::PushPlusChannel`] | 微信公众号推送（国内常用） |
//! | Server酱 | [`serverchan::ServerChanChannel`] | 微信推送（国内常用） |
//! | ntfy | [`ntfy::NtfyChannel`] | 自托管 / 公共 ntfy.sh 推送 |
//! | Gotify | [`gotify::GotifyChannel`] | 自托管 Gotify 推送 |
//!
//! # 与上游渠道的关系（避免重复）
//!
//! 上游已提供以下渠道能力，**本模块不重复实现**：
//! - `rt-workflow::EmailExecutor`：SMTP 邮件（基于 lettre）
//! - `rt-workflow::WebhookSendExecutor`：通用 Webhook（POST/PUT/PATCH/GET/DELETE + 凭证注入）
//! - `rt-workflow::NotificationExecutor`：Slack/WeCom/DingTalk/Feishu incoming webhook
//! - `rt-messaging::PlatformAdapter` × 8：Telegram/Discord/WeChat/Feishu/QQ/DingTalk/Slack/WhatsApp 双向 IM
//!
//! 上游 IM 平台适配为 `NotificationChannel` 的工作在 wiring 层完成（`src/init/`），
//! 因为 `notification` crate（implementor）不能依赖 `rt-messaging`（hybrid）。
//!
//! 本模块的 4 种渠道是 DSA 特有的单向轻量推送服务，上游不覆盖，
//! 且均为纯 HTTP 实现，不依赖任何 axagent-* crate。

pub mod gotify;
pub mod ntfy;
pub mod pushplus;
pub mod serverchan;

pub use gotify::{GotifyChannel, GotifyConfig};
pub use ntfy::{NtfyChannel, NtfyConfig};
pub use pushplus::{PushPlusChannel, PushPlusConfig};
pub use serverchan::{ServerChanChannel, ServerChanConfig};
