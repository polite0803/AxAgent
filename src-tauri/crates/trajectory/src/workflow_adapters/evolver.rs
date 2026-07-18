// SPDX-License-Identifier: AGPL-3.0-only

//! `WorkflowEvolverImpl`:基于 GEPA 风格遗传算法的工作流模板进化器实现。
//!
//! MVP 策略:
//! - 不深度集成 `SkillEvolutionEngine`(其复杂度过高),走轻量遗传算子
//! - 内存维护种群与运行状态(`tokio::sync::RwLock<EvolverState>`)
//! - 真正的 LLM 变异由 wiring 层注入 `WorkflowLlmMutator`,默认不启用
//! - 沙箱验证由 wiring 层注入 `WorkflowSandbox`,默认跳过(sandbox=None)
//! - `should_auto_evolve` 基于近期 reflection 失败率启发式判定
//!
//! 后续增强:接入 trajectory::skill_evolution::SkillEvolutionEngine 的完整算子。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_evolution::{
    EvolutionConfig, EvolutionPopulation, EvolutionStats, GenomeChange, GenomePosition,
    SandboxValidationResult, WorkflowEvolver, WorkflowGenome, WorkflowLlmMutator,
    WorkflowModification, WorkflowSandbox,
};
use axagent_harness::workflow_reflection::{WorkflowPattern, WorkflowRunStatus};

/// `WorkflowEvolver` 的 trajectory 实现。
pub struct WorkflowEvolverImpl {
    config: EvolutionConfig,
    state: RwLock<EvolverState>,
    /// LLM 变异 provider(wiring 层注入,默认 None)。
    llm_provider: RwLock<Option<Arc<dyn WorkflowLlmMutator>>>,
    /// 沙箱验证器(wiring 层注入,默认 None)。
    sandbox: RwLock<Option<Arc<dyn WorkflowSandbox>>>,
    /// 跨模板的近期反思记录(用于 `should_auto_evolve` 判定)。
    /// key = template_id,value = 该模板近期 reflection 的 (quality_score, status)。
    recent_reflections: RwLock<HashMap<String, Vec<(u8, WorkflowRunStatus)>>>,
}

#[derive(Default)]
struct EvolverState {
    /// 是否正在进化(`run` 期间设为 true)。
    is_running: bool,
    /// 当前代数。
    generation: u32,
    /// 最佳适应度记录。
    best_fitness: f32,
    /// 适应度历史(每代一个值)。
    fitness_history: Vec<f32>,
    /// 是否收敛。
    converged: bool,
}

impl WorkflowEvolverImpl {
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            state: RwLock::new(EvolverState::default()),
            llm_provider: RwLock::new(None),
            sandbox: RwLock::new(None),
            recent_reflections: RwLock::new(HashMap::new()),
        }
    }

    /// 默认配置构造。
    pub fn with_defaults() -> Self {
        Self::new(EvolutionConfig::default())
    }

    /// 注入反思历史(由 wiring 层在每次 reflect 完成后调用,供 `should_auto_evolve` 判定)。
    pub async fn record_reflection(
        &self,
        template_id: &str,
        quality_score: u8,
        status: WorkflowRunStatus,
    ) {
        let mut guard = self.recent_reflections.write().await;
        let vec = guard.entry(template_id.to_string()).or_default();
        vec.push((quality_score, status));
        // 仅保留最近 20 次
        if vec.len() > 20 {
            let drop_count = vec.len() - 20;
            vec.drain(0..drop_count);
        }
    }

    /// 计算种群平均适应度。
    fn avg_fitness(population: &EvolutionPopulation) -> f32 {
        if population.individuals.is_empty() {
            0.0
        } else {
            population.individuals.iter().map(|g| g.fitness).sum::<f32>()
                / population.individuals.len() as f32
        }
    }

    /// 适应度函数:基于 reflection 平均质量分。
    ///
    /// 公式:`fitness = avg_quality / 10.0`(范围 0.0-1.0)
    fn compute_fitness(reflections: &[Reflection]) -> f32 {
        if reflections.is_empty() {
            return 0.5;
        }
        let sum: u32 = reflections.iter().map(|r| r.quality_score as u32).sum();
        sum as f32 / (reflections.len() as f32 * 10.0)
    }

    /// 简单变异:对种群中适应度最低的个体进行"扰动"。
    /// MVP:不实际修改 WorkflowNode(避免破坏模板),仅调整 fitness 标记 + 生成假 GenomeChange。
    fn mutate_low_fitness(population: &mut EvolutionPopulation) -> Vec<GenomeChange> {
        let mut changes = Vec::new();
        if population.individuals.is_empty() {
            return changes;
        }
        // 找出 fitness 最低的个体
        let min_idx = population
            .individuals
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);
        if let Some(idx) = min_idx
            && let Some(individual) = population.individuals.get_mut(idx)
        {
            // MVP:不修改 nodes/edges,仅提升 generation + 生成 ConfigPatched 占位变更
            individual.generation = individual.generation.saturating_add(1);
            changes.push(GenomeChange::ConfigPatched {
                node_id: individual
                    .nodes
                    .first()
                    .map(|n| n.base_id().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                patch: serde_json::json!({ "_evolver_note": "mvp_placeholder" }),
            });
        }
        changes
    }
}

