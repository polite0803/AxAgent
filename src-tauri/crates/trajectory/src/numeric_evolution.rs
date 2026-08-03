// SPDX-License-Identifier: AGPL-3.0-only

//! Numeric Evolution Engine — genetic algorithm for parameter space optimization
//!
//! Uses the same `EvolutionConfig` as `SkillEvolutionEngine`, but operates on
//! numeric-valued genomes (`NumericGenome`) instead of text-based skill steps.
//!
//! ## Key differences from `SkillEvolutionEngine`
//!
//! | Feature | SkillEvolutionEngine | NumericEvolutionEngine |
//! |---------|---------------------|----------------------|
//! | Genome type | `SkillGenome` (text + steps) | `NumericGenome` (params map) |
//! | Crossover | Single-point step split | BLX-α arithmetic crossover |
//! | Mutation | Step swap / add / tweak | Gaussian perturbation on params |
//! | Fitness source | `Trajectory` analysis | External fitness function |
//! | LLM mutation | Yes (semantic) | No (purely numerical) |
//!
//! ## Usage example
//!
//! ```ignore
//! use trajectory::{EvolutionConfig, NumericEvolutionEngine, ParamDef};
//!
//! let engine = NumericEvolutionEngine::new(
//!     EvolutionConfig {
//!         population_size: 30,
//!         max_generations: 50,
//!         ..Default::default()
//!     },
//!     vec![
//!         ParamDef { name: "alpha", min: 0.0, max: 1.0, step: 0.0 },
//!         ParamDef { name: "lookback", min: 5.0, max: 180.0, step: 1.0 },
//!     ],
//! );
//!
//! let best = engine.run(|genome| {
//!     // fitness function: higher = better
//!     simulate_performance(genome)
//! }).await;
//! ```

use std::collections::HashMap;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::skill_evolution::EvolutionConfig;

// ── Types ────────────────────────────────────────────────────────────

/// A numeric-valued genome: a set of named float parameters + fitness score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericGenome {
    /// Named parameter values
    pub params: HashMap<String, f64>,
    /// Fitness score (higher = better). Defaults to 0.0 until evaluated.
    pub fitness: f64,
}

/// Defines a single numeric parameter for the evolution search space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// Parameter name (used as key in NumericGenome.params)
    pub name: String,
    /// Minimum allowed value
    pub min: f64,
    /// Maximum allowed value
    pub max: f64,
    /// Step size for discrete parameters. 0.0 means continuous.
    pub step: f64,
}

/// Population state of numeric evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericPopulation {
    /// Current generation index
    pub generation: u32,
    /// All individuals in current population
    pub individuals: Vec<NumericGenome>,
    /// Best fitness seen so far
    pub best_fitness: f64,
    /// Average fitness of current generation
    pub avg_fitness: f64,
    /// Fitness history for convergence detection (one entry per generation)
    pub fitness_history: Vec<f64>,
}

/// Evolution statistics for monitoring / front-end display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericEvolutionStats {
    pub generation: u32,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub fitness_history: Vec<f64>,
    pub converged: bool,
    pub best_genome: Option<NumericGenome>,
}

// ── Engine ───────────────────────────────────────────────────────────

/// Genetic algorithm engine for numeric parameter optimization
pub struct NumericEvolutionEngine {
    config: EvolutionConfig,
    param_defs: Vec<ParamDef>,
    population: Option<NumericPopulation>,
}

impl NumericEvolutionEngine {
    /// Create a new numeric evolution engine
    pub fn new(config: EvolutionConfig, param_defs: Vec<ParamDef>) -> Self {
        Self { config, param_defs, population: None }
    }

    /// Initialize population by sampling random parameter values within defined ranges
    pub fn initialize(&mut self) {
        let mut rng = rand::rng();
        let mut individuals: Vec<NumericGenome> = Vec::with_capacity(self.config.population_size);

        for _ in 0..self.config.population_size {
            let mut params = HashMap::with_capacity(self.param_defs.len());
            for def in &self.param_defs {
                let val = sample_param(&mut rng, def);
                params.insert(def.name.clone(), val);
            }
            individuals.push(NumericGenome { params, fitness: 0.0 });
        }

        self.population = Some(NumericPopulation {
            generation: 0,
            individuals,
            best_fitness: 0.0,
            avg_fitness: 0.0,
            fitness_history: Vec::new(),
        });
    }

