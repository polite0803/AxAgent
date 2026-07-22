// SPDX-License-Identifier: AGPL-3.0-only

//! DynamicSubGraph — runtime generation of DAG subgraphs from sub-task plans.
//!
//! Given a `DecompositionPlan` (list of `SubTask`s), this module
//! constructs concrete `WorkflowNode` and `WorkflowEdge` instances
//! that form a valid DAG executable by the work engine.
//!
//! Each sub-task becomes an Agent node with role-specific system prompt
//! and handover template. Edges encode dependency ordering.
//!
//! # Validation
//!
//! Generated graphs are checked for:
//! - No cycles (topological sort)
//! - No isolated nodes
//! - Valid dependency references

use std::collections::{HashMap, HashSet, VecDeque};

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, AgentRole, EdgeType, OutputMode, Position, RetryConfig, SubGraph,
    WorkflowEdge, WorkflowNode, WorkflowNodeBase,
};

use crate::types::{DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask};

// ── GeneratedSubGraph ─────────────────────────────────────────────────

/// A concrete subgraph ready for execution.
#[derive(Debug, Clone)]
pub struct GeneratedSubGraph {
    /// Subgraph identifier (derived from mission).
    pub id: String,
    /// All Agent nodes (one per sub-task).
    pub nodes: Vec<WorkflowNode>,
    /// Dependency edges.
    pub edges: Vec<WorkflowEdge>,
    /// The original decomposition plan this subgraph was generated from.
    pub plan: DecompositionPlan,
}

impl GeneratedSubGraph {
    /// Returns the Agent node matching a sub-task ID.
    pub fn node_for_sub_task(&self, sub_task_id: &str) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| n.base().id == sub_task_id)
    }

    /// Returns the Workflow struct for submission to the work engine.
    pub fn to_workflow(&self) -> SubGraph {
        SubGraph { nodes: self.nodes.clone(), edges: self.edges.clone() }
    }
}

// ── DynamicSubGraph ───────────────────────────────────────────────────

/// Builds executable DAG subgraphs from decomposition plans at runtime.
pub struct DynamicSubGraph {
    /// Counter for unique node/edge IDs.
    counter: u64,
}

impl DynamicSubGraph {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Generate a subgraph from a decomposition plan.
    ///
    /// Strategy affects edge topology:
    /// - Ordered / Pipeline: serial chain edges
    /// - FanOut: no edges between peer tasks (all independent)
    /// - Race: all tasks start together, converge on first completion
    /// - Dynamic: LLM is expected to provide edges; we validate and preserve
    pub fn generate(
        &mut self,
        plan: &DecompositionPlan,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        // 1. Build Agent nodes for each sub-task
        let nodes = plan.sub_tasks.iter().map(|st| self.build_agent_node(st)).collect::<Vec<_>>();

        // 2. Build edges based on strategy and declared dependencies
        let edges = self.build_edges(plan, &nodes)?;

        // 3. Validate the generated graph
        self.validate(&nodes, &edges)?;

        let id =
            format!("orchestrator_subgraph_{}", uuid::Uuid::new_v4().to_string().replace('-', "_"));

        Ok(GeneratedSubGraph { id, nodes, edges, plan: plan.clone() })
    }

