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
pub use confidence::{ConfidenceAction, ConfidenceConfig, ConfidenceOutput};
pub mod channel_adapter;
pub mod constants;
pub mod contracts;
pub use contracts::HarnessToolExecutor;
pub mod conversation_model;
pub use conversation_model::{ContentBlock, ConversationMessage, SessionInfo, TokenUsage};
pub mod core_error;
pub mod error_codes;
mod persistence_mod;
pub mod plan_types;
pub mod platform_config;
pub mod rag_config;
pub mod types;
pub mod url_utils;
pub mod util_fns;
pub mod workflow_node_deserializer;
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

// ── JSON Schema 校验（权威实现）──
pub mod json_schema;

// ── 序列化/反序列化 Schema 校验 ──
pub mod serialization;

// ── 工具系统模块 ──
pub mod output_sanitizer;
pub mod tool;
pub mod tool_permissions;
pub mod tool_validation;

// ── 依赖注入容器 ──
pub mod graph_dtos;
pub mod louvain_dtos;
pub mod note_dtos;
pub mod page_type;
pub mod repo_dtos;
pub mod repositories;
pub mod service_registry;
pub mod wiki_dtos;

// ── Harness 约束修复模块 ──
pub mod consistency_check;
pub mod hallucination_guard;

// ── 原有 Harness 模块 ──
pub mod business_rules;
pub use business_rules::{
    BusinessRule, BusinessRuleEvaluator, RuleAction, RuleEvaluationOutcome, RuleResult,
};
pub mod context_builder;
pub mod context_contributor;
pub use context_contributor::{ContextContributor, ContextRequest};
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
pub mod trajectory_service;
// ── Webhook 契约 ──
pub mod webhook_subscription;
/// 关键 Webhook 类型重导出 — struct/enum 级
pub use webhook_subscription::{
    DispatchResult, WebhookEvent, WebhookPayload, WebhookSubscription, WebhookSubscriptionInfo,
    WebhookSubscriptionService,
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
pub use llm_execution::{LlmExecutionService, SharedLlmExecutionService};

// ── LLM 执行边界（原 runtime-core，上移至 harness 以满足铁律 4 共享类型权威） ──
pub mod retry_policy;
pub use retry_policy::{BackoffStrategy, FallbackStrategy, RetryPolicy};
pub mod llm_executor;
pub use llm_executor::{LlmCallConfig, LlmUsage, execute_llm, execute_llm_stream};
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

pub mod trajectory_scorer;
pub mod trajectory_types;

// ── Provider 契约重导出 ──
pub use context_builder::build_provider_request_context;
pub use has_provider_registry::HasProviderRegistry;
pub use provider::{ProviderAdapter, ProviderProxyConfig, ProviderRequestContext};
pub use url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};

// ── PromptGuard 契约重导出 ──
pub use prompt_guard::PromptGuard;

// ── SessionTracer 契约重导出 ──
pub use session_tracer::SessionTracer;

// ── NpmRegistry 契约重导出 ──
pub use npm_registry::{NpmRegistryService, parse_npm_package_spec};

// ── RhaiEngine 契约重导出 ──
pub use rhai_engine::{RhaiEngineAdapter, RhaiToolFn};

// ── Planner 契约重导出 ──
pub use planner::PlannerAdapter;

// ── TrajectoryService 契约重导出 ──
pub use trajectory_service::{IntegrityCheck, IntegrityResult, TaskComplexity, TrajectoryService};

// ── Tool 契约重导出 ──
pub use tool::{
    DefaultInputSanitizer, DefaultOutputSanitizer, InputSanitizer, OutputSanitizer,
    PermissionResult, ProgressEntry, SanitizeContext, Tool, ToolCategory, ToolContext, ToolInfo,
    ToolPermissions, ToolResult, parse_tool_name,
};

// ── Registry 契约重导出 ──
pub use registry::ToolRegistry;

// ── ToolExecutionAudit 契约（让 tools crate 不依赖 dao） ──
pub mod tool_audit;
pub use tool_audit::ToolExecutionAudit;

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
    HarnessInterceptor, InterceptPoint, InterceptorChain, InterceptorContext, InterceptorResult,
};

// ── PromptProvider 契约（让 runtime-core 不依赖 kit） ──
pub mod prompt_provider;
pub use prompt_provider::{PromptLang, PromptProvider, StaticPromptProvider};

// ── AgentSession 持久化契约（让 agent 不依赖 dao） ──
pub mod agent_session_repo;
pub use agent_session_repo::AgentSessionRepository;

pub mod runtime_types;

pub mod kit_bridge;

pub mod cache_service;
pub use cache_service::{CacheService, SharedCacheService};

// ── HookService 契约 ──
pub mod hook_service;
pub use hook_service::{HookService, SharedHookService};

// ── FeatureFlagProvider 契约 ──
pub mod feature_flag_provider;
pub use feature_flag_provider::{FeatureFlagProvider, SharedFeatureFlagProvider};

// ── P1: MemoryStore 契约（记忆外溢/共享） ──
pub mod memory;
pub use memory::{
    MemoryActionResultDto, MemoryAddRequest, MemoryFeedbackRequest, MemoryGroupedDto,
    MemorySearchItem, MemorySearchRequest, MemoryStore, MemoryTreeItem, MemoryUpdateRequest,
};