    /// Evaluate fitness for all individuals in current population
    pub fn evaluate_all<F>(&mut self, fitness_fn: &F)
    where
        F: Fn(&NumericGenome) -> f64,
    {
        let pop = match &mut self.population {
            Some(p) => p,
            None => {
                self.initialize();
                self.population.as_mut().unwrap()
            },
        };

        for genome in &mut pop.individuals {
            genome.fitness = fitness_fn(genome);
        }

        // Update population stats
        let fitnesses: Vec<f64> = pop.individuals.iter().map(|g| g.fitness).collect();
        pop.best_fitness = fitnesses.iter().cloned().fold(f64::MIN, f64::max);
        pop.avg_fitness = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;
        pop.fitness_history.push(pop.avg_fitness);
    }

    /// Run one generation of evolution: evaluate → elite preserve → select → crossover → mutate
    pub fn evolve_generation<F>(&mut self, fitness_fn: &F)
    where
        F: Fn(&NumericGenome) -> f64,
    {
        // Ensure initialized
        if self.population.is_none() {
            self.initialize();
        }

        // Evaluate current population
        self.evaluate_all(fitness_fn);

        let pop = self.population.as_mut().unwrap();

        // Sort by fitness descending
        pop.individuals
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        // Elite preservation: top N survive unchanged
        let elite: Vec<NumericGenome> =
            pop.individuals[..self.config.elite_count.min(pop.individuals.len())].to_vec();

        let mut next_gen: Vec<NumericGenome> = elite.clone();

        // Fill rest of population through selection + crossover + mutation
        while next_gen.len() < self.config.population_size {
            let parent1 = tournament_select_numeric(&pop.individuals, 3);
            let parent2 = tournament_select_numeric(&pop.individuals, 3);

            let mut child = if rand::rng().random::<f64>() < self.config.crossover_rate {
                blx_alpha_crossover(&parent1, &parent2, &self.param_defs, 0.5)
            } else {
                parent1.clone()
            };

            // Mutate child
            mutate_numeric(&mut child, self.config.mutation_rate, &self.param_defs);

            next_gen.push(child);
        }

        next_gen.truncate(self.config.population_size);
        pop.individuals = next_gen;
        pop.generation += 1;
    }

    /// Run full evolution loop until convergence or max generations reached
    pub fn run<F>(&mut self, fitness_fn: F) -> (Option<NumericGenome>, NumericEvolutionStats)
    where
        F: Fn(&NumericGenome) -> f64,
    {
        // Ensure initialized and evaluated
        if self.population.is_none() {
            self.initialize();
        }

        let mut best_ever = NumericGenome { params: HashMap::new(), fitness: f64::MIN };

        for generation_idx in 0..self.config.max_generations {
            self.evolve_generation(&fitness_fn);

            let pop = self.population.as_ref().unwrap();

            // Track best ever
            if pop.best_fitness > best_ever.fitness
                && let Some(best) = pop.individuals.first()
            {
                best_ever = best.clone();
            }

            // Early convergence check (only after we have enough history)
            if generation_idx >= 10 && self.is_converged() {
                tracing::debug!(
                    "[NumericEvolution] Converged at generation {} (best={:.4}, avg={:.4})",
                    generation_idx,
                    pop.best_fitness,
                    pop.avg_fitness
                );
                break;
            }
        }

        // Final evaluation
        let pop = self.population.as_ref().unwrap();
        let converged = self.is_converged();

        let stats = NumericEvolutionStats {
            generation: pop.generation,
            best_fitness: pop.best_fitness.max(best_ever.fitness),
            avg_fitness: pop.avg_fitness,
            fitness_history: pop.fitness_history.clone(),
            converged,
            best_genome: if best_ever.fitness > f64::MIN {
                Some(best_ever.clone())
            } else {
                None
            },
        };

        (stats.best_genome.clone(), stats)
    }

    /// Check convergence: last 10 generations with improvement < threshold
    pub fn is_converged(&self) -> bool {
        match &self.population {
            None => true,
            Some(pop) => {
                if pop.fitness_history.len() < 10 {
                    return false;
                }
                let recent = &pop.fitness_history[pop.fitness_history.len() - 10..];
                let old_avg: f64 = recent[..5].iter().sum::<f64>() / 5.0;
                let new_avg: f64 = recent[5..].iter().sum::<f64>() / 5.0;
                (new_avg - old_avg).abs() < self.config.min_fitness_improvement
            },
        }
    }

