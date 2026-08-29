// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::await_holding_lock)]

//! Skill Evolution System - GEPA-inspired skill improvement through genetic algorithms
//!
//! Features:
//! - Constraint-gated evolution
//! - Fitness evaluation based on success rate and execution time
//! - Crossover and mutation operators
//! - Multi-objective optimization (quality vs speed)
//!
//! P2-7 评估说明( Won't Fix ):
//! `SkillGenome`(fitness: f64)与 `WorkflowGenome`(fitness: f32)类型不同,
//! 且 crossover/mutate 因基因组结构(steps vs nodes/edges)差异完全不同。
//! 抽取共享泛型算子需引入 `GenomeFitness` trait + 适配器,成本大于收益。
//! 两者的 `tournament_select` 算法各自约 20 行,保持独立实现更清晰。
//! - Convergence detection
//! - LLM-driven semantic mutation (replaces random symbol manipulation)
//! - Execution feedback-driven verification closed loop

pub use axagent_harness::trajectory_types::{
    LlmEvolutionProvider, LlmMutationFuture, LlmMutationRequest, LlmMutationResponse, ProcedureStep,
};

use crate::evidence::{EvolutionDecider, EvolutionDecision};
use crate::skill::{Skill, SkillModification, SkillValidationResult};
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_generations: usize,
    pub convergence_threshold: f64,
    pub min_fitness_improvement: f64,
    pub use_llm_mutation: bool,
    pub use_execution_validation: bool,
    pub validation_rounds: usize,
    /// 自动触发进化的连续失败次数阈值（达到即触发进化）
    pub auto_trigger_consecutive_failures: u32,
    /// 自动触发进化的最小使用次数（与 success_threshold 联动）
    pub auto_trigger_min_usages: u32,
    /// 自动触发进化的成功率阈值（低于此值且达到 min_usages 即触发）
    pub auto_trigger_success_threshold: f64,
    /// T3.2：贝叶斯进化触发低阈值（`P(success) <` 此值触发进化）。
    #[serde(default = "default_evolve_threshold")]
    pub evolve_threshold: f64,
    /// T3.2：贝叶斯稳定标记高阈值（`P(success) >` 此值且证据足够 → 稳定）。
    #[serde(default = "default_stable_threshold")]
    pub stable_threshold: f64,
    /// T3.2：最小（加权）证据量，低于此值视为小样本，走保守分支。
    #[serde(default = "default_min_evidence")]
    pub min_evidence: f64,
}

fn default_evolve_threshold() -> f64 {
    0.4
}

fn default_stable_threshold() -> f64 {
    0.7
}