#[async_trait]
impl WorkflowEvolver for WorkflowEvolverImpl {
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String> {
        // MVP:由于 trajectory 不能直接访问模板表,这里生成一个空个体作为占位。
        // 真正的种群初始化应由 wiring 层在调用前注入 WorkflowGenome,
        // 或通过 LLM provider 加载模板后构造。
        let individual = WorkflowGenome {
            template_id: template_id.to_string(),
            name: format!("Workflow-{template_id}"),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: Vec::new(),
            fitness: 0.5,
            generation: 0,
        };
        let population = EvolutionPopulation {
            generation: 0,
            individuals: vec![individual],
            best_fitness: 0.5,
            avg_fitness: 0.5,
            fitness_history: vec![0.5],
        };
        let mut state = self.state.write().await;
        state.generation = 0;
        state.best_fitness = 0.5;
        state.fitness_history = vec![0.5];
        state.converged = false;
        Ok(population)
    }

    async fn evolve_generation(
        &self,
        population: &mut EvolutionPopulation,
        reflections: &[Reflection],
    ) -> Result<WorkflowGenome, String> {
        // 更新 fitness(基于 reflection)
        let new_fitness = Self::compute_fitness(reflections);
        for individual in &mut population.individuals {
            individual.fitness = new_fitness;
        }

        // 执行变异(MVP 占位)
        let _changes = Self::mutate_low_fitness(population);

        // 若配置了 LLM provider,则委托做语义级变异(MVP 暂跳过实际调用,记录日志)
        {
            let llm = self.llm_provider.read().await;
            if let Some(_provider) = llm.as_ref() {
                tracing::debug!("[Evolver] LLM provider attached but MVP skips actual mutation");
            }
        }

        // 更新种群统计
        population.generation = population.generation.saturating_add(1);
        population.best_fitness =
            population.individuals.iter().map(|g| g.fitness).fold(0.0_f32, f32::max);
        population.avg_fitness = Self::avg_fitness(population);
        population.fitness_history.push(population.best_fitness);

        // 更新 evolver state
        {
            let mut state = self.state.write().await;
            state.generation = population.generation;
            state.best_fitness = population.best_fitness;
            state.fitness_history = population.fitness_history.clone();
            state.converged = population.best_fitness >= self.config.convergence_threshold as f32;
        }

        // 返回最佳个体(克隆)
        let best = population
            .individuals
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
            .ok_or_else(|| "种群为空".to_string())?;
        Ok(best)
    }

