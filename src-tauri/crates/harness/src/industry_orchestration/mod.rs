// SPDX-License-Identifier: AGPL-3.0-only

//! 行业适配器模块
//!
//! 提供行业适配器核心 trait 和注册表，定义行业动态编排、反思、进化接口。

pub mod plan;
pub mod subgraph;
pub mod types;

use async_trait::async_trait;
use std::sync::Arc;

// ── 重导出所有类型以便外部访问 ──
pub use plan::{
    DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask, SubTaskStatus,
};
pub use subgraph::{DynamicSubGraph, GeneratedSubGraph};
pub use types::{
    AcceptanceCriterion, AcceptanceResult, AutoReflectTrigger, AutoTriggerConfig, CriterionResult,
    DependencyType, EvolutionConfig, EvolutionConstraints, ForbiddenOptimization, IndustryContext,
    IndustryLearningConfig, MissionType, PresetWorkflowStep, ProtectedStep, QualityThresholds,
    QualityWeights, ReflectionCheckpoint, ReflectionConfig, ReflectionTemplate,
    ReinforcementLearningConfig, RewardWeightConfig, SelfImprovementConfig, SkillEvolverConfig,
    StepDependency, WorkflowEvolverConfig,
};

// ── IndustryAdapter trait ──────────────────────────────────────────

/// 行业适配器核心 trait
///
/// 每个行业实现此 trait，提供行业特定的：
/// - 动态任务分解策略
/// - 反思模板
/// - 进化约束
/// - 验收标准定义
#[async_trait]
pub trait IndustryAdapter: Send + Sync {
    /// 行业唯一标识
    fn industry_id(&self) -> &str;

    /// 行业显示名称
    fn industry_name(&self) -> &str;

    /// 将用户意图分解为动态任务 DAG
    async fn decompose_mission(
        &self,
        mission: &str,
        context: &IndustryContext,
    ) -> Result<GeneratedSubGraph, OrchestrationError>;

    /// 检测任务类型
    fn detect_mission_type(&self, mission: &str) -> MissionType;

    /// 获取行业特定反思模板
    fn reflection_template(&self) -> &ReflectionTemplate;

    /// 获取行业特定进化约束
    fn evolution_constraints(&self) -> &EvolutionConstraints;

    /// 获取行业特定验收标准定义
    fn acceptance_criteria(&self) -> &[AcceptanceCriterion];

    /// 获取行业学习配置
    fn learning_config(&self) -> &IndustryLearningConfig;

    /// 获取行业预设工作流步骤
    ///
    /// 返回行业的标准工作流步骤模板，用于初始化工作流编排。
    /// 默认实现返回空列表，行业适配器可覆盖此方法。
    fn preset_steps(&self) -> Vec<PresetWorkflowStep> {
        Vec::new()
    }
}

// ── IndustryAdapterRegistry ────────────────────────────────────────

/// 行业适配器注册表
///
/// 管理所有行业适配器的实例，提供按 ID 查找功能。
pub struct IndustryAdapterRegistry {
    adapters: Vec<Arc<dyn IndustryAdapter>>,
}

impl IndustryAdapterRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }

    /// 注册行业适配器
    pub fn register(&mut self, adapter: Arc<dyn IndustryAdapter>) {
        self.adapters.push(adapter);
    }

    /// 按行业 ID 查找适配器
    pub fn get(&self, industry_id: &str) -> Option<&Arc<dyn IndustryAdapter>> {
        self.adapters.iter().find(|a| a.industry_id() == industry_id)
    }

    /// 获取所有已注册行业 ID 列表
    pub fn list_industries(&self) -> Vec<&str> {
        self.adapters.iter().map(|a| a.industry_id()).collect()
    }

    /// 获取所有已注册行业适配器引用
    pub fn all(&self) -> &[Arc<dyn IndustryAdapter>] {
        &self.adapters
    }

    /// 获取已注册行业数量
    pub fn count(&self) -> usize {
        self.adapters.len()
    }
}

impl Default for IndustryAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
