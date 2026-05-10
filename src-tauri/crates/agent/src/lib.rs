//! AxAgent Agent - ClawCode Runtime Integration

#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::result_large_err)]

pub mod ab_testing;
pub mod academic_search;
pub mod action_executor;
pub mod agent_adapter;
pub mod agent_config;
pub mod agent_runtime;
pub mod blackboard;
pub mod checkpoint;
pub mod citation_tracker;
pub mod content_synthesizer;
pub mod context_files;
pub mod coordinator;
pub mod credibility_evaluator;
pub mod deep_research;
pub mod environment_probe;
pub mod error_classifier;
pub mod error_recovery_engine;
pub mod evaluator;
pub mod event_bus;
pub mod event_emitter;
pub mod fact_checker;
pub mod fine_tune;
pub mod frontend_adapter;
pub mod graph_insights;
pub mod health_checker;
pub mod hierarchical_planner;
pub mod ingest_pipeline;
pub mod ingest_queue;
pub mod insight_generator;
pub mod interrupt;
pub mod lint_checker;
pub mod llm_bridge;
pub mod loop_detector;
pub mod metrics;
pub mod outline_builder;
pub mod proactive_mode;
pub mod project_memory;
pub mod provider_adapter;
pub mod purpose_manager;
pub mod query_engine;
pub mod react_engine;
pub mod reasoning_state;
pub mod recovery_strategies;
pub mod reference_builder;
pub mod reflector;
pub mod relevance;
pub mod report_generator;
pub mod research_agent;
pub mod research_state;
pub mod retry_policy;
pub mod rl_optimizer;
pub mod schema_manager;
pub mod search_orchestrator;
pub mod search_planner;
pub mod search_provider;
pub mod self_verifier;
pub mod session_manager;
pub mod source_classifier;
pub mod source_validator;
pub mod steer_manager;
pub mod task;
pub mod task_decomposer;
pub mod task_executor;
pub mod thought_chain;
pub mod tool_recommender;
pub mod traits;
pub mod trajectory_recorder;
pub mod tree_of_thoughts;
pub mod verification_agent;
pub mod vision_pipeline;
pub mod web_search;
pub mod wiki_compiler;

pub use ab_testing::{
    ExperimentConfig, ExperimentGroup, ExperimentMetric, ExperimentResult, ExperimentRunner,
    ExperimentStatus, GroupStats, MetricComparison, TrialResult,
};
pub use academic_search::{
    AcademicSearchConfig, AcademicSearchProvider, AcademicSearchProviderBuilder,
};
pub use action_executor::{ActionError, ActionExecutor, ActionResult};
pub use agent_adapter::{AgentImplAdapter, AgentRuntimeAdapter, AgentRuntimeManager};
pub use agent_config::{AgentConfig, ConfigManager, ConfigSnapshot, DebugMode};
pub use agent_runtime::{
    AgentEvent, AgentOutput, AgentRuntime, AgentRuntimeConfig, AgentRuntimeError,
};
pub use blackboard::{
    Blackboard, BlackboardEntry, BlackboardEvent, BlackboardManager, EntryPriority,
};
pub use checkpoint::{Checkpoint, CheckpointBuilder, CheckpointManager};
pub use citation_tracker::{
    CitationContext, CitationQuerier, CitationStats, CitationTracker, CitationUsage,
    CitationUsageCount,
};
pub use content_synthesizer::{ContentFormatter, ContentSynthesizer};
pub use coordinator::{
    AgentCoordinator, AgentError, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput,
};
pub use credibility_evaluator::{
    CredibilityAssessment, CredibilityEvaluator, CredibilityFactor, CredibilityRanking,
    CredibilityScore, FactorDimension,
};
pub use environment_probe::{EnvironmentProbe, EnvironmentSnapshot, FileInfo, ProbeConfig};
pub use error_classifier::{ClassifiedError, ErrorClassifier, ErrorType};
pub use error_recovery_engine::{
    ErrorRecoveryEngine, RecoveryConfig, RecoveryContext, RecoveryEvent,
};
pub use evaluator::{
    Benchmark, BenchmarkCategory, BenchmarkMetadata, BenchmarkReport, BenchmarkResult,
    BenchmarkSuite, BenchmarkTask, Dataset, DatasetRegistry, Difficulty, EvaluationCriteria,
    EvaluationMetric, EvaluationRunner, MetricsCalculator, ReportGenerator as BenchmarkReportGen,
    RunnerConfig, TaskInput, TaskOutput, TaskResult,
};
pub use event_bus::{
    AgentEventBus, AgentEventBusBuilder, AgentEventType, AgentPermissionPayload, EventSubscription,
    UnifiedAgentEvent,
};
pub use fact_checker::{
    Claim, ClaimExtractor, EvidenceType, FactCheckResult, FactCheckStatus, FactChecker,
    SourceEvidence,
};
pub use frontend_adapter::{
    FrontendEventAdapter, FrontendEventFilter, FrontendEventPayload, FrontendEventType,
    TauriEventAdapter, TauriEventEnvelope,
};
pub use health_checker::{
    HealthCheckResult, HealthCheckRunner, HealthChecker, HealthMetric, HealthStatus,
    HealthThresholds,
};
pub use hierarchical_planner::{
    HierarchicalPlanner, Phase, PhaseStatus, Plan, PlanBuilder, PlanProgress, PlanStatus,
    PlanVersion, PlannedTask, ReplanAction, ReplanReason, ReplanRecord, TaskBuilder, TaskStatus,
};
pub use insight_generator::{Insight, InsightCategory, InsightGenerator, InsightStats};
// 所有工具相关类型已统一在 axagent-tools，此处重导出保持兼容
pub use axagent_tools::registry::UnifiedToolRegistry as ToolRegistry;
pub use axagent_tools::registry::{McpServerConfig, McpToolConfig};
pub use axagent_tools::{ToolContext, ToolError, ToolExecutionRecorder, ToolResult};

