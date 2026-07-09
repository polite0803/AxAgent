// SPDX-License-Identifier: AGPL-3.0-only

//! LLM Provider URL 解析工具函数 — re-export from harness。
//!
//! 权威实现位于 `axagent_harness::url_utils`。
//! 本文件保留 re-export 以兼容 `axagent_providers::url_utils::*` 的已有调用路径。

pub use axagent_harness::url_utils::*;
