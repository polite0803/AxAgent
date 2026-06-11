// SPDX-License-Identifier: AGPL-3.0-only

//! ADAS-style architecture auto-search module
//!
//! Provides evolutionary search over agent architecture graphs including:
//! - Representation of agent architectures as directed graphs
//! - Genetic operations (mutation, crossover) on architecture graphs
//! - Heuristic and pluggable evaluation of architecture fitness
//! - Meta-agent guided architecture generation

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentNodeType {
    LlmCall,
    ToolInvocation,
    Condition,
    Parallel,
    Sequential,
    MemoryRead,
    MemoryWrite,
    Custom(String),
}

impl AgentNodeType {
    pub fn is_control_flow(&self) -> bool {
        matches!(self, Self::Condition | Self::Parallel | Self::Sequential)
    }

    pub fn is_memory(&self) -> bool {
        matches!(self, Self::MemoryRead | Self::MemoryWrite)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: String,
    pub node_type: AgentNodeType,
    pub config: HashMap<String, String>,
    pub prompt_template: Option<String>,
}

impl AgentNode {
    pub fn new(node_type: AgentNodeType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            node_type,
            config: HashMap::new(),
            prompt_template: None,
        }
    }

    pub fn with_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }

    pub fn with_prompt_template(mut self, template: impl Into<String>) -> Self {
        self.prompt_template = Some(template.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEdge {
    pub source_id: String,
    pub target_id: String,
    pub condition: Option<String>,
}

impl AgentEdge {
    pub fn new(source_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArchitecture {
    pub id: String,
    pub name: String,
    pub nodes: Vec<AgentNode>,
    pub edges: Vec<AgentEdge>,
    pub fitness: f64,
    pub generation: u32,
}

impl AgentArchitecture {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            fitness: 0.0,
            generation: 0,
        }
    }

    pub fn add_node(&mut self, node: AgentNode) -> String {
        let id = node.id.clone();
        self.nodes.push(node);
        id
    }

    pub fn add_edge(&mut self, edge: AgentEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&AgentNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut AgentNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn node_ids(&self) -> HashSet<&str> {
        self.nodes.iter().map(|n| n.id.as_str()).collect()
    }

    pub fn successors(&self, node_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.source_id == node_id)
            .map(|e| e.target_id.as_str())
            .collect()
    }

    pub fn predecessors(&self, node_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.target_id == node_id)
            .map(|e| e.source_id.as_str())
            .collect()
    }

    pub fn root_nodes(&self) -> Vec<&AgentNode> {
        let has_incoming: HashSet<&str> = self.edges.iter().map(|e| e.target_id.as_str()).collect();
        self.nodes
            .iter()
            .filter(|n| !has_incoming.contains(n.id.as_str()))
            .collect()
    }

    pub fn leaf_nodes(&self) -> Vec<&AgentNode> {
        let has_outgoing: HashSet<&str> = self.edges.iter().map(|e| e.source_id.as_str()).collect();
        self.nodes
            .iter()
            .filter(|n| !has_outgoing.contains(n.id.as_str()))
            .collect()
    }

    pub fn connected_components(&self) -> Vec<HashSet<String>> {
        let mut parent: HashMap<&str, &str> = HashMap::new();
        for node in &self.nodes {
            parent.insert(&node.id, &node.id);
        }

        fn find<'a>(parent: &HashMap<&'a str, &'a str>, x: &'a str) -> &'a str {
            let p = parent[x];
            if p == x { p } else { find(parent, p) }
        }

        for edge in &self.edges {
            let root_s = find(&parent, &edge.source_id);
            let root_t = find(&parent, &edge.target_id);
            if root_s != root_t {
                parent.insert(root_s, root_t);
            }
        }

        let mut components: HashMap<String, HashSet<String>> = HashMap::new();
        for node in &self.nodes {
            let root = find(&parent, &node.id).to_string();
            components.entry(root).or_default().insert(node.id.clone());
        }
        components.into_values().collect()
    }

    pub fn node_type_diversity(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        let types: HashSet<String> = self
            .nodes
            .iter()
            .map(|n| serde_json::to_string(&n.node_type).unwrap_or_default())
            .collect();
        types.len() as f64 / self.nodes.len() as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSearchConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub max_generations: u32,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_nodes: usize,
    pub max_depth: usize,
}

