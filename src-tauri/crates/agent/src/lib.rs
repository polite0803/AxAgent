//! AxAgent Agent - ClawCode Runtime Integration

#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]

pub mod academic_search;
pub mod action_executor;
pub mod agent_adapter;
pub mod agent_config;
pub mod agent_runtime;
pub mod citation_tracker;
pub mod content_synthesizer;
pub mod coordinator;
pub mod credibility_evaluator;
pub mod error_classifier;
pub mod error_recovery_engine;
pub mod evaluator;
pub mod event_bus;
pub mod event_emitter;
pub mod fact_checker;
pub mod fine_tune;
pub mod frontend_adapter;
pub mod health_checker;
pub mod insight_generator;
pub mod local_tool_registry;
pub mod loop_detector;
pub mod metrics;
pub mod outline_builder;
pub mod provider_adapter;
pub mod react_engine;
pub mod traits;
pub mod reasoning_state;
pub mod recovery_strategies;
pub mod reference_builder;
pub mod reflector;
pub mod report_generator;
pub mod research_agent;
pub mod research_state;
pub mod retry_policy;
pub mod rl_optimizer;
pub mod search_orchestrator;
pub mod search_planner;
pub mod search_provider;
pub mod self_verifier;
pub mod session_manager;
pub mod source_classifier;
pub mod source_validator;
pub mod task;
pub mod task_decomposer;
pub mod task_executor;
pub mod thought_chain;
pub mod tool_recommender;
pub mod tool_registry;
pub mod trajectory_recorder;
pub mod web_search;

pub use academic_search::{
    AcademicSearchConfig, AcademicSearchProvider, AcademicSearchProviderBuilder,
};
pub use action_executor::{ActionError, ActionExecutor, ActionResult};
pub use agent_config::{AgentConfig, ConfigManager, ConfigSnapshot, DebugMode};
pub use agent_adapter::{
    AgentImplAdapter, AgentRuntimeAdapter, AgentRuntimeManager,
};
pub use agent_runtime::{AgentEvent, AgentRuntime, AgentRuntimeConfig, AgentRuntimeError, AgentOutput};
pub use citation_tracker::{
    CitationContext, CitationQuerier, CitationStats, CitationTracker, CitationUsage,
    CitationUsageCount,
};
pub use coordinator::{
    AgentError, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput,
    UnifiedAgentCoordinator, TypedAgentCoordinator,
};
pub use content_synthesizer::{ContentFormatter, ContentSynthesizer};
pub use credibility_evaluator::{
    CredibilityAssessment, CredibilityEvaluator, CredibilityFactor, CredibilityRanking,
    CredibilityScore, FactorDimension,
};
pub use error_classifier::{ClassifiedError, ErrorClassifier, ErrorType};
pub use error_recovery_engine::{
    ErrorRecoveryEngine, RecoveryConfig, RecoveryContext, RecoveryEvent,
};
pub use event_bus::{
    AgentEventBus, AgentEventBusBuilder, AgentEventType, EventSubscription, UnifiedAgentEvent,
};
pub use evaluator::{
    Benchmark, BenchmarkCategory, BenchmarkMetadata, BenchmarkReport, BenchmarkResult,
    BenchmarkSuite, BenchmarkTask, Dataset, DatasetRegistry, Difficulty, EvaluationCriteria,
    EvaluationMetric, EvaluationRunner, MetricsCalculator, ReportGenerator as BenchmarkReportGen,
    RunnerConfig, TaskInput, TaskOutput, TaskResult,
};
pub use event_emitter::AgentPermissionPayload;
pub use fact_checker::{
    Claim, ClaimExtractor, EvidenceType, FactCheckResult, FactCheckStatus, FactChecker,
    SourceEvidence,
};
pub use frontend_adapter::{
    FrontendEventAdapter, FrontendEventFilter, FrontendEventPayload, FrontendEventType,
    TauriEventAdapter, TauriEventEnvelope,
};
pub use health_checker::{
    HealthCheckResult, HealthCheckRunner, HealthChecker, HealthMetric, HealthStatus, HealthThresholds,
};
pub use insight_generator::{Insight, InsightCategory, InsightGenerator, InsightStats};
pub use loop_detector::{
    LoopDetector, LoopDetectorConfig, LoopWarning, LoopWarningLevel, ToolCallStats,
};
pub use metrics::{
    log_with_fields, record_timing_async, MetricType, MetricValue, MetricsCollector,
    StructuredLogEntry, TimedGuard, TimingStats,
};
pub use local_tool_registry::{LocalToolDef, LocalToolGroup, LocalToolRegistry};
pub use outline_builder::{OutlineBuilder, OutlineStyle, OutlineValidationError};
pub use provider_adapter::{AxAgentApiClient, StreamEventCallback};
pub use react_engine::{ReActEngine, ReActError, ReActResult};
pub use reasoning_state::{ActionType, ReActConfig, ReasoningState};
pub use recovery_strategies::{
    RecoveryAdjustment, RecoveryAttempt, RecoveryResult, RecoveryStrategy,
};
pub use reference_builder::{ReferenceBuilder, ReferenceFormat, ReferenceFormatter};
pub use reflector::{Reflection, ReflectionConfig, Reflector, TaskExecutionRecord};
pub use report_generator::{ReportError, ReportExporter, ReportGenerator, ReportStyle};
pub use research_agent::{ResearchAgent, ResearchError, ResearchEvent};
pub use research_state::{
    Citation, ReportFormat, ResearchConfig, ResearchPhase, ResearchProgress, ResearchReport,
    ResearchState, ResearchStatus, SearchPlan, SearchQuery, SearchResult, SourceType,
};
pub use retry_policy::{RetryError, RetryPolicy, RetryState};
pub use search_orchestrator::{OrchestratorError, SearchOrchestrator, SearchOrchestratorBuilder};
pub use search_planner::{ResearchDepth, SearchPlanner, SearchPlannerConfig};
pub use search_provider::{
    ContentMetadata, DateRange, ExtractError, ExtractedContent, RelevanceScorer, SearchError,
    SearchProvider, SearchProviderRegistry, SearchProviderType, SearchQueryBuilder,
    SearchResultProcessor,
};
pub use self_verifier::{SemanticValidator, SelfVerifier, VerificationError, VerificationResult};
pub use session_manager::{
    AgentSession, ChannelPermissionPrompter, SessionManager, TauriHookProgressReporter,
};
pub use source_classifier::{
    CategoryStats, SourceCategory, SourceClassification, SourceClassifier,
};
pub use source_validator::{
    DomainInfo, IssueCode, IssueSeverity, SourceFilter, SourceValidationResult, ValidationIssue,
    ValidatorConfig,
};
pub use task::{TaskGraph, TaskNode, TaskStatus, TaskType};
pub use task_decomposer::{DecompositionError, DecompositionResult, LlmClient, TaskDecomposer};
pub use task_executor::{ExecutionError, ExecutionEvent, ExecutionProgress, TaskExecutor};
pub use thought_chain::{
    Action, ChainSummary, ThoughtChain, ThoughtChainEmitter, ThoughtEvent, ThoughtStep,
};
pub use tool_registry::{
    McpRegistry, McpServerConfig, McpToolConfig, ToolContext, ToolError, ToolExecutionRecorder,
    ToolRegistry, ToolResult,
};
pub use trajectory_recorder::TrajectoryRecorder;
pub use web_search::{WebSearchConfig, WebSearchProvider, WebSearchProviderBuilder};
