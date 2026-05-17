//! Core runtime primitives extracted from axagent-runtime.
//!
//! This crate owns the foundational types that other crates (agent, tools)
//! depend on: Session, ConversationRuntime, Config, Hooks, Permissions, etc.
//!
//! axagent-runtime re-exports everything from this crate, so consumers that
//! import from `axagent_runtime` continue to work without changes.

pub mod cache_guard;
pub mod compact;
pub mod compact_thresholds;
pub mod compact_warning;
pub mod config;
pub mod config_validate;
pub mod conversation;
pub mod cron_job;
pub mod feature_flags;
pub mod fork_bridge;
pub mod hook_chain;
pub mod hooks;
pub mod json;
pub mod message_importance;
pub mod permission_enforcer;
pub mod permissions;
pub mod plugin_hooks;
pub mod prompt_cache;
pub mod sandbox;
pub mod session;
pub mod session_control;
pub mod session_memory_compact;
pub mod usage;

// ── Public Re-exports ────────────────────────────────────────────────

pub use cache_guard::CacheGuard;

pub use cron_job::{CronJob, CronJobStatus, CronJobStore, TaskConfig, TaskRunResult};

pub use compact::{
    adaptive_compaction_config, cleanup_task_boundary, compact_session, decay_weight,
    detect_task_boundary, estimate_message_tokens, estimate_session_tokens,
    evaluate_compact_threshold, format_compact_summary, get_compact_continuation_message,
    should_compact, smart_compact, summarize_turn, CompactionConfig, CompactionResult,
};

pub use compact_thresholds::{
    recommended_compaction_config, should_auto_compact, should_reactive_compact,
    AutoCompactTracking, CompactThresholdState, AUTOCOMPACT_BUFFER_TOKENS,
    ERROR_THRESHOLD_BUFFER_TOKENS, MANUAL_COMPACT_BUFFER_TOKENS,
    MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES, WARNING_THRESHOLD_BUFFER_TOKENS,
};

pub use compact_warning::{
    compute_warning_level, CompactWarning, CompactWarningState, WarningLevel,
    DEFAULT_SUPPRESSION_TTL_SECS, MIN_WARNING_INTERVAL_SECS,
};

pub use config::{
    ConfigEntry, ConfigError, ConfigLoader, ConfigSource, McpConfigCollection,
    McpManagedProxyServerConfig, McpOAuthConfig, McpRemoteServerConfig, McpSdkServerConfig,
    McpServerConfig, McpStdioServerConfig, McpTransport, McpWebSocketServerConfig, OAuthConfig,
    ProviderFallbackConfig, ResolvedPermissionMode, RuntimeConfig, RuntimeFeatureConfig,
    RuntimeHookConfig, RuntimePermissionRuleConfig, RuntimePluginConfig, ScopedMcpServerConfig,
    CLAW_SETTINGS_SCHEMA_NAME,
};

pub use conversation::{
    auto_compaction_threshold_from_env, ApiClient, ApiRequest, AssistantEvent, AutoCompactionEvent,
    ConversationRuntime, PromptCacheEvent, RuntimeError, StaticToolExecutor, ToolError,
    ToolErrorKind, ToolExecutor, TurnSummary,
};

pub use feature_flags::{
    global_feature_flags, init_global_feature_flags, FeatureFlagDef, FeatureFlags,
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
    build_linux_sandbox_command, detect_container_environment, detect_container_environment_from,
    resolve_sandbox_status, resolve_sandbox_status_for_request, ContainerEnvironment,
    FilesystemIsolationMode, LinuxSandboxCommand, SandboxConfig, SandboxDetectionInputs,
    SandboxRequest, SandboxStatus,
};

pub use session::{
    ContentBlock, ConversationMessage, MessageRole, Session, SessionCompaction, SessionError,
    SessionFork, SessionPromptEntry,
};

pub use usage::{
    format_usd, pricing_for_model, ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker,
};

pub use config_validate::{
    check_unsupported_format, format_diagnostics, validate_config_file, ConfigDiagnostic,
    DiagnosticKind, ValidationResult,
};

pub use session_memory_compact::{
    to_compaction_result, try_session_memory_compact, SessionMemoryCompactConfig,
    SessionMemoryCompactResult, StructuredMemory,
};

pub use session_control::SessionStore;

pub use message_importance::{score_message, select_top_messages};

pub use hook_chain::HookChain;

pub use plugin_hooks::{
    HookContext, HookDecision, LlmCallContext, LlmCallResult, PluginHook, SharedHook,
    ToolCallContext, ToolCallResult,
};
