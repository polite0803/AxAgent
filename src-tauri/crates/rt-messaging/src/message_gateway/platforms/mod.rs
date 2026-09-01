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

use axagent_harness::PlatformMessageCallback;

/// 消息平台适配器 — 权威定义在 harness（platform.adapter 接缝契约）。
pub use axagent_harness::MessagePlatformAdapter;

/// 取当前消息回调 —— 统一经 message.callback 能力接缝获取。
///
/// wiring 层在启动时把 PlatformBridge 注册进能力注册表；外部插件可经
/// `register_external_message_callback` 替换同一接缝（内置与插件平权）。
/// 此前本模块持有独立的 `OnceLock` 静态副本（平行通道），已收敛删除。
pub fn get_message_callback() -> Option<Arc<dyn PlatformMessageCallback>> {
    axagent_harness::get_capability_registry().get_message_callback()
}
