//! Trajectory learning crate for claw-code
//!
//! Provides research-grade trajectory learning capabilities including:
//! - Trajectory recording and storage
//! - Batch trajectory generation
//! - RL reward signal computation
//! - Skill optimization closed-loop
//! - Cross-session pattern learning

#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::non_canonical_partial_ord_impl)]
#![allow(clippy::manual_strip)]

mod adaptation;
mod auto_memory;
mod batch;
mod behavior_learner;
mod behavior_tracker;
mod compactor;
mod context;
mod context_predictor;
mod dream_consolidation;
mod fts5;
mod hooks;
mod insight;
mod memory;
mod memory_provider;
mod memory_providers;
mod nudge;
mod parallel_execution;
mod pattern;
mod pattern_analyzer;
mod platform_integration;
mod preference_learner;
mod proactive_assistant;
mod reminder_manager;
mod rl;
mod rl_trainer;
mod scheduled_task;
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
mod training_env;
mod trajectory;
mod trajectory_compressor;
mod user_profile;

pub use adaptation::*;
pub use auto_memory::*;
pub use batch::*;
pub use behavior_learner::*;
pub use behavior_tracker::*;
pub use compactor::{
    verify_compression_integrity, IntegrityCheck, IntegrityCheckResult, MessageRecord,
    SessionCompactor,
};
pub use context::*;
pub use dream_consolidation::{
    DreamConsolidationConfig, DreamConsolidationResult, DreamConsolidationState, DreamConsolidator,
    DreamEventEmitter,
};
pub use fts5::*;
pub use hooks::*;
pub use insight::*;
pub use memory::*;
pub use memory_provider::{
    MemoryEntry, MemoryProvider, MemoryProviderRegistry, MemoryQuery, MemoryQueryResult, MemoryType,
};
pub use memory_providers::*;
pub use nudge::{
    Nudge, NudgeAction, NudgeCandidate, NudgeConfig, NudgeContext, NudgeEntity, NudgeMessage,
    NudgeService, NudgeSession, NudgeType, Urgency,
};
pub use parallel_execution::*;
pub use platform_integration::*;
pub use preference_learner::*;
pub use rl::*;
pub use rl_trainer::{RLTrainer, TrainingEpisode, TrainingReport};
pub use scheduled_task::*;
pub use skill::*;
pub use skill_decomposition::*;
pub use skill_evolution::*;
pub use skill_manager::*;
pub use skill_matcher::*;
pub use skill_proposal::*;
pub use skills_hub_adapter::{
    HermesCommand, HermesExample, HermesParameter, HermesSkillManifest, SkillsHubAdapter,
};
pub use skills_hub_client::{
    SkillsHubClient, SkillsHubConfig, SkillsHubSearchResult, SkillsHubSkill,
};
pub use storage::*;
pub use style_applier::{
    CodeStyleTemplate, CodeTemplate, StyleApplier, StylePattern, StylePatternType,
};
pub use style_extractor::{
    DocumentStyleProfile, ExtractedCodePatterns, FormattingPreferences, IndentStyle, LineEnding,
    StyleExtractor,
};
pub use style_migrator::*;
pub use style_vectorizer::{
    CodeSample, MessageSample, StyleDimensions, StyleVector, StyleVectorizer,
};
pub use sub_agent::*;
pub use training_env::{EvaluationResult, RewardComputation, TrainingEnv};
pub use trajectory::*;
pub use trajectory_compressor::{CompressedStep, CompressedToolCall, TrajectoryCompressor};

