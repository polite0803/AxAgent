// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{HashMap, HashSet};

use super::note_graph::LinkGraph;

pub use axagent_harness::louvain_dtos::LouvainResult;

const MAX_ITERATIONS: usize = 100;
const MIN_GAIN: f64 = 1e-6;

pub struct LouvainDetector {
    graph: LinkGraph,
    node_to_community: HashMap<String, i32>,
    community_to_nodes: HashMap<i32, Vec<String>>,
    total_edges: f64,
}

impl LouvainDetector {
    pub fn new(graph: LinkGraph) -> Self {
        let total_edges = graph.edge_count() as f64;
        let node_to_community: HashMap<String, i32> =
            graph.get_node_ids().iter().enumerate().map(|(i, id)| (id.clone(), i as i32)).collect();

        let community_to_nodes: HashMap<i32, Vec<String>> = graph
            .get_node_ids()
            .iter()
            .enumerate()
            .map(|(i, id)| (i as i32, vec![id.clone()]))
            .collect();

        Self { graph, node_to_community, community_to_nodes, total_edges }
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
        self.node_to_community.insert(node_id.to_string(), community);
        self.community_to_nodes.entry(community).or_default().push(node_id.to_string());
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

            let top_node =
                nodes.iter().max_by_key(|n| self.graph.get_degree(n)).cloned().unwrap_or_default();

            let top_title = self.graph.get_node_title(&top_node).unwrap_or(&top_node).to_string();
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

    /// 计算模块度（modularity）。
    ///
    /// 原实现是 O(N²) 双层 for 循环遍历所有节点对，10 万节点 = 10^10 次操作。
    /// 重写为 O(N + E)：
    /// - 遍历所有边一次，累加同社区的边贡献 `Σ A_ij`（intra-edge 总和）
    /// - 遍历所有社区一次，累加度数平方贡献 `Σ (Σ ki)²`
    ///
    /// 公式：Q = (1/2m) * [ Σ_intra A_ij - Σ_c (Σ_tot_c)² / (2m) ]
    /// 其中 Σ_intra A_ij = 同社区内的边数（每条边算两次，因为是无向图），
    /// Σ_tot_c = 社区 c 内所有节点的度数之和。
    fn compute_modularity(&self) -> f64 {
        if self.total_edges == 0.0 {
            return 1.0;
        }

        let m = self.total_edges;
        let two_m = 2.0 * m;

        // Phase 1: 累加同社区内的边贡献（O(E)）
        // 每条无向边 (u,v) 若 u,v 同社区，贡献 A_uv = 1（无向图每条边算两次）
        let mut intra_edge_sum: f64 = 0.0;
        let mut community_degree_sum: HashMap<i32, f64> = HashMap::new();

        let nodes = self.graph.get_node_ids();
        for node_id in &nodes {
            let node_comm = match self.node_to_community.get(node_id) {
                Some(&c) => c,
                None => continue,
            };
            let ki = self.graph.get_degree(node_id) as f64;

            // 累加该节点对社区度数总和的贡献
            *community_degree_sum.entry(node_comm).or_insert(0.0) += ki;

            // 遍历邻居，统计同社区的边
            for neighbor in self.graph.get_neighbors(node_id) {
                if let Some(&neighbor_comm) = self.node_to_community.get(&neighbor)
                    && neighbor_comm == node_comm
                {
                    intra_edge_sum += 1.0;
                }
            }
        }
        // 无向图每条边被 (u,v) 和 (v,u) 各算一次，intra_edge_sum 已经是双倍

        // Phase 2: 累加社区度数平方贡献（O(C)，C = 社区数 ≤ N）
        let mut degree_sq_sum: f64 = 0.0;
        for deg_sum in community_degree_sum.values() {
            degree_sq_sum += deg_sum * deg_sum;
        }

        // Q = (intra_edge_sum - degree_sq_sum / (2m)) / (2m)
        // intra_edge_sum 已经是双倍边数，对应公式中的 2 * Σ A_ij
        (intra_edge_sum - degree_sq_sum / two_m) / two_m
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
                (0.0..=1.0).contains(&score),
                "Community {} cohesion out of range: {}",
                cid,
                score
            );
        }
    }
}
