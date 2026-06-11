// SPDX-License-Identifier: AGPL-3.0-only

//! Skill Evolution System - GEPA-inspired skill improvement through genetic algorithms
//!
//! Features:
//! - Constraint-gated evolution
//! - Fitness evaluation based on success rate and execution time
//! - Crossover and mutation operators
//! - Multi-objective optimization (quality vs speed)
//! - Convergence detection
//! - LLM-driven semantic mutation (replaces random symbol manipulation)
//! - Execution feedback-driven verification closed loop

pub use axagent_harness::trajectory_types::{
    LlmEvolutionProvider, LlmMutationFuture, LlmMutationRequest, LlmMutationResponse, ProcedureStep,
};

use crate::skill::{Skill, SkillModification, ValidationResult};
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
            use_execution_validation: false,
            validation_rounds: 3,
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
        self.individuals.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let elite: Vec<SkillGenome> = self.individuals[..config.elite_count].to_vec();

        let mut new_individuals = elite.clone();

        while new_individuals.len() < config.population_size {
            let parent1 = tournament_select(&self.individuals, 3);
            let parent2 = tournament_select(&self.individuals, 3);

            let child = if rand::thread_rng().r#gen::<f64>() < config.crossover_rate {
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
    use rand::seq::SliceRandom;

    let mut rng = rand::thread_rng();
    let size = population.len().min(tournament_size);
    let indices: Vec<usize> = (0..population.len()).collect();
    let selected: Vec<usize> = indices.choose_multiple(&mut rng, size).cloned().collect();

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
    let mut rng = rand::thread_rng();

    let cross_point = rng.gen_range(1..parent1.steps.len().max(2));

    let mut child_steps = parent1.steps[..cross_point.min(parent1.steps.len())].to_vec();
    if cross_point < parent2.steps.len() {
        child_steps.extend_from_slice(&parent2.steps[cross_point..]);
    }

    let child_content = serialize_steps(&child_steps);

    SkillGenome {
        skill_id: parent1.skill_id.clone(),
        content: child_content,
        description: if rng.r#gen::<bool>() {
            parent1.description.clone()
        } else {
            parent2.description.clone()
        },
        steps: child_steps,
        fitness: 0.0,
    }
}