// ── P2: MemoryScanner 契约（本地日历/文件扫描） ──
pub mod scanner;
pub use scanner::{MemoryScanner, ScanResult, ScannedItem, ScannerConfig};

// ── P3: BrowserController 契约（浏览器自动化） ──
pub mod browser;
pub use browser::{
    BrowserController, BrowserNavigateResult, BrowserScreenshotResult, ExtractedElement,
};

// ── P5: Agent 契约（统一 agent 接口 + 注册表） ──
pub mod agent;
pub use agent::{
    Agent, AgentCapability, AgentExecuteRequest, AgentInfo, AgentPlan, AgentRegistry, AgentResult,
    PlanStep,
};

// ── P6: 自学习系统契约 ──
pub mod rl;
pub use rl::{
    RLConfig, RLEngine, RLTrainer, RewardWeights, TrainingEpisode, TrainingReport, TrainingStep,
};
pub mod dream;
pub use dream::{
    ConsolidationDataProvider, DistilledKnowledge, DreamConsolidationConfig,
    DreamConsolidationResult, DreamConsolidator, ExperienceRecord, ReplaySample,
};
pub mod profile;
pub use profile::{
    CodingStyleProfile, CommentStyle, CommunicationProfile, DomainKnowledgeProfile, ExpertiseLevel,
    IndentationStyle, LearningState, NamingConvention, ProfileTone, ProfileUpdate, UserProfile,
    UserProfileService, WorkHabitProfile,
};
pub mod style;
pub use style::{
    CodeSample, CodeStyleTemplate, DocumentStyleProfile, ExtractedCodePatterns, FunctionPattern,
    MessageSample, NamingPattern, StructurePattern, StyleApplier, StyleExtractor, StylePattern,
    StylePatternType, StyleVector, StyleVectorizer,
};

// ── P7: RAG 契约（向量检索 / 重排 / 知识图谱 / 文档索引） ──
pub mod rag_provider;
pub use rag_provider::{
    EmbeddingProvider, RAGProvider, RAGQuery, RerankProvider, RetrievalQuality, SelfRagProvider,
    VectorQueryResult, VectorStoreProvider,
};
pub mod knowledge_graph;
pub use knowledge_graph::{
    EntityExtractor, EntityGraphProvider, ExtractedEntity, ExtractedRelation,
};
pub mod indexer;
pub use indexer::{ChunkProvider, DocumentChunk, DocumentIndexer, IndexConfig, IndexJobStatus};

// ── P8: 网关/平台管理契约 ──
pub mod gateway_service;
pub use gateway_service::{GatewayInfo, GatewayService, GatewayStatus};
pub mod platform_manager;
pub use platform_manager::{PlatformConnectionInfo, PlatformManager, PlatformMessageHandler};

// ── Credential 服务契约 ──
pub mod credential_service;
pub use credential_service::{CredentialService, SharedCredentialService, SmtpServiceConfig};

// ── P9: 安全防护契约（限流 / SSRF / 内容过滤 / 工具指标 / 熔断 / 访问控制） ──
pub mod rate_limiter;
pub use rate_limiter::{RateLimitConfig, RateLimitResult, RateLimitStatus, RateLimiter};
pub mod ssrf_guard;
pub use ssrf_guard::{SsrFConfig, SsrFGuard, UrlSafety};
pub mod content_filter;
pub use content_filter::{ContentFilter, ContentFilterConfig, ContentType, FilterAction};
pub mod tool_metrics;
pub use tool_metrics::{ToolCallRecord, ToolMetricsCollector, ToolMetricsSnapshot};
pub mod circuit_breaker;
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerSnapshot, CircuitState,
};
pub mod tool_access;
pub use tool_access::{AccessDecision, ToolAccessControl, ToolAccessRequest};

// ── P10: 开发者体验契约（可观测 / 基准测试 / 开发体验） ──
pub mod observability;
pub use observability::{ObservabilityProvider, ObservabilitySpanType};
pub mod benchmark;
pub use benchmark::{BenchmarkReport, BenchmarkRunner, BenchmarkTask, Difficulty, TaskResult};
pub mod dev_experience;
pub use dev_experience::{DevExperienceProvider, EnvironmentInfo, LogLevel};

// ── MCP 服务契约（让 tools/gateway 不依赖 mcp crate） ──
pub mod mcp_service;
pub use mcp_service::{
    DiscoveredMcpTool, McpClientService, McpServerConfig, McpServerStore, McpToolCallResult,
};

// ── 工具体系运行时服务（让 tools 不依赖 runtime-core） ──
pub mod tool_service;
pub use tool_service::{
    CronJobData, CronJobStore, HookEventFirer, McpTransport, NoopCronJobStore, NoopHookEventFirer,
};

// ── 会话压缩核心逻辑（无 HookRunner 依赖） ──
pub mod compact_session;
pub use compact_session::{
    cleanup_task_boundary, compact_session, decay_weight, detect_task_boundary,
    format_compact_summary, get_compact_continuation_message, summarize_turn,
};
