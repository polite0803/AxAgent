//! axagent-harness — Harness 契约层
//!
//! 本 crate 是 AxAgent Harness 架构的核心：
//! 只包含 trait 接口定义和纯数据 DTO（数据传输对象），
//! **零业务逻辑、零具体实现**。
//!
//! 设计原则：
//! - 依赖方向：组件 → harness ← 实现。组件（如 agent）只依赖 harness 的 trait，
//!   不依赖其他组件的具体实现 crate。
//! - 最小依赖：仅依赖 `axagent-core`（核心类型）、`async-trait`、`serde`、`futures`、`tokio`。
//! - 无运行时行为：所有实现都在下游 crate（`axagent-providers`、`axagent-tools` 等）。

pub mod error;
pub mod provider;
pub mod registry;
pub mod tool;

// ── Provider 契约重导出 ──
pub use provider::{
    ProviderAdapter, ProviderProxyConfig, ProviderRequestContext, default_version_for_type,
    resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};

// ── Tool 契约重导出 ──
pub use tool::{
    PermissionResult, ProgressEntry, Tool, ToolCategory, ToolContext, ToolInfo, ToolResult,
    parse_tool_name,
};

// ── Registry 契约重导出 ──
pub use registry::ToolRegistry;
// ProviderRegistry trait 通过 axagent_harness::registry::ProviderRegistry 路径访问

// ── Error 重导出 ──
pub use error::{ToolError, ToolErrorKind};
