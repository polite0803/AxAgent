// SPDX-License-Identifier: AGPL-3.0-only

//! 出站推送通知渠道契约 —— re-export 自 `axagent-harness`。
//!
//! 归属修正（2026-07-16）：本模块为**通用出站推送契约（trait + 纯 DTO）**，
//! 与 harness 的 `channel_adapter`（IM 双向通道契约）对称，已迁回 harness 契约层。
//! 通用推送实现（notification crate）与投资域实现（本 crate）均从 harness 引用，
//! 实现依赖倒置、互不绑定。
//!
//! 本文件保留为薄 re-export，维持 `axagent_analysis_engine::notification_channel::*`
//! 路径的向后兼容；权威定义见 `axagent_harness::notification_channel`。

pub use axagent_harness::notification_channel::*;
