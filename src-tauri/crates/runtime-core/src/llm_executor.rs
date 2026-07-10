// SPDX-License-Identifier: AGPL-3.0-only

//! 兼容垫片：LLM 执行边界已上移至 `axagent-harness`（铁律 4：共享类型权威在 harness）。
//!
//! 保留本模块仅为了不破坏 runtime-core 内部的 `crate::llm_executor::*` 引用
//! 以及 `lib.rs` 的 `pub use llm_executor::{LlmCallConfig, execute_llm, execute_llm_stream}` 重导出。

pub use axagent_harness::llm_executor::*;
