// SPDX-License-Identifier: AGPL-3.0-only

//! 动态子图生成
//!
//! 运行时根据分解计划生成 DAG 子图，供工作流引擎执行。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, MultiAgentNode, MultiAgentNodeConfig, OutputMode,
    Position, RetryConfig, SubGraph, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
};

use super::plan::{DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask};

// ── GeneratedSubGraph ─────────────────────────────────────────────────

/// 已生成的可执行子图
#[derive(Debug, Clone)]
pub struct GeneratedSubGraph {
    /// 子图标识符
    pub id: String,
    /// 所有 Agent 节点（每个子任务一个）
    pub nodes: Vec<WorkflowNode>,
    /// 依赖边
    pub edges: Vec<WorkflowEdge>,
    /// 生成此子图的原始分解计划
    pub plan: DecompositionPlan,
}

impl GeneratedSubGraph {
    /// 查找匹配子任务 ID 的 Agent 节点
    pub fn node_for_sub_task(&self, sub_task_id: &str) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| n.base().id == sub_task_id)
    }

    /// 获取提交给工作流引擎的 Workflow
    pub fn to_workflow(&self) -> SubGraph {
        SubGraph { nodes: self.nodes.clone(), edges: self.edges.clone() }
    }
}

// ── DynamicSubGraph ───────────────────────────────────────────────────

/// 运行时从分解计划构建可执行 DAG 子图
pub struct DynamicSubGraph {
    /// 唯一节点/边 ID 计数器
    counter: u64,
}

impl DynamicSubGraph {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// 从分解计划生成子图
    pub fn generate(
        &mut self,
        plan: &DecompositionPlan,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        let nodes = plan.sub_tasks.iter().map(|st| self.build_node(st)).collect::<Vec<_>>();
        let edges = self.build_edges(plan, &nodes)?;
        self.validate(&nodes, &edges)?;

        let id =
            format!("orchestrator_subgraph_{}", uuid::Uuid::new_v4().to_string().replace('-', "_"));

        Ok(GeneratedSubGraph { id, nodes, edges, plan: plan.clone() })
    }

    /// 为子任务构建节点
    fn build_node(&mut self, sub_task: &SubTask) -> WorkflowNode {
        if sub_task.multi_agent {
            self.build_multi_agent_node(sub_task)
        } else {
            self.build_single_agent_node(sub_task)
        }
    }

    /// 为子任务构建单 Agent 节点
    fn build_single_agent_node(&mut self, sub_task: &SubTask) -> WorkflowNode {
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
            tools: sub_task.tools.clone(),
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

    /// 为子任务构建多 Agent 节点
    fn build_multi_agent_node(&mut self, sub_task: &SubTask) -> WorkflowNode {
        self.counter += 1;

        let base = WorkflowNodeBase {
            id: sub_task.id.clone(),
            title: format!("[多智能体] {}", sub_task.name),
            description: Some(sub_task.description.clone()),
            position: Position::default(),
            retry: RetryConfig {
                enabled: true,
                max_retries: sub_task.max_retries,
                ..Default::default()
            },
            timeout: Some(900),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        };

        let mode = sub_task.coordination_mode.clone().unwrap_or_else(|| "swarm".to_string());

        let config = MultiAgentNodeConfig {
            task: sub_task.description.clone(),
            role: Some(sub_task.role.clone()),
            model: None,
            output_var: sub_task.output_var.clone(),
            mode,
            max_rounds: sub_task.max_rounds,
            input_mapping: None,
        };

        WorkflowNode::MultiAgent(MultiAgentNode { base, config })
    }

    /// 根据策略和声明的依赖构建边
    fn build_edges(
        &mut self,
        plan: &DecompositionPlan,
        nodes: &[WorkflowNode],
    ) -> Result<Vec<WorkflowEdge>, OrchestrationError> {
        let mut edges = Vec::new();

        // 从 sub_task.dependencies 构建显式依赖边
        for sub_task in &plan.sub_tasks {
            for dep_id in &sub_task.dependencies {
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

        // 当没有显式依赖时，应用策略级拓扑
        match plan.strategy {
            OrchestrationStrategy::Ordered | OrchestrationStrategy::Pipeline => {
                let mut prev_id: Option<&str> = None;
                for st in &plan.sub_tasks {
                    if let Some(prev) = prev_id {
                        let has_incoming = edges.iter().any(|e| e.target == st.id);
                        if !has_incoming {
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
                // 所有节点独立，不添加隐式边
            },
            OrchestrationStrategy::Debate => {
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
                // LLM 应已填充依赖关系；仅做验证
            },
        }

        Ok(edges)
    }

    /// 验证子图正确性：
    /// 1. 无环（Kahn 算法）
    /// 2. 无孤立节点
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
            in_degree.entry(edge.source.as_str()).or_insert(0);
            adjacency.entry(edge.target.as_str()).or_default();
        }

        // Kahn 算法检测环 + 拓扑排序
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

        // 检查孤立节点
        let has_incoming: HashSet<&str> = edges.iter().map(|e| e.target.as_str()).collect();
        let has_outgoing: HashSet<&str> = edges.iter().map(|e| e.source.as_str()).collect();
        let all_connected: HashSet<&str> = has_incoming.union(&has_outgoing).copied().collect();

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

    /// 根据 Agent 角色获取默认系统提示词
    fn default_system_prompt(role: &str, task_description: &str) -> String {
        let role_desc = match role {
            "researcher" => {
                "You are a research analyst. Gather information, analyze data, and produce structured findings."
            },
            "planner" => {
                "You are a planning specialist. Break down complex problems into actionable steps."
            },
            "developer" => "You are a software developer. Write, modify, and test code changes.",
            "reviewer" => {
                "You are a code reviewer. Evaluate quality, correctness, and adherence to standards."
            },
            "synthesizer" => {
                "You are a synthesis specialist. Combine multiple inputs into a cohesive output."
            },
            "executor" => {
                "You are an execution specialist. Carry out defined tasks precisely and report results."
            },
            "coordinator" => {
                "You are a coordinator. Manage dependencies and handovers between tasks."
            },
            "browser" => {
                "You are a web research agent. Find, extract, and summarize information from the web."
            },
            _ => {
                "You are an AI agent. Complete the task described below to the best of your ability."
            },
        };

        format!(
            "{}\n\n## Task\n{}\n\n## Instructions\n1. Complete the task described above.\n2. After completion, produce a structured handover.\n3. Output your result in the designated output variable.",
            role_desc, task_description
        )
    }
}

impl Default for DynamicSubGraph {
    fn default() -> Self {
        Self::new()
    }
}
