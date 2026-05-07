use std::collections::{HashMap, HashSet};

use axagent_core::repo::louvain::LouvainResult;
use axagent_core::repo::note_graph::{LinkGraph, PageType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GapType {
    IsolatedPages,
    SparseCommunity,
    BridgeNode,
    UnlinkedConcept,
}

impl GapType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IsolatedPages => "isolated_pages",
            Self::SparseCommunity => "sparse_community",
            Self::BridgeNode => "bridge_node",
            Self::UnlinkedConcept => "unlinked_concept",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::IsolatedPages => "Pages with few or no connections",
            Self::SparseCommunity => "Knowledge areas with weak internal cross-references",
            Self::BridgeNode => "Critical junction pages connecting multiple knowledge areas",
            Self::UnlinkedConcept => "Concepts without any cross-references",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurprisingConnection {
    pub page_a: String,
    pub page_b: String,
    pub title_a: String,
    pub title_b: String,
    pub surprise_score: f64,
    pub reasons: Vec<String>,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub gap_type: GapType,
    pub node_ids: Vec<String>,
    pub node_titles: Vec<String>,
    pub description: String,
    pub community_id: Option<i32>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeNode {
    pub node_id: String,
    pub node_title: String,
    pub connected_communities: Vec<i32>,
    pub community_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInsights {
    pub surprising_connections: Vec<SurprisingConnection>,
    pub knowledge_gaps: Vec<KnowledgeGap>,
    pub bridge_nodes: Vec<BridgeNode>,
    pub isolated_pages: Vec<String>,
    pub stats: GraphInsightStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInsightStats {
    pub total_pages: usize,
    pub total_edges: usize,
    pub num_communities: usize,
    pub avg_cohesion: f64,
    pub modularity: f64,
}

pub struct GraphInsightAnalyzer {
    graph: LinkGraph,
    louvain: LouvainResult,
    source_map: HashMap<String, Vec<String>>,
}

impl GraphInsightAnalyzer {
    pub fn new(
        graph: LinkGraph,
        louvain: LouvainResult,
        source_map: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            graph,
            louvain,
            source_map,
        }
    }

    pub fn analyze(&self) -> GraphInsights {
        GraphInsights {
            surprising_connections: self.find_surprising_connections(),
            knowledge_gaps: self.find_knowledge_gaps(),
            bridge_nodes: self.find_bridge_nodes(),
            isolated_pages: self.find_isolated_pages(),
            stats: self.compute_stats(),
        }
    }

    fn find_surprising_connections(&self) -> Vec<SurprisingConnection> {
        let mut connections = Vec::new();

        for i in 0..self.graph.get_node_ids().len() {
            let nodes = self.graph.get_node_ids();
            for j in (i + 1)..nodes.len() {
                let a = &nodes[i];
                let b = &nodes[j];

                let signal = self.graph.compute_relevance(a, b, &self.source_map);

                if signal.total_score() < 0.5 {
                    continue;
                }

                let mut surprise_score = 0.0f64;
                let mut reasons = Vec::new();

                let community_a = self.louvain.communities.get(a).copied();
                let community_b = self.louvain.communities.get(b).copied();

                if community_a != community_b {
                    surprise_score += 2.0;
                    reasons.push("Cross-community connection".to_string());
                }

                let type_a = self.graph.get_page_type(a);
                let type_b = self.graph.get_page_type(b);
                if type_a != type_b {
                    surprise_score += 1.5;
                    reasons.push(format!("Cross-type link: {:?} ↔ {:?}", type_a, type_b));
                }

                let degree_a = self.graph.get_degree(a);
                let degree_b = self.graph.get_degree(b);
                if (degree_a <= 2 && degree_b >= 8) || (degree_b <= 2 && degree_a >= 8) {
                    surprise_score += 1.0;
                    reasons.push("Peripheral-hub coupling".to_string());
                }

                if signal.direct_link == 0.0 && signal.source_overlap > 0.5 {
                    surprise_score += 1.5;
                    reasons.push("Strong source overlap without direct link".to_string());
                }

                if surprise_score > 2.0 {
                    connections.push(SurprisingConnection {
                        page_a: a.clone(),
                        page_b: b.clone(),
                        title_a: self.graph.get_node_title(a).unwrap_or(a).to_string(),
                        title_b: self.graph.get_node_title(b).unwrap_or(b).to_string(),
                        surprise_score,
                        reasons,
                        dismissed: false,
                    });
                }
            }
        }

        connections.sort_by(|a, b| {
            b.surprise_score
                .partial_cmp(&a.surprise_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        connections
    }

    fn find_knowledge_gaps(&self) -> Vec<KnowledgeGap> {
        let mut gaps = Vec::new();

        let isolated: Vec<String> = self
            .graph
            .get_node_ids()
            .into_iter()
            .filter(|n| self.graph.get_degree(n) <= 1)
            .collect();

        if !isolated.is_empty() {
            let titles: Vec<String> = isolated
                .iter()
                .map(|n| self.graph.get_node_title(n).unwrap_or(n).to_string())
                .collect();

            gaps.push(KnowledgeGap {
                gap_type: GapType::IsolatedPages,
                node_ids: isolated.clone(),
                node_titles: titles,
                description: format!(
                    "{} page(s) with degree ≤ 1: weak integration into the wiki",
                    isolated.len()
                ),
                community_id: None,
                suggested_action: "Consider linking these pages to related concepts or entities"
                    .to_string(),
            });
        }

        for (&community_id, &cohesion) in &self.louvain.cohesion_scores {
            if cohesion < 0.15 {
                let members = self.louvain.get_community_nodes(community_id);
                if members.len() >= 3 {
                    let titles: Vec<String> = members
                        .iter()
                        .map(|n| self.graph.get_node_title(n).unwrap_or(n).to_string())
                        .collect();

                    gaps.push(KnowledgeGap {
                        gap_type: GapType::SparseCommunity,
                        node_ids: members,
                        node_titles: titles,
                        description: format!(
                            "Community {} has low internal cross-references (cohesion: {:.2})",
                            community_id, cohesion
                        ),
                        community_id: Some(community_id),
                        suggested_action:
                            "Add more [[wikilinks]] between pages in this knowledge area"
                                .to_string(),
                    });
                }
            }
        }

        let unlinked_concepts: Vec<String> = self
            .graph
            .get_node_ids()
            .into_iter()
            .filter(|n| {
                let pt = self.graph.get_page_type(n);
                (pt == PageType::Concept || pt == PageType::Entity) && self.graph.get_degree(n) == 0
            })
            .collect();

        if !unlinked_concepts.is_empty() {
            let titles: Vec<String> = unlinked_concepts
                .iter()
                .map(|n| self.graph.get_node_title(n).unwrap_or(n).to_string())
                .collect();

            gaps.push(KnowledgeGap {
                gap_type: GapType::UnlinkedConcept,
                node_ids: unlinked_concepts,
                node_titles: titles,
                description: "Concepts/entities with no cross-references".to_string(),
                community_id: None,
                suggested_action: "Link these concepts to related pages or create new connections"
                    .to_string(),
            });
        }

        gaps
    }

    fn find_bridge_nodes(&self) -> Vec<BridgeNode> {
        self.graph
            .get_node_ids()
            .into_iter()
            .filter_map(|node_id| {
                let neighbors = self.graph.get_neighbors(&node_id);
                let mut connected_communities: HashSet<i32> = HashSet::new();

                for neighbor in &neighbors {
                    if let Some(&c) = self.louvain.communities.get(neighbor) {
                        connected_communities.insert(c);
                    }
                }

                if let Some(&self_community) = self.louvain.communities.get(&node_id) {
                    connected_communities.remove(&self_community);
                }

                let community_count = connected_communities.len();
                if community_count >= 3 {
                    Some(BridgeNode {
                        node_id: node_id.clone(),
                        node_title: self
                            .graph
                            .get_node_title(&node_id)
                            .unwrap_or(&node_id)
                            .to_string(),
                        connected_communities: connected_communities.into_iter().collect(),
                        community_count,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn find_isolated_pages(&self) -> Vec<String> {
        self.graph
            .get_node_ids()
            .into_iter()
            .filter(|n| self.graph.get_degree(n) == 0)
            .collect()
    }

    fn compute_stats(&self) -> GraphInsightStats {
        let avg_cohesion = if self.louvain.cohesion_scores.is_empty() {
            0.0
        } else {
            self.louvain.cohesion_scores.values().sum::<f64>()
                / self.louvain.cohesion_scores.len() as f64
        };

        GraphInsightStats {
            total_pages: self.graph.node_count(),
            total_edges: self.graph.edge_count(),
            num_communities: self.louvain.num_communities,
            avg_cohesion,
            modularity: self.louvain.modularity,
        }
    }
}

pub fn analyze_graph(
    graph: LinkGraph,
    louvain: LouvainResult,
    source_map: HashMap<String, Vec<String>>,
) -> GraphInsights {
    let analyzer = GraphInsightAnalyzer::new(graph, louvain, source_map);
    analyzer.analyze()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_type_as_str() {
        assert_eq!(GapType::IsolatedPages.as_str(), "isolated_pages");
        assert_eq!(GapType::SparseCommunity.as_str(), "sparse_community");
        assert_eq!(GapType::BridgeNode.as_str(), "bridge_node");
        assert_eq!(GapType::UnlinkedConcept.as_str(), "unlinked_concept");
    }

    #[test]
    fn test_gap_type_description() {
        assert!(!GapType::IsolatedPages.description().is_empty());
        assert!(!GapType::SparseCommunity.description().is_empty());
        assert!(!GapType::BridgeNode.description().is_empty());
        assert!(!GapType::UnlinkedConcept.description().is_empty());
    }

    #[test]
    fn test_gap_type_equality() {
        assert_eq!(GapType::IsolatedPages, GapType::IsolatedPages);
        assert_ne!(GapType::IsolatedPages, GapType::SparseCommunity);
    }

    #[test]
    fn test_surprising_connection_serialization() {
        let conn = SurprisingConnection {
            page_a: "a".to_string(),
            page_b: "b".to_string(),
            title_a: "Title A".to_string(),
            title_b: "Title B".to_string(),
            surprise_score: 3.5,
            reasons: vec!["Cross-community connection".to_string()],
            dismissed: false,
        };
        let json = serde_json::to_string(&conn).unwrap();
        let deserialized: SurprisingConnection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page_a, "a");
        assert_eq!(deserialized.surprise_score, 3.5);
        assert!(!deserialized.dismissed);
    }

    #[test]
    fn test_knowledge_gap_serialization() {
        let gap = KnowledgeGap {
            gap_type: GapType::IsolatedPages,
            node_ids: vec!["n1".to_string()],
            node_titles: vec!["Page 1".to_string()],
            description: "Test gap".to_string(),
            community_id: None,
            suggested_action: "Link pages".to_string(),
        };
        let json = serde_json::to_string(&gap).unwrap();
        let deserialized: KnowledgeGap = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.gap_type, GapType::IsolatedPages);
        assert!(deserialized.community_id.is_none());
    }

    #[test]
    fn test_bridge_node_serialization() {
        let node = BridgeNode {
            node_id: "n1".to_string(),
            node_title: "Bridge".to_string(),
            connected_communities: vec![1, 2, 3],
            community_count: 3,
        };
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: BridgeNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.community_count, 3);
        assert_eq!(deserialized.connected_communities.len(), 3);
    }

    #[test]
    fn test_graph_insight_stats_serialization() {
        let stats = GraphInsightStats {
            total_pages: 100,
            total_edges: 250,
            num_communities: 5,
            avg_cohesion: 0.65,
            modularity: 0.42,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: GraphInsightStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_pages, 100);
        assert_eq!(deserialized.total_edges, 250);
    }

    #[test]
    fn test_graph_insights_serialization() {
        let insights = GraphInsights {
            surprising_connections: vec![],
            knowledge_gaps: vec![],
            bridge_nodes: vec![],
            isolated_pages: vec!["p1".to_string()],
            stats: GraphInsightStats {
                total_pages: 10,
                total_edges: 20,
                num_communities: 2,
                avg_cohesion: 0.5,
                modularity: 0.3,
            },
        };
        let json = serde_json::to_string(&insights).unwrap();
        let deserialized: GraphInsights = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.isolated_pages.len(), 1);
        assert!(deserialized.surprising_connections.is_empty());
    }

    #[test]
    fn test_knowledge_gap_with_community() {
        let gap = KnowledgeGap {
            gap_type: GapType::SparseCommunity,
            node_ids: vec!["n1".to_string(), "n2".to_string()],
            node_titles: vec!["A".to_string(), "B".to_string()],
            description: "Low cohesion".to_string(),
            community_id: Some(3),
            suggested_action: "Add wikilinks".to_string(),
        };
        assert_eq!(gap.community_id, Some(3));
        assert_eq!(gap.node_ids.len(), 2);
    }

    #[test]
    fn test_surprising_connection_dismissed() {
        let conn = SurprisingConnection {
            page_a: "a".to_string(),
            page_b: "b".to_string(),
            title_a: "A".to_string(),
            title_b: "B".to_string(),
            surprise_score: 2.5,
            reasons: vec![],
            dismissed: true,
        };
        assert!(conn.dismissed);
    }
}
