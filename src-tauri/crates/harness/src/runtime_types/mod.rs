// SPDX-License-Identifier: AGPL-3.0-only

//! 运行时核心类型 — 原 `axagent-runtime-core` 的类型定义物理搬迁至此。
//!
//! 这些类型定义位于 harness 层，使得 consumer crate（如 `axagent-agent`）
//! 无需直接依赖 `axagent-runtime-core`。runtime-core 通过
//! `pub use axagent_harness::runtime_types::*` 保持向后兼容。

pub mod capability_gap;
pub mod compact;
pub mod conversation;
pub mod execution_progress;
pub mod fork_bridge;
pub mod hooks;
pub mod permission_enforcer;
pub mod permissions;
pub mod runtime_mutation;
pub mod session;