pub use context_predictor::{
    ActionType, ActivityLevel, ContextFeatures, ContextPredictor, PatternMatch, PredictionResult,
    PredictionRule,
};
pub use pattern::{
    CrossSessionInsight, CrossSessionLearner, DetectedPattern, PatternConfig, PatternLearner,
    PatternStatistics, PatternType,
};
pub use pattern_analyzer::{
    CodingPatternMatch, ExtractedPatterns, PatternAnalyzer, TemporalPattern, ToolPreferencePattern,
    TopicPattern,
};
pub use proactive_assistant::{
    CapabilityType, ContextPrediction, ContextWindow, PredictedIntent, Priority, ProactiveAction,
    ProactiveAssistant, ProactiveCapability, ProactiveConfig, ProactiveSuggestion,
    RecurrenceFrequency, Reminder, ReminderRecurrence, SuggestionAction, SuggestionType,
    TriggerCondition, TriggerConditionType,
};
pub use reminder_manager::{
    ReminderError, ReminderManager, ReminderManagerConfig, ReminderNotification, ReminderSchedule,
};
pub use suggestion_engine::{
    CodingStylePreference, CommunicationStylePreference, CommunicationTone, DetailLevel,
    DocumentationLevel, SuggestionEngine, SuggestionEngineConfig, UserPreferenceProfile,
    WorkHabitPreference,
};
pub use task_prefetcher::{
    PrefetchResult, PrefetchResults, PrefetchType, PrefetcherConfig, TaskPrefetcher,
};
pub use user_profile::{
    calculate_confidence, ExpertiseLevel, ProfileUpdate, UpdateSource, UserProfile,
};

pub mod prelude {
    pub use crate::auto_memory::{
        AutoMemoryExtractor, ExtractedMemory, MemoryExtractionResult, MemoryType,
    };
    pub use crate::batch::{
        BatchAnalysis, BatchConfig, BatchProcessor, BatchResult, PatternStat, QualityDistribution,
        SamplingStrategy,
    };
    pub use crate::fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};
    pub use crate::memory::{
        AutoAction, ClosedLoopConfig, ClosedLoopService, Entity, EntityType, GraphQuery,
        MemoryActionResult, MemoryConfig, MemoryEntry, MemoryRegistry, MemoryService, MemoryUsage,
        NudgeCandidate, PeriodicNudge, Relationship, RelationshipType, SearchResult,
        SkillUpgradeProposal, WorkingMemory,
    };
    pub use crate::parallel_execution::{
        ExecutionResult, ExecutionStatus, ExecutionStrategy, ParallelExecution,
        ParallelExecutionService, ParallelTask, TaskResultSummary, TaskStatus,
    };
    pub use crate::pattern::{
        CrossSessionInsight, CrossSessionLearner, DetectedPattern, PatternConfig, PatternLearner,
        PatternStatistics, PatternStep, PatternType,
    };
    pub use crate::platform_integration::{
        DiscordHandler, DiscordMessage, MessagePlatform, OutgoingMessage, PlatformConfig,
        PlatformIntegrationService, PlatformMessage, PlatformSession, TelegramHandler,
        TelegramMessage,
    };
    pub use crate::rl::{RLConfig, RLEngine, RLState, RewardNormalizer, RewardWeights};
    pub use crate::scheduled_task::{
        DailySummaryConfig, ScheduledTask, ScheduledTaskService, ScheduledTaskStatus,
        SummaryFormat, TaskConfig, TaskDefinition, TaskRunResult, TaskType,
    };
    pub use crate::skill::{
        EvolutionOutcome, HermesMetadata, Impact, MetricsDelta, ModificationType, Skill,
        SkillAnalysis, SkillConfig, SkillContext, SkillCreator, SkillEvolution, SkillExecution,
        SkillMetadata, SkillModification, SkillOptimizer, SkillOutcome, SkillProposal,
        SkillReference, TaskComplexity, ValidationResult,
    };
    pub use crate::skill_proposal::{create_skill_from_proposal, SkillProposalService};
    pub use crate::storage::{TrajectoryStatistics, TrajectoryStorage};
    pub use crate::sub_agent::{
        AgentMailbox, AgentMessage, AgentMessageError, AgentMessageKind, MessageBus, SubAgent,
        SubAgentMetadata, SubAgentQuery, SubAgentRegistry, SubAgentResult, SubAgentStatus,
        TaskDeduplicator,
    };
    pub use crate::trajectory::{
        ExportFormat, MessageRole, RLTrainingEntry, RewardSignal, RewardType, ToolCall, ToolResult,
        Trajectory, TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern,
        TrajectoryQuality, TrajectoryQuery, TrajectoryStep,
    };
}
