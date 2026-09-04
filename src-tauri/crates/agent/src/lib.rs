// SPDX-License-Identifier: AGPL-3.0-only

//! AxAgent Agent — 公共 API 通过 `pub use` 重导出定义。
//! `pub mod` 模块为内部实现，外部调用者应优先使用重导出路径。
//!
//! # 公共 API 边界
//! 下方 `pub mod` 中仅 16 个模块被外部引用（标注 `// external`），
//! 其余为 crate 内部模块，仅因集成测试需要而保持 `pub`。
//! 重导出见 `// ── Public API re-exports ──` 区块。

// ── 模块声明 (internal / external as noted) ────────────────────────────

pub mod academic_search;
pub mod action_executor;
pub mod agent_round_adapter;
pub mod checkpoint;
pub mod citation_tracker;
pub mod content_synthesizer;
pub mod context_contributors;
pub mod context_files;
pub mod context_window;
pub mod credibility_evaluator;
pub mod cycle_detector;
pub mod deep_research;
pub mod environment_probe;
pub mod multi_agent_hook;
pub mod shared_blackboard;
// error_classifier merged into recovery_strategies
pub mod error_recovery_engine;
pub mod evaluator;
pub mod event_bus;
pub mod event_emitter;
pub mod experience_pipeline;
pub mod fact_checker;
pub mod fallback_adapter;
pub mod feedback_orchestrator;
pub mod fine_tune;
pub mod frontend_adapter;
pub mod goal_evaluator;
pub mod graph_insights;
pub mod guardrails;
pub mod harness_adapter;
pub mod health_checker;
pub mod hierarchical_planner;
pub mod ingest_pipeline;
pub mod ingest_queue;
pub mod insight_generator;
pub mod interrupt;
pub mod ir_renderer;
pub mod lint_checker;
pub mod llm_bridge;
pub mod llm_dispatcher;
pub mod metrics;
pub mod noop_kit;
pub mod outline_builder;
pub mod personality;
pub mod pre_validator;
pub mod project_memory;
pub mod provider_adapter;
pub mod provider_fallback;
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
pub mod self_improvement_executor;
pub mod self_verifier;
pub mod session_manager;
pub mod shadow_fs;
pub mod source_classifier;
pub mod source_validator;
pub mod steer_manager;
pub mod task;
pub mod task_decomposer;
pub mod task_executor;
pub mod think_scrubber;
pub mod thought_chain;
pub mod tool_recommender;
pub mod trajectory_recorder;
pub mod tree_of_thoughts;
pub mod verification_agent;
pub mod vision_pipeline;
pub mod web_search;
pub mod wiki_compiler;

// ── Public API re-exports ─────────────────────────────────────────────
// 仅重导出外部实际引用的类型（~55 个），不暴露内部实现细节。

// session_manager — 外部引用：app_state, state/*, init/*, commands/*
pub use session_manager::{
    AgentSession, ChannelPermissionPrompter, SelfImprovementFlags, SessionManager,
    TauriHookProgressReporter,
};

// shared_blackboard — 外部引用：consumer crate 通过 harness trait 操控多 Agent 协作
pub use shared_blackboard::BlackboardHandle;

// reflector — 外部引用：commands/reflection, init/state
pub use reflector::{Reflection, Reflector, TaskExecutionRecord};

// project_memory — 外部引用：commands/conversations/streaming(文件级记忆按任务相关性检索)
pub use project_memory::{
    MemoryCategory, MemoryFileEntry, MemoryIndex, MemorySearchResult, ProjectMemory,
};

// provider_adapter — 外部引用：commands/agent, commands/plan
pub use provider_adapter::{AxAgentApiClient, StreamEventCallback};

// fallback_adapter — 外部引用：commands/agent
pub use fallback_adapter::FallbackProviderAdapter;

// llm_dispatcher — 外部引用：commands/fleet
pub use llm_dispatcher::LlmDispatcher;

// llm_bridge — 外部引用：runtime/llm_bridge
pub use llm_bridge::ProviderLlmBridge;

// recovery_strategies — 外部引用：runtime/error_recovery, commands/*
pub use recovery_strategies::{
    ClassifiedError, ErrorClassifier, ErrorType, FailoverReason, RecoveryAdjustment,
    RecoveryAttempt, RecoveryResult, RecoveryStrategy,
};

