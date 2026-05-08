use axagent_core::error::AxAgentError;
use axagent_core::token_counter::estimate_tokens;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThoughtStatus {
    Generated,
    Explored,
    Pruned,
    Selected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: String,
    pub content: String,
    pub evaluation_score: f64,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub status: ThoughtStatus,
    pub tool_calls: Vec<ToolCallResult>,
}

impl ThoughtNode {
    pub fn new(id: String, content: String, parent: Option<String>) -> Self {
        Self {
            id,
            content,
            evaluation_score: 0.0,
            children: Vec::new(),
            parent,
            status: ThoughtStatus::Generated,
            tool_calls: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToTStateSummary {
    pub root_id: String,
    pub nodes: Vec<ToTNodeInfo>,
    pub edges: Vec<ToTEdge>,
    pub selected_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToTNodeInfo {
    pub id: String,
    pub content: String,
    pub evaluation_score: f64,
    pub status: ThoughtStatus,
    pub tool_call_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToTEdge {
    pub from: String,
    pub to: String,
}

pub struct TreeOfThoughtsEngine {
    pub branching_factor: usize,
    pub max_depth: usize,
    pub evaluation_threshold: f64,
    pub tree: HashMap<String, ThoughtNode>,
    pub root_id: String,
    pub next_id_counter: usize,
}

impl TreeOfThoughtsEngine {
    pub fn new(
        branching_factor: usize,
        max_depth: usize,
        evaluation_threshold: f64,
    ) -> Self {
        let root_id = format!("node_0");
        let mut tree = HashMap::new();
        tree.insert(
            root_id.clone(),
            ThoughtNode::new(root_id.clone(), "Root: Initial problem analysis".to_string(), None),
        );

        Self {
            branching_factor,
            max_depth,
            evaluation_threshold,
            tree,
            root_id,
            next_id_counter: 1,
        }
    }

    fn next_node_id(&mut self) -> String {
        let id = format!("node_{}", self.next_id_counter);
        self.next_id_counter += 1;
        id
    }

    pub fn get_node(&self, node_id: &str) -> Option<&ThoughtNode> {
        self.tree.get(node_id)
    }

    fn get_depth(&self, node_id: &str) -> usize {
        let mut depth = 0;
        let mut current = node_id.to_string();
        while let Some(node) = self.tree.get(&current) {
            if let Some(parent_id) = &node.parent {
                current = parent_id.clone();
                depth += 1;
            } else {
                break;
            }
        }
        depth
    }

    #[allow(clippy::ptr_arg)]
    #[allow(dead_code)]
    fn collect_leaves(&self, node_id: &String) -> Vec<String> {
        if let Some(node) = self.tree.get(node_id) {
            if node.is_leaf() || node.status == ThoughtStatus::Pruned {
                return vec![node_id.clone()];
            }
            node.children
                .iter()
                .flat_map(|child_id| self.collect_leaves(child_id))
                .collect()
        } else {
            vec![]
        }
    }

    pub async fn generate_branching_options(
        &mut self,
        parent_id: String,
        context: &str,
        llm_client: &Arc<dyn LlmReasoningProvider>,
    ) -> Result<Vec<String>, AxAgentError> {
        let (parent_content, parent_status, _parent_children, parent_depth) = {
            let parent = self
                .tree
                .get(&parent_id)
                .ok_or_else(|| AxAgentError::Agent {
                    source: None,
                    context: format!("Parent node '{}' not found", parent_id),
                })?;
            (
                parent.content.clone(),
                parent.status.clone(),
                parent.children.len(),
                self.get_depth(&parent_id),
            )
        };

        if parent_depth >= self.max_depth {
            debug!(
                "Max depth {} reached for node {}",
                self.max_depth, parent_id
            );
            return Ok(vec![]);
        }

        if parent_status == ThoughtStatus::Pruned {
            warn!("Attempting to expand pruned node {}", parent_id);
            return Ok(vec![]);
        }

        let count = self.branching_factor;

        debug!(
            "Generating {} branching options for parent {} (current depth: {})",
            count, parent_id, parent_depth
        );

        let mut child_ids = Vec::new();

        for i in 0..count {
            let child_id = self.next_node_id();

            let thinking_prompt = format!(
                "Given the following reasoning context and the parent thought:\n\n\
Parent thought: {}\nContext: {}\nBranch index: {} of {}\n\n\
Generate the next distinct reasoning step. Each branch should explore a different \
approach, perspective, or sub-problem decomposition. Be concise and focused.",
                truncate_string(&parent_content, 200),
                truncate_string(context, 500),
                i + 1,
                count,
            );

            let thought_content = match llm_client.think_branch(&thinking_prompt).await {
                Ok(content) => content,
                Err(e) => {
                    warn!("LLM think_branch failed for branch {}/{}: {}", i + 1, count, e);
                    format!(
                        "Alternative reasoning path {}: Explore {} from a different angle based on context: {}",
                        i + 1,
                        truncate_string(&parent_content, 80),
                        truncate_string(context, 100),
                    )
                }
            };

            let mut child_node = ThoughtNode::new(
                child_id.clone(),
                thought_content,
                Some(parent_id.clone()),
            );

            let tokens = estimate_tokens(&child_node.content);
            trace!(
                "Generated child node {} with {} tokens from parent {}",
                child_id, tokens, parent_id
            );

            child_node.evaluation_score = 0.0;

            if let Some(parent_mut) = self.tree.get_mut(&parent_id) {
                parent_mut.add_child(child_id.clone());
            }

            self.tree.insert(child_id.clone(), child_node);
            child_ids.push(child_id.clone());
        }

        debug!(
            "Generated {} children for parent {}: {:?}",
            child_ids.len(),
            parent_id,
            child_ids
        );

        Ok(child_ids)
    }

    pub async fn evaluate_thought(
        &self,
        node_id: &str,
        context: &str,
        llm_client: &Arc<dyn LlmReasoningProvider>,
    ) -> Result<f64, AxAgentError> {
        let node = self
            .tree
            .get(node_id)
            .ok_or_else(|| AxAgentError::Agent {
                source: None,
                context: format!("Node '{}' not found for evaluation", node_id),
            })?;

        let path_to_node = self.get_path_to_root(node_id);
        let path_summary: String = path_to_node
            .iter()
            .rev()
            .filter_map(|id| self.tree.get(id))
            .map(|n| truncate_string(&n.content, 150))
            .collect::<Vec<_>>()
            .join(" -> ");

        let eval_prompt = format!(
            "Evaluate the following reasoning step on a scale from 0.0 to 1.0.\n\n\
Reasoning path: {}\n\nCurrent step: {}\nContext: {}\n\n\
Score criteria:\n\
- 0.8-1.0: Highly promising, logically sound, directly addresses the goal\n\
- 0.5-0.8: Moderately promising, reasonable but may need refinement\n\
- 0.3-0.5: Weak, partial progress, significant gaps\n\
- 0.0-0.3: Unpromising, flawed reasoning or irrelevant\n\n\
Respond with only a number between 0.0 and 1.0.",
            truncate_string(&path_summary, 500),
            truncate_string(&node.content, 300),
            truncate_string(context, 300),
        );

        let score = match llm_client.evaluate_thought(&eval_prompt).await {
            Ok(response) => {
                let score_str = response.trim();
                score_str
                    .parse::<f64>()
                    .unwrap_or_else(|_| {
                        let digits: String = score_str.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
                        digits.parse::<f64>().unwrap_or(0.5)
                    })
                    .clamp(0.0, 1.0)
            }
            Err(e) => {
                warn!("LLM evaluation failed for node {}: {}", node_id, e);
                self.heuristic_evaluate(node)
            }
        };

        debug!("Node {} evaluated with score: {:.3}", node_id, score);
        Ok(score)
    }

    fn heuristic_evaluate(&self, node: &ThoughtNode) -> f64 {
        let content_len = node.content.len();
        let word_count = node.content.split_whitespace().count();

        let length_score = (content_len.min(500) as f64) / 500.0 * 0.3;
        let diversity_score = {
            let unique_words: std::collections::HashSet<&str> =
                node.content.split_whitespace().collect();
            if word_count > 0 {
                (unique_words.len() as f64 / word_count as f64) * 0.3
            } else {
                0.0
            }
        };
        let structure_score = {
            let has_structure = node.content.contains("because")
                || node.content.contains("therefore")
                || node.content.contains("however")
                || node.content.contains("first")
                || node.content.contains("next");
            if has_structure { 0.4 } else { 0.2 }
        };

        (length_score + diversity_score + structure_score).clamp(0.0, 1.0)
    }

    pub async fn evaluate_and_score_node(
        &mut self,
        node_id: &str,
        context: &str,
        llm_client: &Arc<dyn LlmReasoningProvider>,
    ) -> Result<f64, AxAgentError> {
        let score = self.evaluate_thought(node_id, context, llm_client).await?;

        if let Some(node) = self.tree.get_mut(node_id) {
            node.evaluation_score = score;
            node.status = ThoughtStatus::Explored;
        }

        Ok(score)
    }

    pub fn prune_below_threshold(&mut self, threshold: f64) -> Vec<String> {
        let mut pruned = Vec::new();

        let node_ids: Vec<String> = self.tree.keys().cloned().collect();
        for node_id in node_ids {
            if let Some(node) = self.tree.get(&node_id) {
                if node.status == ThoughtStatus::Generated
                    && node.evaluation_score < threshold
                {
                    pruned.push(node_id.clone());
                }
            }
        }

        for node_id in &pruned {
            if let Some(node) = self.tree.get_mut(node_id) {
                node.status = ThoughtStatus::Pruned;
            }
            debug!("Pruned node {} (score: {:.3})", node_id,
                self.tree.get(node_id).map(|n| n.evaluation_score).unwrap_or(0.0));
        }

        debug!("Pruned {} nodes below threshold {}", pruned.len(), threshold);
        pruned
    }

    pub fn select_best_path(&self) -> Vec<String> {
        if !self.tree.contains_key(&self.root_id) {
            return vec![];
        }

        let mut path = vec![self.root_id.clone()];
        let mut current = self.root_id.clone();

        loop {
            let node = match self.tree.get(&current) {
                Some(n) => n,
                None => break,
            };

            let active_children: Vec<&String> = node
                .children
                .iter()
                .filter(|child_id| {
                    self.tree
                        .get(*child_id)
                        .map(|child| child.status != ThoughtStatus::Pruned)
                        .unwrap_or(false)
                })
                .collect();

            if active_children.is_empty() {
                break;
            }

            let best_child = active_children
                .iter()
                .max_by(|a, b| {
                    let score_a = self
                        .tree
                        .get(a.as_str())
                        .map(|n| n.evaluation_score)
                        .unwrap_or(0.0);
                    let score_b = self
                        .tree
                        .get(b.as_str())
                        .map(|n| n.evaluation_score)
                        .unwrap_or(0.0);
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            current = (*best_child).clone();
            path.push(current.clone());
        }

        trace!("Selected best path with {} nodes: {:?}", path.len(), path);
        path
    }

    pub fn select_best_leaf(&self) -> Option<String> {
        let best_path = self.select_best_path();
        best_path.last().cloned()
    }

    pub fn get_current_state(&self) -> ToTStateSummary {
        let selected_path = self.select_best_path();

        let nodes: Vec<ToTNodeInfo> = self
            .tree
            .values()
            .map(|node| ToTNodeInfo {
                id: node.id.clone(),
                content: node.content.clone(),
                evaluation_score: node.evaluation_score,
                status: node.status.clone(),
                tool_call_count: node.tool_calls.len(),
            })
            .collect();

        let edges: Vec<ToTEdge> = self
            .tree
            .values()
            .flat_map(|node| {
                node.children.iter().map(|child_id| ToTEdge {
                    from: node.id.clone(),
                    to: child_id.clone(),
                })
            })
            .collect();

        ToTStateSummary {
            root_id: self.root_id.clone(),
            nodes,
            edges,
            selected_path,
        }
    }

    pub fn backtrack_to(&mut self, node_id: &str) -> Result<(), AxAgentError> {
        if !self.tree.contains_key(node_id) {
            return Err(AxAgentError::Agent {
                source: None,
                context: format!("Cannot backtrack to node '{}': not found", node_id),
            });
        }

        let descendants = self.collect_all_descendants(node_id);

        for desc_id in &descendants {
            self.tree.remove(desc_id);
        }

        if let Some(node) = self.tree.get_mut(node_id) {
            node.children.clear();
            node.status = ThoughtStatus::Explored;
        }

        debug!(
            "Backtracked to node {}: removed {} descendant nodes",
            node_id,
            descendants.len()
        );

        Ok(())
    }

    fn collect_all_descendants(&self, node_id: &str) -> Vec<String> {
        let mut descendants = Vec::new();
        let mut stack = vec![node_id.to_string()];

        while let Some(current) = stack.pop() {
            if let Some(node) = self.tree.get(&current) {
                for child_id in &node.children {
                    descendants.push(child_id.clone());
                    stack.push(child_id.clone());
                }
            }
        }

        descendants
    }

    pub fn get_path_to_root(&self, node_id: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = node_id.to_string();

        while let Some(node) = self.tree.get(&current) {
            path.push(current.clone());
            match &node.parent {
                Some(parent_id) => current = parent_id.clone(),
                None => break,
            }
        }

        path
    }

    pub fn mark_node_selected(&mut self, node_id: &str) {
        if let Some(node) = self.tree.get_mut(node_id) {
            node.status = ThoughtStatus::Selected;
        }
    }

    pub fn add_tool_result(
        &mut self,
        node_id: &str,
        tool_name: String,
        output: String,
        is_error: bool,
    ) {
        if let Some(node) = self.tree.get_mut(node_id) {
            node.tool_calls.push(ToolCallResult {
                tool_name,
                output,
                is_error,
            });
        }
    }

    pub fn total_nodes(&self) -> usize {
        self.tree.len()
    }

    pub fn explored_nodes(&self) -> usize {
        self.tree
            .values()
            .filter(|n| n.status == ThoughtStatus::Explored)
            .count()
    }

    pub fn pruned_nodes(&self) -> usize {
        self.tree
            .values()
            .filter(|n| n.status == ThoughtStatus::Pruned)
            .count()
    }

    pub fn get_leaves(&self) -> Vec<String> {
        self.tree
            .values()
            .filter(|n| n.is_leaf() && n.status != ThoughtStatus::Pruned)
            .map(|n| n.id.clone())
            .collect()
    }
}

#[async_trait::async_trait]
pub trait LlmReasoningProvider: Send + Sync {
    async fn think_branch(&self, prompt: &str) -> Result<String, AxAgentError>;
    async fn evaluate_thought(&self, prompt: &str) -> Result<String, AxAgentError>;
}

pub struct DefaultToTReasoningProvider {
    adapter: Option<Arc<dyn ProviderAdapter>>,
    ctx: Option<ProviderRequestContext>,
    model: String,
}

impl DefaultToTReasoningProvider {
    pub fn new() -> Self {
        Self {
            adapter: None,
            ctx: None,
            model: "gpt-4o".to_string(),
        }
    }

    pub fn with_llm(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        Self {
            adapter: Some(adapter),
            ctx: Some(ctx),
            model,
        }
    }

    pub fn from_provider_adapter(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        Self::with_llm(adapter, ctx, model)
    }

    async fn call_llm(&self, system_prompt: &str, user_prompt: &str) -> Result<String, AxAgentError> {
        if let (Some(adapter), Some(ctx)) = (&self.adapter, &self.ctx) {
            let request = ChatRequest {
                model: self.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: ChatContent::Text(system_prompt.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: ChatContent::Text(user_prompt.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                stream: false,
                temperature: Some(0.7),
                max_tokens: Some(1024),
                top_p: None,
                tools: None,
                thinking_budget: None,
                use_max_completion_tokens: None,
                thinking_param_style: None,
                api_mode: None,
                instructions: None,
                conversation: None,
                previous_response_id: None,
                store: None,
            };

            let response = adapter
                .chat(ctx, request)
                .await
                .map_err(|e| AxAgentError::Provider(e.to_string()))?;

            Ok(response.content)
        } else {
            Ok(self.heuristic_response(user_prompt))
        }
    }

    fn heuristic_response(&self, prompt: &str) -> String {
        if prompt.contains("scale from 0.0 to 1.0") {
            "0.5".to_string()
        } else {
            "Exploring alternative reasoning path based on the given context.".to_string()
        }
    }
}

impl Default for DefaultToTReasoningProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmReasoningProvider for DefaultToTReasoningProvider {
    async fn think_branch(&self, prompt: &str) -> Result<String, AxAgentError> {
        let system_prompt = "You are a reasoning engine exploring multiple paths. Generate a distinct \
        reasoning step that takes a different approach or perspective from other branches. Be concise, \
        logical, and focused on making progress toward solving the problem.";

        match self.call_llm(system_prompt, prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => Ok(self.heuristic_response(prompt)),
        }
    }

    async fn evaluate_thought(&self, prompt: &str) -> Result<String, AxAgentError> {
        let system_prompt = "You are an evaluation engine. Score reasoning steps on a scale from 0.0 \
        to 1.0 based on logical soundness, relevance to the goal, and promise of leading to a correct \
        solution. Respond with ONLY a number between 0.0 and 1.0.";

        match self.call_llm(system_prompt, prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => Ok("0.5".to_string()),
        }
    }
}

pub struct ProviderAdapterBridge {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
}

impl ProviderAdapterBridge {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        Self { adapter, ctx, model }
    }
}

#[async_trait::async_trait]
impl LlmReasoningProvider for ProviderAdapterBridge {
    async fn think_branch(&self, prompt: &str) -> Result<String, AxAgentError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text("You are a reasoning engine exploring multiple paths. Generate a distinct reasoning step.".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(1024),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        match self.adapter.chat(&self.ctx, request).await {
            Ok(response) => Ok(response.content),
            Err(e) => Err(AxAgentError::Provider(e.to_string())),
        }
    }

    async fn evaluate_thought(&self, prompt: &str) -> Result<String, AxAgentError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text("You are an evaluation engine. Score reasoning steps on a scale from 0.0 to 1.0. Respond with ONLY a number.".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(64),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        match self.adapter.chat(&self.ctx, request).await {
            Ok(response) => Ok(response.content),
            Err(e) => Err(AxAgentError::Provider(e.to_string())),
        }
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = max_len.saturating_sub(3);
        format!("{}...", &s[..end.min(s.len())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thought_node_creation() {
        let node = ThoughtNode::new("n1".to_string(), "test content".to_string(), None);
        assert_eq!(node.id, "n1");
        assert_eq!(node.content, "test content");
        assert_eq!(node.evaluation_score, 0.0);
        assert!(node.children.is_empty());
        assert!(node.parent.is_none());
        assert_eq!(node.status, ThoughtStatus::Generated);
        assert!(node.tool_calls.is_empty());
        assert!(node.is_leaf());
    }

    #[test]
    fn test_thought_node_add_child() {
        let mut node = ThoughtNode::new("n1".to_string(), "parent".to_string(), None);
        node.add_child("n2".to_string());
        node.add_child("n3".to_string());
        assert_eq!(node.children.len(), 2);
        assert!(!node.is_leaf());
    }

    #[test]
    fn test_engine_creation() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        assert_eq!(engine.branching_factor, 3);
        assert_eq!(engine.max_depth, 5);
        assert_eq!(engine.evaluation_threshold, 0.3);
        assert_eq!(engine.total_nodes(), 1);
        assert_eq!(engine.root_id, "node_0");
    }

    #[test]
    fn test_engine_default_values() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        assert!(engine.get_node(&engine.root_id).is_some());
        let root = engine.get_node(&engine.root_id).unwrap();
        assert!(root.parent.is_none());
        assert!(root.children.is_empty());
    }

    #[test]
    fn test_get_node_exists_and_missing() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        assert!(engine.get_node("node_0").is_some());
        assert!(engine.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_get_path_to_root() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let child_id = engine.next_node_id();
        let child = ThoughtNode::new(child_id.clone(), "child".to_string(), Some(root_id.clone()));
        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);

        let path = engine.get_path_to_root(&child_id);
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], child_id);
        assert_eq!(path[1], root_id);
    }

    #[test]
    fn test_get_depth() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        assert_eq!(engine.get_depth(&root_id), 0);

        let child_id = engine.next_node_id();
        let child = ThoughtNode::new(child_id.clone(), "child".to_string(), Some(root_id.clone()));
        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);

        assert_eq!(engine.get_depth(&child_id), 1);

        let grandchild_id = engine.next_node_id();
        let grandchild = ThoughtNode::new(
            grandchild_id.clone(),
            "grandchild".to_string(),
            Some(child_id.clone()),
        );
        if let Some(node) = engine.tree.get_mut(&child_id) {
            node.add_child(grandchild_id.clone());
        }
        engine.tree.insert(grandchild_id.clone(), grandchild);

        assert_eq!(engine.get_depth(&grandchild_id), 2);
    }

    #[test]
    fn test_select_best_path_single_root() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let path = engine.select_best_path();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], engine.root_id);
    }

    #[test]
    fn test_select_best_path_with_scored_children() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let child1_id = engine.next_node_id();
        let mut child1 = ThoughtNode::new(child1_id.clone(), "weak branch".to_string(), Some(root_id.clone()));
        child1.evaluation_score = 0.3;
        child1.status = ThoughtStatus::Explored;

        let child2_id = engine.next_node_id();
        let mut child2 = ThoughtNode::new(child2_id.clone(), "strong branch".to_string(), Some(root_id.clone()));
        child2.evaluation_score = 0.9;
        child2.status = ThoughtStatus::Explored;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child1_id.clone());
            root.add_child(child2_id.clone());
        }
        engine.tree.insert(child1_id, child1);
        engine.tree.insert(child2_id.clone(), child2);

        let path = engine.select_best_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], root_id);
        assert_eq!(path[1], child2_id);
    }

    #[test]
    fn test_select_best_path_skips_pruned() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let pruned_id = engine.next_node_id();
        let mut pruned = ThoughtNode::new(pruned_id.clone(), "pruned".to_string(), Some(root_id.clone()));
        pruned.status = ThoughtStatus::Pruned;
        pruned.evaluation_score = 0.1;

        let good_id = engine.next_node_id();
        let mut good = ThoughtNode::new(good_id.clone(), "good".to_string(), Some(root_id.clone()));
        good.status = ThoughtStatus::Explored;
        good.evaluation_score = 0.7;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(pruned_id.clone());
            root.add_child(good_id.clone());
        }
        engine.tree.insert(pruned_id, pruned);
        engine.tree.insert(good_id.clone(), good);

        let path = engine.select_best_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[1], good_id);
    }

    #[test]
    fn test_prune_below_threshold() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let low_id = engine.next_node_id();
        let mut low = ThoughtNode::new(low_id.clone(), "low score".to_string(), Some(root_id.clone()));
        low.evaluation_score = 0.2;
        low.status = ThoughtStatus::Generated;

        let high_id = engine.next_node_id();
        let mut high = ThoughtNode::new(high_id.clone(), "high score".to_string(), Some(root_id.clone()));
        high.evaluation_score = 0.8;
        high.status = ThoughtStatus::Generated;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(low_id.clone());
            root.add_child(high_id.clone());
        }
        engine.tree.insert(low_id.clone(), low);
        engine.tree.insert(high_id.clone(), high);

        let pruned = engine.prune_below_threshold(0.5);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0], low_id);

        assert_eq!(engine.tree.get(&low_id).unwrap().status, ThoughtStatus::Pruned);
        assert_eq!(engine.tree.get(&high_id).unwrap().status, ThoughtStatus::Generated);
    }

    #[test]
    fn test_prune_does_not_affect_explored_nodes() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let low_explored_id = engine.next_node_id();
        let mut low_explored = ThoughtNode::new(low_explored_id.clone(), "low but explored".to_string(), Some(root_id.clone()));
        low_explored.evaluation_score = 0.1;
        low_explored.status = ThoughtStatus::Explored;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(low_explored_id.clone());
        }
        engine.tree.insert(low_explored_id.clone(), low_explored);

        let pruned = engine.prune_below_threshold(0.5);
        assert!(pruned.is_empty());
        assert_eq!(
            engine.tree.get(&low_explored_id).unwrap().status,
            ThoughtStatus::Explored
        );
    }

    #[test]
    fn test_backtrack_to_existing() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let child_id = engine.next_node_id();
        let mut child = ThoughtNode::new(child_id.clone(), "child".to_string(), Some(root_id.clone()));
        child.evaluation_score = 0.5;
        child.status = ThoughtStatus::Explored;

        let grandchild_id = engine.next_node_id();
        let grandchild = ThoughtNode::new(grandchild_id.clone(), "grandchild".to_string(), Some(child_id.clone()));

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child_id.clone());
        }
        if let Some(node) = engine.tree.get_mut(&child_id) {
            node.add_child(grandchild_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);
        engine.tree.insert(grandchild_id.clone(), grandchild);

        assert_eq!(engine.total_nodes(), 4);

        let result = engine.backtrack_to(&child_id);
        assert!(result.is_ok());
        assert_eq!(engine.total_nodes(), 2);
        assert!(engine.tree.get(&child_id).unwrap().children.is_empty());
        assert_eq!(engine.tree.get(&child_id).unwrap().status, ThoughtStatus::Explored);
    }

    #[test]
    fn test_backtrack_to_nonexistent() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let result = engine.backtrack_to("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_state() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let child_id = engine.next_node_id();
        let mut child = ThoughtNode::new(child_id.clone(), "child content".to_string(), Some(root_id.clone()));
        child.evaluation_score = 0.7;
        child.status = ThoughtStatus::Explored;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);

        let state = engine.get_current_state();
        assert_eq!(state.root_id, root_id);
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.edges.len(), 1);
        assert_eq!(state.edges[0].from, root_id);
        assert_eq!(state.edges[0].to, child_id);
    }

    #[test]
    fn test_add_tool_result() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        engine.add_tool_result(
            &root_id,
            "search".to_string(),
            "found results".to_string(),
            false,
        );

        let node = engine.get_node(&root_id).unwrap();
        assert_eq!(node.tool_calls.len(), 1);
        assert_eq!(node.tool_calls[0].tool_name, "search");
        assert!(!node.tool_calls[0].is_error);
    }

    #[test]
    fn test_mark_node_selected() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        engine.mark_node_selected(&root_id);
        assert_eq!(engine.get_node(&root_id).unwrap().status, ThoughtStatus::Selected);
    }

    #[test]
    fn test_total_explored_pruned_counts() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let explored_id = engine.next_node_id();
        let mut explored = ThoughtNode::new(explored_id.clone(), "explored".to_string(), Some(root_id.clone()));
        explored.status = ThoughtStatus::Explored;

        let pruned_id = engine.next_node_id();
        let mut pruned = ThoughtNode::new(pruned_id.clone(), "pruned".to_string(), Some(root_id.clone()));
        pruned.status = ThoughtStatus::Pruned;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(explored_id.clone());
            root.add_child(pruned_id.clone());
        }
        engine.tree.insert(explored_id, explored);
        engine.tree.insert(pruned_id, pruned);

        assert_eq!(engine.total_nodes(), 3);
        assert_eq!(engine.explored_nodes(), 1);
        assert_eq!(engine.pruned_nodes(), 1);
    }

    #[test]
    fn test_get_leaves() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let leaf1_id = engine.next_node_id();
        let leaf1 = ThoughtNode::new(leaf1_id.clone(), "leaf1".to_string(), Some(root_id.clone()));

        let leaf2_id = engine.next_node_id();
        let mut leaf2 = ThoughtNode::new(leaf2_id.clone(), "leaf2".to_string(), Some(root_id.clone()));
        leaf2.status = ThoughtStatus::Pruned;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(leaf1_id.clone());
            root.add_child(leaf2_id.clone());
        }
        engine.tree.insert(leaf1_id.clone(), leaf1);
        engine.tree.insert(leaf2_id.clone(), leaf2);

        let leaves = engine.get_leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0], leaf1_id);
    }

    #[test]
    fn test_heuristic_evaluate() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let mut node = ThoughtNode::new("n1".to_string(), "".to_string(), None);
        node.content = "This is a short thought".to_string();
        let score = engine.heuristic_evaluate(&node);
        assert!(score >= 0.0 && score <= 1.0);

        node.content = "First, we analyze the problem because it is important. \
            Therefore, we must consider all factors. However, next we should verify our approach."
            .to_string();
        let score2 = engine.heuristic_evaluate(&node);
        assert!(score2 >= 0.0 && score2 <= 1.0);
        assert!(score2 > score);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    #[test]
    fn test_select_best_leaf() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let child_id = engine.next_node_id();
        let mut child = ThoughtNode::new(child_id.clone(), "child".to_string(), Some(root_id.clone()));
        child.evaluation_score = 0.8;
        child.status = ThoughtStatus::Explored;

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(child_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);

        let leaf = engine.select_best_leaf();
        assert!(leaf.is_some());
        assert_eq!(leaf.unwrap(), child_id);
    }

    #[test]
    fn test_default_tot_reasoning_provider() {
        let provider = DefaultToTReasoningProvider::new();
        assert_eq!(provider.model, "gpt-4o");
        assert!(provider.adapter.is_none());
    }

    #[test]
    fn test_heuristic_response_thinking() {
        let provider = DefaultToTReasoningProvider::new();
        let result = provider.heuristic_response("generate reasoning step");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_heuristic_response_evaluation() {
        let provider = DefaultToTReasoningProvider::new();
        let result = provider.heuristic_response("scale from 0.0 to 1.0");
        assert_eq!(result, "0.5");
    }

    #[test]
    fn test_collect_leaves() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let leaf1_id = engine.next_node_id();
        let leaf1 = ThoughtNode::new(leaf1_id.clone(), "leaf1".to_string(), Some(root_id.clone()));

        let child_id = engine.next_node_id();
        let mut child = ThoughtNode::new(child_id.clone(), "child".to_string(), Some(root_id.clone()));

        let leaf2_id = engine.next_node_id();
        let leaf2 = ThoughtNode::new(leaf2_id.clone(), "leaf2".to_string(), Some(child_id.clone()));

        if let Some(root) = engine.tree.get_mut(&root_id) {
            root.add_child(leaf1_id.clone());
            root.add_child(child_id.clone());
        }
        if let Some(node) = engine.tree.get_mut(&child_id) {
            node.add_child(leaf2_id.clone());
        }
        engine.tree.insert(leaf1_id.clone(), leaf1);
        engine.tree.insert(child_id.clone(), child);
        engine.tree.insert(leaf2_id.clone(), leaf2);

        let leaves = engine.collect_leaves(&root_id);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&leaf1_id));
        assert!(leaves.contains(&leaf2_id));
    }

    #[test]
    fn test_generate_branching_max_depth() {
        let mut engine = TreeOfThoughtsEngine::new(3, 1, 0.3);

        let child_id = engine.next_node_id();
        let child = ThoughtNode::new(child_id.clone(), "child at max".to_string(), Some(engine.root_id.clone()));
        if let Some(root) = engine.tree.get_mut(&engine.root_id) {
            root.add_child(child_id.clone());
        }
        engine.tree.insert(child_id.clone(), child);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let provider: Arc<dyn LlmReasoningProvider> = Arc::new(DefaultToTReasoningProvider::new());
        let result = rt.block_on(engine.generate_branching_options(child_id, "test", &provider));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_generate_branching_pruned_parent() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let mut root = engine.tree.get_mut(&root_id).unwrap();
        root.status = ThoughtStatus::Pruned;

        let provider: Arc<dyn LlmReasoningProvider> = Arc::new(DefaultToTReasoningProvider::new());
        let result = engine.generate_branching_options(root_id, "test", &provider).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_thought_nonexistent_node() {
        let engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let provider: Arc<dyn LlmReasoningProvider> = Arc::new(DefaultToTReasoningProvider::new());
        let result = engine.evaluate_thought("nonexistent", "test", &provider).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_thought_status_serialization() {
        let status = ThoughtStatus::Generated;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ThoughtStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ThoughtStatus::Generated);
    }

    #[test]
    fn test_thought_node_serialization() {
        let mut node = ThoughtNode::new("n1".to_string(), "content".to_string(), None);
        node.evaluation_score = 0.75;
        node.status = ThoughtStatus::Explored;
        node.add_child("n2".to_string());

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: ThoughtNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "n1");
        assert_eq!(deserialized.evaluation_score, 0.75);
        assert_eq!(deserialized.status, ThoughtStatus::Explored);
        assert_eq!(deserialized.children.len(), 1);
    }

    #[test]
    fn test_tot_state_summary_serialization() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let state = engine.get_current_state();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ToTStateSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.root_id, engine.root_id);
    }

    #[test]
    fn test_tool_call_result_serialization() {
        let result = ToolCallResult {
            tool_name: "search".to_string(),
            output: "found data".to_string(),
            is_error: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ToolCallResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_name, "search");
        assert!(!deserialized.is_error);
    }

    #[tokio::test]
    async fn test_evaluate_and_score_node() {
        let mut engine = TreeOfThoughtsEngine::new(3, 5, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn LlmReasoningProvider> = Arc::new(DefaultToTReasoningProvider::new());
        let score = engine.evaluate_and_score_node(&root_id, "test context", &provider).await;
        assert!(score.is_ok());
        let s = score.unwrap();
        assert!(s >= 0.0 && s <= 1.0);

        let node = engine.get_node(&root_id).unwrap();
        assert_eq!(node.status, ThoughtStatus::Explored);
        assert!((node.evaluation_score - s).abs() < f64::EPSILON);
    }
}
