// SPDX-License-Identifier: AGPL-3.0-only

//! 消息平台回调契约 — 「一切皆插件」message.callback 接缝的权威定义。
//!
//! `PlatformMessageCallback` 是消息平台入站消息的统一处理契约：
//! 平台适配器（telegram / discord / wechat / ...）收到消息后调用
//! [`PlatformMessageCallback::on_message`]，把消息交给 Agent 世界处理并取回回复。
//!
//! 权威定义在此（harness，foundation），具体实现（如 `PlatformBridge`）位于
//! `axagent-rt-messaging`（hybrid）。wiring 层将内置实现注册到能力注册表的
//! `message.callback` 接缝，外部插件可经 `register_external_message_callback`
//! 替换同一接缝（内置与插件平权）。

use async_trait::async_trait;

/// 消息平台入站消息回调。
///
/// 实现方：`axagent_rt_messaging::message_gateway::platform_bridge::PlatformBridge`。
/// 调用方：各平台适配器（平台收到消息后调用）。
#[async_trait]
pub trait PlatformMessageCallback: Send + Sync + std::any::Any {
    /// 处理一条入站消息，返回回复文本（可为 `None` 表示不回复）。
    async fn on_message(
        &self,
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        chat_id: &str,
        text: &str,
    ) -> Option<String>;

    /// 保存平台消息去重游标（适配器每次处理后调用）。
    async fn save_cursor(&self, platform: &str, cursor: i64);
}
