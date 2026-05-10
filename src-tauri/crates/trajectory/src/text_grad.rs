//! TextGrad-style text gradient backpropagation for agent self-evolution
//!
//! Implements a computation graph over agent components (Prompts, Tools, Memory)
//! where text feedback propagates in reverse topological order, analogous to
//! gradient backpropagation in neural networks. Each node accumulates a
//! "text gradient" — a natural-language suggestion for improvement — which is
//! then applied to modify the node's content.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Prompt,
    Tool,
    Memory,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Prompt => write!(f, "prompt"),
            NodeType::Tool => write!(f, "tool"),
            NodeType::Memory => write!(f, "memory"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationNode {
    pub id: String,
    pub node_type: NodeType,
    pub content: String,
    pub gradient: Option<String>,
}

impl ComputationNode {
    pub fn new(node_type: NodeType, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            node_type,
            content: content.into(),
            gradient: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_gradient(mut self, gradient: impl Into<String>) -> Self {
        self.gradient = Some(gradient.into());
        self
    }

    pub fn clear_gradient(&mut self) {
        self.gradient = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationEdge {
    pub source_id: String,
    pub target_id: String,
}

impl ComputationEdge {
    pub fn new(source_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationGraph {
    pub nodes: Vec<ComputationNode>,
    pub edges: Vec<ComputationEdge>,
}

impl ComputationGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: ComputationNode) -> &str {
        self.nodes.push(node);
        self.nodes.last().map(|n| n.id.as_str()).unwrap_or("")
    }

    pub fn add_edge(&mut self, edge: ComputationEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&ComputationNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut ComputationNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn predecessors(&self, node_id: &str) -> Vec<&ComputationNode> {
        self.edges
            .iter()
            .filter(|e| e.target_id == node_id)
            .filter_map(|e| self.get_node(&e.source_id))
            .collect()
    }

    pub fn successors(&self, node_id: &str) -> Vec<&ComputationNode> {
        self.edges
            .iter()
            .filter(|e| e.source_id == node_id)
            .filter_map(|e| self.get_node(&e.target_id))
            .collect()
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(&node.id).or_insert(0);
            adjacency.entry(&node.id).or_default();
        }

        for edge in &self.edges {
            *in_degree.entry(&edge.target_id).or_insert(0) += 1;
            adjacency
                .entry(&edge.source_id)
                .or_default()
                .push(&edge.target_id);
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::with_capacity(self.nodes.len());

        while let Some(id) = queue.pop_front() {
            sorted.push(id.to_string());
            if let Some(neighbors) = adjacency.get(id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            return Err("Cycle detected in computation graph".to_string());
        }

        Ok(sorted)
    }

    pub fn reverse_topological_sort(&self) -> Result<Vec<String>, String> {
        let mut sorted = self.topological_sort()?;
        sorted.reverse();
        Ok(sorted)
    }

    pub fn backward(&mut self, output_feedback: &str) -> Result<(), String> {
        let order = self.reverse_topological_sort()?;

        for node_id in &order {
            let successor_feedbacks: Vec<String> = {
                let succs = self.successors(node_id);
                succs
                    .iter()
                    .filter_map(|s| s.gradient.as_ref().cloned())
                    .collect()
            };

            let combined_feedback = if successor_feedbacks.is_empty() {
                output_feedback.to_string()
            } else {
                let mut fb = output_feedback.to_string();
                fb.push_str("\n\nDownstream gradient contributions:\n");
                for (i, sf) in successor_feedbacks.iter().enumerate() {
                    fb.push_str(&format!("{}. {}\n", i + 1, sf));
                }
                fb
            };

            if let Some(node) = self.get_node_mut(node_id) {
                let gradient = match node.node_type {
                    NodeType::Prompt => {
                        format!(
                            "Prompt improvement suggestion based on feedback:\n{}\n\n\
                             Current prompt: {}\n\
                             Suggested revision: Consider adjusting the prompt to address the feedback above.",
                            combined_feedback, node.content
                        )
                    },
                    NodeType::Tool => {
                        format!(
                            "Tool usage improvement suggestion based on feedback:\n{}\n\n\
                             Current tool description: {}\n\
                             Suggested revision: Consider modifying the tool behavior or parameters to address the feedback above.",
                            combined_feedback, node.content
                        )
                    },
                    NodeType::Memory => {
                        format!(
                            "Memory entry improvement suggestion based on feedback:\n{}\n\n\
                             Current memory: {}\n\
                             Suggested revision: Consider updating the memory content to address the feedback above.",
                            combined_feedback, node.content
                        )
                    },
                };
                node.gradient = Some(gradient);
            }
        }

        Ok(())
    }

    pub fn leaf_nodes(&self) -> Vec<&ComputationNode> {
        let target_ids: HashSet<&str> = self.edges.iter().map(|e| e.target_id.as_str()).collect();

        self.nodes
            .iter()
            .filter(|n| !target_ids.contains(n.id.as_str()))
            .collect()
    }

    pub fn root_nodes(&self) -> Vec<&ComputationNode> {
        let source_ids: HashSet<&str> = self.edges.iter().map(|e| e.source_id.as_str()).collect();

        self.nodes
            .iter()
            .filter(|n| !source_ids.contains(n.id.as_str()))
            .collect()
    }

    pub fn output_nodes(&self) -> Vec<&ComputationNode> {
        let source_ids: HashSet<&str> = self.edges.iter().map(|e| e.source_id.as_str()).collect();

        self.nodes
            .iter()
            .filter(|n| !source_ids.contains(n.id.as_str()))
            .collect()
    }

    pub fn clear_gradients(&mut self) {
        for node in &mut self.nodes {
            node.clear_gradient();
        }
    }
}

impl Default for ComputationGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGradConfig {
    pub max_iterations: usize,
    pub gradient_threshold: f64,
    pub learning_rate: f64,
    pub max_gradient_length: usize,
    pub apply_prompt_gradients: bool,
    pub apply_tool_gradients: bool,
    pub apply_memory_gradients: bool,
    pub convergence_window: usize,
}

impl Default for TextGradConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            gradient_threshold: 0.01,
            learning_rate: 1.0,
            max_gradient_length: 2000,
            convergence_window: 3,
            apply_prompt_gradients: true,
            apply_tool_gradients: true,
            apply_memory_gradients: true,
        }
    }
}

pub trait LlmTextGradProvider: Send + Sync {
    fn compute_gradient(
        &self,
        node_content: &str,
        output_feedback: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>>;
}

pub struct DefaultTextGradProvider {
    max_gradient_length: usize,
}

impl Default for DefaultTextGradProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTextGradProvider {
    pub fn new() -> Self {
        Self {
            max_gradient_length: 2000,
        }
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_gradient_length = max_length;
        self
    }

    fn heuristic_gradient(node_content: &str, output_feedback: &str) -> String {
        let mut suggestions = Vec::new();

        if output_feedback.contains("error")
            || output_feedback.contains("Error")
            || output_feedback.contains("failed")
        {
            suggestions.push(
                "Add error handling and recovery instructions to address reported failures."
                    .to_string(),
            );
        }

        if output_feedback.contains("slow")
            || output_feedback.contains("timeout")
            || output_feedback.contains("performance")
        {
            suggestions.push(
                "Optimize for performance: reduce unnecessary steps or add caching hints."
                    .to_string(),
            );
        }

        if output_feedback.contains("incorrect")
            || output_feedback.contains("wrong")
            || output_feedback.contains("unexpected")
        {
            suggestions.push(
                "Clarify the expected behavior and add validation checks to prevent incorrect results."
                    .to_string(),
            );
        }

        if output_feedback.contains("missing") || output_feedback.contains("incomplete") {
            suggestions.push(
                "Add missing information or steps to ensure completeness of the output."
                    .to_string(),
            );
        }

        if output_feedback.contains("verbose")
            || output_feedback.contains("too long")
            || output_feedback.contains("concise")
        {
            suggestions
                .push("Reduce verbosity: be more concise and focused in the output.".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push(format!(
                "Based on feedback: \"{}\", consider refining the content to better align with expected outcomes.",
                if output_feedback.len() > 100 {
                    format!("{}...", &output_feedback[..100])
                } else {
                    output_feedback.to_string()
                }
            ));
        }

        let content_len = node_content.len();
        if content_len > 500 {
            suggestions.push(format!(
                "Current content is {} characters long. Consider simplifying or restructuring for clarity.",
                content_len
            ));
        }

        suggestions.join(" ")
    }
}

impl LlmTextGradProvider for DefaultTextGradProvider {
    fn compute_gradient(
        &self,
        node_content: &str,
        output_feedback: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
        let gradient = Self::heuristic_gradient(node_content, output_feedback);
        let truncated = if gradient.len() > self.max_gradient_length {
            gradient[..self.max_gradient_length].to_string()
        } else {
            gradient
        };
        Box::pin(async move { Ok(truncated) })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardResult {
    pub output: String,
    pub node_outputs: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeResult {
    pub iterations: usize,
    pub converged: bool,
    pub final_output: String,
    pub gradient_norms: Vec<f64>,
    pub node_modifications: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGradStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub gradient_count: usize,
}

pub struct TextGradEngine {
    graph: ComputationGraph,
    provider: Box<dyn LlmTextGradProvider>,
    config: TextGradConfig,
}

impl TextGradEngine {
    pub fn new(graph: ComputationGraph, config: TextGradConfig) -> Self {
        Self {
            graph,
            provider: Box::new(DefaultTextGradProvider::new()),
            config,
        }
    }

    pub fn with_provider(
        graph: ComputationGraph,
        config: TextGradConfig,
        provider: Box<dyn LlmTextGradProvider>,
    ) -> Self {
        Self {
            graph,
            provider,
            config,
        }
    }

    pub fn set_provider(&mut self, provider: impl LlmTextGradProvider + 'static) {
        self.provider = Box::new(provider);
    }

    pub fn add_node(
        &mut self,
        id: impl Into<String>,
        content: impl Into<String>,
        node_type: Option<impl Into<String>>,
    ) {
        let nt = node_type
            .map(|t| match t.into().as_str() {
                "prompt" | "Prompt" => NodeType::Prompt,
                "tool" | "Tool" => NodeType::Tool,
                _ => NodeType::Memory,
            })
            .unwrap_or(NodeType::Memory);
        let node = ComputationNode::new(nt, content).with_id(id);
        self.graph.add_node(node);
    }

    pub fn add_edge(
        &mut self,
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        _weight: f64,
    ) {
        self.graph
            .add_edge(ComputationEdge::new(source_id, target_id));
    }

    pub async fn backward(
        &mut self,
        _output_node_id: &str,
        feedback: &str,
    ) -> Result<HashMap<String, String>, String> {
        self.backward_text_grad(feedback).await
    }

    pub fn stats(&self) -> TextGradStats {
        let gradient_count = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.gradient.is_some())
            .count();
        TextGradStats {
            node_count: self.graph.nodes.len(),
            edge_count: self.graph.edges.len(),
            gradient_count,
        }
    }

    pub fn graph(&self) -> &ComputationGraph {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut ComputationGraph {
        &mut self.graph
    }

    pub fn config(&self) -> &TextGradConfig {
        &self.config
    }

    pub fn forward(&self) -> ForwardResult {
        let mut node_outputs: HashMap<String, String> = HashMap::new();
        let order = match self.graph.topological_sort() {
            Ok(o) => o,
            Err(_) => {
                return ForwardResult {
                    output: String::new(),
                    node_outputs,
                }
            },
        };

        for node_id in &order {
            if let Some(node) = self.graph.get_node(node_id) {
                let preds = self.graph.predecessors(node_id);
                let input_context: String = preds
                    .iter()
                    .filter_map(|p| node_outputs.get(&p.id))
                    .cloned()
                    .collect::<Vec<String>>()
                    .join("\n");

                let output = if input_context.is_empty() {
                    node.content.clone()
                } else {
                    format!("{}\n{}", input_context, node.content)
                };

                node_outputs.insert(node_id.clone(), output);
            }
        }

        let output_nodes = self.graph.output_nodes();
        let final_output: String = output_nodes
            .iter()
            .filter_map(|n| node_outputs.get(&n.id))
            .cloned()
            .collect::<Vec<String>>()
            .join("\n");

        ForwardResult {
            output: final_output,
            node_outputs,
        }
    }

    pub async fn backward_text_grad(
        &mut self,
        output_feedback: &str,
    ) -> Result<HashMap<String, String>, String> {
        self.graph.clear_gradients();

        let order = self.graph.reverse_topological_sort()?;

        for node_id in &order {
            let predecessor_gradients: Vec<String> = {
                let preds = self.graph.predecessors(node_id);
                preds
                    .iter()
                    .filter_map(|p| p.gradient.as_ref().cloned())
                    .collect()
            };

            let combined_feedback = if predecessor_gradients.is_empty() {
                output_feedback.to_string()
            } else {
                let mut fb = output_feedback.to_string();
                fb.push_str("\n\nUpstream gradient contributions:\n");
                for (i, pg) in predecessor_gradients.iter().enumerate() {
                    fb.push_str(&format!("{}. {}\n", i + 1, pg));
                }
                fb
            };

            let node_content = self
                .graph
                .get_node(node_id)
                .map(|n| n.content.clone())
                .unwrap_or_default();

            let gradient = self
                .provider
                .compute_gradient(&node_content, &combined_feedback)
                .await
                .map_err(|e| format!("Gradient computation failed for node {}: {}", node_id, e))?;

            let truncated = if gradient.len() > self.config.max_gradient_length {
                gradient[..self.config.max_gradient_length].to_string()
            } else {
                gradient
            };

            if let Some(node) = self.graph.get_node_mut(node_id) {
                node.gradient = Some(truncated);
            }
        }

        let gradients: HashMap<String, String> = self
            .graph
            .nodes
            .iter()
            .filter_map(|n| n.gradient.as_ref().map(|g| (n.id.clone(), g.clone())))
            .collect();

        Ok(gradients)
    }

    pub fn apply_gradients(&mut self) -> HashMap<String, String> {
        let mut modifications = HashMap::new();

        let learning_rate = self.config.learning_rate;

        let pending: Vec<(String, String, String)> = self
            .graph
            .nodes
            .iter()
            .filter_map(|node| {
                let should_apply = match node.node_type {
                    NodeType::Prompt => self.config.apply_prompt_gradients,
                    NodeType::Tool => self.config.apply_tool_gradients,
                    NodeType::Memory => self.config.apply_memory_gradients,
                };

                if !should_apply {
                    return None;
                }

                if let Some(ref gradient) = node.gradient {
                    let new_content =
                        Self::merge_gradient_static(&node.content, gradient, learning_rate);
                    if new_content != node.content {
                        Some((node.id.clone(), node.content.clone(), new_content))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for (id, old_content, new_content) in pending {
            modifications.insert(
                id.clone(),
                format!(
                    "Modified node: {} -> {}",
                    summarize_content(&old_content),
                    summarize_content(&new_content)
                ),
            );
            if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == id) {
                node.content = new_content;
                node.gradient = None;
            }
        }

        modifications
    }

    fn merge_gradient_static(content: &str, gradient: &str, learning_rate: f64) -> String {
        if learning_rate >= 1.0 {
            format!("{}\n\n<!-- TextGrad revision -->\n{}", content, gradient)
        } else {
            let keep_chars = (content.len() as f64 * (1.0 - learning_rate)) as usize;
            let keep_chars = keep_chars.max(1).min(content.len());
            let preserved = &content[..keep_chars];
            format!(
                "{}\n\n<!-- TextGrad revision (lr={}) -->\n{}",
                preserved, learning_rate, gradient
            )
        }
    }

    pub async fn optimize(&mut self, initial_feedback: &str) -> Result<OptimizeResult, String> {
        let mut gradient_norms = Vec::new();
        let mut all_modifications = HashMap::new();
        let mut converged = false;
        let mut iterations = 0;

        for i in 0..self.config.max_iterations {
            iterations = i + 1;

            let forward_result = self.forward();

            let feedback = if i == 0 {
                initial_feedback.to_string()
            } else {
                format!(
                    "Iteration {} feedback: Previous output was:\n{}\n\nOriginal feedback: {}",
                    i + 1,
                    forward_result.output,
                    initial_feedback
                )
            };

            let gradients = self.backward_text_grad(&feedback).await?;

            let norm = compute_gradient_norm(&gradients);
            gradient_norms.push(norm);

            let modifications = self.apply_gradients();
            for (k, v) in modifications {
                all_modifications.insert(k, v);
            }

            if gradient_norms.len() >= self.config.convergence_window {
                let window =
                    &gradient_norms[gradient_norms.len() - self.config.convergence_window..];
                let avg_norm: f64 = window.iter().sum::<f64>() / window.len() as f64;
                if avg_norm < self.config.gradient_threshold {
                    converged = true;
                    break;
                }

                if window.len() >= 2 {
                    let first_half: f64 =
                        window[..window.len() / 2].iter().sum::<f64>() / (window.len() / 2) as f64;
                    let second_half: f64 = window[window.len() / 2..].iter().sum::<f64>()
                        / (window.len() - window.len() / 2) as f64;
                    let delta = (second_half - first_half).abs();
                    if delta < self.config.gradient_threshold {
                        converged = true;
                        break;
                    }
                }
            }
        }

        let final_forward = self.forward();

        Ok(OptimizeResult {
            iterations,
            converged,
            final_output: final_forward.output,
            gradient_norms,
            node_modifications: all_modifications,
        })
    }
}

fn compute_gradient_norm(gradients: &HashMap<String, String>) -> f64 {
    if gradients.is_empty() {
        return 0.0;
    }

    let total_chars: usize = gradients.values().map(|g| g.len()).sum();
    let avg_chars = total_chars as f64 / gradients.len() as f64;

    let information_density = gradients
        .values()
        .map(|g| {
            let unique_words: HashSet<&str> = g.split_whitespace().collect();
            let total_words = g.split_whitespace().count();
            if total_words == 0 {
                0.0
            } else {
                unique_words.len() as f64 / total_words as f64
            }
        })
        .sum::<f64>()
        / gradients.len() as f64;

    (avg_chars / 100.0) * information_density
}

fn summarize_content(content: &str) -> String {
    if content.len() <= 50 {
        content.to_string()
    } else {
        format!("{}...", &content[..47])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_display() {
        assert_eq!(NodeType::Prompt.to_string(), "prompt");
        assert_eq!(NodeType::Tool.to_string(), "tool");
        assert_eq!(NodeType::Memory.to_string(), "memory");
    }

    #[test]
    fn test_computation_node_creation() {
        let node = ComputationNode::new(NodeType::Prompt, "You are a helpful assistant");
        assert_eq!(node.node_type, NodeType::Prompt);
        assert_eq!(node.content, "You are a helpful assistant");
        assert!(node.gradient.is_none());
        assert!(!node.id.is_empty());
    }

    #[test]
    fn test_computation_node_with_id() {
        let node = ComputationNode::new(NodeType::Tool, "search").with_id("tool_search");
        assert_eq!(node.id, "tool_search");
    }

    #[test]
    fn test_computation_node_with_gradient() {
        let node = ComputationNode::new(NodeType::Memory, "user prefers dark mode")
            .with_gradient("Consider adding theme preference details");
        assert_eq!(node.gradient.as_deref(), Some("Consider adding theme preference details"));
    }

    #[test]
    fn test_clear_gradient() {
        let mut node = ComputationNode::new(NodeType::Prompt, "test");
        node.gradient = Some("some gradient".to_string());
        node.clear_gradient();
        assert!(node.gradient.is_none());
    }

    #[test]
    fn test_computation_edge_creation() {
        let edge = ComputationEdge::new("node_a", "node_b");
        assert_eq!(edge.source_id, "node_a");
        assert_eq!(edge.target_id, "node_b");
    }

    #[test]
    fn test_graph_add_node() {
        let mut graph = ComputationGraph::new();
        let node = ComputationNode::new(NodeType::Prompt, "test prompt");
        let id = node.id.clone();
        graph.add_node(node);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.get_node(&id).is_some());
    }

    #[test]
    fn test_graph_add_edge() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt");
        let n2 = ComputationNode::new(NodeType::Tool, "tool");
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new(&id1, &id2));
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_graph_predecessors() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt");
        let n2 = ComputationNode::new(NodeType::Tool, "tool");
        let n3 = ComputationNode::new(NodeType::Memory, "memory");
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        let id3 = n3.id.clone();
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new(&id1, &id2));
        graph.add_edge(ComputationEdge::new(&id3, &id2));

        let preds = graph.predecessors(&id2);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn test_graph_successors() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt");
        let n2 = ComputationNode::new(NodeType::Tool, "tool");
        let n3 = ComputationNode::new(NodeType::Memory, "memory");
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        let id3 = n3.id.clone();
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new(&id1, &id2));
        graph.add_edge(ComputationEdge::new(&id1, &id3));

        let succs = graph.successors(&id1);
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool").with_id("b");
        let n3 = ComputationNode::new(NodeType::Memory, "memory").with_id("c");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new("a", "b"));
        graph.add_edge(ComputationEdge::new("b", "c"));

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_topological_sort_diamond() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool1").with_id("b");
        let n3 = ComputationNode::new(NodeType::Tool, "tool2").with_id("c");
        let n4 = ComputationNode::new(NodeType::Memory, "memory").with_id("d");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_node(n4);
        graph.add_edge(ComputationEdge::new("a", "b"));
        graph.add_edge(ComputationEdge::new("a", "c"));
        graph.add_edge(ComputationEdge::new("b", "d"));
        graph.add_edge(ComputationEdge::new("c", "d"));

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted[0], "a");
        assert_eq!(sorted[3], "d");
        assert!(sorted[1..3].contains(&"b".to_string()));
        assert!(sorted[1..3].contains(&"c".to_string()));
    }

    #[test]
    fn test_topological_sort_cycle_detected() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool").with_id("b");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new("a", "b"));
        graph.add_edge(ComputationEdge::new("b", "a"));

        let result = graph.topological_sort();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle"));
    }

    #[test]
    fn test_reverse_topological_sort() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool").with_id("b");
        let n3 = ComputationNode::new(NodeType::Memory, "memory").with_id("c");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new("a", "b"));
        graph.add_edge(ComputationEdge::new("b", "c"));

        let sorted = graph.reverse_topological_sort().unwrap();
        assert_eq!(sorted, vec!["c", "b", "a"]);
    }

    #[test]
    fn test_backward_propagation() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "You are an assistant").with_id("prompt");
        let n2 = ComputationNode::new(NodeType::Tool, "search tool").with_id("tool");
        let n3 = ComputationNode::new(NodeType::Memory, "context memory").with_id("memory");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new("prompt", "tool"));
        graph.add_edge(ComputationEdge::new("tool", "memory"));

        let result = graph.backward("The output was incorrect");
        assert!(result.is_ok());

        assert!(graph.get_node("memory").unwrap().gradient.is_some());
        assert!(graph.get_node("tool").unwrap().gradient.is_some());
        assert!(graph.get_node("prompt").unwrap().gradient.is_some());
    }

    #[test]
    fn test_leaf_and_output_nodes() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool").with_id("b");
        let n3 = ComputationNode::new(NodeType::Memory, "memory").with_id("c");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_edge(ComputationEdge::new("a", "b"));
        graph.add_edge(ComputationEdge::new("b", "c"));

        let leaves = graph.leaf_nodes();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].id, "a");

        let outputs = graph.output_nodes();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id, "c");
    }

    #[test]
    fn test_clear_gradients() {
        let mut graph = ComputationGraph::new();
        let mut n1 = ComputationNode::new(NodeType::Prompt, "prompt");
        n1.gradient = Some("some gradient".to_string());
        graph.add_node(n1);

        graph.clear_gradients();
        assert!(graph.nodes[0].gradient.is_none());
    }

    #[test]
    fn test_text_grad_config_default() {
        let config = TextGradConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert!(config.apply_prompt_gradients);
        assert!(config.apply_tool_gradients);
        assert!(config.apply_memory_gradients);
        assert_eq!(config.convergence_window, 3);
    }

    #[test]
    fn test_default_text_grad_provider() {
        let provider = DefaultTextGradProvider::new();
        assert_eq!(provider.max_gradient_length, 2000);
    }

    #[tokio::test]
    async fn test_default_provider_compute_gradient() {
        let provider = DefaultTextGradProvider::new();
        let gradient = provider
            .compute_gradient("You are an assistant", "The output had an error")
            .await
            .unwrap();
        assert!(!gradient.is_empty());
        assert!(gradient.contains("error") || gradient.contains("Error"));
    }

    #[tokio::test]
    async fn test_default_provider_performance_feedback() {
        let provider = DefaultTextGradProvider::new();
        let gradient = provider
            .compute_gradient("Search the web", "The tool was too slow and timed out")
            .await
            .unwrap();
        assert!(
            gradient.to_lowercase().contains("performance")
                || gradient.to_lowercase().contains("optim")
        );
    }

    #[tokio::test]
    async fn test_default_provider_generic_feedback() {
        let provider = DefaultTextGradProvider::new();
        let gradient = provider
            .compute_gradient("Do something", "The result was okay")
            .await
            .unwrap();
        assert!(!gradient.is_empty());
    }

    #[test]
    fn test_engine_forward() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "Hello").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "World").with_id("b");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new("a", "b"));

        let engine = TextGradEngine::new(graph, TextGradConfig::default());
        let result = engine.forward();

        assert!(!result.output.is_empty());
        assert!(result.output.contains("Hello"));
        assert!(result.output.contains("World"));
    }

    #[tokio::test]
    async fn test_engine_backward_text_grad() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "You are helpful").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "search").with_id("b");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new("a", "b"));

        let mut engine = TextGradEngine::new(graph, TextGradConfig::default());
        let gradients = engine
            .backward_text_grad("The output was incorrect")
            .await
            .unwrap();

        assert!(!gradients.is_empty());
        assert!(gradients.contains_key("a"));
        assert!(gradients.contains_key("b"));
    }

    #[tokio::test]
    async fn test_engine_apply_gradients() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "You are helpful").with_id("a");
        graph.add_node(n1);

        let mut engine = TextGradEngine::new(graph, TextGradConfig::default());

        engine.graph_mut().get_node_mut("a").unwrap().gradient =
            Some("Add more specificity".to_string());

        let modifications = engine.apply_gradients();
        assert!(!modifications.is_empty());
        assert!(engine
            .graph()
            .get_node("a")
            .unwrap()
            .content
            .contains("TextGrad revision"));
        assert!(engine.graph().get_node("a").unwrap().gradient.is_none());
    }

    #[tokio::test]
    async fn test_engine_apply_gradients_respects_config() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "tool").with_id("b");
        graph.add_node(n1);
        graph.add_node(n2);

        let config = TextGradConfig {
            apply_prompt_gradients: false,
            apply_tool_gradients: true,
            ..Default::default()
        };

        let mut engine = TextGradEngine::new(graph, config);
        engine.graph_mut().get_node_mut("a").unwrap().gradient =
            Some("gradient for prompt".to_string());
        engine.graph_mut().get_node_mut("b").unwrap().gradient =
            Some("gradient for tool".to_string());

        let modifications = engine.apply_gradients();

        assert!(!modifications.contains_key("a"));
        assert!(modifications.contains_key("b"));
    }

    #[tokio::test]
    async fn test_engine_optimize() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "You are an assistant").with_id("prompt");
        let n2 = ComputationNode::new(NodeType::Tool, "search tool").with_id("tool");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new("prompt", "tool"));

        let config = TextGradConfig {
            max_iterations: 3,
            gradient_threshold: 0.001,
            ..Default::default()
        };

        let mut engine = TextGradEngine::new(graph, config);
        let result = engine.optimize("The output had errors").await.unwrap();

        assert!(result.iterations <= 3);
        assert!(!result.final_output.is_empty());
        assert!(!result.gradient_norms.is_empty());
    }

    #[tokio::test]
    async fn test_engine_optimize_convergence() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Memory, "simple fact").with_id("mem");
        graph.add_node(n1);

        let config = TextGradConfig {
            max_iterations: 10,
            gradient_threshold: 100.0,
            ..Default::default()
        };

        let mut engine = TextGradEngine::new(graph, config);
        let result = engine.optimize("Minor feedback").await.unwrap();

        assert!(result.converged);
        assert!(result.iterations < 10);
    }

    #[test]
    fn test_compute_gradient_norm_empty() {
        let gradients: HashMap<String, String> = HashMap::new();
        let norm = compute_gradient_norm(&gradients);
        assert_eq!(norm, 0.0);
    }

    #[test]
    fn test_compute_gradient_norm_nonempty() {
        let mut gradients = HashMap::new();
        gradients.insert("a".to_string(), "This is a gradient with some words".to_string());
        let norm = compute_gradient_norm(&gradients);
        assert!(norm > 0.0);
    }

    #[test]
    fn test_summarize_content_short() {
        let result = summarize_content("short");
        assert_eq!(result, "short");
    }

    #[test]
    fn test_summarize_content_long() {
        let long = "a".repeat(100);
        let result = summarize_content(&long);
        assert!(result.ends_with("..."));
        assert!(result.len() < long.len());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut graph = ComputationGraph::new();
        let n1 = ComputationNode::new(NodeType::Prompt, "test prompt").with_id("a");
        let n2 = ComputationNode::new(NodeType::Tool, "test tool").with_id("b");
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_edge(ComputationEdge::new("a", "b"));

        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: ComputationGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.nodes.len(), 2);
        assert_eq!(deserialized.edges.len(), 1);
        assert_eq!(deserialized.nodes[0].node_type, NodeType::Prompt);
    }

    #[test]
    fn test_config_serialization() {
        let config = TextGradConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TextGradConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_iterations, config.max_iterations);
        assert_eq!(deserialized.gradient_threshold, config.gradient_threshold);
    }

    #[test]
    fn test_forward_result_serialization() {
        let result = ForwardResult {
            output: "test output".to_string(),
            node_outputs: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), "node a output".to_string());
                m
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ForwardResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.output, "test output");
    }

    #[test]
    fn test_optimize_result_serialization() {
        let result = OptimizeResult {
            iterations: 5,
            converged: true,
            final_output: "done".to_string(),
            gradient_norms: vec![1.0, 0.5, 0.1],
            node_modifications: HashMap::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: OptimizeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.iterations, 5);
        assert!(deserialized.converged);
    }

    #[tokio::test]
    async fn test_full_pipeline() {
        let mut graph = ComputationGraph::new();
        let prompt =
            ComputationNode::new(NodeType::Prompt, "You are a coding assistant").with_id("prompt");
        let tool = ComputationNode::new(NodeType::Tool, "code_search tool").with_id("tool");
        let memory = ComputationNode::new(NodeType::Memory, "user prefers Rust").with_id("memory");
        graph.add_node(prompt);
        graph.add_node(tool);
        graph.add_node(memory);
        graph.add_edge(ComputationEdge::new("prompt", "tool"));
        graph.add_edge(ComputationEdge::new("tool", "memory"));

        let config = TextGradConfig {
            max_iterations: 2,
            gradient_threshold: 0.001,
            ..TextGradConfig::default()
        };

        let mut engine = TextGradEngine::new(graph, config);

        let forward1 = engine.forward();
        assert!(!forward1.output.is_empty());

        let gradients = engine
            .backward_text_grad("The code search returned incorrect results")
            .await
            .unwrap();
        assert_eq!(gradients.len(), 3);

        let modifications = engine.apply_gradients();
        assert!(!modifications.is_empty());

        let forward2 = engine.forward();
        assert!(forward2.output.contains("TextGrad revision"));
    }
}
