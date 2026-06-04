//! axagent-harness — Harness 契约层
//!
//! 自底而上：本 crate 是 AxAgent 架构中最底层的非数据层，
//! 仅包含 trait 接口定义、纯数据 DTO、常量和错误类型。
//!
//! **零业务逻辑、零具体实现**。不依赖任何其他 axagent-* crate。
//!
//! 设计原则：
//! - 依赖方向：组件 → harness ← 实现
//! - 最小依赖：仅 serde、async-trait、chrono、uuid、sea-orm（re-export）
//! - 无运行时行为：所有实现都在下游 crate

// ── 共享数据类型 ──
pub mod constants;
pub mod core_error;
pub mod error_codes;
mod persistence_mod;
pub mod plan_types;
pub mod platform_config;
pub mod rag_config;
pub mod types;
pub mod util_fns;
pub mod workflow_types;

// ── Persistence 契约 ──
/// `Persistence` trait（实际定义在 `persistence_mod`）
pub use persistence_mod::{DatabaseConnection, Persistence, SharedPersistence};

// ── 共享错误类型 ──
/// `AxAgentError`（统一错误枚举）
pub use core_error::*;

// ── 共享常量 ──
pub use constants::*;

// ── 共享错误码 ──
pub use error_codes::*;

// ── 原有 Harness 模块 ──
pub mod context_builder;
pub mod error;
pub mod has_provider_registry;
pub mod inference_engine;
pub mod npm_registry;
pub mod persistence;
pub mod planner;
pub mod prompt_guard;
pub mod provider;
pub mod registry;
pub mod rhai_engine;
pub mod session_tracer;
pub mod storage_backend;
pub mod test_support;
pub mod tool;
pub mod trajectory_service;
pub mod url_utils;
// ── Webhook 契约 ──
pub mod webhook_subscription;
/// 关键 Webhook 类型重导出 — struct/enum 级
pub use webhook_subscription::{
    DispatchResult, NoopWebhookSubscriptionService, WebhookEvent, WebhookPayload,
    WebhookSubscription, WebhookSubscriptionInfo, WebhookSubscriptionService,
};
pub mod trajectory_types;

// ── Provider 契约重导出 ──
pub use context_builder::build_provider_request_context;
pub use has_provider_registry::HasProviderRegistry;
pub use provider::{ProviderAdapter, ProviderProxyConfig, ProviderRequestContext};
pub use url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};

// ── PromptGuard 契约重导出 ──
pub use prompt_guard::{NoopPromptGuard, PromptGuard};

// ── SessionTracer 契约重导出 ──
pub use session_tracer::{NoopSessionTracer, SessionTracer};

// ── NpmRegistry 契约重导出 ──
pub use npm_registry::{NoopNpmRegistryService, NpmRegistryService, parse_npm_package_spec};

// ── RhaiEngine 契约重导出 ──
pub use rhai_engine::{NoopRhaiEngineAdapter, RhaiEngineAdapter, RhaiToolFn};

// ── Planner 契约重导出 ──
pub use planner::{NoopPlannerAdapter, PlannerAdapter};

// ── TrajectoryService 契约重导出 ──
pub use trajectory_service::{
    IntegrityCheck, IntegrityResult, NoopTrajectoryService, TaskComplexity, TrajectoryService,
};

// ── Tool 契约重导出 ──
pub use tool::{
    PermissionResult, ProgressEntry, Tool, ToolCategory, ToolContext, ToolInfo, ToolResult,
    parse_tool_name,
};

// ── Registry 契约重导出 ──
pub use registry::ToolRegistry;

// ── StorageBackend 契约 ──
pub use storage_backend::{ListResult, StorageBackend, StorageObject, StorageObjectMeta};

// ── InferenceEngine 契约 ──
pub use inference_engine::InferenceEngine;

// ── Error 重导出 ──
pub use error::{ToolError, ToolErrorKind};
