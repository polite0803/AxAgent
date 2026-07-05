// SPDX-License-Identifier: AGPL-3.0-only

//! Domain crate re-export facades.
//!
//! These modules re-export types from implementation crates so that the
//! root Tauri app (`axagent`) does NOT need direct Cargo.toml dependencies
//! on domain implementation crates. All commands access domain types
//! through `axagent_runtime::domain::*`.
//!
//! Dependency chain:  commands → runtime::domain → agent/trajectory/…
//!                                                                   ↓
//!                                                              harness (contract)

/// Re-exports from `axagent-agent` (智能体引擎)
pub mod agent {
    pub use axagent_agent::*;
}

/// Re-exports from `axagent-trajectory` (轨迹/学习/技能/画像)
pub mod trajectory {
    pub use axagent_trajectory::*;
}

/// Re-exports from `axagent-plugins` (插件生命周期)
pub mod plugins {
    pub use axagent_plugins::*;
}

/// Re-exports from `axagent-providers` (LLM 提供商适配器)
pub mod providers {
    pub use axagent_providers::*;
}
