// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory learning crate for claw-code
//!
//! Provides research-grade trajectory learning capabilities including:
//! - Trajectory recording and storage
//! - Batch trajectory generation
//! - RL reward signal computation
//! - Skill optimization closed-loop
//! - Cross-session pattern learning

// dead_code 策略(2026-08-29): 按项目规范禁止用 allow 标记绕过死代码检查,
//   未接入代码一律删除。本 crate 已移除 crate 级与全部模块级压制。
#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::non_canonical_partial_ord_impl)]
#![allow(clippy::manual_strip)]

mod adaptation;
mod auto_memory;
mod auto_tool;
mod batch;
mod coevolution;
mod compactor;
mod constitution;
mod context;
mod context_predictor;
mod dream_consolidation;
mod dream_data_provider;
mod evidence;
mod fts5;
mod insight;
mod intrinsic_reward;
mod learning_graph;
mod memory;
mod memory_provider;
mod memory_providers;
mod nudge;
pub mod numeric_evolution;
mod parallel_execution;
mod pattern;
mod proactive_assistant;
mod process_reward;
mod reminder_manager;
mod replay;
mod rl;
mod sandbox_executor;
mod skill;
mod skill_decomposition;
pub mod skill_evolution;
mod skill_learning;
mod skill_proposal;
mod skills_hub_adapter;
mod storage;
mod style_applier;
mod style_extractor;
mod style_vectorizer;
mod sub_agent;
mod suggestion_engine;
mod task_prefetcher;
mod text_grad;
mod trajectory;
mod trajectory_impl;
mod user_profile;
mod workflow_adapters;
mod workflow_evolution_tick;
// ── Fleet 持久化实现 ──
mod fleet_repository;

// ── Explicit re-exports (only types used externally) ──────────────────

pub use adaptation::{
    ContentFormat, FeedbackSignal, FeedbackSource, FeedbackType, RealTimeLearning, TechnicalLevel,
    Verbosity,
};

pub use auto_memory::{AutoMemoryExtractor, ExtractedMemory, MemoryType};

pub use auto_tool::{
    AutoToolCreator, AutoToolCreatorConfig, DefaultLlmToolProvider, DefaultSandboxToolTester,
    slugify,
};

pub use batch::{
    BatchAnalysis, BatchConfig, BatchProcessor, BatchResult, PatternStat, QualityDistribution,
    SamplingStrategy,
};

pub use coevolution::{CoevolutionConfig, CoevolutionEnvironment, DifficultyLevel};

pub use compactor::{
    IntegrityCheck, IntegrityCheckResult, MessageRecord, SessionCompactor,
    verify_compression_integrity,
};

pub use constitution::{
    ConstitutionConfig, ConstitutionalRule, ImmutableConstitution, ViolationSeverity,
};

pub use context::{ContextAssembler, TokenBudget};

pub use context_predictor::{ContextFeatures, ContextPredictor, PredictionResult};

pub use dream_consolidation::{
    ConsolidationDataProvider, ConsolidationSuggestion, ContrastivePair, DistilledKnowledge,
    DreamConsolidationConfig, DreamConsolidationResult, DreamConsolidationState, DreamConsolidator,
    DreamEventEmitter, ExperienceRecord, KnowledgeType, ReplaySample, SuggestionType,
};

pub use dream_data_provider::TrajectoryDreamDataProvider;

pub use fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};

pub use insight::{InsightCategory, LearningInsight, LearningInsightSystem};

pub use learning_graph::{
    CategoryCount, GraphEdge, GraphNode, GraphStats, LearningGraph, NodeKind, build_learning_graph,
};

pub use intrinsic_reward::{IntrinsicMotivationConfig, IntrinsicMotivationEngine};

pub use memory_providers::closed_loop::{
    AutoAction, ClosedLoopConfig, ClosedLoopService, PeriodicNudge, SkillUpgradeProposal,
};
pub use memory_providers::entity::{Entity, EntityType, Relationship, RelationshipType};
pub use memory_providers::service::{
    AddMemoryRequest, MemoryNature, MemoryProvenance, MemoryService, MemoryTier,
};
// G21: MemoryHookProvider — 会话生命周期记忆同步 Hook
pub use memory_providers::memory_hook_provider::{MemoryHookConfig, MemoryHookProvider};

pub use nudge::{NudgeAction, NudgeCandidate, NudgeContext, NudgeEntity, NudgeService, Urgency};