// LocalToolRegistry 兼容类型
#[derive(Debug, Clone)]
pub struct LocalToolDef {
    pub group_id: String,
    pub group_name: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub env_json: Option<String>,
    pub timeout_secs: Option<i32>,
}
#[derive(Debug, Clone)]
pub struct LocalToolGroup {
    pub group_id: String,
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<LocalToolDef>,
}

pub struct LocalToolRegistry {
    pub flat_tools: Vec<axagent_tools::builtin_tools::FlatBuiltinTool>,
    pub enabled: std::collections::HashMap<String, bool>,
    pub group_names: std::collections::HashMap<String, String>,
    pub tool_defs: std::collections::HashMap<String, LocalToolDef>,
}
impl LocalToolRegistry {
    pub fn init_from_registry() -> Self {
        let flat = axagent_tools::builtin_tools::get_all_builtin_tools_flat();
        let mut enabled = std::collections::HashMap::new();
        let mut group_names = std::collections::HashMap::new();
        let mut tool_defs = std::collections::HashMap::new();

        // 加载旧 builtin 工具
        for ft in &flat {
            enabled.entry(ft.server_id.clone()).or_insert(true);
            group_names
                .entry(ft.server_id.clone())
                .or_insert_with(|| ft.server_name.clone());
            tool_defs.insert(
                ft.tool_name.clone(),
                LocalToolDef {
                    group_id: ft.server_id.clone(),
                    group_name: ft.server_name.clone(),
                    tool_name: ft.tool_name.clone(),
                    description: ft.description.clone(),
                    input_schema: ft.input_schema.clone(),
                    env_json: ft.env_json.clone(),
                    timeout_secs: ft.timeout_secs,
                },
            );
        }

        // 加载 52 个新统一工具（按 ToolCategory 分组）
        let unified = axagent_tools::registry::UnifiedToolRegistry::new();
        // 不调用 init_all() 因为会重复注册，直接用内置的 new()
        let unified_tools = unified.tools.list_all();
        let category_map: Vec<(&str, &str)> = vec![
            ("builtin-file-read", "文件读取"),
            ("builtin-file-write", "文件写入"),
            ("builtin-shell", "Shell 命令"),
            ("builtin-network", "网络请求"),
            ("builtin-system-tools", "系统工具"),
            ("builtin-agent", "Agent 工具"),
        ];
        for (gid, gname) in &category_map {
            enabled.entry(gid.to_string()).or_insert(true);
            group_names
                .entry(gid.to_string())
                .or_insert_with(|| gname.to_string());
        }
        for info in &unified_tools {
            let gid = match info.category {
                axagent_tools::ToolCategory::FileRead => "builtin-file-read",
                axagent_tools::ToolCategory::FileWrite => "builtin-file-write",
                axagent_tools::ToolCategory::Shell => "builtin-shell",
                axagent_tools::ToolCategory::Network => "builtin-network",
                axagent_tools::ToolCategory::System => "builtin-system-tools",
                axagent_tools::ToolCategory::Agent => "builtin-agent",
            };
            tool_defs.insert(
                info.name.clone(),
                LocalToolDef {
                    group_id: gid.to_string(),
                    group_name: gid.to_string(),
                    tool_name: info.name.clone(),
                    description: info.description.clone(),
                    input_schema: info.input_schema.clone(),
                    env_json: None,
                    timeout_secs: None,
                },
            );
        }

        Self {
            flat_tools: flat,
            enabled,
            group_names,
            tool_defs,
        }
    }
    pub async fn load_enabled_state(&mut self, db: &sea_orm::DatabaseConnection) {
        use axagent_core::entity::settings::Entity as SettingsEntity;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let key = "tool_groups_enabled";
        let result = SettingsEntity::find()
            .filter(axagent_core::entity::settings::Column::Key.eq(key))
            .one(db)
            .await;

        if let Ok(Some(record)) = result {
            if let Ok(map) =
                serde_json::from_str::<std::collections::HashMap<String, bool>>(&record.value)
            {
                for (gid, is_enabled) in map {
                    self.enabled.insert(gid, is_enabled);
                }
            }
        }
    }
    pub fn is_enabled(&self, tool_name: &str) -> bool {
        self.tool_defs.contains_key(tool_name)
    }
    pub fn contains(&self, tool_name: &str) -> bool {
        self.tool_defs.contains_key(tool_name)
    }
    pub fn get_enabled_chat_tools(&self) -> Vec<axagent_core::types::ChatTool> {
        self.tool_defs
            .values()
            .filter(|d| self.is_enabled(&d.tool_name))
            .map(|d| axagent_core::types::ChatTool {
                r#type: "function".into(),
                function: axagent_core::types::ChatToolFunction {
                    name: d.tool_name.clone(),
                    description: Some(d.description.clone()),
                    parameters: Some(d.input_schema.clone()),
                },
            })
            .collect()
    }
    pub fn set_env_json(&mut self, tool_name: &str, env_json: String) {
        if let Some(def) = self.tool_defs.get_mut(tool_name) {
            def.env_json = Some(env_json);
        }
    }
    pub fn set_timeout_secs(&mut self, tool_name: &str, timeout_secs: i32) {
        if let Some(def) = self.tool_defs.get_mut(tool_name) {
            def.timeout_secs = Some(timeout_secs);
        }
    }
    pub fn all_tool_defs(&self) -> &std::collections::HashMap<String, LocalToolDef> {
        &self.tool_defs
    }
    pub fn all_tool_names(&self) -> Vec<String> {
        self.tool_defs.keys().cloned().collect()
    }
    pub fn get_group_id(&self, tool_name: &str) -> Option<&str> {
        self.tool_defs.get(tool_name).map(|d| d.group_id.as_str())
    }
    pub fn get_tool_groups(&self) -> Vec<LocalToolGroup> {
        let mut groups_map: std::collections::HashMap<String, Vec<LocalToolDef>> =
            std::collections::HashMap::new();
        for def in self.tool_defs.values() {
            groups_map
                .entry(def.group_id.clone())
                .or_default()
                .push(def.clone());
        }
        let mut groups: Vec<LocalToolGroup> = groups_map
            .into_iter()
            .map(|(gid, tools)| {
                let gname = self
                    .group_names
                    .get(&gid)
                    .cloned()
                    .unwrap_or_else(|| gid.clone());
                let enabled = self.enabled.get(&gid).copied().unwrap_or(true);
                LocalToolGroup {
                    group_id: gid,
                    group_name: gname,
                    enabled,
                    tools,
                }
            })
            .collect();
        groups.sort_by_key(|g| g.group_id.clone());
        groups
    }
    pub async fn toggle_group(
        &mut self,
        db: &sea_orm::DatabaseConnection,
        gid: &str,
    ) -> Result<bool, String> {
        let current = self.enabled.get(gid).copied().unwrap_or(true);
        let new_state = !current;
        self.enabled.insert(gid.to_string(), new_state);

        let key = "tool_groups_enabled";
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let existing = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(key))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        let serialized = serde_json::to_string(&self.enabled).map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let mut active: settings_model::ActiveModel = record.into();
                active.value = Set(serialized);
                active.update(db).await.map_err(|e| e.to_string())?;
            },
            None => {
                let active = settings_model::ActiveModel {
                    key: Set(key.to_string()),
                    value: Set(serialized),
                };
                active.insert(db).await.map_err(|e| e.to_string())?;
            },
        }

        Ok(new_state)
    }
    pub fn enabled_tool_names(&self) -> Vec<String> {
        self.tool_defs
            .keys()
            .filter(|n| self.is_enabled(n))
            .cloned()
            .collect()
    }
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<String, String> {
        // 委托给 axagent_tools
        let mut unified = axagent_tools::registry::UnifiedToolRegistry::new();
        unified.init_all();
        let input_str = input.to_string();
        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle
                .block_on(unified.execute(tool_name, &input_str))
                .map(|r| r.content)
                .map_err(|e| e.to_string())
        })
    }
}

