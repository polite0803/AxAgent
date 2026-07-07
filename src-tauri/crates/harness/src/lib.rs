// SPDX-License-Identifier: AGPL-3.0-only

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

// ── 国际化 ──
pub mod i18n;
pub use i18n::{I18nKey, Locale, fmt_msg, fmt_msg_with, msg};

// ── 共享数据类型 ──
pub mod audit_trail;
pub use audit_trail::{AuditEntry, AuditRecorder};
pub mod cache_interceptor;
pub use cache_interceptor::{HarnessCache, LlmCacheKey};
pub mod confidence;
pub mod credential;
pub use confidence::{ConfidenceAction, ConfidenceConfig, ConfidenceOutput};
pub mod channel_adapter;
pub use channel_adapter::*;
pub mod constants;
pub mod contracts;
pub use contracts::HarnessToolExecutor;
pub mod conversation_model;
pub mod core_error;
pub mod error_codes;
mod persistence_mod;
pub mod plan_types;
pub mod platform_config;
pub mod rag_config;
pub mod types;
pub mod url_utils;
pub mod util_fns;
pub mod workflow_types;
#[macro_use]
pub mod reliability;

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

// ── 序列化/反序列化 Schema 校验 ──
pub mod serialization;

// ── Harness 约束修复模块 ──
pub mod consistency_check;
pub mod hallucination_guard;

// ── 原有 Harness 模块 ──
pub mod business_rules;
pub use business_rules::{BusinessRule, BusinessRuleEngine, RuleResult};
pub mod context_builder;
pub mod error;
pub mod has_provider_registry;
pub mod inference_engine;
pub mod model_knowledge;
pub use model_knowledge::ModelKnowledgeProvider;
pub mod npm_registry;
pub mod persistence;
pub mod planner;
pub mod plugin_hook;
pub use plugin_hook::{
    HookContext, HookDecision, LlmCallContext, LlmCallResult, PluginHook, SharedHook,
    ToolCallContext, ToolCallResult,
};
pub mod prompt_guard;
pub mod provider;
pub mod registry;
pub mod rhai_engine;
pub mod session_tracer;
pub mod storage_backend;
pub mod test_support;
pub mod tool;
pub mod trajectory_service;
// ── Webhook 契约 ──
pub mod webhook_subscription;
/// 关键 Webhook 类型重导出 — struct/enum 级
pub use webhook_subscription::{
    DispatchResult, NoopWebhookSubscriptionService, WebhookEvent, WebhookPayload,
    WebhookSubscription, WebhookSubscriptionInfo, WebhookSubscriptionService,
};

// ── 消息平台 Webhook 契约 ──
pub mod messaging_webhook;
pub use messaging_webhook::{WeChatWebhookHandler, WhatsAppWebhookHandler};

// ── 迁移相关 ──
pub mod migration_types;
pub use migration_types::{
    BackupInfo, DetectedPlatform, MigrationEntry, MigrationItem, MigrationReport,
};

// ── 工具扩展契约 ──
pub mod tools_ext;
pub use tools_ext::{MigrationRunner, PluginAgentDescriptor, PluginAgentProvider};

// ── 搜索层数据源 trait（让 search crate 不依赖 dao / document-parser） ──
pub mod search_sources;
pub use search_sources::{
    DocumentParser, KnowledgeSource, MemorySource, SettingsSource, WikiSource,
};

// ── Marketplace 契约（让 gateway / kit 不依赖 dao / entities） ──
pub mod llm_execution;
pub use llm_execution::{
    LlmCallConfig as HarnessLlmCallConfig, LlmCallResult as HarnessLlmCallResult,
    LlmExecutionService, NoopLlmExecutionService, SharedLlmExecutionService,
};
pub mod marketplace;
pub use marketplace::{
    CreateReviewRequest, MarketplaceService, MarketplaceStats, ReviewResponse, UpdateReviewRequest,
};

// ── Gateway 平台层 trait（让 gateway crate 不依赖 dao / crypto） ──
pub mod platform_adapter;
pub use platform_adapter::{
    CryptoService, GatewayKeyRepository, GatewayRequestLogRepository, PlatformAdapter,
    ProviderRepository, SettingsRepository,
};

// ── 路径编解码 trait（让 dao crate 不依赖 storage） ──
pub mod path_vars;
pub use path_vars::PathEncoder;

// ── MCP 共享类型（让 dao crate 不依赖 mcp） ──
pub mod mcp_types;
pub use mcp_types::DiscoveredTool;

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
    DefaultInputSanitizer, DefaultOutputSanitizer, InputSanitizer, NoopOutputSanitizer,
    OutputSanitizer, PermissionResult, ProgressEntry, SanitizeContext, Tool, ToolCategory,
    ToolContext, ToolInfo, ToolPermissions, ToolResult, parse_tool_name,
};

// ── Registry 契约重导出 ──
pub use registry::ToolRegistry;

// ── StorageBackend 契约 ──
pub use storage_backend::{ListResult, StorageBackend, StorageObject, StorageObjectMeta};

// ── 约束检查重导出 ──
pub use consistency_check::{
    ConsistencyCheckConfig, ConsistencyMode, ConsistencyResult, check_consistency,
};
pub use hallucination_guard::{AnchorResult, HallucinationGuardConfig, check_anchor};

