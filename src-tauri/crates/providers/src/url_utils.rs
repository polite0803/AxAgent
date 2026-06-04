//! 转发到 `axagent-harness::url_utils`，保留向后兼容。
//!
//! 迁移历史：URL 解析工具函数最初在 `axagent-harness`，后来迁到 `axagent-providers`，
//! 现已迁回 `axagent-harness`（harness 才是契约层）。本模块作为薄壳重新导出，
//! 老代码 `use axagent_providers::url_utils::*` 仍可工作。

pub use axagent_harness::url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};