    /// Build a single Agent node for a sub-task.
    fn build_agent_node(&mut self, sub_task: &SubTask) -> WorkflowNode {
        self.counter += 1;

        let base = WorkflowNodeBase {
            id: sub_task.id.clone(),
            title: sub_task.name.clone(),
            description: Some(sub_task.description.clone()),
            position: Position::default(),
            retry: RetryConfig {
                enabled: true,
                max_retries: sub_task.max_retries,
                ..Default::default()
            },
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        };

        let system_prompt = sub_task
            .system_prompt
            .clone()
            .unwrap_or_else(|| Self::default_system_prompt(&sub_task.role, &sub_task.description));

        let config = AgentNodeConfig {
            system_prompt,
            context_sources: vec!["orchestrator_context".to_string()],
            input_mapping: std::collections::HashMap::new(),
            output_var: sub_task.output_var.clone(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: sub_task.tools.clone(), // Propagate tools from sub-task decomposition
            exposed_tools: vec![],
            output_mode: OutputMode::Text,
            agent_profile_id: Some(sub_task.role.as_str().to_string()),
            max_tool_rounds: None,
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: None,
            consistency_check: None,
            hallucination_guard: None,
            fallback_model: None,
            task_scene: None,
            stream_chunk_timeout_secs: None,
        };

        WorkflowNode::Agent(AgentNode { base, config })
    }

    /// Build edges encoding dependency ordering.
    fn build_edges(
        &mut self,
        plan: &DecompositionPlan,
        nodes: &[WorkflowNode],
    ) -> Result<Vec<WorkflowEdge>, OrchestrationError> {
        let mut edges = Vec::new();

        // Build explicit dependency edges from sub_task.dependencies
        for sub_task in &plan.sub_tasks {
            for dep_id in &sub_task.dependencies {
                // Verify dependency node exists
                if !nodes.iter().any(|n| n.base().id == *dep_id) {
                    return Err(OrchestrationError::SubgraphGenerationFailed(format!(
                        "Dependency '{}' referenced by '{}' does not exist in subgraph",
                        dep_id, sub_task.id
                    )));
                }
                self.counter += 1;
                edges.push(WorkflowEdge {
                    id: format!("edge_{}", self.counter),
                    source: dep_id.clone(),
                    source_handle: None,
                    target: sub_task.id.clone(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        // Apply strategy-level topology when no explicit deps
        match plan.strategy {
            OrchestrationStrategy::Ordered | OrchestrationStrategy::Pipeline => {
                // Add implicit serial edges between sub-tasks that have no explicit deps
                let mut prev_id: Option<&str> = None;
                for st in &plan.sub_tasks {
                    if let Some(prev) = prev_id {
                        // Only add if no existing edge connects them
                        let already_connected =
                            edges.iter().any(|e| e.source == prev && e.target == st.id);
                        if !already_connected && st.dependencies.is_empty() {
                            self.counter += 1;
                            edges.push(WorkflowEdge {
                                id: format!("edge_{}", self.counter),
                                source: prev.to_string(),
                                source_handle: None,
                                target: st.id.clone(),
                                target_handle: None,
                                edge_type: EdgeType::Direct,
                                label: None,
                            });
                        }
                    }
                    prev_id = Some(&st.id);
                }
            },
            OrchestrationStrategy::FanOut | OrchestrationStrategy::Race => {
                // No implicit edges — all nodes are independent
            },
            OrchestrationStrategy::Debate => {
                // All nodes connect to a virtual adjudicator (last node)
                if plan.sub_tasks.len() >= 2 {
                    let last_idx = plan.sub_tasks.len() - 1;
                    let adjudicator_id = &plan.sub_tasks[last_idx].id;
                    for st in plan.sub_tasks.iter().take(last_idx) {
                        let already_connected =
                            edges.iter().any(|e| e.source == st.id && e.target == *adjudicator_id);
                        if !already_connected {
                            self.counter += 1;
                            edges.push(WorkflowEdge {
                                id: format!("edge_{}", self.counter),
                                source: st.id.clone(),
                                source_handle: None,
                                target: adjudicator_id.clone(),
                                target_handle: None,
                                edge_type: EdgeType::Grouping,
                                label: None,
                            });
                        }
                    }
                }
            },
            OrchestrationStrategy::Dynamic => {
                // LLM is expected to have populated dependencies; validate only
            },
        }

        Ok(edges)
    }

    /// Validate subgraph correctness:
    /// (1) No cycles (Kahn's algorithm)
    /// (2) No isolated nodes
    fn validate(
        &self,
        nodes: &[WorkflowNode],
        edges: &[WorkflowEdge],
    ) -> Result<(), OrchestrationError> {
        if nodes.is_empty() {
            return Err(OrchestrationError::SubgraphGenerationFailed(
                "Subgraph has no nodes".to_string(),
            ));
        }

        // Build adjacency list for cycle detection
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in nodes {
            let id = node.base().id.as_str();
            in_degree.entry(id).or_insert(0);
            adjacency.entry(id).or_default();
        }

        for edge in edges {
            *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
            adjacency.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
            // Ensure both ends are registered
            in_degree.entry(edge.source.as_str()).or_insert(0);
            adjacency.entry(edge.target.as_str()).or_default();
        }

        // Kahn's algorithm for cycle detection + topological order
        let mut queue: VecDeque<&str> =
            in_degree.iter().filter(|entry| *entry.1 == 0).map(|(&id, _)| id).collect();

        let mut sorted = Vec::new();
        while let Some(id) = queue.pop_front() {
            sorted.push(id);
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

        if sorted.len() != nodes.len() {
            return Err(OrchestrationError::SubgraphGenerationFailed(
                "Cycle detected in subgraph".to_string(),
            ));
        }

        // Check for isolated nodes (no incoming and no outgoing edges)
        let has_incoming: HashSet<&str> = edges.iter().map(|e| e.target.as_str()).collect();
        let has_outgoing: HashSet<&str> = edges.iter().map(|e| e.source.as_str()).collect();

        let all_connected: HashSet<&str> = has_incoming.union(&has_outgoing).copied().collect();

        // Isolation is allowed only if there's exactly one node (single-task)
        // Otherwise all nodes must have at least one connection
        if nodes.len() > 1 {
            let isolated: Vec<&str> = nodes
                .iter()
                .map(|n| n.base().id.as_str())
                .filter(|id| !all_connected.contains(id))
                .collect();

            if !isolated.is_empty() {
                return Err(OrchestrationError::SubgraphGenerationFailed(format!(
                    "Isolated nodes detected (no edges): {}",
                    isolated.join(", ")
                )));
            }
        }

        Ok(())
    }

    /// Default system prompt for a given Agent role.
    fn default_system_prompt(role: &AgentRole, task_description: &str) -> String {
        let role_desc = match role {
            AgentRole::Researcher => {
                "You are a research analyst. Gather information, analyze data, and produce structured findings."
            },
            AgentRole::Planner => {
                "You are a planning specialist. Break down complex problems into actionable steps."
            },
            AgentRole::Developer => {
                "You are a software developer. Write, modify, and test code changes."
            },
            AgentRole::Reviewer => {
                "You are a code reviewer. Evaluate quality, correctness, and adherence to standards."
            },
            AgentRole::Synthesizer => {
                "You are a synthesis specialist. Combine multiple inputs into a cohesive output."
            },
            AgentRole::Executor => {
                "You are an execution specialist. Carry out defined tasks precisely and report results."
            },
            AgentRole::Coordinator => {
                "You are a coordinator. Manage dependencies and handovers between tasks."
            },
            AgentRole::Browser => {
                "You are a web research agent. Find, extract, and summarize information from the web."
            },
        };

        format!(
            "{}\n\n## Task\n{}\n\n## Instructions\n1. Complete the task described above.\n2. After completion, produce a structured handover with:\n   - What you completed\n   - Files changed (if any)\n   - Next steps for the following agent\n   - Remaining issues or concerns\n   - Dependencies needed\n   - Validation evidence\n3. Output your result in the designated output variable.",
            role_desc, task_description
        )
    }
}

impl Default for DynamicSubGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SubTask;

    fn make_sub_task(id: &str, name: &str, desc: &str, deps: Vec<&str>) -> SubTask {
        let mut st =
            SubTask::new(id.to_string(), name.to_string(), desc.to_string(), AgentRole::Developer);
        st.dependencies = deps.into_iter().map(|s| s.to_string()).collect();
        st
    }

    #[test]
    fn test_generate_simple_subgraph() {
        let plan = DecompositionPlan {
            mission: "Test mission".to_string(),
            strategy: OrchestrationStrategy::Ordered,
            sub_tasks: vec![
                make_sub_task("task_1", "Analyze", "Analyze code", vec![]),
                make_sub_task("task_2", "Implement", "Implement feature", vec!["task_1"]),
            ],
            max_parallel: 2,
            max_replans: 2,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let mut dsg = DynamicSubGraph::new();
        let result = dsg.generate(&plan);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes.len(), 2);
        // task_2 depends on task_1 → should have one edge
        assert!(graph.edges.iter().any(|e| e.source == "task_1" && e.target == "task_2"));
    }

    #[test]
    fn test_cycle_detection() {
        let plan = DecompositionPlan {
            mission: "Cycle test".to_string(),
            strategy: OrchestrationStrategy::Ordered,
            sub_tasks: vec![
                make_sub_task("a", "A", "Task A", vec!["b"]),
                make_sub_task("b", "B", "Task B", vec!["a"]),
            ],
            max_parallel: 2,
            max_replans: 1,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let dsg = DynamicSubGraph::new();
        let nodes: Vec<_> = plan
            .sub_tasks
            .iter()
            .map(|st| {
                let base = WorkflowNodeBase {
                    id: st.id.clone(),
                    title: st.name.clone(),
                    description: Some(st.description.clone()),
                    position: Position::default(),
                    retry: RetryConfig::default(),
                    timeout: None,
                    enabled: true,
                    parent_id: None,
                    compensation: None,
                    continue_on_fail: false,
                };
                WorkflowNode::Agent(AgentNode {
                    base,
                    config: AgentNodeConfig {
                        system_prompt: "test".to_string(),
                        context_sources: vec![],
                        input_mapping: std::collections::HashMap::new(),
                        output_var: "out".to_string(),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        tools: vec![],
                        exposed_tools: vec![],
                        output_mode: OutputMode::Text,
                        agent_profile_id: None,
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: vec![],
                        model_role: None,
                        consistency_check: None,
                        hallucination_guard: None,
                        fallback_model: None,
                        task_scene: None,
                    },
                })
            })
            .collect();

        let edges = vec![
            WorkflowEdge {
                id: "e1".to_string(),
                source: "a".to_string(),
                source_handle: None,
                target: "b".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            },
            WorkflowEdge {
                id: "e2".to_string(),
                source: "b".to_string(),
                source_handle: None,
                target: "a".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            },
        ];

        let result = dsg.validate(&nodes, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle"));
    }

    #[test]
    fn test_isolated_node_detection() {
        let dsg = DynamicSubGraph::new();
        let nodes: Vec<_> = (1..=3)
            .map(|i| {
                WorkflowNode::Agent(AgentNode {
                    base: WorkflowNodeBase {
                        id: format!("n{}", i),
                        title: format!("Node {}", i),
                        description: None,
                        position: Position::default(),
                        retry: RetryConfig::default(),
                        timeout: None,
                        enabled: true,
                        parent_id: None,
                        compensation: None,
                        continue_on_fail: false,
                    },
                    config: AgentNodeConfig {
                        system_prompt: "test".to_string(),
                        context_sources: vec![],
                        input_mapping: std::collections::HashMap::new(),
                        output_var: "out".to_string(),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        tools: vec![],
                        exposed_tools: vec![],
                        output_mode: OutputMode::Text,
                        agent_profile_id: None,
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: vec![],
                        model_role: None,
                        consistency_check: None,
                        hallucination_guard: None,
                        fallback_model: None,
                        task_scene: None,
                    },
                })
            })
            .collect();

        // Edge only between n1→n2, n3 is isolated
        let edges = vec![WorkflowEdge {
            id: "e1".to_string(),
            source: "n1".to_string(),
            source_handle: None,
            target: "n2".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        }];

        let result = dsg.validate(&nodes, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Isolated"));
    }
}
