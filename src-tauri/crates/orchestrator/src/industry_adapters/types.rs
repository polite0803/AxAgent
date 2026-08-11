// SPDX-License-Identifier: AGPL-3.0-only

//! 行业适配器类型定义 — 从 harness 重导出
//!
//! 权威定义已迁移至 `axagent-harness::industry_orchestration::types`。
//! 本模块仅保留重导出以维持向后兼容。

pub use axagent_harness::industry_orchestration::types::{
    AcceptanceCriterion, AcceptanceResult, AutoReflectTrigger, AutoTriggerConfig, CriterionResult,
    DependencyType, EvolutionConfig, EvolutionConstraints, ForbiddenOptimization,
    IndustryContext, IndustryLearningConfig, MissionType, PresetWorkflowStep, ProtectedStep,
    QualityThresholds, QualityWeights, ReflectionCheckpoint, ReflectionConfig,
    ReflectionTemplate, ReinforcementLearningConfig, RewardWeightConfig,
    SelfImprovementConfig, SkillEvolverConfig, StepDependency, WorkflowEvolverConfig,
};

// 兼容旧名称（ReinforcementLearningConfig 的别名）
pub use axagent_harness::industry_orchestration::types::ReinforcementLearningConfig as RLConfig;