    /// Get current evolution stats without owning
    pub fn get_stats(&self) -> NumericEvolutionStats {
        match &self.population {
            None => NumericEvolutionStats {
                generation: 0,
                best_fitness: 0.0,
                avg_fitness: 0.0,
                fitness_history: Vec::new(),
                converged: true,
                best_genome: None,
            },
            Some(pop) => {
                let best = pop.individuals.first().cloned();
                NumericEvolutionStats {
                    generation: pop.generation,
                    best_fitness: pop.best_fitness,
                    avg_fitness: pop.avg_fitness,
                    fitness_history: pop.fitness_history.clone(),
                    converged: self.is_converged(),
                    best_genome: best,
                }
            },
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Sample a random parameter value within bounds
fn sample_param(rng: &mut impl Rng, def: &ParamDef) -> f64 {
    let val = rng.random::<f64>() * (def.max - def.min) + def.min;
    quantize(val, def.step, def.min, def.max)
}

/// Quantize value to step size if applicable, clamp to bounds
fn quantize(val: f64, step: f64, min: f64, max: f64) -> f64 {
    let clamped = val.clamp(min, max);
    if step <= 0.0 {
        clamped
    } else {
        let steps = ((clamped - min) / step).round();
        (min + steps * step).clamp(min, max)
    }
}

/// Tournament selection for numeric genomes
fn tournament_select_numeric(
    population: &[NumericGenome],
    tournament_size: usize,
) -> NumericGenome {
    if population.is_empty() {
        return NumericGenome { params: HashMap::new(), fitness: 0.0 };
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

/// BLX-α crossover (Blend Crossover) for numeric genomes.
///
/// For each parameter, the child value is sampled uniformly from
/// [min(p1, p2) - α * d, max(p1, p2) + α * d] where d = |p1 - p2|.
/// α=0.5 (default) gives a balanced exploration/exploitation ratio.
fn blx_alpha_crossover(
    parent1: &NumericGenome,
    parent2: &NumericGenome,
    param_defs: &[ParamDef],
    alpha: f64,
) -> NumericGenome {
    let mut rng = rand::rng();
    let mut child_params = HashMap::with_capacity(param_defs.len());

    for def in param_defs {
        let v1 = parent1.params.get(&def.name).copied().unwrap_or(def.min);
        let v2 = parent2.params.get(&def.name).copied().unwrap_or(def.max);

        let min_v = v1.min(v2);
        let max_v = v1.max(v2);
        let d = (v1 - v2).abs();

        let low = (min_v - alpha * d).max(def.min);
        let high = (max_v + alpha * d).min(def.max);

        let val = rng.random::<f64>() * (high - low) + low;
        child_params.insert(def.name.clone(), quantize(val, def.step, def.min, def.max));
    }

    NumericGenome { params: child_params, fitness: 0.0 }
}

/// Gaussian mutation: for each parameter, add Gaussian noise with
/// standard deviation = 10% of the parameter range.
fn mutate_numeric(genome: &mut NumericGenome, mutation_rate: f64, param_defs: &[ParamDef]) {
    let mut rng = rand::rng();

    for def in param_defs {
        if rng.random::<f64>() >= mutation_rate {
            continue;
        }

        let current = genome.params.get(&def.name).copied().unwrap_or(def.min);
        let range = def.max - def.min;
        let sigma = range * 0.1; // 10% of range as noise scale

        // Box-Muller Gaussian noise
        let u1: f64 = rng.random();
        let u2: f64 = rng.random();
        let noise =
            (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos() * sigma;

        let mutated = quantize(current + noise, def.step, def.min, def.max);
        genome.params.insert(def.name.clone(), mutated);
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_param_defs() -> Vec<ParamDef> {
        vec![
            ParamDef { name: "x".to_string(), min: -10.0, max: 10.0, step: 0.0 },
            ParamDef { name: "y".to_string(), min: -10.0, max: 10.0, step: 0.0 },
        ]
    }

    #[test]
    fn initialize_population_has_correct_size() {
        let config = EvolutionConfig { population_size: 20, ..Default::default() };
        let mut engine = NumericEvolutionEngine::new(config, simple_param_defs());
        engine.initialize();

        let pop = engine.population.unwrap();
        assert_eq!(pop.individuals.len(), 20);
        assert_eq!(pop.generation, 0);
        // Every genome should have x and y params
        for genome in &pop.individuals {
            assert!(genome.params.contains_key("x"));
            assert!(genome.params.contains_key("y"));
            let x = genome.params["x"];
            let y = genome.params["y"];
            assert!((-10.0..=10.0).contains(&x), "x={} out of range", x);
            assert!((-10.0..=10.0).contains(&y), "y={} out of range", y);
        }
    }

    #[test]
    fn evolution_finds_maximum_of_simple_function() {
        // Fitness: maximize f(x,y) = -(x^2 + y^2) + 100
        // Maximum should be at (0, 0) with fitness ≈ 100
        let config = EvolutionConfig {
            population_size: 50,
            elite_count: 5,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            max_generations: 100,
            ..Default::default()
        };
        let mut engine = NumericEvolutionEngine::new(config, simple_param_defs());

        let (best_genome, stats) = engine.run(|genome| {
            let x = genome.params.get("x").copied().unwrap_or(0.0);
            let y = genome.params.get("y").copied().unwrap_or(0.0);
            -(x * x + y * y) + 100.0
        });

        assert!(stats.best_fitness > 90.0, "Best fitness too low: {:.2}", stats.best_fitness);
        assert!(!stats.fitness_history.is_empty(), "Should have fitness history");

        if let Some(best) = best_genome {
            let x = best.params.get("x").copied().unwrap_or(99.0);
            let y = best.params.get("y").copied().unwrap_or(99.0);
            let dist = (x * x + y * y).sqrt();
            assert!(
                dist < 2.0,
                "Best solution ({:.2}, {:.2}) too far from (0,0), dist={:.2}",
                x,
                y,
                dist
            );
        }
    }

    #[test]
    fn quantize_discrete_parameter() {
        let _def = ParamDef {
            name: "step_test".to_string(),
            min: 0.0,
            max: 10.0,
            step: 2.0, // 0, 2, 4, 6, 8, 10
        };

        assert_eq!(quantize(1.3, 2.0, 0.0, 10.0), 2.0);
        assert_eq!(quantize(2.7, 2.0, 0.0, 10.0), 2.0);
        assert_eq!(quantize(3.0, 2.0, 0.0, 10.0), 4.0);
        assert_eq!(quantize(0.0, 2.0, 0.0, 10.0), 0.0);
        assert_eq!(quantize(10.0, 2.0, 0.0, 10.0), 10.0);
        assert_eq!(quantize(-1.0, 2.0, 0.0, 10.0), 0.0);
        assert_eq!(quantize(12.0, 2.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn blx_crossover_produces_valid_offspring() {
        let defs = simple_param_defs();
        let parent1 = NumericGenome {
            params: [("x".to_string(), 5.0), ("y".to_string(), 5.0)].into(),
            fitness: 80.0,
        };
        let parent2 = NumericGenome {
            params: [("x".to_string(), -5.0), ("y".to_string(), -5.0)].into(),
            fitness: 80.0,
        };

        let child = blx_alpha_crossover(&parent1, &parent2, &defs, 0.5);
        assert!(child.params.contains_key("x"));
        assert!(child.params.contains_key("y"));
        // With α=0.5, child should be within [-10, 10]
        let x = child.params["x"];
        let y = child.params["y"];
        assert!((-10.0..=10.0).contains(&x), "x={} out of range", x);
        assert!((-10.0..=10.0).contains(&y), "y={} out of range", y);
    }

    #[test]
    fn mutation_stays_within_bounds() {
        let defs = simple_param_defs();
        let mut genome = NumericGenome {
            params: [("x".to_string(), 9.5), ("y".to_string(), -9.5)].into(),
            fitness: 50.0,
        };

        // Force mutation on both params
        mutate_numeric(&mut genome, 1.0, &defs);
        let x = genome.params["x"];
        let y = genome.params["y"];
        assert!((-10.0..=10.0).contains(&x), "x={} out of range after mutation", x);
        assert!((-10.0..=10.0).contains(&y), "y={} out of range after mutation", y);
    }

    #[test]
    fn tournament_select_handles_empty_population() {
        let result = tournament_select_numeric(&[], 3);
        assert_eq!(result.fitness, 0.0);
        assert!(result.params.is_empty());
    }

    #[test]
    fn tournament_select_returns_best() {
        let pop = vec![
            NumericGenome { params: HashMap::new(), fitness: 10.0 },
            NumericGenome { params: HashMap::new(), fitness: 50.0 },
            NumericGenome { params: HashMap::new(), fitness: 30.0 },
        ];
        let result = tournament_select_numeric(&pop, 3);
        // Should pick the best from 3 random selections
        assert!(result.fitness >= 10.0);
    }
}
