// SPDX-License-Identifier: AGPL-3.0-only

// SAFETY: harness crate 中的 std::sync 锁用于同步上下文，不跨 await。
#![allow(clippy::disallowed_types)]

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
pub mod ir_renderer;
pub mod notification_channel;
pub use notification_channel::{
    AlertPayload, AlertSeverity, NotificationChannel, NotificationDispatchResult,
    NotificationDispatchSummary, NotificationPolicy, NotificationRoute, ReportPayload,
    ReportStockSummary, RouteConfig,
};
pub mod constants;
pub mod contracts;
pub use contracts::HarnessToolExecutor;
pub mod conversation_model;
pub use conversation_model::{ContentBlock, ConversationMessage, SessionInfo, TokenUsage};

// ── 模型定价表与成本估算（foundation 层权威定义，下沉自 runtime-core）──
// gateway（consumer）需要在此处换算 cost，而 consumer 不能依赖 runtime-core，
// 故 ModelPricing / pricing_for_model / UsageCostEstimate 必须位于 harness。
pub mod usage_pricing;
pub use usage_pricing::{
    ModelPricing, UsageCostEstimate, cost_for_tokens, format_usd, pricing_for_model,
};
pub mod core_error;
pub mod error_codes;
pub mod orchestration_dispatch;
pub use orchestration_dispatch::{DispatchRequest, SubTaskDispatchResult, SubTaskDispatcher};
pub mod node_output_status;
pub mod persistence_mod;
pub mod plan_compiler;
pub mod plan_types;
pub mod platform_config;
pub mod rag_config;
pub mod template_patch;
pub mod types;
pub mod url_utils;
pub mod util_fns;
pub mod workflow_node_deserializer;
pub mod workflow_types;
pub use node_output_status::NodeOutputStatus;
#[macro_use]
pub mod reliability;
pub mod response_normalizer;

// ── 市场数据契约（DTO + Trait + 工具函数）──
pub mod market_data;
pub use market_data::{
    AdjType, FinancialReport, KLine, MarketDataProvider, StockQuote, StockSearchResult,
    detect_market_type, get_price_limit_pct, get_st_price_limit_pct,
};

// ── P2-C7: 技术指标纯函数（SMA/EMA/RSI/stddev）──
// P3-C8: 追加 Sharpe ratio 统一实现（收口 stock-analysis/astock-data/tools/quant 的 6 处重复）。
// 收口原本散落在 astock-data/quant/market-sim/stock-analysis 的重复实现,
// foundation 层零 axagent-* 依赖,符合 harness 角色。
pub mod indicators;
pub use indicators::{
    A_SHARE_TRADING_DAYS_PER_YEAR, RISK_FREE_ANNUAL_DEFAULT, SharpeComponents, build_ema_series,
    ema_last, rsi_wilder, sharpe_components, sharpe_ratio, sharpe_ratio_annual,
    sharpe_ratio_with_annualization, sma, stddev_sample,
};

// ── 时间旅行(As-Of) DTO 契约 ──
pub mod as_of;
pub use as_of::{
    AsOfContext, AsOfDataKind, AsOfDataScope, AsOfError, AsOfSource, DegradationEntry,
};

// ── 高级股票数据服务契约已迁出至 axagent-stock-analysis（stock_data_service）──
pub mod speech;
pub use speech::{
    AudioChunkStream, SpeakRequest, SpeechCapabilities, SpeechInput, unsupported_speech_stream,
};

// ── 反馈数据湖（统一反馈接口，整合 retrieval_hits / tool_call_logs / memory_access_logs / wiki_edit_logs）──
pub mod feedback_data_lake;
pub use feedback_data_lake::{
    FeedbackDataLake, FeedbackDataLakeRegistry, FeedbackEvent, FeedbackEventType, FeedbackQuery,
    MemoryAccessRecord, RetrievalHitRecord, ToolCallRecord as FeedbackToolCallRecord,
    WikiEditRecord,
};

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
pub mod approval_policy;
pub mod output_sanitizer;
pub mod sandbox_policy;
pub mod session_events;
pub mod tool;
pub mod tool_permissions;
pub mod tool_validation;
pub use approval_policy::ApprovalPolicy;
pub use sandbox_policy::{SandboxMode, SandboxPolicy};
pub use session_events::{
    NullSessionEventSink, SessionEvent, SessionEventPayload, SessionEventSink, SessionEventType,
};