// ── InferenceEngine 契约 ──
pub use inference_engine::InferenceEngine;

// ── Error 重导出 ──
pub use error::{ToolError, ToolErrorKind};

// ── 统一拦截器链 ──
pub mod interceptor;
pub use interceptor::{
    BusinessRuleInterceptor, ConsistencyCheckInterceptor, HarnessInterceptor, InterceptPoint,
    InterceptorChain, InterceptorContext, InterceptorResult, OutputValidationInterceptor,
    PromptGuardInterceptor,
};

// ── PromptProvider 契约（让 runtime-core 不依赖 kit） ──
pub mod prompt_provider;
pub use prompt_provider::{NoopPromptProvider, PromptLang, PromptProvider, StaticPromptProvider};

// ── CacheService 契约 ──
pub mod cache_service;
pub use cache_service::{CacheService, SharedCacheService};

// ── HookService 契约 ──
pub mod hook_service;
pub use hook_service::{HookService, SharedHookService};

// ── FeatureFlagProvider 契约 ──
pub mod feature_flag_provider;
pub use feature_flag_provider::{FeatureFlagProvider, SharedFeatureFlagProvider};

// ── MemoryStore 契约（记忆外溢/共享，让 gateway 不依赖 trajectory） ──
pub mod memory;
pub use memory::{
    MemoryActionResultDto, MemoryAddRequest, MemoryFeedbackRequest, MemoryGroupedDto,
    MemorySearchItem, MemorySearchRequest, MemoryStore, MemoryTreeItem, MemoryUpdateRequest,
    NoopMemoryStore,
};

// ── SchemaValidator 契约（JSON Schema 校验，让 agent/trajectory 不依赖 kit） ──
pub mod schema_validator;
pub use schema_validator::{validate_against_schema, validate_recursive};

// ── MemoryScanner 契约（本地日历/消息扫描，让 scanner crate 不依赖 harness 下游） ──
pub mod scanner;
pub use scanner::{
    MemoryScanner, NoopMemoryScanner, ScanResult, ScannedItem, ScannerConfig,
};

// ── BrowserController 契约（浏览器自动化，让 tools/gateway 不依赖 kit） ──
pub mod browser;
pub use browser::{
    BrowserController, BrowserNavigateResult, BrowserScreenshotResult, ExtractedElement,
    NoopBrowserController,
};

// ── Agent 契约（统一 agent 接口，让 coordinator/coordinator 不依赖 agent crate） ──
pub mod agent;
pub use agent::{
    Agent, AgentCapability, AgentExecuteRequest, AgentInfo, AgentPlan, AgentRegistry,
    AgentResult, NoopAgent, PlanStep,
};

// ── MCP Service 契约（让 tools/gateway 不依赖 mcp crate） ──
pub mod mcp_service;
pub use mcp_service::{
    DiscoveredMcpTool, McpClientService, McpServerConfig, McpServerStore, McpToolCallResult,
    NoopMcpClientService, NoopMcpServerStore,
};

// ── ApiClient 契约（让 runtime-core 不依赖 kit） ──
pub mod api_client;
pub mod compaction_service;
pub mod context_contributor;
pub mod cron_types;
pub mod execution_progress;
pub mod fork_bridge;
pub mod fork_service;
pub mod graph_analysis;
pub mod hook_event_dispatcher;
pub mod hooks;
pub mod html_cleaner;
pub mod markdown_parser;
pub mod messaging_service;
pub use messaging_service::ConversationInfo;
pub mod permission_enforcer;
pub mod permissions;
pub mod plan_compiler;
pub mod repo_dtos;
pub mod repositories;
pub mod runtime_instance;
pub mod screen_vision;
pub mod secure_store;
pub mod slash_command;
pub mod token_budget;
pub mod token_counter;
pub mod workflow_repository;

// ── 自学习金字塔（P6）：RL / Dream / Profile / Style 契约 ──
pub mod rl;
pub mod dream;
pub mod profile;
pub mod style;
pub use rl::*;
pub use dream::*;
pub use profile::*;
pub use style::*;

// ── 知识 RAG / 图谱 / 索引（P7）：RAGProvider / EntityGraphProvider / DocumentIndexer 契约 ──
pub mod rag_provider;
pub mod knowledge_graph;
pub mod indexer;
pub use rag_provider::*;
pub use knowledge_graph::*;
pub use indexer::*;

// ── Gateway + 消息平台契约（P8）──
pub mod gateway_service;
pub mod platform_manager;
pub use gateway_service::*;
pub use platform_manager::*;

// ── 工具生态 + 安全防护层（P9）──
pub mod rate_limiter;
pub mod ssrf_guard;
pub mod content_filter;
pub mod tool_metrics;
pub use rate_limiter::*;
pub use ssrf_guard::*;
pub use content_filter::*;
pub use tool_metrics::*;

// ── P9 续：熔断器 + 工具访问控制 ──
pub mod circuit_breaker;
pub mod tool_access;
pub use circuit_breaker::*;
pub use tool_access::*;

// ── 开发者体验（P10）：可观测性 / 基准测试 / 开发者工具 ──
pub mod observability;
pub mod benchmark;
pub mod dev_experience;
pub use observability::*;
pub use benchmark::*;
pub use dev_experience::*;
