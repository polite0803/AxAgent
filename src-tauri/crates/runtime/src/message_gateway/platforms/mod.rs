//! Re-exported from axagent-rt-messaging。
//!
//! 真正的 `MESSAGE_CALLBACK` static、setter/getter、PlatformAdapter trait
//! 都在 `axagent_rt_messaging::message_gateway::platforms`。
//! 本模块仅作 re-export 转发，避免双 crate 复制。

pub use axagent_rt_messaging::message_gateway::platforms::*;
