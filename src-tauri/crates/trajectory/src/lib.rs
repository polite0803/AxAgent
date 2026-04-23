//! Trajectory learning crate for claw-code
//!
//! Provides research-grade trajectory learning capabilities including:
//! - Trajectory recording and storage
//! - Batch trajectory generation
//! - RL reward signal computation
//! - Skill optimization closed-loop
//! - Cross-session pattern learning

mod auto_memory;
mod batch;
mod fts5;
mod memory;
mod nudge;
mod pattern;
mod parallel_execution;
mod scheduled_task;
#[allow(dead_code)]
mod retrieval;
mod insight;
mod context;
mod compactor;
mod adaptation;
mod rl;
mod skill;
mod skill_evolution;
mod skill_manager;
mod skill_matcher;
mod skill_proposal;
mod platform_integration;
mod user_profile;
#[allow(dead_code)]
mod chat_memory;
mod hooks;
mod storage;
mod sub_agent;
mod trajectory;

pub use auto_memory::*;
pub use batch::*;
pub use fts5::*;
pub use memory::*;
pub use pattern::*;
pub use insight::*;
pub use context::*;
pub use adaptation::*;
pub use rl::*;
pub use skill::*;
pub use skill_evolution::*;
pub use skill_manager::*;
pub use skill_matcher::*;
pub use skill_proposal::*;
pub use platform_integration::*;
pub use user_profile::*;
pub use hooks::*;
pub use sub_agent::*;
pub use trajectory::*;
pub use compactor::{MessageRecord, IntegrityCheckResult, IntegrityCheck, SessionCompactor, verify_compression_integrity};
pub use nudge::{NudgeService, NudgeConfig, Nudge, NudgeSession, NudgeCandidate, NudgeContext, NudgeEntity, Urgency, NudgeAction, NudgeType, NudgeMessage};
pub use parallel_execution::*;
pub use scheduled_task::*;
pub use storage::*;

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
        ClosedLoopConfig, ClosedLoopService, Entity, EntityType, GraphQuery, MemoryActionResult,
        MemoryConfig, MemoryEntry, MemoryRegistry, MemoryService, MemoryUsage, NudgeCandidate,
        PeriodicNudge, Relationship, RelationshipType, SearchResult, WorkingMemory,
    };
    pub use crate::pattern::{
        CrossSessionInsight, CrossSessionLearner, DetectedPattern, PatternConfig, PatternLearner,
        PatternStatistics, PatternStep, PatternType,
    };
    pub use crate::parallel_execution::{
        ExecutionResult, ExecutionStatus, ExecutionStrategy, ParallelExecution,
        ParallelExecutionService, ParallelTask, TaskResultSummary, TaskStatus,
    };
    pub use crate::scheduled_task::{
        DailySummaryConfig, ScheduledTask, ScheduledTaskService, ScheduledTaskStatus,
        SummaryFormat, TaskConfig, TaskDefinition, TaskRunResult, TaskType,
    };
    pub use crate::platform_integration::{
        DiscordHandler, DiscordMessage, MessagePlatform, OutgoingMessage, PlatformConfig,
        PlatformIntegrationService, PlatformMessage, PlatformSession, TelegramHandler,
        TelegramMessage,
    };
    pub use crate::rl::{RLConfig, RLEngine, RLState, RewardNormalizer, RewardWeights};
    pub use crate::skill::{
        EvolutionOutcome, HermesMetadata, Impact, MetricsDelta, ModificationType, Skill,
        SkillAnalysis, SkillConfig, SkillContext, SkillCreator, SkillEvolution, SkillExecution,
        SkillMetadata, SkillModification, SkillOptimizer, SkillOutcome, SkillProposal,
        SkillReference, TaskComplexity, ValidationResult,
    };
    pub use crate::skill_proposal::{create_skill_from_proposal, SkillProposalService};
    pub use crate::storage::{TrajectoryStatistics, TrajectoryStorage};
    pub use crate::sub_agent::{
        AgentMailbox, AgentMessage, AgentMessageError, AgentMessageKind,
        MessageBus, SubAgent, SubAgentMetadata, SubAgentQuery, SubAgentRegistry, SubAgentResult, SubAgentStatus,
        TaskDeduplicator,
    };
    pub use crate::trajectory::{
        ExportFormat, MessageRole, RLTrainingEntry, RewardSignal, RewardType, ToolCall, ToolResult,
        Trajectory, TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern,
        TrajectoryQuality, TrajectoryQuery, TrajectoryStep,
    };
}
