// SPDX-License-Identifier: AGPL-3.0-only

//! 用户画像模块 — re-export 自 harness
//!
//! 权威定义在 `axagent_harness::profile`，本模块通过 `pub use` 引用所有类型，
//! 保持 `crate::user_profile::X` 路径在 trajectory 内部可用（如 preference_learner.rs
//! 的 `crate::user_profile::ExpertiseArea` 等引用），无需修改下游代码。
//!
//! 这样做避免重复定义，符合 AGENTS.md 铁律 4「禁止重复类型体系」。
pub use axagent_harness::profile::*;