fn default_min_evidence() -> f64 {
    3.0
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
            min_fitness_improvement: 0.01,
            use_llm_mutation: true,
            use_execution_validation: true,
            validation_rounds: 3,
            auto_trigger_consecutive_failures: 3,
            auto_trigger_min_usages: 3,
            auto_trigger_success_threshold: 0.5,
            evolve_threshold: 0.4,
            stable_threshold: 0.7,
            min_evidence: 3.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillGenome {
    pub skill_id: String,
    pub content: String,
    pub description: String,
    pub steps: Vec<ProcedureStep>,
    pub fitness: f64,
}

#[derive(Debug, Clone)]
pub struct EvolutionPopulation {
    pub generation: u32,
    pub individuals: Vec<SkillGenome>,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub fitness_history: Vec<f64>,
}

impl EvolutionPopulation {
    pub fn new(skill: &Skill, config: &EvolutionConfig) -> Self {
        let steps = parse_skill_content(&skill.content);
        let base_genome = SkillGenome {
            skill_id: skill.id.clone(),
            content: skill.content.clone(),
            description: skill.description.clone(),
            steps,
            fitness: skill.quality_score,
        };

        let mut individuals = vec![base_genome.clone()];
        for _ in 1..config.population_size {
            individuals.push(mutate_genome(&base_genome, config.mutation_rate));
        }

        Self {
            generation: 0,
            individuals,
            best_fitness: base_genome.fitness,
            avg_fitness: base_genome.fitness,
            fitness_history: Vec::new(),
        }
    }

    pub fn evolve(&mut self, config: &EvolutionConfig) {
        self.individuals
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let elite: Vec<SkillGenome> = self.individuals[..config.elite_count].to_vec();

        let mut new_individuals = elite.clone();

        while new_individuals.len() < config.population_size {
            let parent1 = tournament_select(&self.individuals, 3);
            let parent2 = tournament_select(&self.individuals, 3);

            let child = if rand::rng().random::<f64>() < config.crossover_rate {
                crossover_genomes(&parent1, &parent2)
            } else {
                parent1.clone()
            };

            let mutated = mutate_genome(&child, config.mutation_rate);
            new_individuals.push(mutated);
        }

        self.individuals = new_individuals;
        self.individuals.truncate(config.population_size);

        let fitnesses: Vec<f64> = self.individuals.iter().map(|g| g.fitness).collect();
        self.best_fitness = fitnesses.iter().cloned().fold(f64::MIN, f64::max);
        self.avg_fitness = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;
        self.fitness_history.push(self.avg_fitness);
        self.generation += 1;
    }

    pub fn is_converged(&self, config: &EvolutionConfig) -> bool {
        if self.fitness_history.len() < 10 {
            return false;
        }

        let recent: Vec<f64> = self.fitness_history[self.fitness_history.len() - 10..].to_vec();
        let old_avg = recent[..5].iter().sum::<f64>() / 5.0;
        let new_avg = recent[5..].iter().sum::<f64>() / 5.0;

        (new_avg - old_avg).abs() < config.min_fitness_improvement
    }

    pub fn best_individual(&self) -> Option<&SkillGenome> {
        self.individuals.first()
    }
}

fn tournament_select(population: &[SkillGenome], tournament_size: usize) -> SkillGenome {
    use rand::seq::IndexedRandom;

    // P1-10: 空种群直接 panic 防护
    if population.is_empty() {
        // 返回一个占位的空 genome，避免上游崩溃
        return SkillGenome {
            skill_id: String::new(),
            content: String::new(),
            description: String::new(),
            steps: Vec::new(),
            fitness: 0.0,
        };
    }

    let mut rng = rand::rng();
    let size = population.len().min(tournament_size);
    let indices: Vec<usize> = (0..population.len()).collect();
    let selected: Vec<usize> = indices.sample(&mut rng, size).cloned().collect();

    if selected.is_empty() {
        return population[0].clone();
    }

    let mut best_idx = selected[0];
    let mut best_fitness = population[best_idx].fitness;

    for &idx in &selected[1..] {
        if population[idx].fitness > best_fitness {
            best_idx = idx;
            best_fitness = population[idx].fitness;
        }
    }

    population[best_idx].clone()
}

fn crossover_genomes(parent1: &SkillGenome, parent2: &SkillGenome) -> SkillGenome {
    // P0-7: 任一父本 steps 为空都直接返回 parent1，避免 slice panic
    if parent1.steps.is_empty() || parent2.steps.is_empty() {
        return parent1.clone();
    }

    let mut rng = rand::rng();

    let cross_point = rng.random_range(1..parent1.steps.len().max(2)).min(parent1.steps.len());

    let mut child_steps = parent1.steps[..cross_point].to_vec();
    if cross_point < parent2.steps.len() {
        child_steps.extend_from_slice(&parent2.steps[cross_point..]);
    }

    let child_content = serialize_steps(&child_steps);

    SkillGenome {
        skill_id: parent1.skill_id.clone(),
        content: child_content,
        description: if rng.random::<bool>() {
            parent1.description.clone()
        } else {
            parent2.description.clone()
        },
        steps: child_steps,
        fitness: 0.0,
    }
}

fn mutate_genome(genome: &SkillGenome, mutation_rate: f64) -> SkillGenome {
    let mut rng = rand::rng();
    let mut new_steps: Vec<ProcedureStep> = genome.steps.clone();

    for i in 0..new_steps.len() {
        // P1-9: 修复判断逻辑——当随机数 < mutation_rate 时触发变异
        if rng.random::<f64>() >= mutation_rate {
            continue;
        }

        // 随机选择有意义的变异类型
        match rng.random_range(0u8..4) {
            // 0: 交换相邻步骤顺序
            0 if i + 1 < new_steps.len() => {
                let j = i + 1;
                let old_order = new_steps[i].order;
                new_steps[i].order = new_steps[j].order;
                new_steps[j].order = old_order;
            },
            // 1: 添加基础错误处理
            1 => {
                let action = &new_steps[i].action;
                if !action.contains("verify") && !action.contains("error") {
                    new_steps[i].error_handling =
                        Some("If this step fails, retry once before proceeding".to_string());
                }
            },
            // 2: 添加前置条件
            2 => {
                if new_steps[i].condition.is_none() {
                    new_steps[i].condition =
                        Some("Ensure prerequisites from previous step are met".to_string());
                }
            },
            // 3: 单步说明微调
            _ => {
                let a = &new_steps[i].action;
                let modified = if a.ends_with('.') {
                    format!("{} Double-check the result.", a)
                } else {
                    format!("{}. Verify the output.", a)
                };
                new_steps[i].action = modified;
            },
        }
    }

    let new_content = serialize_steps(&new_steps);

    SkillGenome {
        skill_id: genome.skill_id.clone(),
        content: new_content,
        description: genome.description.clone(),
        steps: new_steps,
        fitness: 0.0,
    }
}

fn parse_skill_content(content: &str) -> Vec<ProcedureStep> {
    let mut steps = Vec::new();
    let mut order = 0;

    let tool_regex = match regex::Regex::new(r"^\d+\.\s*(?:Use\s+)?(\w+)") {
        Ok(re) => re,
        Err(_) => return steps,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("##") {
            continue;
        }

        let tool_match: Option<String> =
            tool_regex.captures(trimmed).and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

        if let Some(tool) = tool_match {
            steps.push(ProcedureStep {
                order,
                action: trimmed.to_string(),
                tool: Some(tool),
                condition: None,
                error_handling: None,
            });
            order += 1;
        }
    }

    steps
}

fn serialize_steps(steps: &[ProcedureStep]) -> String {
    let mut content = String::new();

    for (i, step) in steps.iter().enumerate() {
        if let Some(ref tool) = step.tool {
            content.push_str(&format!("{}. Use {} with args\n", i + 1, tool));
        } else {
            content.push_str(&format!("{}. {}\n", i + 1, step.action));
        }
    }

    content
}

#[cfg(test)]
/// 多维度启发式技能质量评估函数。
/// 在没有 LLM 时作为后备方案，从结构完整性、代码质量、可读性等维度评分。
fn evaluate_skill_quality_heuristic(content: &str) -> f64 {
    let mut score: f64 = 0.0;
    let content_lower = content.to_lowercase();

    // 1. 结构完整性 (0-0.3): 是否包含关键章节
    let mut structure_score: f64 = 0.0;
    let sections = ["# ", "## ", "```", "description", "usage", "example"];
    for section in &sections {
        if content_lower.contains(section) {
            structure_score += 0.05_f64;
        }
    }
    score += structure_score.min(0.3_f64);

    // 2. 代码质量指标 (0-0.25): 错误处理、验证、注释
    let mut quality_score: f64 = 0.0;
    let quality_indicators: [(&str, f64); 6] = [
        ("error", 0.05),
        ("verify", 0.05),
        ("check", 0.03),
        ("validate", 0.05),
        ("retry", 0.04),
        ("fallback", 0.03),
    ];
    for (indicator, weight) in &quality_indicators {
        if content_lower.contains(indicator) {
            quality_score += weight;
        }
    }
    // 检查是否有合理数量的步骤
    let step_count = content_lower.matches("step").count();
    quality_score += (step_count.min(3) as f64) * 0.02_f64;
    score += quality_score.min(0.25_f64);

    // 3. 内容充实度 (0-0.2): 长度适中，不过短也不过长
    let len = content.len() as f64;
    let fullness: f64 = if len < 100.0 {
        0.0
    } else if len < 500.0 {
        (len - 100.0) / 400.0 * 0.15
    } else if len < 5000.0 {
        0.15 + (len - 500.0) / 4500.0 * 0.05
    } else {
        0.2
    };
    score += fullness;

    // 4. 可读性 (0-0.15): 段落结构、格式
    let mut readability: f64 = 0.0;
    if content.contains('\n') {
        readability += 0.03_f64;
    }
    if content.contains("```") {
        readability += 0.04_f64;
    }
    if content.contains("- ") || content.contains("* ") {
        readability += 0.04_f64;
    }
    if content.contains("> ") {
        readability += 0.04_f64;
    }
    score += readability.min(0.15_f64);

    // 5. 基础分 (0-0.1): 确保任何非空内容都有基础分
    if !content.is_empty() {
        score += 0.1_f64;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
pub(crate) struct DefaultLlmEvolutionProvider;

#[cfg(test)]
impl LlmEvolutionProvider for DefaultLlmEvolutionProvider {
    fn generate_mutation(&self, request: &LlmMutationRequest) -> LlmMutationFuture<'_> {
        let steps = request.current_steps.clone();
        let failures = request.failure_evidence.clone();
        Box::pin(async move {
            let mut revised = steps.clone();
            if !failures.is_empty() {
                for step in &mut revised {
                    if step.error_handling.is_none() {
                        step.error_handling =
                            Some("If this step fails, retry with alternative approach".to_string());
                    }
                    step.condition = Some("Verify prerequisites before execution".to_string());
                }
            }
            Ok(LlmMutationResponse {
                revised_steps: revised,
                reasoning: "Added error handling and condition checks based on failure evidence"
                    .to_string(),
                confidence: 0.6,
            })
        })
    }

    /// ⚠️ 此实现仅为测试/演示用途的后备评估函数。
    /// 基于多维度结构分析进行启发式评分，作为无 LLM 时的替代方案。
    /// 生产环境应在 ExternalLlmEvolutionProvider 中替换为真实的 LLM 评估。
    fn evaluate_quality(
        &self,
        content: &str,
        _context: &str,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>> {
        let score = evaluate_skill_quality_heuristic(content);
        Box::pin(async move { Ok(score.clamp(0.0, 1.0)) })
    }
}

pub struct SandboxValidationResult {
    pub passed: bool,
    pub success_rate: f64,
    pub execution_errors: Vec<String>,
    pub avg_execution_time_ms: u64,
}

pub trait SandboxExecutor: Send + Sync {
    fn execute_skill<'a>(
        &'a self,
        genome: &'a SkillGenome,
        test_input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<SandboxValidationResult, String>> + Send + 'a>>;
}

/// Minimum number of trajectories required before the evolution engine
/// performs any mutation. This guard prevents the engine from overfitting
/// on a tiny sample and keeps it dormant for short-lived sessions.
const MIN_TRAJECTORIES_FOR_EVOLUTION: usize = 10;

pub struct SkillEvolutionEngine {
    config: EvolutionConfig,
    population: Option<EvolutionPopulation>,
    /// Lazy-initialized LLM provider behind Arc+RwLock so it can be
    /// injected after construction and swapped at runtime without
    /// rebuilding the entire engine.
    llm_provider: Arc<RwLock<Option<Arc<dyn LlmEvolutionProvider>>>>,
    /// Lazy-initialized sandbox executor, same pattern as llm_provider.
    sandbox: Arc<RwLock<Option<Arc<dyn SandboxExecutor>>>>,
}

impl Default for SkillEvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillEvolutionEngine {
    pub fn new() -> Self {
        Self {
            config: EvolutionConfig::default(),
            population: None,
            llm_provider: Arc::new(RwLock::new(None)),
            sandbox: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_config(config: EvolutionConfig) -> Self {
        Self {
            config,
            population: None,
            llm_provider: Arc::new(RwLock::new(None)),
            sandbox: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_llm_provider(&self, provider: Arc<dyn LlmEvolutionProvider>) {
        // tokio::sync::RwLock: 符合 AGENTS.md 第 8 条（禁止 parking_lot::RwLock）。
        *self.llm_provider.write().await = Some(provider);
    }

    pub async fn set_sandbox(&mut self, executor: Arc<dyn SandboxExecutor>) {
        self.config.use_execution_validation = true;
        // tokio::sync::RwLock: 符合 AGENTS.md 第 8 条（禁止 parking_lot::RwLock）。
        *self.sandbox.write().await = Some(executor);
    }

    pub fn skill_count(&self) -> usize {
        self.population.as_ref().map(|p| p.individuals.len()).unwrap_or(0)
    }

    pub async fn has_llm_provider(&self) -> bool {
        self.llm_provider.read().await.is_some()
    }

    pub async fn has_sandbox(&self) -> bool {
        self.sandbox.read().await.is_some()
    }

    pub fn initialize(&mut self, skill: &Skill) {
        self.population = Some(EvolutionPopulation::new(skill, &self.config));
    }

    pub fn evolve_generation(&mut self, test_trajectories: &[&Trajectory]) -> Option<SkillGenome> {
        let should_evolve = if let Some(ref pop) = self.population {
            pop.generation < self.config.max_generations as u32 && !pop.is_converged(&self.config)
        } else {
            false
        };

        if !should_evolve {
            return self.population.as_ref().and_then(|p| p.best_individual()).cloned();
        }

        if let Some(ref mut pop) = self.population {
            for individual in &mut pop.individuals {
                Self::evaluate_fitness_static(individual, test_trajectories);
            }

            let before_gen = pop.generation;
            pop.evolve(&self.config);

            if before_gen != pop.generation {
                return pop.best_individual().cloned();
            }
        }

        self.population.as_ref().and_then(|p| p.best_individual()).cloned()
    }

    pub async fn evolve_generation_v2(
        &mut self,
        test_trajectories: &[&Trajectory],
    ) -> Option<SkillGenome> {
        // Trajectory count guard: skip evolution when there isn't enough
        // data to produce meaningful mutations.
        if test_trajectories.len() < MIN_TRAJECTORIES_FOR_EVOLUTION {
            return self.population.as_ref().and_then(|p| p.best_individual()).cloned();
        }

        let should_evolve = if let Some(ref pop) = self.population {
            pop.generation < self.config.max_generations as u32 && !pop.is_converged(&self.config)
        } else {
            false
        };

        if !should_evolve {
            return self.population.as_ref().and_then(|p| p.best_individual()).cloned();
        }

        if let Some(ref mut pop) = self.population {
            if self.config.use_llm_mutation {
                // tokio::sync::RwLock: read guard 在 .clone() 后立即释放，不跨 await 点。
                let llm_provider = self.llm_provider.read().await.clone();
                if let Some(ref provider) = llm_provider {
                    for individual in &mut pop.individuals {
                        let failure_evidence: Vec<String> = test_trajectories
                            .iter()
                            .filter(|t| {
                                matches!(
                                    t.outcome,
                                    TrajectoryOutcome::Failure | TrajectoryOutcome::Abandoned
                                ) && t
                                    .topic
                                    .to_lowercase()
                                    .contains(&individual.description.to_lowercase())
                            })
                            .map(|t| t.summary.clone())
                            .take(5)
                            .collect();

                        let success_evidence: Vec<String> = test_trajectories
                            .iter()
                            .filter(|t| {
                                matches!(t.outcome, TrajectoryOutcome::Success)
                                    && t.topic
                                        .to_lowercase()
                                        .contains(&individual.description.to_lowercase())
                            })
                            .map(|t| t.summary.clone())
                            .take(5)
                            .collect();

                        let request = LlmMutationRequest {
                            skill_name: individual.description.clone(),
                            current_steps: individual.steps.clone(),
                            failure_evidence,
                            success_evidence,
                        };

                        if let Ok(response) = provider.generate_mutation(&request).await
                            && response.confidence > 0.5
                        {
                            // P1-8: 保留变异前快照，若新 fitness 更低则回滚
                            let snapshot_steps = individual.steps.clone();
                            let snapshot_content = individual.content.clone();
                            let snapshot_fitness = individual.fitness;
                            individual.steps = response.revised_steps;
                            individual.content = serialize_steps(&individual.steps);
                            Self::evaluate_fitness_static(individual, test_trajectories);
                            if individual.fitness < snapshot_fitness {
                                individual.steps = snapshot_steps;
                                individual.content = snapshot_content;
                                individual.fitness = snapshot_fitness;
                            }
                        }
                    }
                }
            }

            for individual in &mut pop.individuals {
                Self::evaluate_fitness_static(individual, test_trajectories);
            }

            if self.config.use_execution_validation {
                // tokio::sync::RwLock: read guard 在 .clone() 后立即释放，不跨 await 点。
                let sandbox = self.sandbox.read().await.clone();
                if let Some(ref sandbox) = sandbox {
                    for individual in &mut pop.individuals.iter_mut() {
                        let mut total_success = 0.0;
                        let mut rounds = 0;
                        for trajectory in
                            test_trajectories.iter().take(self.config.validation_rounds)
                        {
                            if let Ok(result) =
                                sandbox.execute_skill(individual, &trajectory.topic).await
                            {
                                total_success += result.success_rate;
                                rounds += 1;
                            }
                        }
                        if rounds > 0 {
                            individual.fitness =
                                individual.fitness * 0.6 + (total_success / rounds as f64) * 0.4;
                        }
                    }
                }
            }

            let before_gen = pop.generation;
            pop.evolve(&self.config);

            if before_gen != pop.generation {
                return pop.best_individual().cloned();
            }
        }

        self.population.as_ref().and_then(|p| p.best_individual()).cloned()
    }

    fn evaluate_fitness_static(genome: &mut SkillGenome, test_trajectories: &[&Trajectory]) {
        let relevant: Vec<&Trajectory> = test_trajectories
            .iter()
            .filter(|t| t.topic.to_lowercase().contains(&genome.description.to_lowercase()))
            .cloned()
            .collect();

        if relevant.is_empty() {
            genome.fitness = 0.5;
            return;
        }

        let successes: usize = relevant
            .iter()
            .filter(|t| {
                matches!(t.outcome, TrajectoryOutcome::Success | TrajectoryOutcome::Partial)
            })
            .count();

        let success_rate = successes as f64 / relevant.len() as f64;

        let avg_time: f64 = relevant
            .iter()
            .map(|t| {
                t.steps
                    .iter()
                    .map(|s| s.tool_results.as_ref().map_or(0, |r| r.len()))
                    .sum::<usize>() as f64
            })
            .sum::<f64>()
            / relevant.len() as f64;

        let time_score = (avg_time / 100.0).min(1.0);

        let error_handling_bonus =
            genome.steps.iter().filter(|s| s.error_handling.is_some()).count() as f64
                / genome.steps.len().max(1) as f64
                * 0.1;

        let condition_bonus = genome.steps.iter().filter(|s| s.condition.is_some()).count() as f64
            / genome.steps.len().max(1) as f64
            * 0.05;

        genome.fitness =
            success_rate * 0.7 + time_score * 0.15 + error_handling_bonus + condition_bonus;
    }

    pub async fn run(
        &mut self,
        skill: &Skill,
        test_trajectories: &[&Trajectory],
    ) -> Option<SkillModification> {
        self.initialize(skill);

        // P1-7: 用 max_generations 显式控制循环次数，避免依赖 is_converged
        // 之外依赖 evolve_generation_v2 是否返回 None（它几乎总是 Some）
        for _generation in 0..self.config.max_generations {
            // 提前检查是否已收敛
            if let Some(ref pop) = self.population {
                if pop.is_converged(&self.config) {
                    break;
                }
                if pop.generation >= self.config.max_generations as u32 {
                    break;
                }
            }
            if self.evolve_generation_v2(test_trajectories).await.is_none() {
                break;
            }
        }

        self.population.as_ref()?.best_individual().map(|best| {
            let is_improved = best.fitness > skill.quality_score;

            let validation_result = if is_improved && self.config.use_execution_validation {
                Some(SkillValidationResult {
                    success: true,
                    quality_delta: best.fitness - skill.quality_score,
                    issues: Vec::new(),
                })
            } else if is_improved {
                Some(SkillValidationResult {
                    success: true,
                    quality_delta: best.fitness - skill.quality_score,
                    issues: vec!["Validation without execution - consider enabling sandbox validation".to_string()],
                })
            } else {
                None
            };

            SkillModification {
                modification_type: crate::skill::ModificationType::LogicRevision,
                old_content: Some(skill.content.clone()),
                new_content: best.content.clone(),
                reason: format!(
                    "Evolution improved fitness from {:.3} to {:.3} in {} generations (llm_mutation={}, exec_validation={})",
                    skill.quality_score,
                    best.fitness,
                    self.population.as_ref().map(|p| p.generation).unwrap_or(0),
                    self.config.use_llm_mutation,
                    self.config.use_execution_validation,
                ),
                confidence: best.fitness,
                validation_result,
            }
        })
    }

    pub fn get_stats(&self) -> EvolutionStats {
        match &self.population {
            Some(pop) => EvolutionStats {
                generation: pop.generation,
                best_fitness: pop.best_fitness,
                avg_fitness: pop.avg_fitness,
                fitness_history: pop.fitness_history.clone(),
                converged: pop.is_converged(&self.config),
            },
            None => EvolutionStats {
                generation: 0,
                best_fitness: 0.0,
                avg_fitness: 0.0,
                fitness_history: vec![],
                converged: false,
            },
        }
    }

    pub fn is_running(&self) -> bool {
        self.population.is_some()
    }

    /// 判断 Skill 是否需要自动进化（T3.2：贝叶斯后验决策，替代 if-else 启发式）。
    ///
    /// 用 `EvolutionDecider::from_skill` 从累计统计构建 Beta 后验，按
    /// `evolve_threshold / stable_threshold / min_evidence` 输出决策，
    /// 仅 `EvolutionDecision::Evolve` 返回 true。语义等价性：默认阈值
    /// （触发 0.4 / 最小证据 3.0）与原启发式（成功率 < 0.5 且 >= 3 次使用）
    /// 基本一致，但额外融入连续失败加权与 95% 置信下界的小样本保护。
    pub fn should_auto_evolve(&self, skill: &Skill) -> bool {
        let decider = EvolutionDecider::from_skill(skill).with_thresholds(
            self.config.evolve_threshold,
            self.config.stable_threshold,
            self.config.min_evidence,
        );
        decider.should_evolve()
    }

    /// T3.2：返回完整的贝叶斯进化决策（`Evolve / Stable / Observe`）+ 中文原因，
    /// 供决策标签持久化 / 日志展示。
    pub fn evolution_decision(&self, skill: &Skill) -> (EvolutionDecision, String) {
        let decider = EvolutionDecider::from_skill(skill).with_thresholds(
            self.config.evolve_threshold,
            self.config.stable_threshold,
            self.config.min_evidence,
        );
        decider.describe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: u32,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub fitness_history: Vec<f64>,
    pub converged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_config() {
        let config = EvolutionConfig::default();
        assert_eq!(config.population_size, 20);
        assert_eq!(config.elite_count, 4);
        assert!(config.use_llm_mutation);
    }

    #[test]
    fn test_skill_genome_creation() {
        let genome = SkillGenome {
            skill_id: "test".to_string(),
            content: "1. Use tool1\n2. Use tool2".to_string(),
            description: "Test skill".to_string(),
            steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            fitness: 0.5,
        };

        assert_eq!(genome.steps.len(), 1);
    }

    #[test]
    fn test_parse_skill_content() {
        let content = "1. Use write_file with args\n2. Use execute_bash";
        let steps = parse_skill_content(content);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_llm_mutation_request_serialization() {
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec!["timeout".to_string()],
            success_evidence: vec![],
        };
        let json = serde_json::to_string(&request).expect("测试：JSON序列化应成功");
        assert!(json.contains("test"));
    }

    #[tokio::test]
    async fn test_default_llm_provider() {
        let provider = DefaultLlmEvolutionProvider;
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec!["error occurred".to_string()],
            success_evidence: vec![],
        };
        let response = provider.generate_mutation(&request).await.expect("测试：异步操作应成功");
        assert!(response.revised_steps[0].error_handling.is_some());
    }

    #[tokio::test]
    async fn test_engine_with_llm_provider() {
        let engine = SkillEvolutionEngine::new();
        engine.set_llm_provider(Arc::new(DefaultLlmEvolutionProvider)).await;
        assert!(engine.llm_provider.read().await.is_some());
    }

    // ── T3.4：LLM 结构化 diff + 沙箱验证 集成测试 ──

    /// 构造最小 `Skill` 测试实例（Skill 未实现 Default，需手动构造）。
    fn test_skill() -> Skill {
        use chrono::Utc;
        Skill {
            id: "skill-evolve-test".to_string(),
            name: "测试技能".to_string(),
            description: "测试技能 分析".to_string(),
            version: "1.0.0".to_string(),
            content: "1. Use execute_bash\n2. Verify output".to_string(),
            category: "test".to_string(),
            tags: vec![],
            platforms: vec![],
            scenarios: vec![],
            quality_score: 0.3,
            success_rate: 0.3,
            avg_execution_time_ms: 100,
            total_usages: 10,
            successful_usages: 3,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            consecutive_failures: 3,
            last_failure_at: None,
            metadata: crate::skill::SkillMetadata::default(),
        }
    }

    fn test_trajectory(topic: &str, summary: &str, outcome: TrajectoryOutcome) -> Trajectory {
        Trajectory::new(
            "sess-evolve".to_string(),
            "user-1".to_string(),
            topic.to_string(),
            summary.to_string(),
            outcome,
            1000,
            vec![],
        )
    }

    #[tokio::test]
    async fn test_evolve_generation_v2_uses_llm_diff_and_sandbox() {
        use crate::sandbox_executor::DryRunSandboxExecutor;

        // 构造 > MIN_TRAJECTORIES_FOR_EVOLUTION 的轨迹，主题命中技能描述
        let mut trajectories = Vec::new();
        for i in 0..12 {
            let outcome = if i % 3 == 0 {
                TrajectoryOutcome::Success
            } else {
                TrajectoryOutcome::Failure
            };
            trajectories.push(test_trajectory("测试技能 分析", &format!("summary {i}"), outcome));
        }
        let refs: Vec<&Trajectory> = trajectories.iter().collect();

        let mut engine = SkillEvolutionEngine::new();
        engine.set_llm_provider(Arc::new(DefaultLlmEvolutionProvider)).await;
        engine.set_sandbox(Arc::new(DryRunSandboxExecutor::with_default_policy())).await;

        let skill = test_skill();
        engine.initialize(&skill);
        let before_gen = engine.population.as_ref().unwrap().generation;

        let result = engine.evolve_generation_v2(&refs).await;
        let after_gen = engine.population.as_ref().unwrap().generation;

        // 进化代际推进，说明 LLM 变异 + 沙箱验证闭环被执行
        assert!(after_gen > before_gen, "进化代际应推进");
        let best = result.expect("应返回 best individual");
        assert!(!best.steps.is_empty(), "best individual 应包含步骤");
    }

    #[tokio::test]
    async fn test_evolve_generation_v2_skips_small_sample() {
        // 轨迹数 < MIN_TRAJECTORIES_FOR_EVOLUTION → 不执行进化，返回 best
        let trajectories =
            vec![test_trajectory("测试技能 分析", "s", TrajectoryOutcome::Failure); 3];
        let refs: Vec<&Trajectory> = trajectories.iter().collect();

        let mut engine = SkillEvolutionEngine::new();
        engine.initialize(&test_skill());
        let before_gen = engine.population.as_ref().unwrap().generation;

        let result = engine.evolve_generation_v2(&refs).await;
        let after_gen = engine.population.as_ref().unwrap().generation;

        assert_eq!(after_gen, before_gen, "小样本不应推进进化代际");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_evolution_run_returns_modification() {
        // 完整 run() 闭环：初始化 → 多代进化 → 产出 SkillModification
        let mut trajectories = Vec::new();
        for i in 0..12 {
            let outcome = if i % 4 == 0 {
                TrajectoryOutcome::Success
            } else {
                TrajectoryOutcome::Failure
            };
            trajectories.push(test_trajectory("测试技能 分析", &format!("summary {i}"), outcome));
        }
        let refs: Vec<&Trajectory> = trajectories.iter().collect();

        let mut engine = SkillEvolutionEngine::new();
        engine.set_llm_provider(Arc::new(DefaultLlmEvolutionProvider)).await;
        let skill = test_skill();
        let modification = engine.run(&skill, &refs).await;
        // run() 至少返回 best individual 映射的修改
        assert!(modification.is_some());
    }
}