fn mutate_genome(genome: &SkillGenome, mutation_rate: f64) -> SkillGenome {
    let mut rng = rand::thread_rng();
    let mut new_steps: Vec<ProcedureStep> = genome.steps.clone();

    for i in 0..new_steps.len() {
        if rng.r#gen::<f64>() >= mutation_rate {
            continue;
        }

        // 随机选择有意义的变异类型
        match rng.gen_range(0u8..4) {
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

        let tool_match: Option<String> = tool_regex
            .captures(trimmed)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

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

pub struct DefaultLlmEvolutionProvider;

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

    fn evaluate_quality(
        &self,
        content: &str,
        _context: &str,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>> {
        let score = (content.len() as f64 / 500.0).min(1.0) * 0.3
            + if content.contains("error") || content.contains("Error") {
                0.3
            } else {
                0.0
            }
            + if content.contains("verify") || content.contains("check") {
                0.2
            } else {
                0.0
            }
            + 0.2;
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

pub struct SkillEvolutionEngine {
    config: EvolutionConfig,
    population: Option<EvolutionPopulation>,
    llm_provider: Option<Arc<dyn LlmEvolutionProvider>>,
    sandbox: Option<Arc<dyn SandboxExecutor>>,
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
            llm_provider: None,
            sandbox: None,
        }
    }

    pub fn with_config(config: EvolutionConfig) -> Self {
        Self {
            config,
            population: None,
            llm_provider: None,
            sandbox: None,
        }
    }

    pub fn set_llm_provider(&mut self, provider: Arc<dyn LlmEvolutionProvider>) {
        self.llm_provider = Some(provider);
    }

    pub fn set_sandbox(&mut self, executor: Arc<dyn SandboxExecutor>) {
        self.config.use_execution_validation = true;
        self.sandbox = Some(executor);
    }

    pub fn skill_count(&self) -> usize {
        self.population
            .as_ref()
            .map(|p| p.individuals.len())
            .unwrap_or(0)
    }

    pub fn has_llm_provider(&self) -> bool {
        self.llm_provider.is_some()
    }

    pub fn has_sandbox(&self) -> bool {
        self.sandbox.is_some()
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
            return self
                .population
                .as_ref()
                .and_then(|p| p.best_individual())
                .cloned();
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

        self.population
            .as_ref()
            .and_then(|p| p.best_individual())
            .cloned()
    }

    pub async fn evolve_generation_v2(
        &mut self,
        test_trajectories: &[&Trajectory],
    ) -> Option<SkillGenome> {
        let should_evolve = if let Some(ref pop) = self.population {
            pop.generation < self.config.max_generations as u32 && !pop.is_converged(&self.config)
        } else {
            false
        };

        if !should_evolve {
            return self
                .population
                .as_ref()
                .and_then(|p| p.best_individual())
                .cloned();
        }

        if let Some(ref mut pop) = self.population {
            if self.config.use_llm_mutation
                && let Some(ref provider) = self.llm_provider
            {
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
                        individual.steps = response.revised_steps;
                        individual.content = serialize_steps(&individual.steps);
                    }
                }
            }

            for individual in &mut pop.individuals {
                Self::evaluate_fitness_static(individual, test_trajectories);
            }

            if self.config.use_execution_validation
                && let Some(ref sandbox) = self.sandbox
            {
                for individual in &mut pop.individuals.iter_mut() {
                    let mut total_success = 0.0;
                    let mut rounds = 0;
                    for trajectory in test_trajectories.iter().take(self.config.validation_rounds) {
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

            let before_gen = pop.generation;
            pop.evolve(&self.config);

            if before_gen != pop.generation {
                return pop.best_individual().cloned();
            }
        }

        self.population
            .as_ref()
            .and_then(|p| p.best_individual())
            .cloned()
    }

    fn evaluate_fitness_static(genome: &mut SkillGenome, test_trajectories: &[&Trajectory]) {
        let relevant: Vec<&Trajectory> = test_trajectories
            .iter()
            .filter(|t| {
                t.topic
                    .to_lowercase()
                    .contains(&genome.description.to_lowercase())
            })
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

        let error_handling_bonus = genome
            .steps
            .iter()
            .filter(|s| s.error_handling.is_some())
            .count() as f64
            / genome.steps.len().max(1) as f64
            * 0.1;

        let condition_bonus = genome
            .steps
            .iter()
            .filter(|s| s.condition.is_some())
            .count() as f64
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

        loop {
            if self.evolve_generation_v2(test_trajectories).await.is_none() {
                break;
            }

            if let Some(ref pop) = self.population {
                if pop.is_converged(&self.config) {
                    break;
                }
                if pop.generation >= self.config.max_generations as u32 {
                    break;
                }
            }
        }

        self.population.as_ref()?.best_individual().map(|best| {
            let is_improved = best.fitness > skill.quality_score;

            let validation_result = if is_improved && self.config.use_execution_validation {
                Some(ValidationResult {
                    success: true,
                    quality_delta: best.fitness - skill.quality_score,
                    issues: Vec::new(),
                })
            } else if is_improved {
                Some(ValidationResult {
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
        let json = serde_json::to_string(&request).unwrap();
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
        let response = provider.generate_mutation(&request).await.unwrap();
        assert!(response.revised_steps[0].error_handling.is_some());
    }

    #[tokio::test]
    async fn test_engine_with_llm_provider() {
        let mut engine = SkillEvolutionEngine::new();
        engine.set_llm_provider(Arc::new(DefaultLlmEvolutionProvider));
        assert!(engine.llm_provider.is_some());
    }
}