// ── Agent 单轮 ReAct 执行器契约(2.5 P1)──
// trait 定义在 foundation 层,由 wiring 把 SessionManager 适配器注入到 WorkEngine,
// rt-workflow 的 AgentExecutor 通过 trait 对象调用,实现"委托"语义。
// 未注入时 AgentExecutor 走 inline ReAct fallback(向后兼容)。
pub mod agent_turn_runner;
pub use agent_turn_runner::{
    AgentToolCallRecord, AgentTurnRequest, AgentTurnResult, AgentTurnRunner,
};

// ── 依赖注入容器 ──
pub mod graph_dtos;
pub mod louvain_dtos;
pub mod note_dtos;
pub mod page_type;
pub mod repo_dtos;
pub mod repositories;
pub mod service_registry;
pub mod streaming;
pub mod wiki_dtos;

/// 3.7 P2:TaskScene 下沉到 harness(foundation 层)。
///
/// 历史上定义在 `axagent_runtime::prompt`(wiring 层),导致 hybrid 层
/// (`rt-workflow`)无法引用。现在权威定义在本模块,各层 `pub use` 引用。
pub mod task_scene;
pub use task_scene::TaskScene;

// ── P0: 任务形态分类器（原则三核心标尺：上下文保留成本 × 安全隔离需求）──
pub mod task_shape;
pub use task_shape::{
    ContextRetentionCost, ExecutionStrategy, SecurityIsolationNeed, TaskShapeClassifier,
    TaskShapeDecision, TaskShapeLlmClassifier, resolve_effective_permission,
};

// ── ServiceRegistryProvider 契约重导出 ──
pub use service_registry::ServiceRegistryProvider;

// ── Real-time 流式管道契约 ──
pub use streaming::{AgentStreamChunk, AgentStreamReporter, NoopStreamReporter, StreamChunkKind};

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
pub mod cron_blueprint;
pub use cron_blueprint::{
    BlueprintCronJobData, BlueprintParam, BlueprintParamType, BlueprintRiskLevel, CronBlueprint,
    CronBlueprintType, FrequencySuggestion, GuardCheckResult, LifecycleGuard, UsagePattern,
};
pub mod training_pipeline;
pub use training_pipeline::{
    BatchGenerationConfig, CompressionPipeline, CompressionStep, CompressionStepType, DataStats,
    SamplingStrategy, TimeRangeFilter, TrainingDataReport, TrainingQualityMetrics,
};
pub mod skill_enhancement;
pub use skill_enhancement::{
    ActivationRule, ActivationRuleType, ConditionalActivation, CuratedCategory, DailyUsageStats,
    RatingCriteria, SkillBundle, SkillCuratedCollection, SkillDetails, SkillDisclosureLevel,
    SkillExample, SkillParameter, SkillRecommendation, SkillSummary, SkillUsageStats,
    TriggerCondition, TriggerConditionType,
};
pub mod terminal_enhancement;
pub use terminal_enhancement::{
    DockerBackendConfig, InfrastructureError, InfrastructureErrorClassifier,
    InfrastructureErrorType, OutputSpillConfig, OutputTruncator, ResourceLimits, SpillResult,
    SshAuthMethod, SshBackendConfig, TerminalBackendConfig, TerminalBackendType, VolumeMount,
};
pub mod learning_graph;
pub use learning_graph::{
    GraphLayout, GraphTagCount, LayoutAlgorithm, LearnedItem, LearningEdge, LearningEdgeType,
    LearningGraph, LearningGraphStats, LearningNode, LearningNodeType, LearningStats, NodePosition,
};
pub mod verification_recipe;
pub use verification_recipe::{
    ExpectedOutcome, FailureAction, PassCriteria, StepResult, StepStatus, VerificationRating,
    VerificationRecipe, VerificationRecipeType, VerificationReport, VerificationSeverity,
    VerificationStep, VerificationStepType, VerificationTrigger, VerificationTriggerType,
};
pub mod moa_degradation;
pub use moa_degradation::{
    DegradationState, DegradationStrategy, DegradationTrigger, MoADegradationConfig,
};
pub mod auxiliary_client;
pub use auxiliary_client::{
    AuxiliaryTask, AuxiliaryTaskStatus, AuxiliaryTaskType, CostLimit, TaskAllocationStrategy,
    TaskPriority, TemperatureContract,
};
pub mod gateway_operations;
pub use gateway_operations::{
    AuthorizationStatus, GatewayOpsStatus, GracefulShutdownConfig, LifecycleEvent,
    LifecycleEventType, LifecycleLedger, PairingAuthorization, ShutdownPhase, ShutdownPhaseStatus,
    ShutdownProgress,
};
pub mod insight_dashboard;
pub use insight_dashboard::{
    CostDataPoint, CostReport, EfficiencyMetrics, InsightRecommendation, InsightSeverity,
    RecommendationType, ReportPeriod, SkillUsage, TokenUsageStats, UsageReport,
};
pub mod lsp_integration;
pub use lsp_integration::{
    CompletionItemKind, CursorPosition, DiagnosticSeverity, LspCodeContext, LspCompletionItem,
    LspConfig, LspConnectionType, LspDiagnostic, LspError, LspHoverInfo, LspLocation, LspMethod,
    LspRange, LspRequest, LspResponse, LspServer, LspServerType,
};
pub mod error;
pub mod error_classifier;
pub use error_classifier::{
    ClassifiedError, ErrorType, FailoverReason, RecoveryAdjustment, RecoveryAttempt,
    RecoveryResult, RecoveryStrategy, SuggestedAction,
};
pub mod has_provider_registry;
pub mod inference_engine;
pub mod model_knowledge;
pub use model_knowledge::ModelKnowledgeProvider;
pub mod npm_registry;
pub mod persistence;
pub mod planner;
pub mod plugin_hook;
pub use plugin_hook::{
    ApiCallContext, ApiCallResult, HookContext, HookDecision, LlmCallContext, LlmCallResult,
    PluginHook, SharedHook, ToolCallContext, ToolCallResult,
};
// G16: DojoExtension Protocol — 扩展接入契约（health/tool_specs/execute_command/dashboard_cards/prompt_context）
pub mod dojo_extension;
pub use dojo_extension::{
    DojoCommandSpec, DojoDashboardCard, DojoDashboardCardType, DojoExtension, DojoExtensionHealth,
    DojoExtensionRegistry, DojoPromptContext, DojoToolSpec,
};
// G17: Cron delivery → gateway 闭环 — 投递配置 DTO + Sink trait
pub mod cron_delivery;
pub use cron_delivery::{
    CronDeliveryChannel, CronDeliveryConfig, CronDeliveryPayload, CronDeliverySink,
    NoopDeliverySink,
};
pub mod prompt_guard;
pub mod provider;
pub mod registry;
pub mod rhai_ast_cache;
pub mod rhai_engine;
pub use rhai_ast_cache::{cache_size, get_or_compile_ast};