impl Default for ArchitectureSearchConfig {
    fn default() -> Self {
        Self {
            population_size: 30,
            elite_count: 4,
            max_generations: 50,
            mutation_rate: 0.3,
            crossover_rate: 0.5,
            max_nodes: 12,
            max_depth: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSearchSpace {
    pub allowed_node_types: Vec<AgentNodeType>,
    pub max_nodes: usize,
    pub available_tools: Vec<String>,
}

impl ArchitectureSearchSpace {
    pub fn default_search_space() -> Self {
        Self {
            allowed_node_types: vec![
                AgentNodeType::LlmCall,
                AgentNodeType::ToolInvocation,
                AgentNodeType::Condition,
                AgentNodeType::Parallel,
                AgentNodeType::Sequential,
                AgentNodeType::MemoryRead,
                AgentNodeType::MemoryWrite,
            ],
            max_nodes: 12,
            available_tools: Vec::new(),
        }
    }
}

pub trait ArchitectureEvaluator: Send + Sync {
    fn evaluate(
        &self,
        architecture: &AgentArchitecture,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>>;
}

pub struct DefaultArchitectureEvaluator {
    optimal_node_range: (usize, usize),
}

impl DefaultArchitectureEvaluator {
    pub fn new() -> Self {
        Self {
            optimal_node_range: (3, 8),
        }
    }

    pub fn with_optimal_range(mut self, min: usize, max: usize) -> Self {
        self.optimal_node_range = (min, max);
        self
    }

    fn compute_balance_score(&self, arch: &AgentArchitecture) -> f64 {
        let n = arch.nodes.len();
        if n == 0 {
            return 0.0;
        }
        let (lo, hi) = self.optimal_node_range;
        if n >= lo && n <= hi {
            1.0
        } else if n < lo {
            n as f64 / lo as f64
        } else {
            let excess = (n - hi) as f64;
            1.0 / (1.0 + excess * 0.2)
        }
    }

    fn compute_diversity_score(&self, arch: &AgentArchitecture) -> f64 {
        arch.node_type_diversity()
    }

    fn compute_connectivity_score(&self, arch: &AgentArchitecture) -> f64 {
        if arch.nodes.is_empty() {
            return 0.0;
        }
        let components = arch.connected_components();
        if components.len() <= 1 {
            1.0
        } else {
            1.0 / components.len() as f64
        }
    }

    fn compute_io_flow_score(&self, arch: &AgentArchitecture) -> f64 {
        if arch.nodes.is_empty() {
            return 0.0;
        }
        let roots = arch.root_nodes();
        let leaves = arch.leaf_nodes();
        let has_root = !roots.is_empty();
        let has_leaf = !leaves.is_empty();
        let root_has_llm_or_seq = roots
            .iter()
            .any(|n| matches!(n.node_type, AgentNodeType::LlmCall | AgentNodeType::Sequential));
        let leaf_has_llm = leaves
            .iter()
            .any(|n| matches!(n.node_type, AgentNodeType::LlmCall));
        let mut score = 0.0;
        if has_root {
            score += 0.25;
        }
        if has_leaf {
            score += 0.25;
        }
        if root_has_llm_or_seq {
            score += 0.25;
        }
        if leaf_has_llm {
            score += 0.25;
        }
        score
    }
}

impl Default for DefaultArchitectureEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchitectureEvaluator for DefaultArchitectureEvaluator {
    fn evaluate(
        &self,
        architecture: &AgentArchitecture,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>> {
        let balance = self.compute_balance_score(architecture);
        let diversity = self.compute_diversity_score(architecture);
        let connectivity = self.compute_connectivity_score(architecture);
        let io_flow = self.compute_io_flow_score(architecture);
        let fitness = balance * 0.25 + diversity * 0.25 + connectivity * 0.25 + io_flow * 0.25;
        Box::pin(async move { Ok(fitness) })
    }
}

pub trait MetaAgentProvider: Send + Sync {
    fn generate_architecture(
        &self,
        search_space: &ArchitectureSearchSpace,
        history: &[AgentArchitecture],
    ) -> Pin<Box<dyn Future<Output = Result<AgentArchitecture, String>> + Send + '_>>;
}

pub struct RandomMetaAgent;

impl RandomMetaAgent {
    pub fn new() -> Self {
        Self
    }

    fn generate_random_architecture(
        search_space: &ArchitectureSearchSpace,
        generation: u32,
    ) -> AgentArchitecture {
        let mut rng = rand::thread_rng();
        let node_count = rng.gen_range(2..=search_space.max_nodes.min(8));
        let mut arch = AgentArchitecture::new(format!("arch_gen{}", generation));
        arch.generation = generation;

        let mut node_ids: Vec<String> = Vec::new();
        for _ in 0..node_count {
            let node_type_idx = rng.gen_range(0..search_space.allowed_node_types.len());
            let node_type = search_space.allowed_node_types[node_type_idx].clone();
            let mut node = AgentNode::new(node_type);
            if !search_space.available_tools.is_empty()
                && matches!(node.node_type, AgentNodeType::ToolInvocation)
            {
                let tool_idx = rng.gen_range(0..search_space.available_tools.len());
                node.config
                    .insert("tool".into(), search_space.available_tools[tool_idx].clone());
            }
            let id = node.id.clone();
            arch.add_node(node);
            node_ids.push(id);
        }

        if node_ids.len() > 1 {
            for i in 0..node_ids.len() - 1 {
                if rng.gen_bool(0.7) {
                    arch.add_edge(AgentEdge::new(&node_ids[i], &node_ids[i + 1]));
                }
            }
            for i in 0..node_ids.len() {
                for j in (i + 2)..node_ids.len() {
                    if rng.gen_bool(0.15) {
                        arch.add_edge(AgentEdge::new(&node_ids[i], &node_ids[j]));
                    }
                }
            }
        }

        arch
    }
}

impl Default for RandomMetaAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaAgentProvider for RandomMetaAgent {
    fn generate_architecture(
        &self,
        search_space: &ArchitectureSearchSpace,
        _history: &[AgentArchitecture],
    ) -> Pin<Box<dyn Future<Output = Result<AgentArchitecture, String>> + Send + '_>> {
        let arch = Self::generate_random_architecture(search_space, 0);
        Box::pin(async move { Ok(arch) })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStatistics {
    pub generation: u32,
    pub population_size: usize,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub diversity_index: f64,
}

pub struct ArchitectureSearchEngine<E: ArchitectureEvaluator, M: MetaAgentProvider> {
    pub config: ArchitectureSearchConfig,
    pub search_space: ArchitectureSearchSpace,
    pub evaluator: E,
    pub meta_agent: M,
    pub population: Vec<AgentArchitecture>,
    pub generation: u32,
    pub best_fitness: f64,
}

impl<E: ArchitectureEvaluator, M: MetaAgentProvider> ArchitectureSearchEngine<E, M> {
    pub fn new(
        config: ArchitectureSearchConfig,
        search_space: ArchitectureSearchSpace,
        evaluator: E,
        meta_agent: M,
    ) -> Self {
        Self {
            config,
            search_space,
            evaluator,
            meta_agent,
            population: Vec::new(),
            generation: 0,
            best_fitness: f64::NEG_INFINITY,
        }
    }

    pub fn initialize_population(&mut self) {
        self.population.clear();
        for i in 0..self.config.population_size {
            let mut arch =
                RandomMetaAgent::generate_random_architecture(&self.search_space, self.generation);
            arch.name = format!("init_{}", i);
            self.population.push(arch);
        }
    }

    pub fn mutate_architecture(&self, arch: &AgentArchitecture) -> AgentArchitecture {
        let mut rng = rand::thread_rng();
        let mut mutant = arch.clone();
        mutant.id = Uuid::new_v4().to_string();
        mutant.generation = self.generation;
        mutant.fitness = 0.0;

        let mutation_type: f64 = rng.r#gen();

        if mutation_type < 0.33 && mutant.nodes.len() < self.config.max_nodes {
            let node_type_idx = rng.gen_range(0..self.search_space.allowed_node_types.len());
            let node_type = self.search_space.allowed_node_types[node_type_idx].clone();
            let mut node = AgentNode::new(node_type);
            if !self.search_space.available_tools.is_empty()
                && matches!(node.node_type, AgentNodeType::ToolInvocation)
            {
                let tool_idx = rng.gen_range(0..self.search_space.available_tools.len());
                node.config
                    .insert("tool".into(), self.search_space.available_tools[tool_idx].clone());
            }
            let new_id = node.id.clone();
            mutant.add_node(node);
            if !mutant.nodes.is_empty() {
                let existing_ids: Vec<String> = mutant.nodes.iter().map(|n| n.id.clone()).collect();
                let target_idx = rng.gen_range(0..existing_ids.len());
                if rng.gen_bool(0.5) {
                    mutant.add_edge(AgentEdge::new(&new_id, &existing_ids[target_idx]));
                } else {
                    mutant.add_edge(AgentEdge::new(&existing_ids[target_idx], &new_id));
                }
            }
        } else if mutation_type < 0.66 && !mutant.nodes.is_empty() {
            let remove_idx = rng.gen_range(0..mutant.nodes.len());
            let removed_id = mutant.nodes[remove_idx].id.clone();
            mutant.nodes.remove(remove_idx);
            mutant
                .edges
                .retain(|e| e.source_id != removed_id && e.target_id != removed_id);
        } else if !mutant.nodes.is_empty() {
            let modify_idx = rng.gen_range(0..mutant.nodes.len());
            let node_type_idx = rng.gen_range(0..self.search_space.allowed_node_types.len());
            mutant.nodes[modify_idx].node_type =
                self.search_space.allowed_node_types[node_type_idx].clone();
            if !self.search_space.available_tools.is_empty()
                && matches!(mutant.nodes[modify_idx].node_type, AgentNodeType::ToolInvocation)
            {
                let tool_idx = rng.gen_range(0..self.search_space.available_tools.len());
                mutant.nodes[modify_idx]
                    .config
                    .insert("tool".into(), self.search_space.available_tools[tool_idx].clone());
            }
        }

        mutant
    }

    pub fn crossover_architectures(
        &self,
        parent1: &AgentArchitecture,
        parent2: &AgentArchitecture,
    ) -> AgentArchitecture {
        let mut rng = rand::thread_rng();
        let mut child =
            AgentArchitecture::new(format!("cross_{}_{}", parent1.generation, parent2.generation));
        child.generation = self.generation;

        if parent1.nodes.is_empty() && parent2.nodes.is_empty() {
            return child;
        }

        let split1 = if parent1.nodes.is_empty() {
            0
        } else {
            rng.gen_range(0..=parent1.nodes.len())
        };
        let split2 = if parent2.nodes.is_empty() {
            0
        } else {
            rng.gen_range(0..=parent2.nodes.len())
        };

        let mut id_map: HashMap<String, String> = HashMap::new();

        for node in &parent1.nodes[..split1] {
            let old_id = node.id.clone();
            let mut new_node = node.clone();
            new_node.id = Uuid::new_v4().to_string();
            id_map.insert(old_id, new_node.id.clone());
            child.nodes.push(new_node);
        }

        for node in &parent2.nodes[split2..] {
            let old_id = node.id.clone();
            let mut new_node = node.clone();
            new_node.id = Uuid::new_v4().to_string();
            id_map.insert(old_id, new_node.id.clone());
            if child.nodes.len() < self.config.max_nodes {
                child.nodes.push(new_node);
            }
        }

        let child_ids: HashSet<&str> = child.nodes.iter().map(|n| n.id.as_str()).collect();

        let mut edges_to_add: Vec<AgentEdge> = Vec::new();

        for edge in &parent1.edges {
            if let (Some(new_s), Some(new_t)) =
                (id_map.get(&edge.source_id), id_map.get(&edge.target_id))
                && child_ids.contains(new_s.as_str())
                && child_ids.contains(new_t.as_str())
            {
                edges_to_add.push(AgentEdge::new(new_s, new_t));
            }
        }

        for edge in &parent2.edges {
            if let (Some(new_s), Some(new_t)) =
                (id_map.get(&edge.source_id), id_map.get(&edge.target_id))
                && child_ids.contains(new_s.as_str())
                && child_ids.contains(new_t.as_str())
            {
                edges_to_add.push(AgentEdge::new(new_s, new_t));
            }
        }

        for edge in edges_to_add {
            child.add_edge(edge);
        }

        child
    }

    fn tournament_select(&self, tournament_size: usize) -> Option<&AgentArchitecture> {
        if self.population.is_empty() {
            return None;
        }
        let mut rng = rand::thread_rng();
        let mut best: Option<&AgentArchitecture> = None;
        for _ in 0..tournament_size.min(self.population.len()) {
            let idx = rng.gen_range(0..self.population.len());
            let candidate = &self.population[idx];
            if best.is_none()
                || candidate.fitness > best.expect("best is set after first iteration").fitness
            {
                best = Some(candidate);
            }
        }
        best
    }

    pub async fn evolve_generation(&mut self) -> Option<&AgentArchitecture> {
        self.generation += 1;

        let mut sorted = self.population.clone();
        sorted.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut new_pop: Vec<AgentArchitecture> = Vec::new();

        let elite_count = self.config.elite_count.min(sorted.len());
        for arch in sorted.iter().take(elite_count) {
            let mut elite = arch.clone();
            elite.generation = self.generation;
            new_pop.push(elite);
        }

        let mut rng = rand::thread_rng();

        while new_pop.len() < self.config.population_size {
            let r: f64 = rng.r#gen();

            if r < self.config.crossover_rate {
                let p1 = self.tournament_select(3);
                let p2 = self.tournament_select(3);
                if let (Some(p1), Some(p2)) = (p1, p2) {
                    let child = self.crossover_architectures(p1, p2);
                    new_pop.push(child);
                    continue;
                }
            }

            if r < self.config.crossover_rate + self.config.mutation_rate {
                let parent = self.tournament_select(3);
                if let Some(p) = parent {
                    let mutant = self.mutate_architecture(p);
                    new_pop.push(mutant);
                    continue;
                }
            }

            let arch =
                RandomMetaAgent::generate_random_architecture(&self.search_space, self.generation);
            new_pop.push(arch);
        }

        for arch in &mut new_pop {
            match self.evaluator.evaluate(arch).await {
                Ok(fitness) => {
                    arch.fitness = fitness;
                    if fitness > self.best_fitness {
                        self.best_fitness = fitness;
                    }
                },
                Err(_) => {
                    arch.fitness = 0.0;
                },
            }
        }

        self.population = new_pop;
        self.get_best()
    }

    pub async fn search(&mut self, generations: u32) -> AgentArchitecture {
        if self.population.is_empty() {
            self.initialize_population();
            for arch in &mut self.population {
                match self.evaluator.evaluate(arch).await {
                    Ok(fitness) => {
                        arch.fitness = fitness;
                        if fitness > self.best_fitness {
                            self.best_fitness = fitness;
                        }
                    },
                    Err(_) => {
                        arch.fitness = 0.0;
                    },
                }
            }
        }

        for _ in 0..generations {
            self.evolve_generation().await;
        }

        self.get_best()
            .cloned()
            .unwrap_or_else(|| AgentArchitecture::new("empty"))
    }

    pub fn get_best(&self) -> Option<&AgentArchitecture> {
        self.population.iter().max_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn get_statistics(&self) -> SearchStatistics {
        let pop_len = self.population.len();
        let best_fitness = self
            .population
            .iter()
            .map(|a| a.fitness)
            .fold(f64::NEG_INFINITY, f64::max);
        let avg_fitness = if pop_len > 0 {
            self.population.iter().map(|a| a.fitness).sum::<f64>() / pop_len as f64
        } else {
            0.0
        };
        let diversity_index = if pop_len > 0 {
            let all_types: Vec<String> = self
                .population
                .iter()
                .flat_map(|a| {
                    a.nodes
                        .iter()
                        .map(|n| serde_json::to_string(&n.node_type).unwrap_or_default())
                })
                .collect();
            if all_types.is_empty() {
                0.0
            } else {
                let mut counts: HashMap<String, usize> = HashMap::new();
                for t in &all_types {
                    *counts.entry(t.clone()).or_insert(0) += 1;
                }
                let total = all_types.len() as f64;
                counts
                    .values()
                    .map(|&c| c as f64 / total)
                    .map(|p| -p * p.ln())
                    .sum::<f64>()
            }
        } else {
            0.0
        };
        SearchStatistics {
            generation: self.generation,
            population_size: pop_len,
            best_fitness,
            avg_fitness,
            diversity_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_node_type_is_control_flow() {
        assert!(AgentNodeType::Condition.is_control_flow());
        assert!(AgentNodeType::Parallel.is_control_flow());
        assert!(AgentNodeType::Sequential.is_control_flow());
        assert!(!AgentNodeType::LlmCall.is_control_flow());
        assert!(!AgentNodeType::ToolInvocation.is_control_flow());
        assert!(!AgentNodeType::MemoryRead.is_control_flow());
    }

    #[test]
    fn test_agent_node_type_is_memory() {
        assert!(AgentNodeType::MemoryRead.is_memory());
        assert!(AgentNodeType::MemoryWrite.is_memory());
        assert!(!AgentNodeType::LlmCall.is_memory());
    }

    #[test]
    fn test_agent_node_builder() {
        let node = AgentNode::new(AgentNodeType::LlmCall)
            .with_config("model", "gpt-4")
            .with_prompt_template("You are a helpful assistant");
        assert_eq!(node.node_type, AgentNodeType::LlmCall);
        assert_eq!(node.config.get("model").unwrap(), "gpt-4");
        assert_eq!(node.prompt_template.as_deref().unwrap(), "You are a helpful assistant");
    }

    #[test]
    fn test_agent_edge_with_condition() {
        let edge = AgentEdge::new("node_a", "node_b").with_condition("x > 0");
        assert_eq!(edge.source_id, "node_a");
        assert_eq!(edge.target_id, "node_b");
        assert_eq!(edge.condition.as_deref(), Some("x > 0"));
    }

    #[test]
    fn test_agent_architecture_add_nodes_and_edges() {
        let mut arch = AgentArchitecture::new("test_arch");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::ToolInvocation);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        arch.add_node(n1);
        arch.add_node(n2);
        arch.add_edge(AgentEdge::new(&id1, &id2));
        assert_eq!(arch.nodes.len(), 2);
        assert_eq!(arch.edges.len(), 1);
        assert!(arch.get_node(&id1).is_some());
        assert!(arch.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_agent_architecture_successors_and_predecessors() {
        let mut arch = AgentArchitecture::new("test");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::Condition);
        let n3 = AgentNode::new(AgentNodeType::ToolInvocation);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        let id3 = n3.id.clone();
        arch.add_node(n1);
        arch.add_node(n2);
        arch.add_node(n3);
        arch.add_edge(AgentEdge::new(&id1, &id2));
        arch.add_edge(AgentEdge::new(&id2, &id3));
        let succs = arch.successors(&id1);
        assert_eq!(succs.len(), 1);
        assert_eq!(succs[0], id2.as_str());
        let preds = arch.predecessors(&id3);
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0], id2.as_str());
    }

    #[test]
    fn test_agent_architecture_root_and_leaf_nodes() {
        let mut arch = AgentArchitecture::new("test");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::Condition);
        let n3 = AgentNode::new(AgentNodeType::ToolInvocation);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        let id3 = n3.id.clone();
        arch.add_node(n1);
        arch.add_node(n2);
        arch.add_node(n3);
        arch.add_edge(AgentEdge::new(&id1, &id2));
        arch.add_edge(AgentEdge::new(&id2, &id3));
        let roots = arch.root_nodes();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, id1);
        let leaves = arch.leaf_nodes();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].id, id3);
    }

    #[test]
    fn test_connected_components_single() {
        let mut arch = AgentArchitecture::new("test");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::ToolInvocation);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        arch.add_node(n1);
        arch.add_node(n2);
        arch.add_edge(AgentEdge::new(&id1, &id2));
        let components = arch.connected_components();
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn test_connected_components_disconnected() {
        let mut arch = AgentArchitecture::new("test");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::ToolInvocation);
        arch.add_node(n1);
        arch.add_node(n2);
        let components = arch.connected_components();
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn test_node_type_diversity() {
        let mut arch = AgentArchitecture::new("test");
        arch.add_node(AgentNode::new(AgentNodeType::LlmCall));
        arch.add_node(AgentNode::new(AgentNodeType::LlmCall));
        assert!(arch.node_type_diversity() < 1.0);

        let mut diverse = AgentArchitecture::new("diverse");
        diverse.add_node(AgentNode::new(AgentNodeType::LlmCall));
        diverse.add_node(AgentNode::new(AgentNodeType::Condition));
        diverse.add_node(AgentNode::new(AgentNodeType::MemoryRead));
        assert!(diverse.node_type_diversity() > arch.node_type_diversity());
    }

    #[test]
    fn test_default_evaluator_balance_score() {
        let eval = DefaultArchitectureEvaluator::new();
        let mut arch = AgentArchitecture::new("balanced");
        for _ in 0..5 {
            arch.add_node(AgentNode::new(AgentNodeType::LlmCall));
        }
        let score = eval.compute_balance_score(&arch);
        assert!(score > 0.9);

        let mut tiny = AgentArchitecture::new("tiny");
        tiny.add_node(AgentNode::new(AgentNodeType::LlmCall));
        let tiny_score = eval.compute_balance_score(&tiny);
        assert!(tiny_score < score);
    }

    #[test]
    fn test_default_evaluator_connectivity_score() {
        let eval = DefaultArchitectureEvaluator::new();
        let mut connected = AgentArchitecture::new("connected");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::ToolInvocation);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        connected.add_node(n1);
        connected.add_node(n2);
        connected.add_edge(AgentEdge::new(&id1, &id2));
        assert!(eval.compute_connectivity_score(&connected) > 0.9);

        let mut disconnected = AgentArchitecture::new("disconnected");
        disconnected.add_node(AgentNode::new(AgentNodeType::LlmCall));
        disconnected.add_node(AgentNode::new(AgentNodeType::ToolInvocation));
        assert!(eval.compute_connectivity_score(&disconnected) < 0.9);
    }

    #[test]
    fn test_default_evaluator_io_flow_score() {
        let eval = DefaultArchitectureEvaluator::new();
        let mut good_flow = AgentArchitecture::new("good_flow");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::LlmCall);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        good_flow.add_node(n1);
        good_flow.add_node(n2);
        good_flow.add_edge(AgentEdge::new(&id1, &id2));
        let score = eval.compute_io_flow_score(&good_flow);
        assert!(score >= 0.75);
    }

    #[tokio::test]
    async fn test_default_evaluator_evaluate() {
        let eval = DefaultArchitectureEvaluator::new();
        let mut arch = AgentArchitecture::new("test_eval");
        let n1 = AgentNode::new(AgentNodeType::LlmCall);
        let n2 = AgentNode::new(AgentNodeType::ToolInvocation);
        let n3 = AgentNode::new(AgentNodeType::MemoryRead);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        let id3 = n3.id.clone();
        arch.add_node(n1);
        arch.add_node(n2);
        arch.add_node(n3);
        arch.add_edge(AgentEdge::new(&id1, &id2));
        arch.add_edge(AgentEdge::new(&id2, &id3));
        let fitness = eval.evaluate(&arch).await.unwrap();
        assert!(fitness > 0.0);
        assert!(fitness <= 1.0);
    }

    #[test]
    fn test_search_config_default() {
        let config = ArchitectureSearchConfig::default();
        assert_eq!(config.population_size, 30);
        assert_eq!(config.elite_count, 4);
        assert_eq!(config.max_generations, 50);
        assert!((config.mutation_rate - 0.3).abs() < f64::EPSILON);
        assert!((config.crossover_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_search_space_default() {
        let space = ArchitectureSearchSpace::default_search_space();
        assert_eq!(space.allowed_node_types.len(), 7);
        assert!(!space.available_tools.is_empty() || space.available_tools.is_empty());
    }

    #[test]
    fn test_mutate_architecture_add_node() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig::default();
        let search_space = ArchitectureSearchSpace::default_search_space();
        let engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);

        let mut arch = AgentArchitecture::new("test_mutate");
        arch.add_node(AgentNode::new(AgentNodeType::LlmCall));
        arch.add_node(AgentNode::new(AgentNodeType::ToolInvocation));
        arch.add_node(AgentNode::new(AgentNodeType::Condition));

        let mutant = engine.mutate_architecture(&arch);
        assert_ne!(mutant.id, arch.id);
        assert_eq!(mutant.generation, engine.generation);
        assert!((mutant.fitness - 0.0).abs() < f64::EPSILON);
        assert!(mutant.nodes.len() >= 2);
    }

    #[test]
    fn test_crossover_architectures() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig::default();
        let search_space = ArchitectureSearchSpace::default_search_space();
        let engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);

        let mut p1 = AgentArchitecture::new("parent1");
        let mut p2 = AgentArchitecture::new("parent2");
        for _ in 0..4 {
            p1.add_node(AgentNode::new(AgentNodeType::LlmCall));
            p2.add_node(AgentNode::new(AgentNodeType::Condition));
        }
        let p1_ids: Vec<String> = p1.nodes.iter().map(|n| n.id.clone()).collect();
        let p2_ids: Vec<String> = p2.nodes.iter().map(|n| n.id.clone()).collect();
        p1.add_edge(AgentEdge::new(&p1_ids[0], &p1_ids[1]));
        p2.add_edge(AgentEdge::new(&p2_ids[0], &p2_ids[1]));

        let child = engine.crossover_architectures(&p1, &p2);
        assert_ne!(child.id, p1.id);
        assert_ne!(child.id, p2.id);
        assert!(child.nodes.len() <= p1.nodes.len() + p2.nodes.len());
    }

