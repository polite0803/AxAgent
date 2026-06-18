// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory learning crate for claw-code
//!
//! Provides research-grade trajectory learning capabilities including:
//! - Trajectory recording and storage
//! - Batch trajectory generation
//! - RL reward signal computation
//! - Skill optimization closed-loop
//! - Cross-session pattern learning

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::non_canonical_partial_ord_impl)]
#![allow(clippy::manual_strip)]

mod adaptation;
mod arch_search;
mod auto_memory;
mod auto_tool;
mod batch;
mod behavior_learner;
mod behavior_tracker;
mod coevolution;
mod compactor;
mod constitution;
mod context;
mod context_predictor;
mod dream_consolidation;
mod dream_data_provider;
mod fts5;
mod insight;
mod intrinsic_reward;
mod memory;
mod memory_provider;
mod memory_providers;
mod nudge;
mod parallel_execution;
mod pattern;
mod pattern_analyzer;
mod preference_learner;
mod proactive_assistant;
mod process_reward;
mod reminder_manager;
mod rl;
mod rl_trainer;
mod sandbox_executor;
mod skill;
mod skill_decomposition;
mod skill_evolution;
mod skill_manager;
mod skill_matcher;
mod skill_proposal;
mod skills_hub_adapter;
mod skills_hub_client;
mod storage;
mod style_applier;
mod style_extractor;
mod style_migrator;
mod style_vectorizer;
mod sub_agent;
mod suggestion_engine;
mod task_prefetcher;
mod text_grad;
mod training_env;
mod trajectory;
mod trajectory_compressor;
mod user_profile;

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

pub use intrinsic_reward::{IntrinsicMotivationConfig, IntrinsicMotivationEngine};

pub use memory_providers::closed_loop::{
    AutoAction, ClosedLoopConfig, ClosedLoopService, PeriodicNudge, SkillUpgradeProposal,
};
pub use memory_providers::entity::{Entity, EntityType, Relationship, RelationshipType};
pub use memory_providers::service::{
    AddMemoryRequest, MemoryNature, MemoryProvenance, MemoryService, MemoryTier,
};

pub use memory_provider::{
    MemoryEntry, MemoryProvider, MemoryProviderRegistry, MemoryQuery, MemoryQueryResult,
    MemoryType as MemoryProviderType,
};

pub use nudge::{NudgeAction, NudgeCandidate, NudgeContext, NudgeEntity, NudgeService, Urgency};

pub use parallel_execution::{
    ExecutionResult, ExecutionStrategy, ParallelExecution, ParallelExecutionService,
    ParallelExecutionVerifier, ParallelTask, VerificationConfig, VerificationResult,
};

pub use pattern::{CrossSessionLearner, PatternConfig, PatternLearner, PatternType};

pub use preference_learner::{LearningMetrics, PreferenceLearner};

pub use process_reward::ProcessRewardModel;

pub use proactive_assistant::{
    ContextPrediction, PredictedIntent, Priority, ProactiveAssistant, ProactiveConfig,
    ProactiveSuggestion, RecurrenceFrequency, Reminder, ReminderRecurrence, SuggestionAction,
    SuggestionType as ProactiveSuggestionType,
};

pub use rl::{RLConfig, RLEngine, RewardNormalizer, RewardWeights};

pub use rl_trainer::{RLTrainer, TrainingEpisode, TrainingReport};

pub use sandbox_executor::SkillSandboxExecutor;

pub use skill::{HermesMetadata, Skill, SkillMetadata, SkillProposal};

pub use skill_decomposition::{
    CompositeSkillData, DecompositionResult, SkillDecomposer, ToolResolver,
};

pub use skill_evolution::SkillEvolutionEngine;

pub use skill_matcher::{Complexity, estimate_complexity_public};

pub use skill_proposal::SkillProposalService;

pub use skills_hub_adapter::SkillsHubAdapter;

pub use skills_hub_client::{SkillsHubClient, SkillsHubConfig, SkillsHubSearchResult};

pub use storage::{
    TrajectoryCleanupConfig, TrajectoryCleanupTask, TrajectoryStatistics, TrajectoryStorage,
};

pub use style_applier::StyleApplier;

pub use style_extractor::StyleExtractor;

pub use style_vectorizer::{CodeSample, MessageSample, StyleVectorizer};

pub use sub_agent::SubAgentRegistry;

pub use suggestion_engine::SuggestionEngine;

pub use task_prefetcher::{PrefetchResult, PrefetchResults, PrefetchType, TaskPrefetcher};

pub use text_grad::{ComputationGraph, TextGradConfig, TextGradEngine};

pub use trajectory::{
    ExportFormat, MessageRole, RLTrainingEntry, ToolCall, ToolResult, Trajectory,
    TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern, TrajectoryQuality,
    TrajectoryQuery, TrajectoryStep,
};

pub use user_profile::{ExpertiseLevel, UserProfile};
