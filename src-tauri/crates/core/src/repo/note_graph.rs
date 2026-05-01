use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::note::{GraphData, GraphEdge, GraphNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageType {
    Entity,
    Concept,
    SourceSummary,
    Comparison,
    Index,
    Overview,
    Note,
    Unknown,
}

impl PageType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "entity" => Self::Entity,
            "concept" => Self::Concept,
            "source_summary" | "source-summary" => Self::SourceSummary,
            "comparison" => Self::Comparison,
            "index" => Self::Index,
            "overview" => Self::Overview,
            "note" => Self::Note,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Concept => "concept",
            Self::SourceSummary => "source_summary",
            Self::Comparison => "comparison",
            Self::Index => "index",
            Self::Overview => "overview",
            Self::Note => "note",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceSignal {
    pub direct_link: f64,
    pub source_overlap: f64,
    pub adamic_adar: f64,
    pub type_affinity: f64,
}

impl RelevanceSignal {
    pub fn total_score(&self) -> f64 {
        self.direct_link * 3.0
            + self.source_overlap * 4.0
            + self.adamic_adar * 1.5
            + self.type_affinity * 1.0
    }

    pub fn normalized_total(&self) -> f64 {
        let raw = self.total_score();
        let max = 3.0 + 4.0 + 1.5 + 1.0;
        (raw / max).min(1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceEdge {
    pub source: String,
    pub target: String,
    pub signal: RelevanceSignal,
    pub total_score: f64,
}

pub struct LinkGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    adjacency: HashMap<String, Vec<String>>,
    node_titles: HashMap<String, String>,
}

impl LinkGraph {
    pub fn from_graph_data(data: GraphData) -> Self {
        let mut nodes = HashMap::new();
        let mut node_titles = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for node in data.nodes {
            node_titles.insert(node.id.clone(), node.title.clone());
            nodes.insert(node.id.clone(), node);
        }

        for edge in &data.edges {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
            adjacency
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
        }

        Self {
            nodes,
            edges: data.edges,
            adjacency,
            node_titles,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn get_neighbors(&self, node_id: &str) -> Vec<String> {
        self.adjacency.get(node_id).cloned().unwrap_or_default()
    }

    pub fn get_degree(&self, node_id: &str) -> usize {
        self.adjacency.get(node_id).map(|v| v.len()).unwrap_or(0)
    }

    pub fn has_direct_link(&self, a: &str, b: &str) -> bool {
        self.adjacency
            .get(a)
            .map(|neighbors| neighbors.contains(&b.to_string()))
            .unwrap_or(false)
    }

    pub fn get_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    pub fn get_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.get(node_id)
    }

    pub fn get_node_title(&self, node_id: &str) -> Option<&str> {
        self.node_titles.get(node_id).map(|s| s.as_str())
    }

    pub fn get_page_type(&self, node_id: &str) -> PageType {
        self.nodes
            .get(node_id)
            .map(|n| PageType::from_str(&n.node_type))
            .unwrap_or(PageType::Unknown)
    }

    pub fn compute_relevance(
        &self,
        page_a: &str,
        page_b: &str,
        source_map: &HashMap<String, Vec<String>>,
    ) -> RelevanceSignal {
        RelevanceSignal {
            direct_link: if self.has_direct_link(page_a, page_b) {
                1.0
            } else {
                0.0
            },
            source_overlap: self.compute_source_overlap(page_a, page_b, source_map),
            adamic_adar: self.compute_adamic_adar(page_a, page_b),
            type_affinity: self.compute_type_affinity(page_a, page_b),
        }
    }

    pub fn get_top_k_related(
        &self,
        node_id: &str,
        source_map: &HashMap<String, Vec<String>>,
        k: usize,
    ) -> Vec<(String, RelevanceSignal)> {
        let mut scored: Vec<_> = self
            .get_node_ids()
            .into_iter()
            .filter(|id| id != node_id)
            .map(|other| {
                let signal = self.compute_relevance(node_id, &other, source_map);
                (other, signal)
            })
            .filter(|(_, s)| s.total_score() > 0.0)
            .collect();

        scored.sort_by(|a, b| b.1.total_score().partial_cmp(&a.1.total_score()).unwrap());
        scored.truncate(k);
        scored
    }

    pub fn expand_seeds(
        &self,
        seed_ids: &[String],
        source_map: &HashMap<String, Vec<String>>,
        top_k: usize,
        max_hops: usize,
    ) -> Vec<(String, RelevanceSignal)> {
        let mut visited: HashSet<String> = seed_ids.iter().cloned().collect();
        let mut current: HashSet<String> = seed_ids.iter().cloned().collect();
        let mut results: Vec<(String, RelevanceSignal)> = Vec::new();

        for hop in 0..max_hops {
            let decay = 1.0 / (hop + 1) as f64;
            let mut next: HashSet<String> = HashSet::new();

            for seed in &current {
                let neighbors = self.get_neighbors(seed);
                for neighbor in neighbors {
                    if visited.contains(&neighbor) {
                        continue;
                    }
                    visited.insert(neighbor.clone());
                    next.insert(neighbor.clone());

                    let mut signal = self.compute_relevance(seed, &neighbor, source_map);
                    signal.direct_link *= decay;
                    signal.source_overlap *= decay;
                    signal.adamic_adar *= decay;
                    signal.type_affinity *= decay;
                    results.push((neighbor, signal));
                }
            }

            current = next;
        }

        results.sort_by(|a, b| b.1.total_score().partial_cmp(&a.1.total_score()).unwrap());
        results.truncate(top_k);
        results
    }

    pub fn build_relevance_edges(
        &self,
        source_map: &HashMap<String, Vec<String>>,
        threshold: f64,
    ) -> Vec<RelevanceEdge> {
        let nodes = self.get_node_ids();
        let mut edges = Vec::new();

        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let signal = self.compute_relevance(&nodes[i], &nodes[j], source_map);
                let total = signal.total_score();
                if total >= threshold {
                    edges.push(RelevanceEdge {
                        source: nodes[i].clone(),
                        target: nodes[j].clone(),
                        signal,
                        total_score: total,
                    });
                }
            }
        }

        edges
    }

    fn compute_source_overlap(
        &self,
        page_a: &str,
        page_b: &str,
        source_map: &HashMap<String, Vec<String>>,
    ) -> f64 {
        let sources_a: HashSet<_> = source_map
            .get(page_a)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let sources_b: HashSet<_> = source_map
            .get(page_b)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        if sources_a.is_empty() || sources_b.is_empty() {
            return 0.0;
        }

        let intersection = sources_a.intersection(&sources_b).count() as f64;
        let union = sources_a.union(&sources_b).count() as f64;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    fn compute_adamic_adar(&self, page_a: &str, page_b: &str) -> f64 {
        let neighbors_a: HashSet<_> = self.get_neighbors(page_a).into_iter().collect();
        let neighbors_b: HashSet<_> = self.get_neighbors(page_b).into_iter().collect();
        let common: Vec<_> = neighbors_a.intersection(&neighbors_b).collect();

        common
            .iter()
            .map(|n| {
                let degree = self.get_degree(n) as f64;
                if degree > 1.0 {
                    1.0 / (degree - 1.0).ln()
                } else {
                    0.0
                }
            })
            .sum()
    }

    fn compute_type_affinity(&self, page_a: &str, page_b: &str) -> f64 {
        let type_a = self.get_page_type(page_a);
        let type_b = self.get_page_type(page_b);

        if type_a == type_b {
            match type_a {
                PageType::Entity | PageType::Concept => 1.0,
                PageType::SourceSummary => 0.5,
                _ => 0.3,
            }
        } else {
            match (&type_a, &type_b) {
                (PageType::Entity, PageType::Concept) | (PageType::Concept, PageType::Entity) => {
                    0.5
                },
                _ => 0.0,
            }
        }
    }
}