// ── 技能侧反思钩子（自我进化通道二：能力偏弱进化改进）──
pub mod skill_evolution_hook;
pub use skill_evolution_hook::SkillEvolutionHook;

// ── 可逆效果原语（一切皆插件：注册即记录、卸载即回滚）──
pub mod reversible_effect;
pub use reversible_effect::{EffectHandle, EffectScope, NamedEffect, ReversibleEffect};

// ── 运行时能力注册表（内置与外部插件平权的统一接缝，Capability Seam 三件套）──
pub mod capability_registry;
pub use capability_registry::{
    CapabilityError, CapabilityOrigin, CapabilityRegistrationDetail, CapabilityRegistry,
    HasCapabilityRegistry, PluginCapabilityDescriptor, ServiceDefinition, get_capability_registry,
};
pub mod session_tracer;
pub mod storage_backend;
pub mod test_support;
pub mod trajectory_service;
// ── 会话日志不变量（Model-visible means logged，缺陷 #3 05 项）──
pub mod session_log_invariant;
pub use session_log_invariant::{
    DiskSessionLog, InMemorySessionLog, InvariantViolation, ModelVisibleContent,
    SessionLogInvariant, fingerprint,
};
// ── Webhook 契约 ──
pub mod webhook_subscription;
/// 关键 Webhook 类型重导出 — struct/enum 级
pub use webhook_subscription::{
    DispatchResult, WebhookDispatch, WebhookEvent, WebhookEventSink, WebhookPayload,
    WebhookPersistence, WebhookSubscription, WebhookSubscriptionInfo, WebhookSubscriptionService,
};

// ── 消息平台 Webhook 契约 ──
pub mod messaging_webhook;
pub use messaging_webhook::{WeChatWebhookHandler, WhatsAppWebhookHandler};

// ── 消息平台回调契约（message.callback 接缝） ──
pub mod platform_callback;
pub use platform_callback::PlatformMessageCallback;

// ── 消息平台适配器契约（platform.adapter 接缝） ──
pub mod message_adapter;
pub use message_adapter::{DeliveryMode, MediaAttachment, MediaType, MessagePlatformAdapter};

// ── 迁移相关 ──
pub mod migration_types;
pub use migration_types::{
    BackupInfo, DetectedPlatform, MigrationEntry, MigrationItem, MigrationReport,
};