use std::collections::BTreeMap;

#[derive(Clone)]
pub struct McpRegistry {
    mcp_tools: BTreeMap<String, McpToolConfig>,
    mcp_servers: BTreeMap<String, McpServerConfig>,
}
impl McpRegistry {
    pub fn new() -> Self {
        Self {
            mcp_tools: BTreeMap::new(),
            mcp_servers: BTreeMap::new(),
        }
    }
    #[allow(unused)]
    pub fn with_tools_and_servers(
        tools: BTreeMap<String, McpToolConfig>,
        servers: BTreeMap<String, McpServerConfig>,
    ) -> Self {
        Self {
            mcp_tools: tools,
            mcp_servers: servers,
        }
    }
    pub fn execute_mcp_tool(&self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        // 委托给 axagent_tools::registry::UnifiedToolRegistry
        let mut unified = axagent_tools::registry::UnifiedToolRegistry::new();
        for config in self.mcp_servers.values() {
            unified.mcp_servers.insert(
                config.server_id.clone(),
                axagent_tools::registry::McpServerConfig {
                    server_id: config.server_id.clone(),
                    server_name: config.server_name.clone(),
                    transport: config.transport.clone(),
                    command: config.command.clone(),
                    args_json: config.args_json.clone(),
                    env_json: config.env_json.clone(),
                    endpoint: config.endpoint.clone(),
                    execute_timeout_secs: config.execute_timeout_secs,
                    connection_pool_size: config.connection_pool_size,
                    retry_attempts: config.retry_attempts,
                    retry_delay_ms: config.retry_delay_ms,
                },
            );
        }
        for config in self.mcp_tools.values() {
            unified.mcp_tools.insert(
                config.server_name.clone(),
                axagent_tools::registry::McpToolConfig {
                    server_id: config.server_id.clone(),
                    server_name: config.server_name.clone(),
                    tool_name: config.tool_name.clone(),
                    description: config.description.clone(),
                    input_schema: config.input_schema.clone(),
                },
            );
        }
        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle
                .block_on(unified.execute_mcp(tool_name, input))
                .map(|r| r.content)
                .map_err(|e| ToolError::new(e.to_string()))
        })
    }
}
impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub use llm_bridge::{build_llm_bridge_from_db, ProviderLlmBridge};
pub use loop_detector::{
    LoopDetector, LoopDetectorConfig, LoopWarning, LoopWarningLevel, ToolCallStats,
};
pub use metrics::{
    log_with_fields, record_timing_async, MetricType, MetricValue, MetricsCollector,
    StructuredLogEntry, TimedGuard, TimingStats,
};
pub use outline_builder::{OutlineBuilder, OutlineStyle, OutlineValidationError};
pub use provider_adapter::{AxAgentApiClient, StreamEventCallback};
pub use react_engine::{
    DefaultReasoningProvider, LlmDrivenReasoningProvider, LlmReasoningProvider, ReActEngine,
    ReActError, ReActResult,
};
pub use reasoning_state::{ActionType, ReActConfig, ReasoningState};
pub use recovery_strategies::{
    RecoveryAdjustment, RecoveryAttempt, RecoveryResult, RecoveryStrategy,
};
pub use reference_builder::{ReferenceBuilder, ReferenceFormat, ReferenceFormatter};
pub use reflector::{QualityMetrics, Reflection, ReflectionConfig, Reflector, TaskExecutionRecord};
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
pub use self_verifier::{
    detect_state_change, validate_json_output, FieldChange, JsonType, JsonValidationResult,
    LlmSemanticValidator, OutputFormat, RuleBasedValidator, SelfVerifier, SemanticValidator,
    StateDiff, VerificationError, VerificationResult,
};
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
pub use task::{TaskGraph, TaskNode, TaskType};
pub use task_decomposer::{DecompositionError, DecompositionResult, LlmClient, TaskDecomposer};
pub use task_executor::{ExecutionError, ExecutionEvent, ExecutionProgress, TaskExecutor};
pub use thought_chain::{
    Action, ChainSummary, ThoughtChain, ThoughtChainEmitter, ThoughtEvent, ThoughtStep,
};
pub use trajectory_recorder::{
    ReplayComparison, ReplayResult, ReplayStep, TrajectoryRecorder, TrajectoryReplayer,
    TrajectoryStore, TrajectorySummary,
};
pub use tree_of_thoughts::{
    DefaultToTReasoningProvider, LlmReasoningProvider as ToTLlmReasoningProvider,
    ProviderAdapterBridge, ThoughtNode, ThoughtStatus, ToTStateSummary, TreeOfThoughtsEngine,
};
pub use verification_agent::VerificationAgent;
pub use web_search::{WebSearchConfig, WebSearchProvider, WebSearchProviderBuilder};

