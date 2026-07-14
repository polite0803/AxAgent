// SPDX-License-Identifier: AGPL-3.0-only

//! Learning Graph Service
//!
//! Aggregates skills, memory chunks, and learning insights into a unified
//! graph structure (nodes + edges) for the frontend LearningGraphPage.
//!
//! Inspired by Hermes-Agent's `learning_graph.py` — surfaces what the user
//! has learned over time as a navigable, visual graph.

use crate::auto_memory::{ExtractedMemory, MemoryType};
use crate::insight::{InsightCategory, LearningInsight};
use crate::skill::Skill;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Node kind in the learning graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Skill,
    Memory,
    Insight,
}

/// A node in the learning graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: NodeKind,
    pub category: String,
    #[serde(rename = "timestampMs")]
    pub timestamp_ms: i64,
    #[serde(rename = "useCount")]
    pub use_count: u32,
    pub state: String,
    pub detail: Option<String>,
}

/// An edge in the learning graph (undirected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub relation: String,
}

/// Complete learning graph payload for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

/// Aggregated statistics about the learning graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    #[serde(rename = "totalSkills")]
    pub total_skills: usize,
    #[serde(rename = "totalMemories")]
    pub total_memories: usize,
    #[serde(rename = "totalInsights")]
    pub total_insights: usize,
    #[serde(rename = "totalEdges")]
    pub total_edges: usize,
    #[serde(rename = "linkedNodes")]
    pub linked_nodes: usize,
    pub categories: Vec<CategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

/// Build a consistent, content-derived ID for a memory node so that
/// the same memory always maps to the same ID regardless of ordering.
fn memory_node_id(mem: &ExtractedMemory) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    mem.content.hash(&mut hasher);
    mem.source_trajectory.hash(&mut hasher);
    format!("memory:{:x}", hasher.finish())
}

/// Build a LearningGraph from available data sources.
///
/// This is a standalone function rather than a Service to avoid coupling
/// with internal module visibility. All data is passed in explicitly.
pub fn build_learning_graph(
    skills: &[Skill],
    memories: &[ExtractedMemory],
    insights: &[LearningInsight],
) -> LearningGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    // 1. Skill nodes
    for skill in skills {
        nodes.push(GraphNode {
            id: format!("skill:{}", skill.name),
            label: skill.name.clone(),
            kind: NodeKind::Skill,
            category: skill.category.clone(),
            timestamp_ms: skill.created_at.timestamp_millis(),
            use_count: skill.total_usages,
            state: "active".to_string(),
            detail: Some(skill.description.clone()),
        });
    }

    // 2. Memory nodes — stable content-derived IDs
    for mem in memories {
        let label = mem.content.chars().take(60).collect::<String>();
        nodes.push(GraphNode {
            id: memory_node_id(mem),
            label,
            kind: NodeKind::Memory,
            category: match mem.memory_type {
                MemoryType::Preference => "preference",
                MemoryType::Fact => "fact",
                MemoryType::Pattern => "pattern",
                MemoryType::Context => "context",
                MemoryType::Project => "project",
            }
            .to_string(),
            timestamp_ms: mem.created_at,
            use_count: 0,
            state: "active".to_string(),
            detail: Some(mem.content.chars().take(200).collect()),
        });
    }

    // 3. Insight nodes
    for insight in insights {
        nodes.push(GraphNode {
            id: format!("insight:{}", insight.id),
            label: insight.title.chars().take(60).collect(),
            kind: NodeKind::Insight,
            category: match insight.category {
                InsightCategory::Pattern => "pattern",
                InsightCategory::Preference => "preference",
                InsightCategory::Improvement => "improvement",
                InsightCategory::Warning => "warning",
            }
            .to_string(),
            timestamp_ms: insight.created_at,
            use_count: 0,
            state: "active".to_string(),
            detail: Some(insight.description.clone()),
        });
    }

    // 4. Compute edges

    // 4a. Memory ↔ Skill — lexical overlap (token intersection)
    let skill_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.kind == NodeKind::Skill).collect();
    let memory_nodes: Vec<&GraphNode> =
        nodes.iter().filter(|n| n.kind == NodeKind::Memory).collect();
    let insight_nodes: Vec<&GraphNode> =
        nodes.iter().filter(|n| n.kind == NodeKind::Insight).collect();

    for mem_node in &memory_nodes {
        let mem_tokens = tokenize(&mem_node.label);
        for skill_node in &skill_nodes {
            let skill_tokens = tokenize(&skill_node.label);
            let overlap = mem_tokens.intersection(&skill_tokens).count();
            if overlap > 0 {
                let max_len = mem_tokens.len().max(skill_tokens.len()).max(1);
                let weight = overlap as f64 / max_len as f64;
                if weight >= 0.15 {
                    edges.push(GraphEdge {
                        source: mem_node.id.clone(),
                        target: skill_node.id.clone(),
                        weight,
                        relation: "lexical_overlap".to_string(),
                    });
                }
            }
        }
    }

    // 4b. Insight ↔ Skill — match by category name
    for insight_node in &insight_nodes {
        for skill_node in &skill_nodes {
            if insight_node.category == skill_node.category {
                edges.push(GraphEdge {
                    source: insight_node.id.clone(),
                    target: skill_node.id.clone(),
                    weight: 1.0,
                    relation: "category_match".to_string(),
                });
            }
        }
    }

    // 4c. Insight ↔ Memory — lexical overlap (same approach as memory ↔ skill)
    for insight_node in &insight_nodes {
        let insight_tokens = tokenize(&insight_node.label);
        let insight_detail_tokens: HashSet<String> =
            insight_node.detail.as_ref().map(|d| tokenize(d)).unwrap_or_default();
        let combined_insight_tokens: HashSet<String> =
            insight_tokens.union(&insight_detail_tokens).cloned().collect();

        for mem_node in &memory_nodes {
            let mem_tokens = tokenize(&mem_node.label);
            let overlap = combined_insight_tokens.intersection(&mem_tokens).count();
            if overlap > 0 {
                let max_len = combined_insight_tokens.len().max(mem_tokens.len()).max(1);
                let weight = overlap as f64 / max_len as f64;
                if weight >= 0.15 {
                    edges.push(GraphEdge {
                        source: insight_node.id.clone(),
                        target: mem_node.id.clone(),
                        weight,
                        relation: "lexical_overlap".to_string(),
                    });
                }
            }
        }
    }

    let linked: HashSet<String> =
        edges.iter().flat_map(|e| [e.source.clone(), e.target.clone()]).collect();

    let mut cat_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for n in &nodes {
        *cat_map.entry(n.category.clone()).or_insert(0) += 1;
    }
    let categories: Vec<CategoryCount> =
        cat_map.into_iter().map(|(category, count)| CategoryCount { category, count }).collect();

    let total_skills = nodes.iter().filter(|n| n.kind == NodeKind::Skill).count();
    let total_memories = nodes.iter().filter(|n| n.kind == NodeKind::Memory).count();
    let total_insights = nodes.iter().filter(|n| n.kind == NodeKind::Insight).count();

    let total_edges = edges.len();
    LearningGraph {
        nodes,
        edges,
        stats: GraphStats {
            total_skills,
            total_memories,
            total_insights,
            total_edges,
            linked_nodes: linked.len(),
            categories,
        },
    }
}