pub use parallel_execution::{
    ExecutionResult, ExecutionStrategy, ParallelExecution, ParallelExecutionService,
    ParallelExecutionVerifier, ParallelTask, VerificationConfig, VerificationResult,
};

pub use pattern::{CrossSessionLearner, PatternConfig, PatternLearner, PatternType};

pub use process_reward::ProcessRewardModel;

pub use reminder_manager::{
    ReminderError, ReminderManager, ReminderManagerConfig, ReminderNotification, ReminderSchedule,
};

pub use proactive_assistant::{
    ContextPrediction, PredictedIntent, Priority, ProactiveAssistant, ProactiveConfig,
    ProactiveSuggestion, RecurrenceFrequency, Reminder, ReminderRecurrence, SuggestionAction,
    SuggestionType as ProactiveSuggestionType,
};

pub use rl::{RLConfig, RLEngine, RewardNormalizer, RewardWeights};

pub use sandbox_executor::SkillSandboxExecutor;

pub use skill::{HermesMetadata, Skill, SkillMetadata, SkillProposal};

pub use skill_learning::{
    ApprovalStatus, BackgroundReviewResult, DangerousPattern, ErrorCorrection,
    PendingOperationType, PendingSkillOperation, ReviewMessage, RiskLevel, SafetyCheckResult,
    SkillLearnEvent, SkillLearningConfig, SkillLearningManager, SkillSafetyGuard,
};

pub use skill_decomposition::{
    CompositeSkillData, DecompositionResult, SkillDecomposer, ToolResolver,
};

pub use skill_evolution::EvolutionConfig;
pub use skill_evolution::SkillEvolutionEngine;

pub use evidence::{
    DecisionEvidence, EvidenceOutcome, EvolutionDecider, EvolutionDecision, SkillPosterior,
};
pub use numeric_evolution::{
    NumericEvolutionEngine, NumericEvolutionStats, NumericGenome, ParamDef,
};

pub use skill_proposal::SkillProposalService;

pub use skills_hub_adapter::SkillsHubAdapter;

pub use storage::{
    TrajectoryCleanupConfig, TrajectoryCleanupTask, TrajectorySession, TrajectoryStatistics,
    TrajectoryStorage,
};

// ── Fleet 持久化实现 ──
pub use fleet_repository::SeaOrmFleetRepository;

// ── ReplayExecutor 实现 ──
pub use replay::TrajectoryReplayer;

pub use style_applier::StyleApplier;

pub use style_extractor::{
    CommentStyle, DocumentFormat, DocumentStyleProfile, ExtractedCodePatterns,
    FormattingPreferences, IndentStyle, NamingPattern, NamingPatternType, StyleExtractor,
};

pub use style_vectorizer::{CodeSample, MessageSample, StyleVectorizer};

pub use sub_agent::{SubAgent, SubAgentMetadata, SubAgentRegistry, SubAgentStatus};

pub use suggestion_engine::SuggestionEngine;

pub use task_prefetcher::{PrefetchResult, PrefetchResults, PrefetchType, TaskPrefetcher};

pub use text_grad::{ComputationGraph, ComputationNode, NodeType, TextGradConfig, TextGradEngine};

pub use trajectory::{
    ExportFormat, MessageRole, RLTrainingEntry, ToolCall, Trajectory, TrajectoryExportOptions,
    TrajectoryOutcome, TrajectoryPattern, TrajectoryQuality, TrajectoryQuery, TrajectoryStep,
    TrajectoryToolResult,
};

pub use user_profile::{
    CommentStyle as ProfileCommentStyle, DetailLevel, ExpertiseLevel, ExplanationDepth,
    IndentationStyle as ProfileIndentStyle, NamingConvention, ProfileUpdate, Tone, UpdateSource,
    UserProfile,
};

// ── Extension methods migrated from harness ──────────────────────────

pub use trajectory_impl::{ReplayContextExt, RewardCategoryExt, TrajectoryBuilderExt};

// ── 工作流反思/进化/优化三层 trait 实现(阶段 4) ──────────────────────

pub use workflow_adapters::{
    ReflectorConfig, WorkflowEvolverImpl, WorkflowOptimizerImpl, WorkflowReflectorImpl,
};
pub use workflow_evolution_tick::{
    EvolutionTickConfig, EvolutionTickReport, run_tick_for_template, start_workflow_evolution_tick,
    tick_once,
};
