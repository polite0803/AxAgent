use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::note_graph::LinkGraph;

const LOW_COHESION_THRESHOLD: f64 = 0.15;
const MAX_ITERATIONS: usize = 100;
const MIN_GAIN: f64 = 1e-6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LouvainResult {
    pub communities: HashMap<String, i32>,
    pub cohesion_scores: HashMap<i32, f64>,
    pub community_sizes: HashMap<i32, usize>,
    pub top_nodes: HashMap<i32, String>,
    pub modularity: f64,
    pub num_communities: usize,
    pub color_palette: Vec<String>,
}

impl LouvainResult {
    pub fn default_palette() -> Vec<String> {
        vec![
            "#4C72B0".to_string(),
            "#DD8452".to_string(),
            "#55A868".to_string(),
            "#C44E52".to_string(),
            "#8172B3".to_string(),
            "#937860".to_string(),
            "#DA8BC3".to_string(),
            "#8C8C8C".to_string(),
            "#CCB974".to_string(),
            "#64B5CD".to_string(),
            "#E18B6C".to_string(),
            "#7AA153".to_string(),
        ]
    }

    pub fn get_color(&self, community_id: i32) -> String {
        let idx = community_id.rem_euclid(self.color_palette.len() as i32) as usize;
        self.color_palette[idx].clone()
    }

    pub fn is_low_cohesion(&self, community_id: i32) -> bool {
        self.cohesion_scores
            .get(&community_id)
            .map(|s| *s < LOW_COHESION_THRESHOLD)
            .unwrap_or(false)
    }

    pub fn get_community_nodes(&self, community_id: i32) -> Vec<String> {
        self.communities
            .iter()
            .filter(|(_, c)| **c == community_id)
            .map(|(n, _)| n.clone())
            .collect()
    }
}

pub struct LouvainDetector {
    graph: LinkGraph,
    node_to_community: HashMap<String, i32>,
    community_to_nodes: HashMap<i32, Vec<String>>,
    total_edges: f64,
}

impl LouvainDetector {
    pub fn new(graph: LinkGraph) -> Self {
        let total_edges = graph.edge_count() as f64;
        let node_to_community: HashMap<String, i32> = graph
            .get_node_ids()
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i as i32))
            .collect();

        let community_to_nodes: HashMap<i32, Vec<String>> = graph
            .get_node_ids()
            .iter()
            .enumerate()
            .map(|(i, id)| (i as i32, vec![id.clone()]))
            .collect();

        Self {
            graph,
            node_to_community,
            community_to_nodes,
            total_edges,
        }
    }

    pub fn detect(mut self) -> LouvainResult {
        let nodes = self.graph.get_node_ids();
        if nodes.len() <= 1 {
            let mut communities = HashMap::new();
            if let Some(node) = nodes.first() {
                communities.insert(node.clone(), 0);
            }
            let mut cohesion_scores = HashMap::new();
            cohesion_scores.insert(0, 1.0);
            let mut community_sizes = HashMap::new();
            community_sizes.insert(0, nodes.len());
            let mut top_nodes = HashMap::new();
            if let Some(node) = nodes.first() {
                top_nodes.insert(0, self.graph.get_node_title(node).unwrap_or("").to_string());
            }

            return LouvainResult {
                communities,
                cohesion_scores,
                community_sizes,
                top_nodes,
                modularity: 1.0,
                num_communities: 1,
                color_palette: LouvainResult::default_palette(),
            };
        }

        let mut improved = true;
        let mut iteration = 0;

        while improved && iteration < MAX_ITERATIONS {
            improved = false;
            iteration += 1;

            let nodes = self.graph.get_node_ids();
            for node_id in &nodes {
                let current_community = self.node_to_community[&node_id.clone()];

                self.remove_from_community(node_id, current_community);

                let neighbor_communities = self.get_neighbor_communities(node_id);
                let mut best_community = current_community;
                let mut best_gain = 0.0;

                for &candidate_community in &neighbor_communities {
                    let gain = self.modularity_gain(node_id, candidate_community);
                    if gain > best_gain + MIN_GAIN {
                        best_gain = gain;
                        best_community = candidate_community;
                    }
                }

                self.add_to_community(node_id, best_community);

                if best_community != current_community {
                    improved = true;
                }
            }
        }

        self.compute_result()
    }

    fn remove_from_community(&mut self, node_id: &str, community: i32) {
        if let Some(nodes) = self.community_to_nodes.get_mut(&community) {
            nodes.retain(|n| n != node_id);
        }
    }

    fn add_to_community(&mut self, node_id: &str, community: i32) {
        self.node_to_community
            .insert(node_id.to_string(), community);
        self.community_to_nodes
            .entry(community)
            .or_default()
            .push(node_id.to_string());
    }

    fn get_neighbor_communities(&self, node_id: &str) -> Vec<i32> {
        let mut communities = HashSet::new();
        for neighbor in self.graph.get_neighbors(node_id) {
            if let Some(&c) = self.node_to_community.get(&neighbor) {
                communities.insert(c);
            }
        }
        communities.into_iter().collect()
    }

    fn modularity_gain(&self, node_id: &str, target_community: i32) -> f64 {
        let neighbors: HashSet<_> = self.graph.get_neighbors(node_id).into_iter().collect();
        let degree = neighbors.len() as f64;

        if degree == 0.0 || self.total_edges == 0.0 {
            return 0.0;
        }

        let ki: f64 = degree;
        let ki_in: f64 = neighbors
            .iter()
            .filter(|n| self.node_to_community.get(n.as_str()) == Some(&target_community))
            .count() as f64;

        let sigma_tot: f64 = if let Some(nodes) = self.community_to_nodes.get(&target_community) {
            nodes.iter().map(|n| self.graph.get_degree(n) as f64).sum()
        } else {
            0.0
        };

        let m = self.total_edges;

        (ki_in / m) - (sigma_tot * ki) / (2.0 * m * m)
    }

    fn compute_result(self) -> LouvainResult {
        let mut community_sizes = HashMap::new();
        let mut top_nodes = HashMap::new();
        let mut cohesion_scores = HashMap::new();

        for (&community, nodes) in &self.community_to_nodes {
            if nodes.is_empty() {
                continue;
            }

            community_sizes.insert(community, nodes.len());

            let top_node = nodes
                .iter()
                .max_by_key(|n| self.graph.get_degree(n))
                .cloned()
                .unwrap_or_default();

            let top_title = self
                .graph
                .get_node_title(&top_node)
                .unwrap_or(&top_node)
                .to_string();
            top_nodes.insert(community, top_title);

            let cohesion = self.compute_cohesion(nodes);
            cohesion_scores.insert(community, cohesion);
        }

        let modularity = self.compute_modularity();
        let num_communities = community_sizes.len();

        LouvainResult {
            communities: self.node_to_community,
            cohesion_scores,
            community_sizes,
            top_nodes,
            modularity,
            num_communities,
            color_palette: LouvainResult::default_palette(),
        }
    }

    fn compute_cohesion(&self, nodes: &[String]) -> f64 {
        let n = nodes.len();
        if n <= 1 {
            return 1.0;
        }

        let mut intra_edges = 0u64;
        let node_set: HashSet<&str> = nodes.iter().map(|n| n.as_str()).collect();

        for node in nodes {
            for neighbor in self.graph.get_neighbors(node) {
                if node_set.contains(neighbor.as_str()) {
                    intra_edges += 1;
                }
            }
        }

        intra_edges /= 2;

        let possible = (n * (n - 1)) / 2;
        if possible == 0 {
            return 0.0;
        }

        (intra_edges as f64) / (possible as f64)
    }

    fn compute_modularity(&self) -> f64 {
        if self.total_edges == 0.0 {
            return 1.0;
        }

        let mut q = 0.0;
        let m = self.total_edges;

        let nodes = self.graph.get_node_ids();
        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                let ci = self.node_to_community.get(&nodes[i]);
                let cj = self.node_to_community.get(&nodes[j]);

                if ci != cj || ci.is_none() {
                    continue;
                }

                let ki = self.graph.get_degree(&nodes[i]) as f64;
                let kj = self.graph.get_degree(&nodes[j]) as f64;
                let aij = if self.graph.has_direct_link(&nodes[i], &nodes[j]) {
                    1.0
                } else {
                    0.0
                };

                q += aij - (ki * kj) / (2.0 * m);
            }
        }

        q / (2.0 * m)
    }
}

