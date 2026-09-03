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

/// Re-exports from `axagent-cache` (热缓存层)
pub mod cache {
    pub use axagent_cache::*;
}

/// Re-exports from `axagent-crypto` (加密模块)
pub mod crypto {
    pub use axagent_crypto::*;
}

/// Re-exports from `axagent-disk-cache` (磁盘缓存层)
pub mod disk_cache {
    pub use axagent_disk_cache::*;
}

/// Re-exports from `axagent-document-parser` (文档解析)
pub mod document_parser {
    pub use axagent_document_parser::*;
}

/// Re-exports from `axagent-entities` (实体定义)
pub mod entities {
    pub use axagent_entities::*;
}

/// Re-exports from `axagent-gateway` (API 网关)
pub mod gateway {
    pub use axagent_gateway::*;
}

/// Re-exports from `axagent-kit` (工具套件)
pub mod kit {
    pub use axagent_kit::*;
}

/// Re-exports from `axagent-mcp` (MCP 协议)
pub mod mcp {
    pub use axagent_mcp::*;
}

/// Re-exports from `axagent-migration` (数据迁移)
pub mod migration {
    pub use axagent_migration::*;
}

/// Re-exports from `axagent-orchestrator` (编排器)
pub mod orchestrator {
    pub use axagent_orchestrator::*;
}

/// Re-exports from `axagent-plugins` (插件生命周期)
pub mod plugins {
    pub use axagent_plugins::*;
}

/// Re-exports from `axagent-prompt-guard` (提示词防护)
pub mod prompt_guard {
    pub use axagent_prompt_guard::*;
}

/// Re-exports from `axagent-providers` (LLM 提供商适配器)
pub mod providers {
    pub use axagent_providers::*;
}

/// Re-exports from `axagent-rt-dashboard` (运行态仪表盘)
pub mod rt_dashboard {
    pub use axagent_rt_dashboard::*;
}

/// Re-exports from `axagent-rt-messaging` (运行态消息)
pub mod rt_messaging {
    pub use axagent_rt_messaging::*;
}

/// Re-exports from `axagent-rt-theme` (运行态主题)
pub mod rt_theme {
    pub use axagent_rt_theme::*;
}

/// Re-exports from `axagent-rt-webhook` (运行态 Webhook)
pub mod rt_webhook {
    pub use axagent_rt_webhook::*;
}

/// Re-exports from `axagent-rt-workflow` (运行态工作流)
pub mod rt_workflow {
    pub use axagent_rt_workflow::*;
}

/// Re-exports from `axagent-runtime-core` (运行时核心)
pub mod runtime_core {
    pub use axagent_runtime_core::*;
}

/// Re-exports from `axagent-search` (搜索/RAG 引擎)
pub mod search {
    pub use axagent_search::*;
}

/// Re-exports from `axagent-storage` (文件存储层)
pub mod storage {
    pub use axagent_storage::*;
}

/// Re-exports from `axagent-telemetry` (遥测)
pub mod telemetry {
    pub use axagent_telemetry::*;
}

/// Re-exports from `axagent-tools` (统一工具接口)
pub mod tools {
    pub use axagent_tools::*;
}

/// Re-exports from `axagent-trajectory` (轨迹/学习/技能/画像)
pub mod trajectory {
    pub use axagent_trajectory::*;
}
