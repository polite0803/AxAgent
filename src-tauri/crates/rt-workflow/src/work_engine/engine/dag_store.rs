// SPDX-License-Identifier: AGPL-3.0-only

//! DAG store — graph creation, dependency analysis, ready-node computation,
//! and condition-branch skipping.

use std::collections::{HashMap, HashSet};

use axagent_core::workflow_types::{EdgeType, WorkflowEdge, WorkflowNode};

use crate::workflow_engine::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, current_timestamp,
};

use super::WorkEngine;

/// Test helper: build a minimal Tool‑variant WorkflowNode with the given id & enabled flag.
#[cfg(test)]
fn make_tool_node(id: &str, enabled: bool) -> WorkflowNode {
    use axagent_harness::workflow_types::{
        Position, RetryConfig, ToolNode, ToolNodeConfig, WorkflowNodeBase,
    };
    WorkflowNode::Tool(ToolNode {
        base: WorkflowNodeBase {
            id: id.to_string(),
            title: format!("Tool {id}"),
            description: None,
            position: Position { x: 0.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled,
            parent_id: None,
            compensation: None,
        },
        config: ToolNodeConfig {
            tool_name: format!("tool_{id}"),
            input_mapping: HashMap::new(),
            output_var: format!("out_{id}"),
        },
    })
}

/// Test helper: build a WorkflowEdge with default edge type.
#[cfg(test)]
fn make_edge(source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("edge_{source}_{target}"),
        source: source.to_string(),
        target: target.to_string(),
        edge_type: EdgeType::Default,
        source_handle: None,
        source_port_id: None,
        target_port_id: None,
        source_port_type: None,
        target_port_type: None,
        animated: false,
        style: None,
    }
}

/// Build a simple test Workflow from nodes and edges.
#[cfg(test)]
fn make_workflow(id: &str, nodes: Vec<WorkflowNode>, edges: Vec<WorkflowEdge>) -> Workflow {
    let node_states: HashMap<String, NodeRuntimeState> = nodes
        .iter()
        .map(|n| (n.base_id().to_string(), NodeRuntimeState::default()))
        .collect();
    Workflow {
        id: id.to_string(),
        name: "test".to_string(),
        nodes,
        edges,
        status: crate::workflow_engine::WorkflowStatus::Created,
        created_at: 0,
        completed_at: None,
        results: HashMap::new(),
        node_states,
        output: None,
        error_config: None,
        error_workflow_id: None,
    }
}