// ── 工具扩展契约 ──
pub mod tools_ext;
pub use tools_ext::{
    DelegateTaskInput, DelegateTaskResult, DelegateTaskRunner, MigrationRunner,
    PluginAgentDescriptor, PluginAgentProvider,
};

// ── 搜索层数据源 trait（让 search crate 不依赖 dao / document-parser） ──
pub mod search_sources;
pub use search_sources::{
    ContentItem, DocumentParser, KnowledgeSource, KnowledgeSourceMeta, KnowledgeSourceType,
    MemorySource, SearchResult, SettingsSource, UnifiedKnowledgeSource, WikiSource,
};

// ── Marketplace 契约（让 gateway / kit 不依赖 dao / entities） ──
pub mod llm_execution;
pub use llm_execution::{LlmExecutionService, SharedLlmExecutionService};

// ── LLM 执行边界（原 runtime-core，上移至 harness 以满足铁律 4 共享类型权威） ──
pub mod retry_policy;
pub use retry_policy::{BackoffStrategy, FallbackStrategy, RetryError, RetryPolicy};
pub mod llm_executor;
pub use llm_executor::{LlmCallConfig, LlmUsage, execute_llm, execute_llm_stream};
pub mod marketplace;
pub use marketplace::{
    CatalogItem, CatalogPage, CatalogQuery, CreateReviewRequest, MarketplaceCatalogService,
    MarketplaceService, MarketplaceStats, ReviewResponse, UpdateReviewRequest,
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
pub use mcp_types::{
    DiscoveredTool, McpPrompt, McpPromptArgument, McpPromptResult, McpResource, McpResourceContent,
};

// ── 决策仪表盘报告 DTO（借鉴 daily_stock_analysis 推送格式）──
pub mod dashboard_report;
pub use dashboard_report::{
    Catalyst, ChecklistItem, DashboardDigest, DashboardReport, IndexQuote, MarketReviewReport,
    MissingField, RiskAlert, StockSummary, fill_missing_with_placeholders,
    validate_dashboard_report,
};

// ── 出站推送通知渠道契约已迁出至 axagent-stock-analysis（notification_channel）──

pub mod trajectory_scorer;
pub mod trajectory_types;

// ── ReplayExecutor 契约（轨迹回放与回归测试） ──
pub mod replay_executor;
pub use replay_executor::{
    DeviationKind, GoldenTrajectory, RegressionSuite, RegressionSuiteResult, ReplayExecutor,
    ReplayOptions, ReplayReport, StepDeviation, build_replay_report, compare_trajectories,
};

// ── 多模型协同路由契约（Cascade / Chain 模式） ──
pub mod model_cascade;
pub use model_cascade::{
    CascadeModel, CascadeOutcome, EscalationDecision, EscalationReason, EscalationRecord,
    EscalationRule, EscalationRuleBuilder, ModelCallSummary, ModelCascadeExecutor,
    ModelCascadeStrategy, should_escalate,
};

// ── 路由决策桥接（RouteDecision ↔ ProviderRequestContext） ──
// ModelTier / RouteDecision 等应用层类型留在 src/smart_router/，
// harness 只定义 tier → 具体 model/provider 的映射契约。
pub mod route_bridge;
pub use route_bridge::{
    ModelTierResolver, TierModelMapping, apply_mapping_to_context, apply_tier_to_request,
};

// ── Provider 契约重导出 ──
pub use context_builder::build_provider_request_context;
pub use has_provider_registry::HasProviderRegistry;
pub use provider::{
    ProviderAdapter, ProviderProxyConfig, ProviderRequestContext, RealtimeProviderConfig,
};
pub use url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};

// ── PromptGuard 契约重导出 ──
pub use prompt_guard::{DynamicGuardRule, PatternPromptGuard, PromptGuard, PromptRejection};

// ── SessionTracer 契约重导出 ──
pub use session_tracer::SessionTracer;

// ── NpmRegistry 契约重导出 ──
pub use npm_registry::{NpmRegistryService, parse_npm_package_spec};

// ── RhaiEngine 契约重导出 ──
pub use rhai_engine::{
    RhaiEngineAdapter, RhaiToolFn, dynamic_to_json_value, json_value_to_dynamic,
    register_common_functions,
};

// ── Planner 契约重导出 ──
pub use planner::PlannerAdapter;

// ── TrajectoryService 契约重导出 ──
pub use trajectory_service::{IntegrityCheck, IntegrityResult, TaskComplexity, TrajectoryService};

