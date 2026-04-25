//! Skill Evolution System - GEPA-inspired skill improvement through genetic algorithms
//!
//! Features:
//! - Constraint-gated evolution
//! - Fitness evaluation based on success rate and execution time
//! - Crossover and mutation operators
//! - Multi-objective optimization (quality vs speed)
//! - Convergence detection

use crate::skill::{Skill, SkillModification, ValidationResult};
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_generations: usize,
    pub convergence_threshold: f64,
    pub min_fitness_improvement: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub order: usize,
    pub action: String,
    pub tool: Option<String>,
    pub condition: Option<String>,
    pub error_handling: Option<String>,
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

            let child = if rand::thread_rng().gen::<f64>() < config.crossover_rate {
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

        let recent: Vec<f64> = self.fitness_history[self.fitness_history.len()..].to_vec();
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

    let cross_point = rng.gen_range(1..parent1.steps.len().max(1));

    let mut child_steps = parent1.steps[..cross_point].to_vec();
    child_steps.extend_from_slice(&parent2.steps[cross_point..]);

    let child_content = serialize_steps(&child_steps);

    SkillGenome {
        skill_id: parent1.skill_id.clone(),
        content: child_content,
        description: if rng.gen::<bool>() {
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

    for step in &mut new_steps {
        if rng.gen::<f64>() < mutation_rate {
            if rng.gen::<f64>() < 0.3 {
                step.action = step.action.to_uppercase();
            } else if rng.gen::<f64>() < 0.3 {
                step.action = step.action.to_lowercase();
            }

            if rng.gen::<f64>() < mutation_rate && step.order > 0 {
                step.order = step.order.saturating_sub(1);
            }
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

pub struct SkillEvolutionEngine {
    config: EvolutionConfig,
    population: Option<EvolutionPopulation>,
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
        }
    }

    pub fn with_config(config: EvolutionConfig) -> Self {
        Self {
            config,
            population: None,
        }
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
                matches!(
                    t.outcome,
                    TrajectoryOutcome::Success | TrajectoryOutcome::Partial
                )
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

        genome.fitness = success_rate * 0.8 + time_score * 0.2;
    }

    pub fn run(
        &mut self,
        skill: &Skill,
        test_trajectories: &[&Trajectory],
    ) -> Option<SkillModification> {
        self.initialize(skill);

        loop {
            if self.evolve_generation(test_trajectories).is_none() {
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

            SkillModification {
                modification_type: crate::skill::ModificationType::LogicRevision,
                old_content: Some(skill.content.clone()),
                new_content: best.content.clone(),
                reason: format!(
                    "Evolution improved fitness from {:.3} to {:.3} in {} generations",
                    skill.quality_score,
                    best.fitness,
                    self.population.as_ref().map(|p| p.generation).unwrap_or(0)
                ),
                confidence: best.fitness,
                validation_result: if is_improved {
                    Some(ValidationResult {
                        success: true,
                        quality_delta: best.fitness - skill.quality_score,
                        issues: Vec::new(),
                    })
                } else {
                    None
                },
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
}