// hierarchical_planner — 外部引用：commands/plan, commands/dojo_sdk (G19 Plan 三件套)
pub use hierarchical_planner::{
    ActionType, HierarchicalPlanner, Phase, PhaseStatus, Plan, PlanStatus, PlannedTask, TaskStatus,
};

// insight_generator — 外部引用：commands/reflection
pub use insight_generator::{Insight, InsightCategory, InsightStats};

// rl_optimizer — 外部引用：commands/rl
pub use rl_optimizer::{Policy, PolicyType, ThresholdScheduler, TrainingStats};

// experience_pipeline — 外部引用：commands/rl, init
pub use experience_pipeline::{ExperiencePipeline, PipelineStats};

// feedback_orchestrator — 外部引用：commands/rl, init
pub use feedback_orchestrator::{
    FeedbackBatchResult, FeedbackOrchestrator, FeedbackRecordInput, OrchestratorAction,
    OrchestratorStats, classify_feedback_rating,
};

// vision_pipeline — 外部引用：commands/screen_vision
pub use vision_pipeline::{VisionPipeline, VisionResult, VisionTask};

// personality — 外部引用：commands/personality
pub use personality::{Personality, PersonalityManager};

// tool_recommender — 外部引用：commands/tool_recommender
pub use tool_recommender::{
    ContextAnalyzer, ToolRecommendation, ToolRecommender,
    patterns::{UsagePattern, UsagePatternDB},
};

// evaluator — 外部引用：commands/evaluator
pub use evaluator::{
    Benchmark, BenchmarkReport, BenchmarkResult, BenchmarkSuite, Dataset, DatasetLoader,
    DatasetMetadata, DatasetRegistry, EvaluationRunner, ReportGenerator, RunnerConfig,
};

// fine_tune — 外部引用：commands/fine_tune
pub use fine_tune::{
    ActiveModelConfig, BaseModelInfo, FineTuneTrainer, ModelManager, TrainingJob,
    lora::{LoRAAdapterInfo, LoRAConfig, LoRAConfigBuilder},
    trainer::TrainingStats as FineTuneTrainingStats,
};

// deep_research — 外部引用：commands/research
pub use deep_research::{
    Contradiction, CorroboratedFinding, DeepResearchConfig, DeepResearchResult,
    DeepResearchSearchItem, DeepResearcher, DeepResearcherBuilder, ResearchFinding, ResearchPhase,
    ResearchQuery, ResearchRound,
};

// provider_fallback — 外部引用：runtime/llm_bridge, commands/provider
pub use provider_fallback::{
    FallbackConfig, ProviderEntry, ProviderFallbackManager, ProviderHealthSummary, ProviderTier,
};

// multi_agent_hook — 外部引用：init/services 注册 PluginHook
pub use multi_agent_hook::{
    MultiAgentTriggerConfig, MultiAgentTriggerHook, create_multi_agent_trigger_hook,
    create_multi_agent_trigger_hook_with_config,
};

// guardrails — 外部引用：runtime-core 调用护栏
pub use guardrails::{
    GuardrailDecision, GuardrailThresholds, ToolCallGuardrailController, ToolStatsSnapshot,
};

// think_scrubber — 外部引用：gateway 清理思考链
pub use think_scrubber::{ScrubberConfig, ThinkScrubber};

// 模块级引用 — 外部引用：commands/llm_wiki（模块已是 pub mod，无需重复 pub use）

// tree_of_thoughts — 外部引用：commands/agent（ToT 求解器）
pub use tree_of_thoughts::{
    DefaultToTReasoningProvider, LlmReasoningProvider, TreeOfThoughtsEngine,
};

// self_improvement_executor — 外部引用：agent_round_adapter / reasoning_state / harness 自改进循环
pub use self_improvement_executor::{FinalOutput, SelfImprovementConfig, SelfImprovementExecutor};

// agent_round_adapter — 外部引用：wiring 层包装 ReActEngine 进入自改进循环
pub use agent_round_adapter::AgentRoundAdapter;

// runtime-core 类型透传 — 外部引用：commands/agent

/// 清理 AI 输出内容：去除尾部空白、控制字符等。
pub fn clean_output(content: &str) -> String {
    content.trim_end().to_string()
}
