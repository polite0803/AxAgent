//! Core runtime primitives for the `claw` CLI and supporting crates.
//!
//! This crate owns session persistence, permission evaluation, prompt assembly,
//! MCP plumbing, tool-facing file operations, and the core conversation loop
//! that drives interactive and one-shot turns.

#![allow(clippy::too_many_arguments)]
#![allow(clippy::await_holding_lock)]
#![allow(clippy::wrong_self_convention)]

pub mod adversarial_debate;
pub mod agent_orchestrator;
pub mod agent_roles;
mod bash;
pub mod bash_validation;
pub mod benchmarks;
mod bootstrap;
pub mod branch_lock;
pub mod cache_guard;
pub mod collaboration;
mod compact;
pub mod compact_thresholds;
pub mod compact_warning;
mod config;
pub mod config_validate;
mod conversation;
pub mod cron;
pub mod dashboard_plugin;
pub mod dashboard_registry;
pub mod engine_bridge;
mod file_ops;
pub mod general_engine;
mod git_context;
pub mod git_tools;
pub mod green_contract;
mod hook_chain;
pub mod hook_config;
mod hooks;
mod json;
pub mod lan_transfer;
mod lane_events;
pub mod lsp_client;
pub mod lsp_process;
pub mod lsp_protocol;
mod mcp;
mod mcp_client;
pub mod mcp_lifecycle_hardened;
pub mod mcp_server;
mod mcp_stdio;
pub mod mcp_tool_bridge;
pub mod message_gateway;
pub mod mode_selector;
pub mod module_switch;
mod oauth;
pub mod permission_enforcer;
mod permissions;
pub mod plugin_hooks;
pub mod plugin_lifecycle;
mod policy_engine;
pub mod profile;
pub mod profile_manager;
mod prompt;
pub mod prompt_cache;
pub mod pty;
pub mod reactive_compact;
pub mod recovery_recipes;
mod remote;
pub mod resource_governor;
pub mod sandbox;
mod session;
pub mod session_control;
pub mod shared_memory;
pub mod shell_hooks;
pub mod task_router;
pub mod terminal_analyzer;
pub mod tool_generator;
pub mod transform_pipeline;
pub mod transport_handlers;
pub mod validation_executor;
pub mod webhook_dispatcher;
pub mod webhook_server;
pub mod webhook_subscription;
pub mod work_engine;
pub mod workflow_engine;
pub use session_control::SessionStore;
pub mod session_memory_compact;
pub mod session_search;
pub mod shell_completer;
mod sse;
pub mod stale_base;
pub mod stale_branch;
pub mod summary_compression;
pub mod task_packet;
pub mod task_registry;
pub mod team_cron_registry;
pub mod terminal;
pub mod theme_engine;

#[cfg(test)]
mod trust_resolver;
mod usage;
pub mod worker_boot;