pub use ingest_pipeline::{
    Argument as IngestArgument, ConceptMention, ConnectionHint, Contradiction, EntityMention,
    GeneratedPage, IngestPipeline, IngestResult, IngestSource, IngestSourceType, PageSuggestion,
    ReviewItem, SourceAnalysis, SourceMetadata,
};
pub use ingest_queue::{FolderImportPreviewItem, IngestQueue, IngestTaskStatus, QueuedIngestTask};
pub use purpose_manager::PurposeManager;

pub use graph_insights::{
    analyze_graph, BridgeNode, GapType, GraphInsightAnalyzer, GraphInsightStats, GraphInsights,
    KnowledgeGap, SurprisingConnection,
};

pub use deep_research::{
    Contradiction as DeepResearchContradiction, CorroboratedFinding, DeepResearchConfig,
    DeepResearchResult, DeepResearcher, DeepResearcherBuilder, ResearchFinding,
    ResearchPhase as DeepResearchPhase, ResearchQuery, ResearchRound,
};

pub use relevance::{RankedPage, RelevanceConfig, RelevanceEngine};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_tool_def_fields() {
        let def = LocalToolDef {
            group_id: "g1".into(),
            group_name: "Group One".into(),
            tool_name: "tool_a".into(),
            description: "A test tool".into(),
            input_schema: serde_json::json!({"type": "object"}),
            env_json: None,
            timeout_secs: None,
        };
        assert_eq!(def.group_id, "g1");
        assert_eq!(def.group_name, "Group One");
        assert_eq!(def.tool_name, "tool_a");
        assert_eq!(def.description, "A test tool");
        assert!(def.env_json.is_none());
        assert!(def.timeout_secs.is_none());
    }

    #[test]
    fn test_local_tool_def_with_optional_fields() {
        let def = LocalToolDef {
            group_id: "g2".into(),
            group_name: "Group Two".into(),
            tool_name: "tool_b".into(),
            description: "Another tool".into(),
            input_schema: serde_json::json!({}),
            env_json: Some(r#"{"KEY":"VAL"}"#.into()),
            timeout_secs: Some(60),
        };
        assert_eq!(def.env_json.as_deref(), Some(r#"{"KEY":"VAL"}"#));
        assert_eq!(def.timeout_secs, Some(60));
    }

    #[test]
    fn test_local_tool_group_fields() {
        let group = LocalToolGroup {
            group_id: "g1".into(),
            group_name: "Group One".into(),
            enabled: true,
            tools: vec![],
        };
        assert_eq!(group.group_id, "g1");
        assert_eq!(group.group_name, "Group One");
        assert!(group.enabled);
        assert!(group.tools.is_empty());
    }

    #[test]
    fn test_local_tool_group_disabled() {
        let group = LocalToolGroup {
            group_id: "g2".into(),
            group_name: "Group Two".into(),
            enabled: false,
            tools: vec![LocalToolDef {
                group_id: "g2".into(),
                group_name: "Group Two".into(),
                tool_name: "tool_c".into(),
                description: "desc".into(),
                input_schema: serde_json::json!({}),
                env_json: None,
                timeout_secs: None,
            }],
        };
        assert!(!group.enabled);
        assert_eq!(group.tools.len(), 1);
    }

    #[test]
    fn test_local_tool_registry_init() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(!registry.flat_tools.is_empty() || !registry.tool_defs.is_empty());
    }

    #[test]
    fn test_local_tool_registry_is_enabled_existing() {
        let registry = LocalToolRegistry::init_from_registry();
        if let Some(tool_name) = registry.tool_defs.keys().next() {
            assert!(registry.is_enabled(tool_name));
        }
    }

    #[test]
    fn test_local_tool_registry_is_enabled_nonexistent() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(!registry.is_enabled("nonexistent_tool_xyz_12345"));
    }

    #[test]
    fn test_local_tool_registry_contains_existing() {
        let registry = LocalToolRegistry::init_from_registry();
        if let Some(tool_name) = registry.tool_defs.keys().next() {
            assert!(registry.contains(tool_name));
        }
    }

    #[test]
    fn test_local_tool_registry_contains_nonexistent() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(!registry.contains("no_such_tool_at_all"));
    }

    #[test]
    fn test_local_tool_registry_all_tool_defs() {
        let registry = LocalToolRegistry::init_from_registry();
        let defs = registry.all_tool_defs();
        assert!(!defs.is_empty());
        for (name, def) in defs {
            assert_eq!(name, &def.tool_name);
        }
    }

    #[test]
    fn test_local_tool_registry_all_tool_names() {
        let registry = LocalToolRegistry::init_from_registry();
        let names = registry.all_tool_names();
        assert!(!names.is_empty());
        for name in &names {
            assert!(registry.tool_defs.contains_key(name));
        }
    }

    #[test]
    fn test_local_tool_registry_get_group_id_existing() {
        let registry = LocalToolRegistry::init_from_registry();
        if let Some(tool_name) = registry.tool_defs.keys().next() {
            let gid = registry.get_group_id(tool_name);
            assert!(gid.is_some());
        }
    }

    #[test]
    fn test_local_tool_registry_get_group_id_nonexistent() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(registry.get_group_id("nonexistent_tool").is_none());
    }

    #[test]
    fn test_local_tool_registry_get_tool_groups() {
        let registry = LocalToolRegistry::init_from_registry();
        let groups = registry.get_tool_groups();
        assert!(!groups.is_empty());
        for g in &groups {
            assert!(!g.group_id.is_empty());
            assert!(!g.tools.is_empty());
        }
    }

    #[test]
    fn test_local_tool_registry_get_tool_groups_sorted() {
        let registry = LocalToolRegistry::init_from_registry();
        let groups = registry.get_tool_groups();
        let ids: Vec<&str> = groups.iter().map(|g| g.group_id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn test_local_tool_registry_get_enabled_chat_tools() {
        let registry = LocalToolRegistry::init_from_registry();
        let chat_tools = registry.get_enabled_chat_tools();
        assert!(!chat_tools.is_empty());
        for ct in &chat_tools {
            assert_eq!(ct.r#type, "function");
            assert!(!ct.function.name.is_empty());
        }
    }

    #[test]
    fn test_local_tool_registry_set_env_json() {
        let mut registry = LocalToolRegistry::init_from_registry();
        if let Some(tool_name) = registry.tool_defs.keys().next().cloned() {
            registry.set_env_json(&tool_name, r#"{"API_KEY":"test"}"#.into());
            assert_eq!(
                registry
                    .tool_defs
                    .get(&tool_name)
                    .unwrap()
                    .env_json
                    .as_deref(),
                Some(r#"{"API_KEY":"test"}"#)
            );
        }
    }

    #[test]
    fn test_local_tool_registry_set_env_json_nonexistent() {
        let mut registry = LocalToolRegistry::init_from_registry();
        registry.set_env_json("nonexistent_tool", r#"{"KEY":"VAL"}"#.into());
        assert!(!registry.tool_defs.contains_key("nonexistent_tool"));
    }

    #[test]
    fn test_local_tool_registry_set_timeout_secs() {
        let mut registry = LocalToolRegistry::init_from_registry();
        if let Some(tool_name) = registry.tool_defs.keys().next().cloned() {
            registry.set_timeout_secs(&tool_name, 120);
            assert_eq!(registry.tool_defs.get(&tool_name).unwrap().timeout_secs, Some(120));
        }
    }

    #[test]
    fn test_local_tool_registry_set_timeout_secs_nonexistent() {
        let mut registry = LocalToolRegistry::init_from_registry();
        registry.set_timeout_secs("no_such_tool", 99);
        assert!(!registry.tool_defs.contains_key("no_such_tool"));
    }

    #[test]
    fn test_local_tool_registry_enabled_tool_names() {
        let registry = LocalToolRegistry::init_from_registry();
        let enabled = registry.enabled_tool_names();
        assert!(!enabled.is_empty());
        for name in &enabled {
            assert!(registry.is_enabled(name));
        }
    }

    #[test]
    fn test_mcp_registry_new() {
        let reg = McpRegistry::new();
        assert!(reg.mcp_tools.is_empty());
        assert!(reg.mcp_servers.is_empty());
    }

    #[test]
    fn test_mcp_registry_default() {
        let reg = McpRegistry::default();
        assert!(reg.mcp_tools.is_empty());
        assert!(reg.mcp_servers.is_empty());
    }

    #[test]
    fn test_mcp_registry_with_tools_and_servers() {
        let mut tools = BTreeMap::new();
        let mut servers = BTreeMap::new();
        tools.insert(
            "t1".into(),
            McpToolConfig {
                server_id: "s1".into(),
                server_name: "server1".into(),
                tool_name: "tool1".into(),
                description: Some("desc".into()),
                input_schema: Some(serde_json::json!({})),
            },
        );
        servers.insert(
            "s1".into(),
            McpServerConfig {
                server_id: "s1".into(),
                server_name: "server1".into(),
                transport: "stdio".into(),
                command: Some("echo".into()),
                args_json: None,
                env_json: None,
                endpoint: None,
                execute_timeout_secs: Some(30),
                connection_pool_size: None,
                retry_attempts: None,
                retry_delay_ms: None,
            },
        );
        let reg = McpRegistry::with_tools_and_servers(tools, servers);
        assert_eq!(reg.mcp_tools.len(), 1);
        assert_eq!(reg.mcp_servers.len(), 1);
    }

    #[test]
    fn test_local_tool_registry_enabled_map_populated() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(!registry.enabled.is_empty());
        for (gid, is_enabled) in &registry.enabled {
            assert!(*is_enabled, "Group {} should be enabled by default", gid);
        }
    }

    #[test]
    fn test_local_tool_registry_group_names_populated() {
        let registry = LocalToolRegistry::init_from_registry();
        assert!(!registry.group_names.is_empty());
        for (gid, gname) in &registry.group_names {
            assert!(!gname.is_empty(), "Group name for {} should not be empty", gid);
        }
    }

    #[test]
    fn test_local_tool_registry_tool_defs_have_required_fields() {
        let registry = LocalToolRegistry::init_from_registry();
        for (name, def) in &registry.tool_defs {
            assert_eq!(name, &def.tool_name);
            assert!(!def.group_id.is_empty(), "tool {} missing group_id", name);
            assert!(!def.description.is_empty(), "tool {} missing description", name);
        }
    }

    #[test]
    fn test_local_tool_def_clone() {
        let def = LocalToolDef {
            group_id: "g1".into(),
            group_name: "Group One".into(),
            tool_name: "tool_a".into(),
            description: "desc".into(),
            input_schema: serde_json::json!({"type": "object"}),
            env_json: Some("env".into()),
            timeout_secs: Some(30),
        };
        let cloned = def.clone();
        assert_eq!(cloned.group_id, def.group_id);
        assert_eq!(cloned.tool_name, def.tool_name);
        assert_eq!(cloned.env_json, def.env_json);
        assert_eq!(cloned.timeout_secs, def.timeout_secs);
    }

    #[test]
    fn test_local_tool_group_clone() {
        let group = LocalToolGroup {
            group_id: "g1".into(),
            group_name: "Group One".into(),
            enabled: true,
            tools: vec![],
        };
        let cloned = group.clone();
        assert_eq!(cloned.group_id, group.group_id);
        assert_eq!(cloned.enabled, group.enabled);
    }
}
