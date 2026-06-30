// SPDX-License-Identifier: AGPL-3.0-only

//! Core runtime primitives for the `claw` CLI and supporting crates.
//!
//! This crate owns session persistence, permission evaluation, prompt assembly,
//! MCP plumbing, tool-facing file operations, and the core conversation loop
//! that drives interactive and one-shot turns.
//!
//! ## ⚠️ 本 crate **不**做 `axagent_harness` 顶层 32 条 `pub use` 的兜底镜像
//!
//! 业务组件需要 harness 契约项（如 `Persistence`、`ProviderAdapter`、
//! `Tool`、`PromptGuard`、`StorageBackend` 等）时，请**直接**
//! `use axagent_harness::...`，不要假设 `use axagent_runtime::Persistence`
//! 之类的路径可用——这些符号在 `axagent_harness`，不在本 crate 透传范围。
//!
//! 依赖方向铁律：`业务组件 → harness ← 实现`。
//! 详见 [`axagent_harness`] crate 文档与 `harness/src/lib.rs` 顶部的架构说明。

pub mod adversarial_debate;
pub mod agent_roles;
pub mod api_docs;
pub mod api_server;
mod bash;
pub mod bash_validation;
pub mod benchmarks;
mod bootstrap;
pub mod branch_lock;
pub mod buddy;
pub mod collaboration;
pub mod cron;
pub mod dashboard_plugin;
pub mod dashboard_registry;
pub mod error_recovery;
mod file_ops;
mod git_context;
pub mod git_tools;
pub mod green_contract;
pub mod harness;
pub mod hook_config;
pub mod lan_transfer;
mod lane_events;
pub mod llm_bridge;
pub mod lsp_client;
pub mod lsp_process;
pub mod lsp_protocol;
mod mcp;
pub mod mcp_autostart;
mod mcp_client;
pub mod mcp_lifecycle_hardened;
pub mod mcp_server;
mod mcp_stdio;
pub mod mcp_tool_bridge;
pub mod message_gateway;
pub mod mode_selector;
pub mod module_switch;
mod oauth;
pub mod plugin_lifecycle;
mod policy_engine;
pub mod priority_scheduler;
pub mod profile;
pub mod profile_manager;
mod prompt;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod pty;
pub mod reactive_compact;
// recovery_recipes merged into error_recovery
mod remote;
pub mod resource_governor;
pub mod session_search;
pub mod shared_memory;
pub mod shell_completer;
pub mod shell_hooks;
pub mod stale_base;
pub mod stale_branch;
pub mod summary_compression;
pub mod task_manager;
pub mod task_packet;
pub mod task_registry;
pub mod team_cron_registry;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod terminal;
pub mod terminal_analyzer;
pub mod theme_engine;
pub mod token_budget_predictor;
pub mod tool_generator;
pub mod transform_pipeline;
pub mod transport_handlers;
mod util;
pub mod validation_executor;
pub mod webhook_subscription;
pub mod work_engine;
pub mod workflow_engine;

#[cfg(test)]
mod trust_resolver;
pub mod worker_boot;

pub use api_docs::{ApiDocGenerator, OpenApiSpec};
pub use axagent_runtime_core::*;
pub use bash::{BashCommandInput, BashCommandOutput, execute_bash};
pub use bootstrap::{BootstrapPhase, BootstrapPlan};
pub use branch_lock::{BranchLockCollision, BranchLockIntent, detect_branch_lock_collisions};

pub use file_ops::{
    EditFileOutput, GlobSearchOutput, GrepSearchInput, GrepSearchOutput, ReadFileOutput,
    StructuredPatchHunk, TextFilePayload, WriteFileOutput, edit_file, edit_file_in_workspace,
    glob_search, grep_search, is_symlink_escape, read_file, read_file_in_workspace,
    validate_workspace_boundary, write_file, write_file_in_workspace,
};
pub use git_context::{GitCommitEntry, GitContext};