pub use bash::{execute_bash, BashCommandInput, BashCommandOutput};
pub use bootstrap::{BootstrapPhase, BootstrapPlan};
pub use branch_lock::{detect_branch_lock_collisions, BranchLockCollision, BranchLockIntent};
pub use cache_guard::CacheGuard;
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
pub use config_validate::{
    check_unsupported_format, format_diagnostics, validate_config_file, ConfigDiagnostic,
    DiagnosticKind, ValidationResult,
};
pub use conversation::{
    auto_compaction_threshold_from_env, ApiClient, ApiRequest, AssistantEvent, AutoCompactionEvent,
    ConversationRuntime, PromptCacheEvent, RuntimeError, StaticToolExecutor, ToolError,
    ToolExecutor, TurnSummary,
};
pub use file_ops::{
    edit_file, glob_search, grep_search, read_file, write_file, EditFileOutput, GlobSearchOutput,
    GrepSearchInput, GrepSearchOutput, ReadFileOutput, StructuredPatchHunk, TextFilePayload,
    WriteFileOutput,
};
pub use git_context::{GitCommitEntry, GitContext};
pub use hook_chain::HookChain;
pub use hooks::{
    HookAbortSignal, HookEvent, HookProgressEvent, HookProgressReporter, HookRunResult, HookRunner,
};
pub use lane_events::{
    dedupe_superseded_commit_events, LaneCommitProvenance, LaneEvent, LaneEventBlocker,
    LaneEventName, LaneEventStatus, LaneFailureClass,
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
pub use mcp_server::{McpServer, McpServerSpec, ToolCallHandler, MCP_SERVER_PROTOCOL_VERSION};
pub use mcp_stdio::{
    spawn_mcp_stdio_process, JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse,
    ManagedMcpTool, McpDiscoveryFailure, McpInitializeClientInfo, McpInitializeParams,
    McpInitializeResult, McpInitializeServerInfo, McpListResourcesParams, McpListResourcesResult,
    McpListToolsParams, McpListToolsResult, McpReadResourceParams, McpReadResourceResult,
    McpResource, McpResourceContents, McpServerManager, McpServerManagerError, McpStdioProcess,
    McpTool, McpToolCallContent, McpToolCallParams, McpToolCallResult, McpToolDiscoveryReport,
    UnsupportedMcpServer,
};
pub use oauth::{
    clear_oauth_credentials, code_challenge_s256, credentials_path, generate_pkce_pair,
    generate_state, load_oauth_credentials, loopback_redirect_uri, parse_oauth_callback_query,
    parse_oauth_callback_request_target, save_oauth_credentials, OAuthAuthorizationRequest,
    OAuthCallbackParams, OAuthRefreshRequest, OAuthTokenExchangeRequest, OAuthTokenSet,
    PkceChallengeMethod, PkceCodePair,
};
pub use permissions::{
    PermissionContext, PermissionMode, PermissionOutcome, PermissionOverride, PermissionPolicy,
    PermissionPromptDecision, PermissionPrompter, PermissionRequest,
};
pub use plugin_hooks::{
    HookContext, HookDecision, LlmCallContext, LlmCallResult, PluginHook, SharedHook,
    ToolCallContext, ToolCallResult,
};
pub use plugin_lifecycle::{
    DegradedMode, DiscoveryResult, PluginHealthcheck, PluginLifecycle, PluginLifecycleEvent,
    PluginState, ResourceInfo, ServerHealth, ServerStatus, ToolInfo,
};
pub use policy_engine::{
    evaluate, DiffScope, GreenLevel, LaneBlocker, LaneContext, PolicyAction, PolicyCondition,
    PolicyEngine, PolicyRule, ReconcileReason, ReviewStatus,
};
pub use prompt::{
    load_system_prompt, prepend_bullets, ContextFile, ProjectContext, PromptBuildError,
    SystemPromptBuilder, TaskScene, FRONTIER_MODEL_NAME, SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
};
pub use prompt_cache::{
    CacheBreakSummary, CacheReadEvent, PendingChange, PromptCache, PromptCacheState,
};
pub use reactive_compact::{
    classify_trigger, is_context_overflow_error, is_media_size_error, try_reactive_compact,
    ReactiveCompactResult, ReactiveTrigger,
};
pub use recovery_recipes::{
    attempt_recovery, recipe_for, EscalationPolicy, FailureScenario, RecoveryContext,
    RecoveryEvent, RecoveryRecipe, RecoveryResult, RecoveryStep,
};
pub use remote::{
    inherited_upstream_proxy_env, no_proxy_list, read_token, upstream_proxy_ws_url,
    RemoteSessionContext, UpstreamProxyBootstrap, UpstreamProxyState, DEFAULT_REMOTE_BASE_URL,
    DEFAULT_SESSION_TOKEN_PATH, DEFAULT_SYSTEM_CA_BUNDLE, NO_PROXY_HOSTS, UPSTREAM_PROXY_ENV_KEYS,
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
pub use session_memory_compact::{
    try_session_memory_compact, to_compaction_result, SessionMemoryCompactConfig,
    SessionMemoryCompactResult, StructuredMemory,
};
pub use session_search::{
    IndexedMessage, SearchQuery as RuntimeSearchQuery, SearchResult, SessionSearchEngine,
};
pub use sse::{IncrementalSseParser, SseEvent};
pub use stale_base::{
    check_base_commit, format_stale_base_warning, read_claw_base_file, resolve_expected_base,
    BaseCommitSource, BaseCommitState,
};
pub use stale_branch::{
    apply_policy, check_freshness, BranchFreshness, StaleBranchAction, StaleBranchEvent,
    StaleBranchPolicy,
};
pub use task_packet::{validate_packet, TaskPacket, TaskPacketValidationError, ValidatedPacket};
#[cfg(test)]
pub use trust_resolver::{TrustConfig, TrustDecision, TrustEvent, TrustPolicy, TrustResolver};
pub use usage::{
    format_usd, pricing_for_model, ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker,
};
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