// ── Tool 契约重导出 ──
pub use tool::{
    AskUserBridge, DefaultInputSanitizer, DefaultOutputSanitizer, EstimatedCost, InputSanitizer,
    OutputSanitizer, PermissionResult, ProgressEntry, RollbackContext, RollbackRecord,
    SanitizeContext, Tool, ToolCategory, ToolContext, ToolDomain, ToolInfo, ToolPermissions,
    ToolRanker, ToolResult, parse_tool_name,
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
pub use inference_engine::{InferenceEngine, LoRATrainConfig, LoRATrainResult, SparseVectorEntry};

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
pub mod agent_session_broker;
pub use agent_session_broker::{AgentSessionBroker, AgentSessionStatusView};

pub mod runtime_types;

pub mod kit_bridge;

pub mod cache_service;
pub use cache_service::{CacheService, SharedCacheService};

// ── HookService 契约 ──
pub mod hook_service;
pub use hook_service::{HookService, SharedHookService};

// ── WorkflowHookSink 契约(工作流 Hook 触发端) ──
pub mod workflow_hook_sink;
pub use workflow_hook_sink::{NoopWorkflowHookSink, SharedWorkflowHookSink, WorkflowHookSink};

// ── HookEvent 顶层 re-export(便于业务代码直接 `use axagent_harness::HookEvent`) ──
pub use runtime_types::hooks::HookEvent;

// ── PermissionChecker 契约顶层 re-export(供 NodeDispatcher / 工作流节点权限检查) ──
pub use runtime_types::permission_enforcer::{EnforcementResult, PermissionChecker};

// ── 能力补齐提议契约顶层 re-export(供认知编排器双通道闭环消费) ──
pub use runtime_types::capability_gap::{
    CapabilityGapProposal, CapabilityGapType, PromptAttackCategory,
};

// ── 运行时变异接口顶层 re-export(供自指工具与 wiring 层消费) ──
pub use runtime_types::runtime_mutation::{MutationResult, RuntimeMutationAccess};

// ── 多 Agent 协作契约(Swarm/Debate/SharedBlackboard 统一抽象) ──
pub mod multi_agent;
pub use multi_agent::{
    AgentDecision, BlackboardMessage, ConflictRecord, ConflictResolution, CoordinationMode,
    CoordinationOutcome, MultiAgentCoordination, SharedBlackboard,
};

// ── FeatureFlagProvider 契约 ──
pub mod feature_flag_provider;
pub use feature_flag_provider::{FeatureFlagProvider, SharedFeatureFlagProvider};

// ── P1: MemoryStore 契约（记忆外溢/共享 + 增强能力） ──
pub mod memory;
pub use memory::{
    MemoryActionResultDto, MemoryAddRequest, MemoryFeedbackRequest, MemoryGroupedDto,
    MemoryLifecycleEvent, MemoryLifecycleHook, MemorySearchItem, MemorySearchRequest, MemoryStore,
    MemoryTreeItem, MemoryUpdateRequest, MemoryWriteApprovalConfig, MemoryWriteApprovalRequest,
    MemoryWriteApprovalStatus, NoopMemoryHook, SkillScaffoldStripper, StrippedContent,
    TrivialInputGate,
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
    TrajectoryRewardEngine,
};

// ── P0: 代码验收引擎契约 ──
pub mod code_verifier;
pub use code_verifier::{CodeChange, CodeVerificationResult, CodeVerifierPort, DiffHunk};

// ── P1: 动态路由引擎契约 ──
pub mod route_engine;
pub use route_engine::{
    HardGate, HardGateCriteria, HardGateStatus, NodeExecutionResult, RouteContext, RouteDecision,
    RouteDecisionType, RouteEngine, RouteRule, RouteStrategy,
};
pub mod dream;
pub use dream::{
    ConsolidationDataProvider, ConsolidationSuggestion, ContrastivePair, DistilledKnowledge,
    DreamConsolidationConfig, DreamConsolidationResult, DreamConsolidationState, DreamConsolidator,
    DreamEventEmitter, ExperienceRecord, KnowledgeType, SuggestionType,
};

// ── 反思系统共享 DTO(任务级与工作流级共用) ──
pub mod reflection_types;
pub use reflection_types::{QualityMetrics, Reflection, ReflectionConfig, TaskExecutionRecord};

// ── 自改进循环契约(Loop Engineering 基座层) ──
// trait + DTO 定义在本层,通用执行器 SelfImprovementExecutor 在 agent crate 实现。
// 业务层(AxInvest/AxOPC/AxSim 等)通过 impl SelfImprovingRound 注入领域评估。
pub mod self_improving_loop;
pub use self_improving_loop::{
    LoopError, NextAction, RoundEvaluation, RoundResult, RoundStep, SelfImprovingRound,
};

// ── 工作流反思/进化/优化三层 trait 契约 ──
pub mod workflow_template_repo;
pub use workflow_template_repo::WorkflowTemplateRepo;

pub mod workflow_reflection;
pub use workflow_reflection::{
    BottleneckNode, BottleneckReason, FailureCategory, NodeExecutionSnapshot, NodeFailureAnalysis,
    ProposedChange, WorkflowExecutionRecord, WorkflowPattern, WorkflowReflectionMetadata,
    WorkflowReflector, WorkflowRunStatus,
};
pub mod workflow_evolution;
pub use workflow_evolution::{
    EvolutionArtifactValidator, EvolutionConfig, EvolutionPopulation, EvolutionStats, GenomeChange,
    GenomePosition, SandboxValidationResult, WorkflowDagExecutor, WorkflowEvolver, WorkflowGenome,
    WorkflowGenomeLoader, WorkflowLlmMutator, WorkflowModification, WorkflowSandbox,
    workflow_genome_from_generated,
};
pub mod workflow_optimization;
pub use workflow_optimization::{
    ProposedChange as WorkflowOptimizationProposedChange, SuggestionCategory, SuggestionPriority,
    WorkflowOptimizer, WorkflowSuggestion,
};
// ── 用户自适应共享枚举（Verbosity/TechnicalLevel/ContentFormat） ──
// 同时被 profile::UserProfile::update_style 和 trajectory 的 RealTimeLearning 使用
pub mod adaptation;
pub use adaptation::{ContentFormat, TechnicalLevel, Verbosity};
pub mod profile;
pub use profile::{
    CodePattern, CodingStyleProfile, CommentStyle, CommunicationProfile, DetailLevel,
    DomainKnowledgeProfile, ExpertiseArea, ExpertiseLevel, ExplanationDepth, FormatPreference,
    IndentationStyle, LearningState, LearningTaskType, ModuleOrgStyle, NamingConvention,
    ProfileUpdate, RecentTopic, ResponseLength, SkillLevel, TimeRange, Tone, ToolUsagePattern,
    UpdateSource, UserProfile, UserProfileService, WorkHabitProfile, WorkflowPreference,
    calculate_confidence,
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
    EntityExtractor, EntityGraphProvider, ExtractEntitiesFromDocumentsInput, ExtractEntitiesResult,
    ExtractedEntity, ExtractedRelation, GraphEnhancedContextChunk, GraphEnhancedSearchInput,
    GraphEnhancedSearchResult, GraphRelationEdge,
};
pub mod indexer;
pub use indexer::{ChunkProvider, DocumentChunk, DocumentIndexer, IndexConfig, IndexJobStatus};

// ── 能力发现契约（Capability Discovery Pipeline） ──
pub mod capability;
pub use capability::{
    CallerPermissions, CapabilityDomain, CapabilityEvolvability, CapabilityExposure,
    CapabilityKind, CapabilityLevel, CapabilityPassport, CapabilityPassportDto, CapabilitySource,
    CapabilityStats, CapabilityToolRef, DiscoveryWeights, InputModality, KnowledgeSnippet,
    ModalitySupport, OutputCapabilities, PlaceholderDef, PlanningComplexity, SecurityLevel,
    SessionBudget, Visibility,
};
/// 轻量实体抽取器（P1：语义解析的实体部分，无 LLM）
pub mod entity_extractor;
pub use entity_extractor::CapabilityEntity;
pub mod capability_indexer;
pub use capability_indexer::{
    CAPABILITY_COLLECTION, CAPABILITY_NEGATIVE_COLLECTION, CapabilityIndexStats, CapabilityIndexer,
    IndexResult,
};
pub mod capability_retriever;
pub use capability_retriever::{
    CapabilityCandidate, CapabilityLayer, CapabilityQuery, CapabilityRetrievalResult,
    CapabilityRetriever,
};
pub mod session_state;
pub use session_state::{
    DEFAULT_AGENT_ID, NS_SKILL_LOADED, SessionStateEntry, SessionStateStore, StateScope,
    namespace_prefix, scoped_key,
};
pub mod dynamic_tools;
pub use dynamic_tools::DynamicToolSet;
pub mod capability_filter;
pub use capability_filter::{
    CapabilityFilter, FilterContext, FilterDecision, FilterDimension, FilteredCandidates,
    OutputDeviceType, PiiType, RejectedCandidate, TaskPlanningLevel,
};
pub mod capability_ranker;
pub use capability_ranker::{CapabilityRanker, RankedCapability, RankingResult};
pub mod capability_circuit;
pub use capability_circuit::{
    CapabilityCircuitBreaker, CapabilityCircuitSnapshot, CapabilityCompleter, CapabilityHotSwapper,
    CapabilitySuggestion, ContextEntity, ProtectedCapability, ProtectionReason, RefreshReport,
    SelfReferenceCheckResult, SelfReferenceCircuitBreaker, UserContextSnapshot,
};
// ── L2 集群清单(三层路由树第二层) ──
pub mod capability_clusters;
pub use capability_clusters::{
    CapabilityCluster, all_clusters, clusters_by_domain, derive_cluster_for_passport, find_cluster,
    find_cluster_by_segment,
};
// ── 路径地址与路由图(三层路由树地址编码 + DAG 邻接表) ──
pub mod routing_path;
pub use routing_path::{RoutingGraph, RoutingPath};
// ── RAR 召回器契约(检索增强路由,软引导能力推荐) ──
pub mod rar_recaller;
pub use rar_recaller::{RarRecallResult, RarRecaller, build_rar_prompt};
pub mod capability_router;
pub use capability_router::{
    CapabilityDiscoveryRequest, CapabilityDiscoveryResult, CapabilityRouter,
    DefaultCapabilityRouter, PhaseTiming,
};

// ── L1/L2 分层路由契约 ──
pub mod domain_router;
pub use domain_router::{
    DomainDecision, DomainRouter, DomainRouterImpl, DomainRoutingResult, DomainRoutingRule,
    DomainRuleType, LlmReasoner, MatchMode, default_domain_rules,
};
pub mod cluster_router;
pub use cluster_router::{
    ClusterRouter, ClusterRouterImpl, ClusterRoutingResult, ClusterRoutingRule,
    derive_cluster_from_query,
};
pub mod layered_prompt_engine;
pub use layered_prompt_engine::{
    LayeredPromptEngine, LayeredPromptResult, PromptLayer, PromptSegment, PromptTemplate,
    estimate_tokens,
};

// ── RAR 检索增强（三层路由树第二层） ──
pub mod rar_router;
pub use rar_router::{
    DefaultRarRouter, FilteredReason, RarCandidate, RarCircuitBreaker, RarError, RarFilterReason,
    RarRouter, RarSearchResult, build_rar_few_shot_prompt, compute_relevance_score,
    default_top_k_for_cluster,
};

// ── 工作流图谱（三层路由树第三层） ──
pub mod workflow_graph;
pub use workflow_graph::{
    EdgeType, GraphRouteResult, RouteLevel, WorkflowGraph, WorkflowGraphEdge, WorkflowGraphNode,
    WorkflowGraphRouter, WorkflowGraphSync,
};

// ── 能力组装器（CapabilityPassport → WorkflowNode/Edge 桥接层） ──
pub mod assembly_builder;
pub use assembly_builder::{AssemblyBuilder, AssemblyResult, DefaultAssemblyBuilder};

// ── 认知路由器（三层路由树协调器 · Phase 4 集成层） ──
pub mod cognitive_router;
pub use cognitive_router::{
    CandidateSummary, CognitiveRouter, CognitiveRouterConfig, DefaultCognitiveRouter,
    ExecutionMode, ModeHint, RouteStage, RouteStageRecord, RoutingDecisionV2, build_route_path,
    parse_route_path,
};

// ── 双注册表管理器（元能力隔离核心 Layer 1 + Layer 4 + Layer 5） ──
pub mod dual_registry;
pub use dual_registry::{
    DualRegistry, Privilege, PrivilegedCaller, PrivilegedChainStep, PrivilegedExecutionResult,
    PrivilegedHealthStatus, RegistryError, RegistryType, RouterSelfUpdateManager, RoutingRule,
    SystemConfigStore, SystemPrivilegedPipeline,
};

// ── P8: 网关/平台管理契约 ──
pub mod gateway_service;
pub use gateway_service::{GatewayInfo, GatewayService, GatewayStatus};
pub mod platform_manager;
pub use platform_manager::{PlatformConnectionInfo, PlatformManager, PlatformMessageHandler};

// ── Credential 服务契约 ──
pub mod credential_service;
pub use credential_service::{CredentialService, SharedCredentialService, SmtpServiceConfig};

// ── 数据库查询服务契约 ──
pub mod database_query_service;
pub use database_query_service::{DatabaseQueryResult, DatabaseQueryService};

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

// ── 量化策略契约（让 market-sim / quant 等 consumer 共享 Strategy trait + 数据类型） ──
pub mod strategy_contract;
pub use strategy_contract::{
    Bar, CloseReason, EquityPoint, Fill, Order, OrderType, Position, Side, Signal, SignalAction,
    Strategy, StrategyCtx, Trade,
};

// ── 荐股策略包契约（YAML 自然语言策略包格式，让用户可配置策略参数） ──
pub mod strategy_pack;
pub use strategy_pack::{
    StrategyPack, StrategyPackManifest, StrategyPackSpec, StrategyPackStrategyEntry,
};

// ── 统一事件总线契约（跨 crate 事件流标准入口） ──
// agent / rt-workflow / orchestrator 三方通过 `Arc<dyn EventBus>` 桥接,
// 保留各自原有 event_bus,统一总线作为额外发布通道。
pub mod event_bus;
pub use event_bus::{DomainEvent, EventBus, EventBusSubscription, EventCategory};

// P2 类型化事件派发总线（四派发模式 + 订阅裁决，见 typed_event.rs）。
pub mod typed_event;
pub use typed_event::{
    DispatchMode, DispatchOutcome, EventDispatchBus, EventMatcher, EventSubscriber,
    SubscriberVerdict,
};

// ── Fleet 多办公室 AI 团队契约 ──
pub mod fleet;

// ── Obsidian Vault 集成契约 ──
// 参考 DeepTutor `deeptutor/capabilities/obsidian/`：
// KB 类型 = ConnectedVault 时，agent 通过 9 个 obsidian_* 工具直接读写 live vault，
// 不走 RAG 索引。VaultSource trait 由 tools crate 实现，agent capability 注入。
pub mod vault;
pub use vault::{
    KbKind, LinkHit, NoteContent, NoteHit, NoteRef, OBSIDIAN_TOOL_NAMES, TagCount, VaultBinding,
    VaultError, VaultSource,
};

// ── 设备同步契约（多端同步/管理的核心类型） ──
pub mod device_sync;
pub use device_sync::{
    ChangeLogEntry, ChangeOperation, ConflictInfo, ConflictResolutionStrategy, DeviceInfo,
    DeviceManager, DeviceSyncStatus, DeviceType, EntityType, PairingCode, PairingRequest,
    PairingResponse, SyncEngine, SyncResult, TrustLevel, VersionVectorEntry,
};

// ── 行业编排契约（让 analysis-engine 等 consumer 不依赖 orchestrator） ──
pub mod industry_orchestration;
pub use industry_orchestration::{
    AcceptanceCriterion, AcceptanceResult, AutoReflectTrigger, AutoTriggerConfig, CriterionResult,
    DecompositionPlan, DependencyType, DynamicSubGraph, EvolutionConstraints,
    ForbiddenOptimization, GeneratedSubGraph, IndustryAdapter, IndustryAdapterRegistry,
    IndustryContext, IndustryLearningConfig, MissionType, OrchestrationError,
    OrchestrationStrategy, PresetWorkflowStep, ProtectedStep, QualityThresholds, QualityWeights,
    ReflectionCheckpoint, ReflectionTemplate, ReinforcementLearningConfig, RewardWeightConfig,
    SelfImprovementConfig, SkillEvolverConfig, StepDependency, SubTask, SubTaskStatus,
    WorkflowEvolverConfig,
};

// ── 协议层：强类型 Schema 系统（刚性协议 + 柔性节点架构核心）──
pub mod schema;
pub use schema::{
    NodeContract, PrimitiveType, SchemaFormat, SchemaValidationError, SchemaValidationResult,
};

// ── 执行层：业务状态机（刚性轨道 + 柔性节点）──
pub mod business_state_machine;
pub use business_state_machine::{
    BusinessState, BusinessStateMachine, FsmContext, FsmRuntimeState, FsmTransitionError,
    FsmTransitionRecord, FsmValidationError, StateTransition,
};

// ── 观测层：执行轨迹系统（时间旅行调试）──
pub mod execution_trace;
pub use execution_trace::{
    ExecutionTrace, NodeErrorDetail, NodeErrorType, NodeExecutionTrace, SchemaDiffReport,
    SchemaDiffType, TimelinePosition, TokenUsageTrace, ToolCallStatus, ToolCallTrace,
    TraceErrorSummary, TraceStatistics, TraceStatus,
};

// ── 纯文本后处理（LLM 输出清理，上移自 runtime-core，供所有 consumer 共享）──
pub mod text_clean;
pub use text_clean::clean_output;

// ── 类型驱动设计：DTO 尺寸锁定（编译时断言） ──
pub mod dto_locks;