    #[tokio::test]
    async fn test_search_engine_full_run() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig {
            population_size: 10,
            elite_count: 2,
            max_generations: 5,
            mutation_rate: 0.4,
            crossover_rate: 0.4,
            max_nodes: 8,
            max_depth: 4,
        };
        let search_space = ArchitectureSearchSpace::default_search_space();
        let mut engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);

        let best = engine.search(5).await;
        assert!(!best.nodes.is_empty() || best.name == "empty");
        let stats = engine.get_statistics();
        assert_eq!(stats.generation, 5);
        assert_eq!(stats.population_size, 10);
        assert!(stats.best_fitness >= 0.0);
    }

    #[tokio::test]
    async fn test_evolve_generation_increments() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig {
            population_size: 6,
            elite_count: 2,
            max_generations: 3,
            mutation_rate: 0.3,
            crossover_rate: 0.5,
            max_nodes: 6,
            max_depth: 3,
        };
        let search_space = ArchitectureSearchSpace::default_search_space();
        let mut engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);
        engine.initialize_population();
        for arch in &mut engine.population {
            arch.fitness = 0.5;
        }
        engine.generation = 0;
        engine.best_fitness = 0.5;

        engine.evolve_generation().await;
        assert_eq!(engine.generation, 1);
        assert_eq!(engine.population.len(), 6);
    }

    #[test]
    fn test_get_best_returns_highest_fitness() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig::default();
        let search_space = ArchitectureSearchSpace::default_search_space();
        let mut engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);

        let mut a1 = AgentArchitecture::new("low");
        a1.fitness = 0.2;
        let mut a2 = AgentArchitecture::new("high");
        a2.fitness = 0.9;
        let mut a3 = AgentArchitecture::new("mid");
        a3.fitness = 0.5;
        engine.population = vec![a1, a2, a3];

        let best = engine.get_best().unwrap();
        assert!((best.fitness - 0.9).abs() < f64::EPSILON);
        assert_eq!(best.name, "high");
    }

    #[test]
    fn test_get_statistics() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig::default();
        let search_space = ArchitectureSearchSpace::default_search_space();
        let mut engine = ArchitectureSearchEngine::new(config, search_space, eval, meta);

        let mut a1 = AgentArchitecture::new("a1");
        a1.fitness = 0.3;
        a1.add_node(AgentNode::new(AgentNodeType::LlmCall));
        let mut a2 = AgentArchitecture::new("a2");
        a2.fitness = 0.7;
        a2.add_node(AgentNode::new(AgentNodeType::Condition));
        engine.population = vec![a1, a2];
        engine.generation = 3;

        let stats = engine.get_statistics();
        assert_eq!(stats.generation, 3);
        assert_eq!(stats.population_size, 2);
        assert!((stats.best_fitness - 0.7).abs() < f64::EPSILON);
        assert!((stats.avg_fitness - 0.5).abs() < f64::EPSILON);
        assert!(stats.diversity_index >= 0.0);
    }

    #[test]
    fn test_get_best_empty_population() {
        let eval = DefaultArchitectureEvaluator::new();
        let meta = RandomMetaAgent::new();
        let config = ArchitectureSearchConfig::default();
        let search_space = ArchitectureSearchSpace::default_search_space();
        let engine: ArchitectureSearchEngine<DefaultArchitectureEvaluator, RandomMetaAgent> =
            ArchitectureSearchEngine::new(config, search_space, eval, meta);
        assert!(engine.get_best().is_none());
    }

    #[test]
    fn test_agent_node_type_custom_serialization() {
        let custom = AgentNodeType::Custom("MyNode".to_string());
        let json = serde_json::to_string(&custom).unwrap();
        assert!(json.contains("custom"));
        let deserialized: AgentNodeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, custom);
    }
}