impl WorkEngine {
    /// 根据 edges 构建邻接表，返回就绪节点（入度为 0 的节点）。
    pub(crate) fn compute_ready_nodes(workflow: &Workflow) -> Vec<String> {
        let done_or_skipped: HashSet<&str> = workflow
            .node_states
            .iter()
            .filter(|(_, s)| matches!(s.status, NodeStatus::Completed | NodeStatus::Skipped))
            .map(|(id, _)| id.as_str())
            .collect();

        // 计算每个未完成节点的"未完成依赖数"
        let mut remaining_deps: HashMap<&str, usize> = HashMap::new();
        for node in &workflow.nodes {
            remaining_deps.entry(node.base_id()).or_insert(0);
        }
        for edge in &workflow.edges {
            // source 未完成 → target 有未满足的依赖
            if !done_or_skipped.contains(edge.source.as_str()) {
                *remaining_deps.entry(edge.target.as_str()).or_insert(0) += 1;
                continue;
            }

            // ConditionTrue/ConditionFalse 边：根据 condition 节点的输出决定是否激活
            if edge.edge_type == EdgeType::ConditionTrue
                || edge.edge_type == EdgeType::ConditionFalse
            {
                let cond_output = workflow.results.get(edge.source.as_str());
                let result = cond_output
                    .and_then(|o| o.get("result"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);
                // source_handle 回退到 edge_type：ConditionTrue → "true", ConditionFalse → "false"
                let branch = edge
                    .source_handle
                    .as_deref()
                    .unwrap_or(match edge.edge_type {
                        EdgeType::ConditionTrue => "true",
                        EdgeType::ConditionFalse => "false",
                        _ => "true",
                    });
                let should_follow = (branch == "true" && result) || (branch == "false" && !result);
                if !should_follow {
                    continue;
                }
            }

            // Switch 边：根据 switch 节点的 matched_label 决定是否激活
            let is_switch_source = workflow
                .nodes
                .iter()
                .any(|n| n.base_id() == edge.source && matches!(n, WorkflowNode::Switch(_)));
            if is_switch_source {
                let switch_output = workflow.results.get(edge.source.as_str());
                let selected_case = switch_output
                    .and_then(|o| o.get("matched_label"))
                    .and_then(|v| v.as_str());
                if let Some(ref handle) = edge.source_handle
                    && selected_case.is_none_or(|case| case != handle.as_str())
                {
                    continue;
                }
            }
        }

        workflow
            .nodes
            .iter()
            .filter(|n| {
                let state = workflow.node_states.get(n.base_id());
                let is_pending = state
                    .is_none_or(|s| matches!(s.status, NodeStatus::Pending | NodeStatus::Ready));
                let deps_met = remaining_deps.get(n.base_id()).copied().unwrap_or(0) == 0;
                is_pending && deps_met && n.base_enabled()
            })
            .map(|n| n.base_id().to_string())
            .collect()
    }
}

// ── Condition 节点分支跳过辅助 ──

/// Condition 节点完成后，将不匹配分支上的所有下游节点标记为 Skipped。
pub(crate) fn skip_disabled_branch_nodes(
    workflow: &mut Workflow,
    edges: &[WorkflowEdge],
    cond_node_id: &str,
) {
    let cond_output = workflow.results.get(cond_node_id);
    let result = cond_output
        .and_then(|o| o.get("result"))
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    // 确定要跳过的分支：result==true → 跳过 "false" 分支；result==false → 跳过 "true" 分支
    let skip_branch = if result { "false" } else { "true" };

    for edge in edges {
        if edge.source != cond_node_id {
            continue;
        }
        if edge.edge_type != EdgeType::ConditionTrue && edge.edge_type != EdgeType::ConditionFalse {
            continue;
        }
        let actual_branch = edge
            .source_handle
            .as_deref()
            .unwrap_or(match edge.edge_type {
                EdgeType::ConditionTrue => "true",
                EdgeType::ConditionFalse => "false",
                _ => "true",
            });
        if actual_branch == skip_branch {
            mark_subtree_skipped(workflow, edges, &edge.target);
        }
    }
}

/// Build a simple test Workflow from nodes and edges.
#[cfg(test)]
fn make_workflow(id: &str, nodes: Vec<WorkflowNode>, edges: Vec<WorkflowEdge>) -> Workflow {
    let node_states: HashMap<String, NodeRuntimeState> = nodes
        .iter()
        .map(|n| (n.base_id().to_string(), NodeRuntimeState::default()))
        .collect();
    Workflow {
        id: id.to_string(),
        name: "test".to_string(),
        nodes,
        edges,
        status: crate::workflow_engine::WorkflowStatus::Created,
        created_at: 0,
        completed_at: None,
        results: HashMap::new(),
        node_states,
        output: None,
    }
}

/// 递归标记节点及其所有下游节点为 Skipped
pub(crate) fn mark_subtree_skipped(workflow: &mut Workflow, edges: &[WorkflowEdge], node_id: &str) {
    // 如果已经标记过（Completed/Failed/Skipped），不再递归
    if let Some(state) = workflow.node_states.get(node_id)
        && matches!(state.status, NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped)
    {
        return;
    }

    workflow
        .node_states
        .entry(node_id.to_string())
        .or_insert_with(|| NodeRuntimeState {
            status: NodeStatus::Skipped,
            attempts: 0,
            error: None,
            started_at: None,
            completed_at: Some(current_timestamp() as i64),
        })
        .status = NodeStatus::Skipped;

    // 递归跳过所有下游节点
    for edge in edges {
        if edge.source == node_id {
            mark_subtree_skipped(workflow, edges, &edge.target);
        }
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_node_all_ready() {
        let n = make_tool_node("a", true);
        let wf = make_workflow("wf1", vec![n], vec![]);
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["a"]);
    }

    #[test]
    fn linear_dag_initial_ready() {
        let a = make_tool_node("a", true);
        let b = make_tool_node("b", true);
        let c = make_tool_node("c", true);
        let edges = vec![make_edge("a", "b"), make_edge("b", "c")];
        let wf = make_workflow("wf2", vec![a, b, c], edges);
        let ready = WorkEngine::compute_ready_nodes(&wf);
        // Only "a" has no incoming edges
        assert_eq!(ready, vec!["a"]);
    }

    #[test]
    fn disabled_node_skipped() {
        let a = make_tool_node("a", false); // disabled
        let b = make_tool_node("b", true);
        let edges = vec![make_edge("a", "b")];
        let wf = make_workflow("wf3", vec![a, b], edges);
        let ready = WorkEngine::compute_ready_nodes(&wf);
        // "a" is disabled, "b" depends on "a" which is not done → no ready nodes
        assert!(ready.is_empty());
    }

    #[test]
    fn completed_node_unblocks_dependents() {
        let a = make_tool_node("a", true);
        let b = make_tool_node("b", true);
        let edges = vec![make_edge("a", "b")];
        let mut wf = make_workflow("wf4", vec![a, b], edges);
        // Mark "a" as completed
        wf.node_states.get_mut("a").unwrap().status = NodeStatus::Completed;
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["b"]);
    }

    #[test]
    fn diamond_dag_only_source_ready() {
        let a = make_tool_node("a", true);
        let b = make_tool_node("b", true);
        let c = make_tool_node("c", true);
        let d = make_tool_node("d", true);
        let edges = vec![
            make_edge("a", "b"),
            make_edge("a", "c"),
            make_edge("b", "d"),
            make_edge("c", "d"),
        ];
        let wf = make_workflow("wf5", vec![a, b, c, d], edges);
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["a"]);
    }
}