pub use lane_events::{
    LaneCommitProvenance, LaneEvent, LaneEventBlocker, LaneEventName, LaneEventStatus,
    LaneFailureClass, dedupe_superseded_commit_events,
};
pub use mcp::{
    mcp_server_signature, mcp_tool_name, mcp_tool_prefix, normalize_name_for_mcp,
    scoped_mcp_config_hash, unwrap_ccr_proxy_url,
};
pub use mcp_client::{
    McpClientAuth, McpClientBootstrap, McpClientTransport, McpManagedProxyTransport,
    McpRemoteTransport, McpSdkTransport, McpStdioTransport,
};
pub use mcp_lifecycle_hardened::{
    McpDegradedReport, McpErrorSurface, McpFailedServer, McpLifecyclePhase, McpLifecycleState,
    McpLifecycleValidator, McpPhaseResult,
};
pub use mcp_server::{MCP_SERVER_PROTOCOL_VERSION, McpServer, McpServerSpec, ToolCallHandler};
pub use mcp_stdio::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, ManagedMcpTool, McpDiscoveryFailure,
    McpInitializeClientInfo, McpInitializeParams, McpInitializeResult, McpInitializeServerInfo,
    McpListResourcesParams, McpListResourcesResult, McpListToolsParams, McpListToolsResult,
    McpReadResourceParams, McpReadResourceResult, McpResource, McpResourceContents,
    McpServerManager, McpServerManagerError, McpStdioProcess, McpTool, McpToolCallContent,
    McpToolCallParams, McpToolCallResult, McpToolDiscoveryReport, UnsupportedMcpServer,
    spawn_mcp_stdio_process,
};
pub use oauth::{
    OAuthAuthorizationRequest, OAuthCallbackParams, OAuthRefreshRequest, OAuthTokenExchangeRequest,
    OAuthTokenSet, PkceChallengeMethod, PkceCodePair, clear_oauth_credentials, code_challenge_s256,
    credentials_path, generate_pkce_pair, generate_state, load_oauth_credentials,
    loopback_redirect_uri, parse_oauth_callback_query, parse_oauth_callback_request_target,
    save_oauth_credentials,
};

// ── Plugin Agent 桥接 ──（从 plugins crate 重导出）
pub use axagent_plugins::agent_provider::{
    PluginAgentDef, PluginAgentRegistry, global_plugin_agents,
};
pub use plugin_lifecycle::{
    DegradedMode, DiscoveryResult, PluginHealthcheck, PluginLifecycle, PluginLifecycleEvent,
    PluginState, ResourceInfo, ServerHealth, ServerStatus, ToolInfo,
};
pub use policy_engine::{
    DiffScope, GreenLevel, LaneBlocker, LaneContext, PolicyAction, PolicyCondition, PolicyEngine,
    PolicyRule, ReconcileReason, ReviewStatus, evaluate,
};
pub use priority_scheduler::{PriorityScheduler, ScheduledTask, SchedulerConfig, TaskPriority};
pub use prompt::{
    ContextFile, FRONTIER_MODEL_NAME, ProjectContext, PromptBuildError,
    SYSTEM_PROMPT_DYNAMIC_BOUNDARY, SystemPromptBuilder, TaskScene, load_system_prompt,
    prepend_bullets,
};

pub use error_recovery::{
    EscalationPolicy, FailureScenario, RecoveryContext, RecoveryEvent, RecoveryRecipe,
    RecoveryResult, RecoveryStep, attempt_recovery, recipe_for,
};
pub use reactive_compact::{
    ReactiveCompactResult, ReactiveTrigger, classify_trigger, is_context_overflow_error,
    is_media_size_error, try_reactive_compact,
};
pub use remote::{
    DEFAULT_REMOTE_BASE_URL, DEFAULT_SESSION_TOKEN_PATH, DEFAULT_SYSTEM_CA_BUNDLE, NO_PROXY_HOSTS,
    RemoteSessionContext, UPSTREAM_PROXY_ENV_KEYS, UpstreamProxyBootstrap, UpstreamProxyState,
    inherited_upstream_proxy_env, no_proxy_list, read_token, upstream_proxy_ws_url,
};

pub use session_search::{
    IndexedMessage, SearchQuery as RuntimeSearchQuery, SearchResult, SessionSearchEngine,
};
pub use stale_base::{
    BaseCommitSource, BaseCommitState, check_base_commit, format_stale_base_warning,
    read_claw_base_file, resolve_expected_base,
};
pub use stale_branch::{
    BranchFreshness, StaleBranchAction, StaleBranchEvent, StaleBranchPolicy, apply_policy,
    check_freshness,
};
pub use task_packet::{TaskPacket, TaskPacketValidationError, ValidatedPacket, validate_packet};
#[cfg(test)]
pub use trust_resolver::{TrustConfig, TrustDecision, TrustEvent, TrustPolicy, TrustResolver};

pub use worker_boot::{
    Worker, WorkerEvent, WorkerEventKind, WorkerEventPayload, WorkerFailure, WorkerFailureKind,
    WorkerPromptTarget, WorkerReadySnapshot, WorkerRegistry, WorkerStatus, WorkerTrustResolution,
};

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