pub fn detect_communities(graph: LinkGraph) -> LouvainResult {
    let detector = LouvainDetector::new(graph);
    detector.detect()
}

#[cfg(test)]
mod tests {
    use super::super::note::{GraphData, GraphEdge, GraphNode};
    use super::*;

    fn make_test_graph() -> LinkGraph {
        let nodes = vec![
            GraphNode {
                id: "a".to_string(),
                title: "Node A".to_string(),
                node_type: "concept".to_string(),
                tags: vec![],
                link_count: 2,
                backlink_count: 2,
                path: "a.md".to_string(),
            },
            GraphNode {
                id: "b".to_string(),
                title: "Node B".to_string(),
                node_type: "concept".to_string(),
                tags: vec![],
                link_count: 2,
                backlink_count: 2,
                path: "b.md".to_string(),
            },
            GraphNode {
                id: "c".to_string(),
                title: "Node C".to_string(),
                node_type: "concept".to_string(),
                tags: vec![],
                link_count: 1,
                backlink_count: 1,
                path: "c.md".to_string(),
            },
            GraphNode {
                id: "d".to_string(),
                title: "Node D".to_string(),
                node_type: "entity".to_string(),
                tags: vec![],
                link_count: 2,
                backlink_count: 2,
                path: "d.md".to_string(),
            },
            GraphNode {
                id: "e".to_string(),
                title: "Node E".to_string(),
                node_type: "entity".to_string(),
                tags: vec![],
                link_count: 2,
                backlink_count: 2,
                path: "e.md".to_string(),
            },
            GraphNode {
                id: "f".to_string(),
                title: "Node F".to_string(),
                node_type: "entity".to_string(),
                tags: vec![],
                link_count: 1,
                backlink_count: 1,
                path: "f.md".to_string(),
            },
        ];

        let edges = vec![
            GraphEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "a".to_string(),
                target: "c".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "b".to_string(),
                target: "c".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "d".to_string(),
                target: "e".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "d".to_string(),
                target: "f".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "e".to_string(),
                target: "f".to_string(),
                edge_type: "link".to_string(),
            },
            GraphEdge {
                source: "b".to_string(),
                target: "d".to_string(),
                edge_type: "link".to_string(),
            },
        ];

        LinkGraph::from_graph_data(GraphData { nodes, edges })
    }

    #[test]
    fn test_detect_communities() {
        let graph = make_test_graph();
        let result = detect_communities(graph);

        assert!(result.num_communities >= 1);
        assert!(result.modularity > 0.0);
        assert!(!result.communities.is_empty());
    }

    #[test]
    fn test_cohesion_score() {
        let graph = make_test_graph();
        let result = detect_communities(graph);

        for (&cid, &score) in &result.cohesion_scores {
            assert!(
                score >= 0.0 && score <= 1.0,
                "Community {} cohesion out of range: {}",
                cid,
                score
            );
        }
    }
}