/// Check if a character is CJK (Chinese / Japanese / Korean).
fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        // CJK Unified Ideographs
        0x4E00..=0x9FFF |
        // CJK Unified Ideographs Extension A
        0x3400..=0x4DBF |
        // CJK Unified Ideographs Extension B
        0x20000..=0x2A6DF |
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF |
        // CJK Unified Ideographs Extension C–H (partial range, catch the key block)
        0x2A700..=0x2B73F |
        0x2B740..=0x2B81F |
        0x2B820..=0x2CEAF |
        // CJK Compatibility
        0xFE30..=0xFE4F |
        // Hiragana / Katakana
        0x3040..=0x30FF |
        0x31F0..=0x31FF |
        // Hangul Syllables
        0xAC00..=0xD7AF
    )
}

/// Tokenize a string into a set of lowercase tokens (length >= 2).
///
/// For CJK text, falls back to individual character tokens so that
/// Chinese/Japanese/Korean is not treated as a single un-splittable token.
fn tokenize(text: &str) -> HashSet<String> {
    let lower = text.to_lowercase();
    let mut tokens: HashSet<String> = HashSet::new();

    // First pass: split on non-alphanumeric (handles space-separated text)
    for t in lower.split(|c: char| !c.is_alphanumeric()) {
        let t = t.trim();
        if t.is_empty() || t.len() < 2 {
            continue;
        }
        // If this token contains CJK characters, split into individual chars
        // so that Chinese text like "Rust编程" produces ["rust", "编", "程"]
        if t.chars().any(is_cjk) {
            for c in t.chars() {
                if c.is_alphanumeric() && !c.is_ascii() {
                    tokens.insert(c.to_string());
                }
            }
            // Also keep ASCII sub-words (e.g. "Rust" in "Rust编程")
            let ascii_part: String = t.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            if ascii_part.len() >= 2 {
                tokens.insert(ascii_part);
            }
        } else {
            tokens.insert(t.to_string());
        }
    }

    tokens
}
