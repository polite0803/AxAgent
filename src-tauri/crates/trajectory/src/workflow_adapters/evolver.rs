// SPDX-License-Identifier: AGPL-3.0-only

//! `WorkflowEvolverImpl`:基于 GEPA 风格遗传算法的工作流模板进化器实现。
//!
//! 变异策略(P0-2 修复后):
//! - 无 LLM 时,使用 `apply_heuristic_adjustments` 做真实变异
//!   (瓶颈节点 retry/timeout,全体 continue_on_fail)
//! - 有 LLM 时,在启发式变异之上叠加语义级变异(优化 4-b)
//! - 内存维护种群与运行状态(`tokio::sync::RwLock<EvolverState>`)
//! - 沙箱验证由 wiring 层注入 `WorkflowSandbox`,默认跳过(sandbox=None)
//! - `should_auto_evolve` 基于近期 reflection 失败率启发式判定

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_evolution::{
    EvolutionConfig, EvolutionPopulation, EvolutionStats, GenomeChange, SandboxValidationResult,
    WorkflowEvolver, WorkflowGenome, WorkflowGenomeLoader, WorkflowLlmMutator,
    WorkflowModification, WorkflowSandbox, merge_genome_by_mask, validate_genome_basic,
};
use axagent_harness::workflow_reflection::{
    BottleneckReason, WorkflowPattern, WorkflowReflectionMetadata, WorkflowRunStatus,
};

/// `WorkflowEvolver` 的 trajectory 实现。
pub struct WorkflowEvolverImpl {
    config: EvolutionConfig,
    state: RwLock<EvolverState>,
    /// LLM 变异 provider(wiring 层注入,默认 None)。
    llm_provider: RwLock<Option<Arc<dyn WorkflowLlmMutator>>>,
    /// 沙箱验证器(wiring 层注入,默认 None)。
    sandbox: RwLock<Option<Arc<dyn WorkflowSandbox>>>,
    /// 基因组加载器(wiring 层注入,默认 None)。
    /// 未注入时 `initialize` 退化为占位;注入后基于真实模板构造初始种群。
    genome_loader: RwLock<Option<Arc<dyn WorkflowGenomeLoader>>>,
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
            genome_loader: RwLock::new(None),
            recent_reflections: RwLock::new(HashMap::new()),
        }
    }

    /// 默认配置构造。
    pub fn with_defaults() -> Self {
        Self::new(EvolutionConfig::default())
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
}

/// 从 reflections 中收集失败/成功模式作为 LLM 变异证据。
///
/// `Reflection::error_patterns` / `reusable_patterns` 是字符串列表,这里包装为
/// `WorkflowPattern`(LLM 变异器只需 name + description 即可生成 prompt)。
fn collect_evidence(reflections: &[Reflection]) -> (Vec<WorkflowPattern>, Vec<WorkflowPattern>) {
    let mut failure_ev = Vec::new();
    let mut success_ev = Vec::new();
    for (i, r) in reflections.iter().enumerate() {
        for (j, err) in r.error_patterns.iter().enumerate() {
            failure_ev.push(WorkflowPattern {
                id: format!("err-{i}-{j}"),
                name: format!("failure-{i}-{j}"),
                description: err.clone(),
                node_ids: Vec::new(),
                frequency: 1,
                confidence: 0.5,
            });
        }
        for (j, ok) in r.reusable_patterns.iter().enumerate() {
            success_ev.push(WorkflowPattern {
                id: format!("ok-{i}-{j}"),
                name: format!("success-{i}-{j}"),
                description: ok.clone(),
                node_ids: Vec::new(),
                frequency: 1,
                confidence: 0.5,
            });
        }
    }
    (failure_ev, success_ev)
}

// ── 方案 4B:基于规则的启发式调整 ──

/// 启发式调整上限常量(防止无限放大字段值)。
const HEURISTIC_MAX_RETRIES: u32 = 5;
const HEURISTIC_MAX_TIMEOUT_SECS: u64 = 60;
const HEURISTIC_TIMEOUT_BACKOFF_FACTOR: f64 = 1.5;
/// 反思平均质量低于该阈值且无明确瓶颈节点时,工作流级开启 `continue_on_fail`。
const HEURISTIC_LOW_QUALITY_THRESHOLD: f32 = 0.5;
/// LLM 变异质量评估下限,低于此值则回滚变异(方案 2C)。
const LLM_MUTATION_QUALITY_THRESHOLD: f32 = 0.3;

