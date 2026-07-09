// SPDX-License-Identifier: AGPL-3.0-only

//! 动态上下文注入器 trait —— 从 harness 层 re-export。
//!
//! 原始定义位于 `axagent_harness::context_contributor`。

pub use axagent_harness::context_contributor::{ContextContributor, ContextRequest};
