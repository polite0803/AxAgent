use std::collections::{HashMap, HashSet};

use axagent_core::repo::note_graph::{LinkGraph, RelevanceEdge, RelevanceSignal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedPage {
    pub page_id: String,
    pub title: String,
    pub score: f64,
    pub signal: RelevanceSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceConfig {
    pub max_results: usize,
    pub min_score_to_link: f64,
    pub max_recommendations: usize,
}

impl Default for RelevanceConfig {
    fn default() -> Self {
        Self {
            max_results: 20,
            min_score_to_link: 1.0,
            max_recommendations: 5,
        }
    }
}

pub struct RelevanceEngine {
    config: RelevanceConfig,
}

impl RelevanceEngine {
    pub fn new(config: RelevanceConfig) -> Self {
        Self { config }
    }

    pub fn find_related_pages(
        &self,
        graph: &LinkGraph,
        page_id: &str,
        source_map: &HashMap<String, Vec<String>>,
    ) -> Vec<RankedPage> {
        let signals = graph.get_top_k_related(page_id, source_map, self.config.max_results);

        signals
            .into_iter()
            .map(|(other_id, signal)| RankedPage {
                title: graph
                    .get_node_title(&other_id)
                    .unwrap_or(&other_id)
                    .to_string(),
                score: signal.total_score(),
                signal,
                page_id: other_id,
            })
            .filter(|r| r.score >= self.config.min_score_to_link)
            .collect()
    }

    pub fn find_bidirectional_connection(
        &self,
        graph: &LinkGraph,
        page_a: &str,
        page_b: &str,
        source_map: &HashMap<String, Vec<String>>,
    ) -> Option<RelevanceEdge> {
        let signal = graph.compute_relevance(page_a, page_b, source_map);

        if signal.total_score() >= self.config.min_score_to_link {
            Some(RelevanceEdge {
                source: page_a.to_string(),
                target: page_b.to_string(),
                total_score: signal.total_score(),
                signal,
            })
        } else {
            None
        }
    }

    pub fn recommend_connections(
        &self,
        graph: &LinkGraph,
        page_id: &str,
        existing_connections: &HashSet<String>,
        source_map: &HashMap<String, Vec<String>>,
    ) -> Vec<RankedPage> {
        let related = self.find_related_pages(graph, page_id, source_map);

        related
            .into_iter()
            .filter(|r| !existing_connections.contains(&r.page_id))
            .take(self.config.max_recommendations)
            .collect()
    }

    pub fn rank_nodes(
        &self,
        graph: &LinkGraph,
        node_ids: &[String],
    ) -> Vec<(String, f64)> {
        let mut incoming_counts: HashMap<String, f64> = HashMap::new();

        let all_edges = graph.build_relevance_edges(
            &HashMap::new(),
            0.0,
        );

        for edge in &all_edges {
            *incoming_counts.entry(edge.target.clone()).or_insert(0.0) += 1.0;
            *incoming_counts.entry(edge.source.clone()).or_insert(0.0) += 0.5;
        }

        let mut ranked: Vec<(String, f64)> = node_ids
            .iter()
            .map(|id| {
                let degree = graph.get_degree(id) as f64;
                let incoming = incoming_counts.get(id).copied().unwrap_or(0.0);
                let score = degree * 0.4 + incoming * 0.6;
                (id.clone(), score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    pub fn find_relevant_edges(
        &self,
        graph: &LinkGraph,
        source_map: &HashMap<String, Vec<String>>,
    ) -> Vec<RelevanceEdge> {
        let mut edges = graph.build_relevance_edges(source_map, self.config.min_score_to_link);
        edges.sort_by(|a, b| {
            a.total_score
                .partial_cmp(&b.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        edges
    }

    pub fn find_strongest_edges(
        &self,
        graph: &LinkGraph,
        source_map: &HashMap<String, Vec<String>>,
        k: usize,
    ) -> Vec<RelevanceEdge> {
        let edges = graph.build_relevance_edges(source_map, self.config.min_score_to_link);
        let mut edges: Vec<_> = edges;
        edges.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        edges.truncate(k);
        edges
    }

    pub fn expand_seeds(
        &self,
        graph: &LinkGraph,
        seed_ids: &[String],
        source_map: &HashMap<String, Vec<String>>,
        max_hops: usize,
    ) -> Vec<RankedPage> {
        let signals = graph.expand_seeds(seed_ids, source_map, self.config.max_results, max_hops);

        signals
            .into_iter()
            .map(|(id, signal)| RankedPage {
                title: graph
                    .get_node_title(&id)
                    .unwrap_or(&id)
                    .to_string(),
                score: signal.total_score(),
                signal,
                page_id: id,
            })
            .collect()
    }
}

impl Default for RelevanceEngine {
    fn default() -> Self {
        Self::new(RelevanceConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_core::repo::note::{GraphData, GraphEdge, GraphNode};

    fn create_test_graph() -> LinkGraph {
        let nodes = vec![
            GraphNode {
                id: "a".to_string(),
                title: "Page A".to_string(),
                node_type: "concept".to_string(),
                tags: vec![],
                link_count: 1,
                backlink_count: 0,
                path: "a.md".to_string(),
            },
            GraphNode {
                id: "b".to_string(),
                title: "Page B".to_string(),
                node_type: "concept".to_string(),
                tags: vec![],
                link_count: 2,
                backlink_count: 1,
                path: "b.md".to_string(),
            },
            GraphNode {
                id: "c".to_string(),
                title: "Page C".to_string(),
                node_type: "entity".to_string(),
                tags: vec![],
                link_count: 0,
                backlink_count: 1,
                path: "c.md".to_string(),
            },
        ];

        let edges = vec![
            GraphEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "b".to_string(),
                target: "c".to_string(),
                edge_type: "link".to_string(),
            },
        ];

        let data = GraphData { nodes, edges };
        LinkGraph::from_graph_data(data)
    }

    fn create_source_map() -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();
        map.insert("a".to_string(), vec!["src1".to_string()]);
        map.insert("b".to_string(), vec!["src1".to_string()]);
        map.insert("c".to_string(), vec!["src2".to_string()]);
        map
    }

    #[test]
    fn test_find_related_pages() {
        let graph = create_test_graph();
        let source_map = create_source_map();
        let engine = RelevanceEngine::new(RelevanceConfig {
            min_score_to_link: 0.0,
            ..Default::default()
        });

        let related = engine.find_related_pages(&graph, "a", &source_map);
        assert!(!related.is_empty());
        assert_eq!(related[0].page_id, "b");
    }

    #[test]
    fn test_find_bidirectional_connection() {
        let graph = create_test_graph();
        let source_map = create_source_map();
        let engine = RelevanceEngine::default();

        let conn = engine.find_bidirectional_connection(&graph, "a", "b", &source_map);
        assert!(conn.is_some());
        let conn = conn.unwrap();
        assert_eq!(conn.source, "a");
        assert_eq!(conn.target, "b");
    }

    #[test]
    fn test_recommend_connections() {
        let graph = create_test_graph();
        let source_map = create_source_map();
        let engine = RelevanceEngine::default();
        let mut existing = HashSet::new();
        existing.insert("b".to_string());

        let recommendations = engine.recommend_connections(&graph, "a", &existing, &source_map);
        assert!(!recommendations.iter().any(|r| r.page_id == "b"));
    }

    #[test]
    fn test_rank_nodes() {
        let graph = create_test_graph();
        let engine = RelevanceEngine::default();
        let node_ids: Vec<String> = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let ranked = engine.rank_nodes(&graph, &node_ids);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].0, "b");
    }

    #[test]
    fn test_find_relevant_edges() {
        let graph = create_test_graph();
        let source_map = create_source_map();
        let engine = RelevanceEngine::default();

        let edges = engine.find_relevant_edges(&graph, &source_map);
        assert!(!edges.is_empty());
    }
}