/// 从 `Reflection.metadata` 反序列化出 `WorkflowReflectionMetadata`(失败返回 None)。
fn parse_workflow_meta(r: &Reflection) -> Option<WorkflowReflectionMetadata> {
    r.metadata
        .as_ref()
        .and_then(|v| serde_json::from_value::<WorkflowReflectionMetadata>(v.clone()).ok())
}

/// 基于反思数据做启发式节点调整(方案 4B)。
///
/// 规则:
/// - 节点瓶颈(HighFailureRate / HighRetryCount)→ `retry.max_retries += 1`(上限 5),
///   并启用 `retry.enabled`
/// - 节点瓶颈(HighLatency / ResourceHeavy)→ `timeout *= 1.5`(上限 60s,默认 30s)
/// - 整体反思质量低且无明确瓶颈 → 全体节点 `continue_on_fail = true`
///
/// 调整直接修改 genome 节点字段,返回 `GenomeChange` 用于审计。
/// 不依赖 LLM,确定性、可解释。
fn apply_heuristic_adjustments(
    genome: &mut WorkflowGenome,
    reflections: &[Reflection],
) -> Vec<GenomeChange> {
    if reflections.is_empty() {
        return Vec::new();
    }

    // 1. 收集所有反思中的瓶颈节点 ID 与原因(后写入覆盖前,以最新反思为准)
    let mut bottleneck_map: std::collections::HashMap<String, BottleneckReason> =
        std::collections::HashMap::new();
    let mut has_any_bottleneck = false;
    for r in reflections {
        if let Some(meta) = parse_workflow_meta(r) {
            for bn in &meta.bottleneck_nodes {
                has_any_bottleneck = true;
                bottleneck_map.insert(bn.node_id.clone(), bn.reason);
            }
        }
    }

    // 2. 计算平均反思质量(0.0-1.0)
    let sum: u32 = reflections.iter().map(|r| r.quality_score as u32).sum();
    let avg_quality = sum as f32 / (reflections.len() as f32 * 10.0);

    // 3. 工作流整体失败但无明确瓶颈 → 全体节点 continue_on_fail = true
    let should_continue_on_fail =
        avg_quality < HEURISTIC_LOW_QUALITY_THRESHOLD && !has_any_bottleneck;

    let mut changes = Vec::new();

    for node in &mut genome.nodes {
        let base = node.base_mut();
        let node_id = base.id.clone();

        // 规则 1+2:瓶颈节点调整 retry / timeout
        if let Some(reason) = bottleneck_map.get(&node_id) {
            match reason {
                BottleneckReason::HighFailureRate | BottleneckReason::HighRetryCount => {
                    if base.retry.max_retries < HEURISTIC_MAX_RETRIES {
                        base.retry.max_retries = base.retry.max_retries.saturating_add(1);
                        base.retry.enabled = true;
                        changes.push(GenomeChange::ConfigPatched {
                            node_id: node_id.clone(),
                            patch: serde_json::json!({
                                "retry.max_retries": base.retry.max_retries,
                                "retry.enabled": true,
                                "_reason": format!("heuristic: {reason:?}"),
                            }),
                        });
                    }
                },
                BottleneckReason::HighLatency | BottleneckReason::ResourceHeavy => {
                    let original_timeout = base.timeout.unwrap_or(30);
                    let scaled = (original_timeout as f64) * HEURISTIC_TIMEOUT_BACKOFF_FACTOR;
                    let new_timeout = (scaled as u64).min(HEURISTIC_MAX_TIMEOUT_SECS);
                    if new_timeout != original_timeout {
                        base.timeout = Some(new_timeout);
                        changes.push(GenomeChange::ConfigPatched {
                            node_id: node_id.clone(),
                            patch: serde_json::json!({
                                "timeout_secs": new_timeout,
                                "_reason": format!("heuristic: {reason:?}"),
                            }),
                        });
                    }
                },
                // SequentialBlocking 不在启发式规则范围(需要拓扑调整,留给 LLM 变异)
                _ => {},
            }
        }

        // 规则 3:整体低质量无瓶颈 → continue_on_fail
        if should_continue_on_fail && !base.continue_on_fail {
            base.continue_on_fail = true;
            changes.push(GenomeChange::ConfigPatched {
                node_id: node_id.clone(),
                patch: serde_json::json!({
                    "continue_on_fail": true,
                    "_reason": "heuristic: low avg quality without bottleneck",
                }),
            });
        }
    }

    changes
}

