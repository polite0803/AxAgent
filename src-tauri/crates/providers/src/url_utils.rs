// SPDX-License-Identifier: AGPL-3.0-only

//! LLM Provider URL 解析工具函数 — 转发至 harness（实际定义）
//!
//! 由 `axagent-harness` 提供实际定义，本文件仅做 re-export 转发。
//! 外部 crate 优先使用 `axagent_harness::url_utils::*`。

pub use axagent_harness::url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};
