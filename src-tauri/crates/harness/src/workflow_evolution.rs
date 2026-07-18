// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板进化 trait 契约(三层 trait 之二:Evolver)
//!
//! 把工作流模板视为"基因组",复用 GEPA 遗传算法(选择/交叉/变异)。
//! 不重定义 `SkillGenome`,新建 `WorkflowGenome` 同构结构。
//!
//! 异步执行:进化流程在后台 `tokio::spawn`,不阻塞工作流引擎主流程。

use crate::reflection_types::Reflection;
use crate::workflow_reflection::WorkflowPattern;
use crate::workflow_types::{WorkflowEdge, WorkflowNode};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 工作流基因组 ──

/// 工作流模板的可进化表示(同构于 `SkillGenome`)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGenome {
    pub template_id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: Vec<serde_json::Value>,
    pub fitness: f32,
    pub generation: u32,
}

// ── 进化配置(与 trajectory::EvolutionConfig 字段对齐,便于转换) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_generations: usize,
    pub convergence_threshold: f64,
    pub use_llm_mutation: bool,
    pub use_execution_validation: bool,
    pub validation_rounds: usize,
    pub auto_trigger_consecutive_failures: u32,
    pub auto_trigger_min_usages: u32,
    pub auto_trigger_success_threshold: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            elite_count: 4,
            mutation_rate: 0.15,
            crossover_rate: 0.7,
            max_generations: 50,
            convergence_threshold: 0.95,
            use_llm_mutation: true,
            use_execution_validation: true,
            validation_rounds: 3,
            auto_trigger_consecutive_failures: 3,
            auto_trigger_min_usages: 3,
            auto_trigger_success_threshold: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPopulation {
    pub generation: u32,
    pub individuals: Vec<WorkflowGenome>,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: u32,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
    pub converged: bool,
}

// ── 进化结果 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowModification {
    pub template_id: String,
    pub generation: u32,
    pub original: WorkflowGenome,
    pub evolved: WorkflowGenome,
    pub fitness_delta: f32,
    pub changes: Vec<GenomeChange>,
    pub validation: SandboxValidationResult,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenomeChange {
    /// `node` 装箱以避免 `large_enum_variant`。
    NodeAdded {
        node: Box<WorkflowNode>,
        position: GenomePosition,
    },
    NodeRemoved {
        node_id: String,
    },
    /// `new_node` 装箱以避免 `large_enum_variant`。
    NodeReplaced {
        node_id: String,
        new_node: Box<WorkflowNode>,
    },
    EdgeAdded {
        edge: WorkflowEdge,
    },
    EdgeRemoved {
        from: String,
        to: String,
    },
    EdgeRewired {
        from: String,
        original_to: String,
        new_to: String,
    },
    ConfigPatched {
        node_id: String,
        patch: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomePosition {
    pub after_node: Option<String>,
    pub before_node: Option<String>,
    pub branch_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxValidationResult {
    pub passed: bool,
    pub success_rate: f32,
    pub execution_errors: Vec<String>,
    pub avg_execution_time_ms: u64,
}

// ── Trait 契约 ──

/// 工作流模板进化器:基于反思结果与历史执行,进化工作流模板。
///
/// **三层 trait 之二**(Reflector / Evolver / Optimizer)。
///
/// 实现方:trajectory crate(复用 `SkillEvolutionEngine` 的遗传算子 + `LlmEvolutionProvider`)。
/// 调用方:wiring 层(批量后台任务)、命令层(`commands/evolution*`)。
///
/// 执行方式:异步(`tokio::spawn`),进化完成后通过事件/回调通知。
#[async_trait]
pub trait WorkflowEvolver: Send + Sync {
    /// 初始化种群(从模板生成初始基因组)。
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String>;

    /// 进化一代。
    ///
    /// `reflections` 为近期反思结果,用于计算适应度与变异方向。
    async fn evolve_generation(
        &self,
        population: &mut EvolutionPopulation,
        reflections: &[Reflection],
    ) -> Result<WorkflowGenome, String>;

    /// 完整进化流程(到收敛或 `max_generations`)。
    ///
    /// 建议在 `tokio::spawn` 中调用,避免阻塞主流程。
    async fn run(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String>;

    /// 自动触发判定(基于近期失败率与使用次数)。
    async fn should_auto_evolve(&self, template_id: &str) -> Result<bool, String>;

    /// 注入 LLM 变异 provider。
    async fn set_llm_provider(&self, provider: Arc<dyn WorkflowLlmMutator>) -> Result<(), String>;

    /// 注入沙箱验证器。
    async fn set_sandbox(&self, sandbox: Arc<dyn WorkflowSandbox>) -> Result<(), String>;

    /// 状态查询。
    async fn get_stats(&self) -> Result<EvolutionStats, String>;
    async fn is_running(&self) -> Result<bool, String>;
}

/// 工作流 LLM 变异器:扩展 `LlmEvolutionProvider` 支持工作流节点语义。
///
/// 实现方可内部委托 `axagent_harness::trajectory_types::LlmEvolutionProvider`,
/// 并将 `ProcedureStep` 与 `WorkflowNode` 互转。
#[async_trait]
pub trait WorkflowLlmMutator: Send + Sync {
    async fn generate_mutation(
        &self,
        genome: &WorkflowGenome,
        failure_evidence: &[WorkflowPattern],
        success_evidence: &[WorkflowPattern],
    ) -> Result<WorkflowGenome, String>;

    async fn evaluate_quality(&self, genome: &WorkflowGenome, context: &str)
    -> Result<f32, String>;
}

/// 工作流沙箱:执行进化后的模板以验证可行性。
///
/// 实现方可委托 `trajectory::sandbox_executor::SkillSandboxExecutor`,
/// 将工作流模板转换为技能 steps 后执行。
#[async_trait]
pub trait WorkflowSandbox: Send + Sync {
    async fn execute(
        &self,
        genome: &WorkflowGenome,
        test_input: &serde_json::Value,
    ) -> Result<SandboxValidationResult, String>;
}