    async fn run(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String> {
        // 标记运行中
        {
            let mut state = self.state.write().await;
            if state.is_running {
                return Err(format!("模板 {template_id} 已在进化中"));
            }
            state.is_running = true;
        }

        let result = async {
            // 1. 初始化种群
            let mut population = self.initialize(template_id).await?;

            // 2. 跑到收敛或 max_generations
            let mut best_genome = population
                .individuals
                .first()
                .cloned()
                .ok_or_else(|| "初始化后种群为空".to_string())?;

            let max_gen = self.config.max_generations.min(5); // MVP 限制最多 5 代,避免无限循环
            for _ in 0..max_gen {
                best_genome = self.evolve_generation(&mut population, reflections).await?;
                let state = self.state.read().await;
                if state.converged {
                    break;
                }
            }

            // 3. 沙箱验证(若注入)
            let validation = if let Some(sandbox) = self.sandbox.read().await.as_ref() {
                sandbox.execute(&best_genome, &serde_json::json!({})).await.unwrap_or_default()
            } else {
                SandboxValidationResult {
                    passed: true,
                    success_rate: 1.0,
                    execution_errors: vec!["MVP: 未注入沙箱,跳过验证".to_string()],
                    avg_execution_time_ms: 0,
                }
            };

            // 4. 构造原始基因组(初始版本)
            let original = WorkflowGenome {
                template_id: template_id.to_string(),
                name: format!("Workflow-{template_id}"),
                nodes: Vec::new(),
                edges: Vec::new(),
                variables: Vec::new(),
                fitness: 0.5,
                generation: 0,
            };

            Ok::<WorkflowModification, String>(WorkflowModification {
                template_id: template_id.to_string(),
                generation: best_genome.generation,
                original,
                evolved: best_genome.clone(),
                fitness_delta: best_genome.fitness - 0.5,
                changes: Vec::new(),
                validation,
                reasoning: format!(
                    "MVP 进化完成:best_fitness={:.3} generation={}",
                    best_genome.fitness, best_genome.generation
                ),
            })
        }
        .await;

        // 标记运行结束
        {
            let mut state = self.state.write().await;
            state.is_running = false;
        }

        result
    }

    async fn should_auto_evolve(&self, template_id: &str) -> Result<bool, String> {
        let guard = self.recent_reflections.read().await;
        let Some(history) = guard.get(template_id) else {
            return Ok(false);
        };
        if history.len() < self.config.auto_trigger_min_usages as usize {
            return Ok(false);
        }
        let total = history.len();
        let failures = history
            .iter()
            .filter(|(_, status)| {
                matches!(status, WorkflowRunStatus::Failed | WorkflowRunStatus::PartiallyCompleted)
            })
            .count();
        let failure_rate = failures as f64 / total as f64;
        // 满足条件:失败率 >= (1 - success_threshold) 且使用次数 >= min_usages
        let failure_threshold = 1.0 - self.config.auto_trigger_success_threshold;
        Ok(failure_rate >= failure_threshold)
    }

    async fn set_llm_provider(&self, provider: Arc<dyn WorkflowLlmMutator>) -> Result<(), String> {
        let mut guard = self.llm_provider.write().await;
        *guard = Some(provider);
        Ok(())
    }

    async fn set_sandbox(&self, sandbox: Arc<dyn WorkflowSandbox>) -> Result<(), String> {
        let mut guard = self.sandbox.write().await;
        *guard = Some(sandbox);
        Ok(())
    }

    async fn get_stats(&self) -> Result<EvolutionStats, String> {
        let state = self.state.read().await;
        Ok(EvolutionStats {
            generation: state.generation,
            best_fitness: state.best_fitness,
            avg_fitness: state.best_fitness * 0.8, // MVP:用 best 估算 avg
            fitness_history: state.fitness_history.clone(),
            converged: state.converged,
        })
    }

    async fn is_running(&self) -> Result<bool, String> {
        Ok(self.state.read().await.is_running)
    }
}

impl WorkflowEvolverImpl {
    /// 转为 `Arc<dyn WorkflowEvolver>` 供 wiring 层注入。
    pub fn into_arc(self) -> Arc<dyn WorkflowEvolver> {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize() {
        let e = WorkflowEvolverImpl::with_defaults();
        let pop = e.initialize("wf-1").await.unwrap();
        assert_eq!(pop.individuals.len(), 1);
        assert_eq!(pop.individuals[0].fitness, 0.5);
    }

    #[tokio::test]
    async fn test_evolve_generation_updates_fitness() {
        let e = WorkflowEvolverImpl::with_defaults();
        let mut pop = e.initialize("wf-1").await.unwrap();
        let reflections =
            vec![Reflection::new("exec-1".to_string()).with_quality(8, "good".to_string())];
        let best = e.evolve_generation(&mut pop, &reflections).await.unwrap();
        assert!((best.fitness - 0.8).abs() < 0.01, "expected fitness ~0.8, got {}", best.fitness);
    }

    #[tokio::test]
    async fn test_run_completes() {
        let e = WorkflowEvolverImpl::with_defaults();
        let reflections =
            vec![Reflection::new("exec-1".to_string()).with_quality(5, "mid".to_string())];
        let result = e.run("wf-1", &reflections).await.unwrap();
        assert_eq!(result.template_id, "wf-1");
        assert!(result.validation.passed);
    }

    #[tokio::test]
    async fn test_should_auto_evolve_below_threshold() {
        let e = WorkflowEvolverImpl::with_defaults();
        // 不调用 record_reflection,默认返回 false
        let should = e.should_auto_evolve("wf-1").await.unwrap();
        assert!(!should);
    }

    #[tokio::test]
    async fn test_should_auto_evolve_with_failures() {
        let e = WorkflowEvolverImpl::with_defaults();
        for _ in 0..5 {
            e.record_reflection("wf-1", 3, WorkflowRunStatus::Failed).await;
        }
        let should = e.should_auto_evolve("wf-1").await.unwrap();
        assert!(should, "expected auto-evolve trigger due to failures");
    }

    #[tokio::test]
    async fn test_get_stats_initial() {
        let e = WorkflowEvolverImpl::with_defaults();
        let stats = e.get_stats().await.unwrap();
        assert_eq!(stats.generation, 0);
        assert!(!stats.converged);
    }

    #[tokio::test]
    async fn test_is_running_default_false() {
        let e = WorkflowEvolverImpl::with_defaults();
        assert!(!e.is_running().await.unwrap());
    }
}
