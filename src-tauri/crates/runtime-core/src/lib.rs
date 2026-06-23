// SPDX-License-Identifier: AGPL-3.0-only

//! Core runtime primitives extracted from axagent-runtime.
//!
//! This crate owns the foundational types that other crates (agent, tools)
//! depend on: Session, ConversationRuntime, Config, Hooks, Permissions, etc.
//!
//! axagent-runtime re-exports everything from this crate, so consumers that
//! import from `axagent_runtime` continue to work without changes.
//!
//! ## ⚠️ 本 crate **不**做 `axagent_harness` 顶层 32 条 `pub use` 的镜像
//!
//! 业务组件需要 harness 契约项（如 `Persistence`、`ProviderAdapter`、
//! `Tool`、`PromptGuard`、`StorageBackend` 等）时，请**直接**
//! `use axagent_harness::...`，不要假设 `use axagent_runtime_core::Persistence`
//! 之类的路径可用——这些符号在 `axagent_harness`，不在本 crate 透传范围。
//!
//! ### 例外：`MessageRole` 单向引用 `harness::types::MessageRole`
//!
//! 本 crate 的 `session::MessageRole` 通过
//! `pub use axagent_harness::types::MessageRole;` 单向指向 harness 权威源，
//! 仅用于消除与 `axagent_harness::MessageRole` 的同名异类。
//! 这是**唯一**一条 harness 顶层项的引用，其余契约项请走 `axagent_harness::...`。
//!
//! 依赖方向铁律：`业务组件 → harness ← 实现`。

pub mod balance;
pub mod cache_guard;
pub mod compact;
pub mod compact_thresholds;
pub mod compact_warning;
pub mod config;
pub mod config_validate;
pub mod conversation;
pub mod cron_job;
pub mod execution_progress;
pub mod feature_flags;
pub mod fork_bridge;
pub mod hook_chain;
pub mod hooks;
pub mod json;
pub mod message_importance;
pub mod permission_enforcer;
pub mod retry_policy;
pub use retry_policy::{BackoffStrategy, FallbackStrategy, RetryPolicy};
pub mod llm_executor;
pub use llm_executor::{LlmCallConfig, execute_llm};
pub mod permissions;
pub mod plugin_hooks;
pub mod prompt_cache;
pub mod sandbox;
pub mod session;
pub mod session_control;
pub mod session_memory_compact;
pub mod usage;

// ── Public Re-exports ────────────────────────────────────────────────

pub use balance::{Balance, BalanceError, BalanceInfo, fetch_deepseek_balance};

pub use cache_guard::CacheGuard;

pub use cron_job::{CronJob, CronJobStatus, CronJobStore, TaskConfig, TaskRunResult};

pub use compact::{
    CompactionConfig, CompactionResult, adaptive_compaction_config, cleanup_task_boundary,
    compact_session, decay_weight, detect_task_boundary, estimate_message_tokens,
    estimate_session_tokens, evaluate_compact_threshold, format_compact_summary,
    get_compact_continuation_message, should_compact, smart_compact, summarize_turn,
};

pub use compact_thresholds::{
    AUTOCOMPACT_BUFFER_TOKENS, AutoCompactTracking, CompactThresholdState,
    ERROR_THRESHOLD_BUFFER_TOKENS, MANUAL_COMPACT_BUFFER_TOKENS,
    MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES, WARNING_THRESHOLD_BUFFER_TOKENS,
    recommended_compaction_config, should_auto_compact, should_reactive_compact,
};

pub use compact_warning::{
    CompactWarning, CompactWarningState, DEFAULT_SUPPRESSION_TTL_SECS, MIN_WARNING_INTERVAL_SECS,
    WarningLevel, compute_warning_level,
};

pub use config::{
    CLAW_SETTINGS_SCHEMA_NAME, ConfigEntry, ConfigError, ConfigLoader, ConfigSource,
    McpConfigCollection, McpManagedProxyServerConfig, McpOAuthConfig, McpRemoteServerConfig,
    McpSdkServerConfig, McpServerConfig, McpStdioServerConfig, McpTransport,
    McpWebSocketServerConfig, OAuthConfig, ProviderFallbackConfig, ResolvedPermissionMode,
    RuntimeConfig, RuntimeFeatureConfig, RuntimeHookConfig, RuntimePermissionRuleConfig,
    RuntimePluginConfig, ScopedMcpServerConfig,
};

pub use conversation::{
    ApiClient, ApiRequest, AssistantEvent, AutoCompactionEvent, ConversationRuntime,
    PromptCacheEvent, RuntimeError, StaticToolExecutor, ToolError, ToolErrorKind, ToolExecutor,
    TurnSummary, auto_compaction_threshold_from_env,
};

pub use execution_progress::{
    AgentExecutionProgress, AgentExecutionProgressSnapshot, ToolExecutionRecord,
};

pub use feature_flags::{
    FeatureFlagDef, FeatureFlags, global_feature_flags, init_global_feature_flags,
};

pub use hooks::{
    HookAbortSignal, HookEvent, HookProgressEvent, HookProgressReporter, HookRunResult, HookRunner,
};

pub use permissions::{
    PermissionContext, PermissionMode, PermissionOutcome, PermissionOverride, PermissionPolicy,
    PermissionPromptDecision, PermissionPrompter, PermissionRequest,
};

pub use prompt_cache::{
    CacheBreakSummary, CacheReadEvent, PendingChange, PromptCache, PromptCacheState,
};

pub use sandbox::{
    ContainerEnvironment, FilesystemIsolationMode, LinuxSandboxCommand, MacosSandboxCommand,
    SandboxConfig, SandboxDetectionInputs, SandboxRequest, SandboxStatus, SeccompStatus,
    WindowsSandboxCommand, apply_seccomp_filter, build_linux_sandbox_command,
    build_macos_sandbox_command, build_windows_sandbox_command, detect_container_environment,
    detect_container_environment_from, resolve_sandbox_status, resolve_sandbox_status_for_request,
    seccomp_status_description,
};

pub use session::{
    ContentBlock, ConversationMessage, MessageRole, Session, SessionCompaction, SessionError,
    SessionFork, SessionPromptEntry,
};

pub use usage::{
    ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker, format_usd, pricing_for_model,
};

pub use config_validate::{
    ConfigDiagnostic, DiagnosticKind, ValidationResult, check_unsupported_format,
    format_diagnostics, validate_config_file,
};

pub use session_memory_compact::{
    SessionMemoryCompactConfig, SessionMemoryCompactResult, StructuredMemory, to_compaction_result,
    try_session_memory_compact,
};

pub use session_control::SessionStore;

pub use message_importance::{score_message, select_top_messages};

pub use hook_chain::HookChain;

pub use plugin_hooks::{
    HookContext, HookDecision, LlmCallContext, LlmCallResult, PluginHook, SharedHook,
    ToolCallContext, ToolCallResult,
};
