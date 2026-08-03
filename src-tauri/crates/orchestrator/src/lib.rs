// SPDX-License-Identifier: AGPL-3.0-only

//! Orchestrator — high-level task decomposition, subgraph generation,
//! execution monitoring, and replanning for multi-agent workflows.
//!
//! The OrchestratorExecutor receives a high-level mission description,
//! decomposes it into subtasks using LLM reasoning, generates a DAG
//! subgraph of Worker nodes, submits the subgraph to the work engine,
//! monitors execution progress, and replans on failures.
//!
//! # Architecture
//!
//! ```text
//! Mission → decompose() → SubTask[] → build_subgraph() → WorkflowGraph
//!                                                              ↓
//!                                engine.execute(subgraph) → monitor() → replan() ↻
//! ```

pub mod decomposer;
pub mod dynamic_subgraph;
pub mod executor;
pub mod industry_adapters;
pub mod industry_learning;
pub mod task_context;
pub mod token_budget;
pub mod types;

pub use dynamic_subgraph::{DynamicSubGraph, GeneratedSubGraph};
pub use executor::{OrchestratorExecutor, OrchestratorState};
pub use industry_adapters::types::ReinforcementLearningConfig;
pub use industry_adapters::types::{
    AcceptanceCriterion, EvolutionConstraints, IndustryContext, IndustryLearningConfig,
    MissionType, ReflectionTemplate, RewardWeightConfig,
};
pub use industry_adapters::{IndustryAdapter, IndustryAdapterRegistry};
pub use industry_learning::{
    DimensionScore, EvolutionRequest, EvolutionResult, ExperiencePoolStats, IndustryLearningEngine,
    LlmInferencePort, RLExperience, RLPolicyUpdate, ReflectionRequest, ReflectionResult,
    SelfImprovementRequest, SelfImprovementResult,
};
pub use task_context::{
    IndustryContextManager, IndustryTaskContext, TaskContextState, TaskContextSummary,
};
pub use token_budget::{
    BudgetDecision, CompactionResult, DryLeafEntry, IndustryTokenBudgetManager,
    IndustryTokenConfig, IndustryTokenStats, TokenUsageSnapshot,
};
pub use types::{
    DecompositionPlan, OrchestrationError, OrchestrationEvent, OrchestrationStrategy,
    StructuredHandover, SubTask, SubTaskStatus, WorkerAssignment,
};
