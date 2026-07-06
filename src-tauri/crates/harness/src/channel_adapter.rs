// SPDX-License-Identifier: AGPL-3.0-only

//! 消息渠道适配器 trait（参考 nomifun-tauri 的 ChannelAdapter 设计）。
//!
//! 定义统一的消息渠道抽象，具体实现（Telegram、Discord 等）通过此 trait
//! 注册到 PlatformManager，实现跨平台消息收发。

use async_trait::async_trait;

/// 收到的消息。
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// 渠道内部用户/对话标识
    pub channel_id: String,
    /// 消息文本内容
    pub text: String,
    /// 发送者用户名（如有）
    pub sender_name: Option<String>,
}

/// 消息渠道适配器 trait。
///
/// 每个 IM 平台实现此 trait 后注册到 `PlatformManager`。
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// 渠道标识名称（如 "telegram"、"discord"）。
    fn name(&self) -> &str;

    /// 向指定渠道发送消息。
    async fn send_message(&self, channel_id: &str, message: &str) -> Result<(), String>;

    /// 发送"正在输入"状态。
    async fn send_typing(&self, channel_id: &str) -> Result<(), String> {
        let _ = channel_id;
        Ok(())
    }
}
