// SPDX-License-Identifier: AGPL-3.0-only

//! 纯文本后处理（兼容层）。
//!
//! 实现已上移至 `axagent-harness::text_clean::clean_output`（foundation 层），
//! 本模块仅 re-export 保持 `axagent_runtime_core::clean_output` API 兼容。

pub use axagent_harness::clean_output;
