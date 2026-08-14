// SPDX-License-Identifier: AGPL-3.0-only

pub mod dingtalk;
pub mod discord;
pub mod feishu;
pub mod qq;
pub mod slack;
pub mod telegram;
pub mod wechat;
pub mod whatsapp;

use std::sync::Arc;
use std::sync::OnceLock;

use axagent_harness::PlatformMessageCallback;

/// 消息平台适配器 — 权威定义在 harness（platform.adapter 接缝契约）。
pub use axagent_harness::MessagePlatformAdapter;

static MESSAGE_CALLBACK: OnceLock<Arc<dyn PlatformMessageCallback>> = OnceLock::new();

pub fn set_message_callback(callback: Arc<dyn PlatformMessageCallback>) {
    let _ = MESSAGE_CALLBACK.set(callback);
}

pub fn get_message_callback() -> Option<Arc<dyn PlatformMessageCallback>> {
    MESSAGE_CALLBACK.get().cloned()
}