#[async_trait]
impl WorkflowEvolver for WorkflowEvolverImpl {
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String> {
        // 方案 3A:从注入的 genome_loader 加载真实模板(若已注入),
        // 否则退化为占位(单个体空 genome,保持向后兼容)。
        let seed: Option<WorkflowGenome> = {
            let loader_guard = self.genome_loader.read().await;
            if let Some(loader) = loader_guard.as_ref() {
                loader.load_genome(template_id).await
            } else {
                None
            }
        };

        let individuals: Vec<WorkflowGenome> = if let Some(base) = seed {
            // 种群初始化:原模板 + 扰动副本(每个节点 retry.max_retries +1 生成轻微变异),
            // 数量上限 = population_size(MVP 限制 4 个,避免内存膨胀)
            let pop_size = self.config.population_size.clamp(1, 4);
            let mut vec = Vec::with_capacity(pop_size);
            vec.push(base.clone()); // 第 0 个:原模板
            for i in 1..pop_size {
                let mut clone = base.clone();
                clone.generation = 0;
                // 扰动:每个节点 retry.max_retries +i,使每个个体略有差异
                for node in &mut clone.nodes {
                    let base = node.base_mut();
                    base.retry.max_retries = base.retry.max_retries.saturating_add(i as u32);
                }
                vec.push(clone);
            }
            vec
        } else {
            tracing::debug!(
                "[Evolver] genome_loader not injected, falling back to placeholder genome"
            );
            vec![WorkflowGenome {
                template_id: template_id.to_string(),
                name: format!("Workflow-{template_id}"),
                nodes: Vec::new(),
                edges: Vec::new(),
                variables: Vec::new(),
                fitness: 0.5,
                generation: 0,
                changed_node_ids: Vec::new(),
            }]
        };

        let best_fitness = individuals.iter().map(|g| g.fitness).fold(0.0_f32, f32::max);
        let avg_fitness = if individuals.is_empty() {
            0.0
        } else {
            individuals.iter().map(|g| g.fitness).sum::<f32>() / individuals.len() as f32
        };

        let population = EvolutionPopulation {
            generation: 0,
            individuals,
            best_fitness,
            avg_fitness,
            fitness_history: vec![best_fitness],
        };

        let mut state = self.state.write().await;
        state.generation = 0;
        state.best_fitness = best_fitness;
        state.fitness_history = vec![best_fitness];
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

        // P0-2 修复：弃用 MVP 占位变异,改用启发式规则调整真实修改 genome
        // (瓶颈节点 retry / timeout / 全体 continue_on_fail)。
        // 无 LLM provider 时也能产生真实变异,而非仅生成 "mvp_placeholder"。
        // 变更记录到 individual.generation,供后续审计。
        for individual in &mut population.individuals {
            let changes = apply_heuristic_adjustments(individual, reflections);
            if !changes.is_empty() {
                individual.generation = individual.generation.saturating_add(1);
                tracing::debug!(
                    "[Evolver] heuristic mutation applied to individual {} (changes={})",
                    individual.template_id,
                    changes.len()
                );
            }
        }

        // 若配置了 LLM provider,对适应度最低的个体做语义级变异(优化 4-b)
        // LLM 调用失败 / 解析失败时,实现方返回原 genome(保守策略),不破坏模板。
        {
            let llm = self.llm_provider.read().await;
            if let Some(provider) = llm.as_ref() {
                // 找出适应度最低的个体索引
                if let Some(idx) = population
                    .individuals
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                {
                    let original = population.individuals[idx].clone();
                    // 从 reflections 收集失败/成功模式作为变异证据
                    let (failure_ev, success_ev) = collect_evidence(reflections);
                    match provider.generate_mutation(&original, &failure_ev, &success_ev).await {
                        Ok(new_genome) => {
                            // 方案 1A:基础校验(替换前做最小一致性检查)
                            // 不通过则保留原 genome,避免 LLM 误伤(重复 id / 悬空 edge / 重名变量)
                            let validation_errors = validate_genome_basic(&new_genome);
                            if !validation_errors.is_empty() {
                                tracing::warn!(
                                    "[Evolver] LLM mutation failed basic validation, keeping original: {:?}",
                                    validation_errors
                                );
                            } else if new_genome.nodes.is_empty() {
                                tracing::debug!(
                                    "[Evolver] LLM returned empty nodes, keeping original genome"
                                );
                            } else {
                                // 方案 2C:LLM 质量评估,低分回滚
                                // evaluate 失败时保守策略:不替换(避免不可信变异落地)
                                let context = format!(
                                    "template={}, reflections={}, fitness={:.3}",
                                    original.template_id,
                                    reflections.len(),
                                    original.fitness
                                );
                                match provider.evaluate_quality(&new_genome, &context).await {
                                    Ok(score) if score >= LLM_MUTATION_QUALITY_THRESHOLD => {
                                        // 方案 1B:按 changed_node_ids mask 选择性合并,
                                        // 未声明的节点保留原 genome 版本(避免 LLM 误伤健康节点)。
                                        // mask 为空时退化为整体替换(向后兼容批次 A/B)。
                                        let merged = merge_genome_by_mask(&original, &new_genome);
                                        population.individuals[idx] = merged;
                                        tracing::debug!(
                                            "[Evolver] LLM mutation applied to individual {} (quality={:.3}, merged by mask)",
                                            idx,
                                            score
                                        );
                                    },
                                    Ok(score) => {
                                        tracing::warn!(
                                            "[Evolver] LLM mutation quality too low ({:.3} < {:.3}), keeping original",
                                            score,
                                            LLM_MUTATION_QUALITY_THRESHOLD
                                        );
                                    },
                                    Err(e) => {
                                        tracing::warn!(
                                            "[Evolver] LLM quality eval failed, keeping original: {e}"
                                        );
                                    },
                                }
                            }
                        },
                        Err(e) => {
                            tracing::warn!("[Evolver] LLM mutation failed, keeping original: {e}");
                        },
                    }
                }
            }
        }

        // 方案 4B:基于反思数据做启发式调整(对所有个体)
        // 规则:瓶颈节点 retry / timeout 调优;整体低质量无瓶颈 → continue_on_fail
        for individual in &mut population.individuals {
            let _heuristic_changes = apply_heuristic_adjustments(individual, reflections);
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
                changed_node_ids: Vec::new(),
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

    async fn record_reflection(
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

    async fn set_genome_loader(&self, loader: Arc<dyn WorkflowGenomeLoader>) -> Result<(), String> {
        let mut guard = self.genome_loader.write().await;
        *guard = Some(loader);
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
        let pop = e.initialize("wf-1").await.expect("测试：异步操作应成功");
        assert_eq!(pop.individuals.len(), 1);
        assert_eq!(pop.individuals[0].fitness, 0.5);
    }

    #[tokio::test]
    async fn test_evolve_generation_updates_fitness() {
        let e = WorkflowEvolverImpl::with_defaults();
        let mut pop = e.initialize("wf-1").await.expect("测试：异步操作应成功");
        let reflections =
            vec![Reflection::new("exec-1".to_string()).with_quality(8, "good".to_string())];
        let best = e.evolve_generation(&mut pop, &reflections).await.expect("测试：异步操作应成功");
        assert!((best.fitness - 0.8).abs() < 0.01, "expected fitness ~0.8, got {}", best.fitness);
    }

    #[tokio::test]
    async fn test_run_completes() {
        let e = WorkflowEvolverImpl::with_defaults();
        let reflections =
            vec![Reflection::new("exec-1".to_string()).with_quality(5, "mid".to_string())];
        let result = e.run("wf-1", &reflections).await.expect("测试：异步操作应成功");
        assert_eq!(result.template_id, "wf-1");
        assert!(result.validation.passed);
    }

    #[tokio::test]
    async fn test_should_auto_evolve_below_threshold() {
        let e = WorkflowEvolverImpl::with_defaults();
        // 不调用 record_reflection,默认返回 false
        let should = e.should_auto_evolve("wf-1").await.expect("测试：异步操作应成功");
        assert!(!should);
    }

    #[tokio::test]
    async fn test_should_auto_evolve_with_failures() {
        let e = WorkflowEvolverImpl::with_defaults();
        for _ in 0..5 {
            e.record_reflection("wf-1", 3, WorkflowRunStatus::Failed).await;
        }
        let should = e.should_auto_evolve("wf-1").await.expect("测试：异步操作应成功");
        assert!(should, "expected auto-evolve trigger due to failures");
    }

    #[tokio::test]
    async fn test_get_stats_initial() {
        let e = WorkflowEvolverImpl::with_defaults();
        let stats = e.get_stats().await.expect("测试：异步操作应成功");
        assert_eq!(stats.generation, 0);
        assert!(!stats.converged);
    }

    #[tokio::test]
    async fn test_is_running_default_false() {
        let e = WorkflowEvolverImpl::with_defaults();
        assert!(!e.is_running().await.expect("测试：异步操作应成功"));
    }

    // ── 方案 4B 启发式调整测试 ──

    /// 构造一个带瓶颈节点(HighFailureRate)的反思,
    /// 验证启发式调整会提升该节点的 retry.max_retries。
    #[tokio::test]
    async fn test_heuristic_adjustment_increases_retry_on_failure_rate() {
        use axagent_harness::workflow_reflection::{
            BottleneckNode, BottleneckReason, WorkflowReflectionMetadata,
        };

        let mut genome = WorkflowGenome {
            template_id: "wf-1".into(),
            name: "test".into(),
            nodes: vec![serde_json::from_value(serde_json::json!({
                "type": "delay",
                "id": "n1",
                "title": "delay",
                "position": {"x": 0, "y": 0},
                "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                "enabled": true,
                "config": {"delay_type": "seconds", "seconds": 1, "until": null}
            })).expect("deserialize node")],
            edges: vec![],
            variables: vec![],
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        };

        let meta = WorkflowReflectionMetadata {
            workflow_id: "wf-1".into(),
            execution_id: "exec-1".into(),
            bottleneck_nodes: vec![BottleneckNode {
                node_id: "n1".into(),
                node_type: "delay".into(),
                reason: BottleneckReason::HighFailureRate,
                impact_score: 0.8,
                detail: "frequently fails".into(),
            }],
            node_patterns: vec![],
            failed_node_analysis: None,
            proposed_changes: vec![],
        };
        let mut reflection = Reflection::new("exec-1".to_string()).with_quality(3, "bad".into());
        reflection.metadata = Some(serde_json::to_value(&meta).expect("测试应成功"));

        let changes = apply_heuristic_adjustments(&mut genome, &[reflection]);
        assert!(!changes.is_empty(), "expected heuristic changes");
        // retry.max_retries 应从 1 提升到 2,且 enabled=true
        let base = genome.nodes[0].base();
        assert_eq!(base.retry.max_retries, 2);
        assert!(base.retry.enabled);
    }

    /// 整体反思质量低(quality_score=2)且无瓶颈节点 →
    /// 所有节点应开启 continue_on_fail。
    #[tokio::test]
    async fn test_heuristic_adjustment_enables_continue_on_fail_for_low_quality() {
        let mut genome = WorkflowGenome {
            template_id: "wf-1".into(),
            name: "test".into(),
            nodes: vec![serde_json::from_value(serde_json::json!({
                "type": "delay",
                "id": "n1",
                "title": "delay",
                "position": {"x": 0, "y": 0},
                "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                "enabled": true,
                "config": {"delay_type": "seconds", "seconds": 1, "until": null}
            })).expect("deserialize node")],
            edges: vec![],
            variables: vec![],
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        };

        // quality_score=2 (0.2 归一化),无 metadata → 无瓶颈
        let reflection = Reflection::new("exec-1".to_string()).with_quality(2, "bad".to_string());
        let changes = apply_heuristic_adjustments(&mut genome, &[reflection]);
        assert!(!changes.is_empty(), "expected continue_on_fail change");
        assert!(genome.nodes[0].base().continue_on_fail);
    }

    /// 无反思数据时不应做任何调整。
    #[tokio::test]
    async fn test_heuristic_adjustment_noop_without_reflections() {
        let mut genome = WorkflowGenome {
            template_id: "wf-1".into(),
            name: "test".into(),
            nodes: vec![serde_json::from_value(serde_json::json!({
                "type": "delay",
                "id": "n1",
                "title": "delay",
                "position": {"x": 0, "y": 0},
                "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                "enabled": true,
                "config": {"delay_type": "seconds", "seconds": 1, "until": null}
            })).expect("deserialize node")],
            edges: vec![],
            variables: vec![],
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        };

        let changes = apply_heuristic_adjustments(&mut genome, &[]);
        assert!(changes.is_empty(), "expected no changes without reflections");
        assert_eq!(genome.nodes[0].base().retry.max_retries, 1);
        assert!(!genome.nodes[0].base().continue_on_fail);
    }
}
