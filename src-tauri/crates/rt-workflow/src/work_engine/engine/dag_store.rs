// SPDX-License-Identifier: AGPL-3.0-only

//! DAG store — graph creation, dependency analysis, ready-node computation,
//! and condition-branch skipping.
//!
//! 阶段 B: Typestate 集成 — 添加类型安全的节点状态管理方法。

use std::collections::{HashMap, HashSet};

use axagent_harness::workflow_types::{EdgeType, WorkflowEdge, WorkflowNode};

use crate::workflow_engine::{NodeRuntimeState, NodeStatus, Workflow, current_timestamp};

use super::WorkEngine;
use super::node_state::{AnyNodeState, PendingNode, ReadyNode, RunningNode, restore_typestate};

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
            continue_on_fail: false,
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
        edge_type: EdgeType::Direct,
        source_handle: None,
        target_handle: None,
        label: None,
    }
}

/// Build a simple test Workflow from nodes and edges.
#[cfg(test)]
fn make_workflow(id: &str, nodes: Vec<WorkflowNode>, edges: Vec<WorkflowEdge>) -> Workflow {
    let node_states: HashMap<String, NodeRuntimeState> =
        nodes.iter().map(|n| (n.base_id().to_string(), NodeRuntimeState::default())).collect();
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
        // Loop 节点的 body_steps 由 LoopExecutor 通过 loop_body_dispatch 驱动，
        // 不参与 DAG 就绪调度（否则会被当孤立节点提前执行一次）。
        let loop_body_steps: HashSet<&str> = workflow
            .nodes
            .iter()
            .filter_map(|n| match n {
                WorkflowNode::Loop(l) => Some(l.config.body_steps.iter().map(|s| s.as_str())),
                _ => None,
            })
            .flatten()
            .collect();

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
                let branch = edge.source_handle.as_deref().unwrap_or(match edge.edge_type {
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
                let selected_case =
                    switch_output.and_then(|o| o.get("matched_label")).and_then(|v| v.as_str());
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
                is_pending && deps_met && n.base_enabled() && !loop_body_steps.contains(n.base_id())
            })
            .map(|n| n.base_id().to_string())
            .collect()
    }

    // ── Typestate 辅助方法（阶段 B） ──

    /// 从 Workflow 创建 Typestate 节点集合（用于渐进式迁移）
    ///
    /// 返回 HashMap<String, Box<dyn AnyNodeState>>，键为节点 ID，值为 Typestate 节点。
    /// 此方法从 `node_states` 恢复 Typestate，确保状态信息不丢失。
    pub(crate) fn to_typestate_map(workflow: &Workflow) -> HashMap<String, Box<dyn AnyNodeState>> {
        workflow
            .nodes
            .iter()
            .map(|node| {
                let node_id = node.base_id().to_string();
                let state = workflow.node_states.get(&node_id).cloned().unwrap_or_default();
                (node_id, restore_typestate(node.clone(), state))
            })
            .collect()
    }

    /// 从 Typestate 集合获取就绪节点列表（类型安全版本）
    ///
    /// 与 `compute_ready_nodes` 不同，此方法返回 `Vec<ReadyNode>`，
    /// 编译器确保只能对就绪状态的节点进行后续操作。
    pub(crate) fn compute_ready_nodes_typed(workflow: &Workflow) -> Vec<ReadyNode> {
        let loop_body_steps: HashSet<&str> = workflow
            .nodes
            .iter()
            .filter_map(|n| match n {
                WorkflowNode::Loop(l) => Some(l.config.body_steps.iter().map(|s| s.as_str())),
                _ => None,
            })
            .flatten()
            .collect();

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
                let branch = edge.source_handle.as_deref().unwrap_or(match edge.edge_type {
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
                let selected_case =
                    switch_output.and_then(|o| o.get("matched_label")).and_then(|v| v.as_str());
                if let Some(ref handle) = edge.source_handle
                    && selected_case.is_none_or(|case| case != handle.as_str())
                {
                    continue;
                }
            }
        }

        // 筛选就绪节点（Pending 状态且依赖满足）
        workflow
            .nodes
            .iter()
            .filter_map(|n| {
                let state = workflow.node_states.get(n.base_id());
                let is_pending = state
                    .is_none_or(|s| matches!(s.status, NodeStatus::Pending | NodeStatus::Ready));
                let deps_met = remaining_deps.get(n.base_id()).copied().unwrap_or(0) == 0;
                if is_pending
                    && deps_met
                    && n.base_enabled()
                    && !loop_body_steps.contains(n.base_id())
                {
                    // 转换为 Typestate Ready 节点
                    let pending = PendingNode::new(n.clone());
                    Some(pending.mark_ready())
                } else {
                    None
                }
            })
            .collect()
    }

    /// 将 Typestate 节点的状态同步回 Workflow.node_states
    ///
    /// 当 Typestate 状态转移完成后，需要将新的状态写回 Workflow，
    /// 以保持数据一致性。
    pub(crate) fn sync_typestate_to_workflow(
        workflow: &mut Workflow,
        typestate_map: &HashMap<String, Box<dyn AnyNodeState>>,
    ) {
        for (node_id, any_state) in typestate_map {
            if let Some(state) = workflow.node_states.get_mut(node_id) {
                *state = any_state.runtime_state().clone();
            } else {
                workflow.node_states.insert(node_id.clone(), any_state.runtime_state().clone());
            }
        }
    }

    /// 标记就绪节点为运行中（类型安全版本）
    ///
    /// 在节点开始执行前调用此方法，返回 `RunningNode`。
    /// 编译器确保只能对 `ReadyNode` 调用此方法。
    pub(crate) fn mark_ready_to_running(
        workflow: &mut Workflow,
        node_id: &str,
    ) -> Option<RunningNode> {
        let node = workflow.nodes.iter().find(|n| n.base_id() == node_id)?.clone();
        let state = workflow.node_states.get(node_id).cloned()?;

        // 只有 Ready 状态才能转换为 Running
        if !matches!(state.status, NodeStatus::Ready) {
            return None;
        }

        // 使用 Typestate 公共 API 进行状态转移
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();

        // 同步回 Workflow
        workflow.node_states.insert(node_id.to_string(), running.runtime_state().clone());

        Some(running)
    }

    /// 标记运行中节点为完成（类型安全版本）
    pub(crate) fn mark_running_to_completed(workflow: &mut Workflow, node_id: &str) -> bool {
        let Some(state) = workflow.node_states.get_mut(node_id) else {
            return false;
        };

        if !matches!(state.status, NodeStatus::Running) {
            return false;
        }

        state.status = NodeStatus::Completed;
        state.completed_at = Some(current_timestamp() as i64);
        state.attempts = 0;
        true
    }

    /// 标记运行中节点为失败（类型安全版本）
    pub(crate) fn mark_running_to_failed(
        workflow: &mut Workflow,
        node_id: &str,
        error: String,
    ) -> bool {
        let Some(state) = workflow.node_states.get_mut(node_id) else {
            return false;
        };

        if !matches!(state.status, NodeStatus::Running) {
            return false;
        }

        state.status = NodeStatus::Failed;
        state.error = Some(error);
        state.completed_at = Some(current_timestamp() as i64);
        state.attempts += 1;
        true
    }

    /// 标记就绪节点为跳过（类型安全版本）
    pub(crate) fn mark_ready_to_skipped(workflow: &mut Workflow, node_id: &str) -> bool {
        let Some(state) = workflow.node_states.get_mut(node_id) else {
            return false;
        };

        if !matches!(state.status, NodeStatus::Ready) {
            return false;
        }

        state.status = NodeStatus::Skipped;
        state.completed_at = Some(current_timestamp() as i64);
        true
    }

    /// 失败节点重试（回到就绪状态）
    pub(crate) fn mark_failed_to_ready(workflow: &mut Workflow, node_id: &str) -> bool {
        let Some(state) = workflow.node_states.get_mut(node_id) else {
            return false;
        };

        if !matches!(state.status, NodeStatus::Failed) {
            return false;
        }

        state.status = NodeStatus::Ready;
        state.error = None;
        state.completed_at = None;
        true
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
    let result =
        cond_output.and_then(|o| o.get("result")).and_then(|r| r.as_bool()).unwrap_or(false);

    // 确定要跳过的分支：result==true → 跳过 "false" 分支；result==false → 跳过 "true" 分支
    let skip_branch = if result { "false" } else { "true" };

    for edge in edges {
        if edge.source != cond_node_id {
            continue;
        }
        if edge.edge_type != EdgeType::ConditionTrue && edge.edge_type != EdgeType::ConditionFalse {
            continue;
        }
        let actual_branch = edge.source_handle.as_deref().unwrap_or(match edge.edge_type {
            EdgeType::ConditionTrue => "true",
            EdgeType::ConditionFalse => "false",
            _ => "true",
        });
        if actual_branch == skip_branch {
            mark_subtree_skipped(workflow, edges, &edge.target);
        }
    }
}

/// P2-20: 标记节点及其所有下游节点为 Skipped —— **改用显式栈迭代**避免栈溢出。
///
/// 之前的递归实现在深 DAG（如 500+ 节点的 financial pipeline）上会因为
/// Rust 默认栈大小（8MB）触发 stack overflow。改成 `Vec<String>` 显式栈：
/// - 每个节点 push 一次
/// - 弹栈时把每个未访问的下游 push 进去
/// - 遇到已 terminal 状态（Completed / Failed / Skipped）就跳过
pub(crate) fn mark_subtree_skipped(workflow: &mut Workflow, edges: &[WorkflowEdge], root_id: &str) {
    let mut stack: Vec<String> = Vec::with_capacity(16);
    stack.push(root_id.to_string());

    // 防御性上限：单次 mark 最多处理 MAX_NODES 个节点，避免恶意/畸形 DAG 触发 DoS
    const MAX_NODES: usize = 10_000;
    let mut processed: usize = 0;

    while let Some(node_id) = stack.pop() {
        if processed >= MAX_NODES {
            tracing::error!(
                processed,
                "mark_subtree_skipped: 超过 MAX_NODES={MAX_NODES}，停止展开以防 DoS"
            );
            break;
        }
        processed += 1;

        // 已 terminal 状态 → 不再展开
        if let Some(state) = workflow.node_states.get(&node_id)
            && matches!(
                state.status,
                NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped
            )
        {
            continue;
        }

        workflow
            .node_states
            .entry(node_id.clone())
            .or_insert_with(|| NodeRuntimeState {
                status: NodeStatus::Skipped,
                attempts: 0,
                error: None,
                started_at: None,
                completed_at: Some(current_timestamp() as i64),
            })
            .status = NodeStatus::Skipped;

        // 把所有下游未访问节点 push 到栈上（继续展开）
        for edge in edges {
            if edge.source == node_id {
                let target = edge.target.clone();
                let already_terminal = workflow.node_states.get(&target).is_some_and(|s| {
                    matches!(
                        s.status,
                        NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped
                    )
                });
                if !already_terminal {
                    stack.push(target);
                }
            }
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
        wf.node_states.get_mut("a").expect("测试：键应存在").status = NodeStatus::Completed;
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

    // ── Loop body 节点过滤测试 ─────────────────────────────────────

    use axagent_harness::workflow_types::{
        LoopNode, LoopNodeConfig, LoopType, Position, RetryConfig, WorkflowNodeBase,
    };

    fn make_loop_node(id: &str, body_steps: Vec<String>) -> WorkflowNode {
        WorkflowNode::Loop(LoopNode {
            base: WorkflowNodeBase {
                id: id.to_string(),
                title: format!("Loop {id}"),
                description: None,
                position: Position { x: 0.0, y: 0.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: LoopNodeConfig {
                loop_type: LoopType::ForEach,
                items_var: None,
                iter_input_var: Some("items".to_string()),
                iteratee_var: Some("item".to_string()),
                iter_output_var: Some("iter_output".to_string()),
                partial_result_var: None,
                max_iterations: None,
                continue_condition: None,
                continue_on_error: false,
                body_steps,
                sub_graph: None,
                interrupt_after_each: false,
                interrupt_nodes: vec![],
            },
        })
    }

    #[test]
    fn loop_body_nodes_not_scheduled_by_dag() {
        // trigger → loop(body: [body1, body2]) → end
        // body 节点无入边，不应出现在就绪列表中
        let trigger = make_tool_node("t1", true);
        let body1 = make_tool_node("lc-draft-chapter", true);
        let body2 = make_tool_node("lc-polish", true);
        let loop_node =
            make_loop_node("loop1", vec!["lc-draft-chapter".to_string(), "lc-polish".to_string()]);
        let end_node = make_tool_node("end1", true);

        let edges = vec![make_edge("t1", "loop1"), make_edge("loop1", "end1")];
        let wf = make_workflow("wf_loop", vec![trigger, body1, body2, loop_node, end_node], edges);

        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["t1"], "只有 trigger 应就绪");
        assert!(
            !ready.contains(&"lc-draft-chapter".to_string()),
            "lc-draft-chapter 不应被 DAG 提前调度"
        );
        assert!(!ready.contains(&"lc-polish".to_string()), "lc-polish 不应被 DAG 提前调度");
    }

    #[test]
    fn normal_dag_unaffected_by_loop_body_filter() {
        let a = make_tool_node("a", true);
        let b = make_tool_node("b", true);
        let c = make_tool_node("c", true);
        let edges = vec![make_edge("a", "b"), make_edge("b", "c")];
        let wf = make_workflow("wf_normal", vec![a, b, c], edges);
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["a"], "普通 DAG 不受 Loop body 过滤影响");
    }

    #[test]
    fn loop_body_in_edges_still_filtered() {
        // trigger → loop → shared-body → end
        // shared-body 同时是 loop 的 body_step + 有入边
        let trigger = make_tool_node("t1", true);
        let body = make_tool_node("shared-body", true);
        let loop_node = make_loop_node("loop1", vec!["shared-body".to_string()]);
        let end_node = make_tool_node("end1", true);

        let edges = vec![
            make_edge("t1", "loop1"),
            make_edge("loop1", "shared-body"),
            make_edge("shared-body", "end1"),
        ];
        let wf = make_workflow("wf_shared", vec![trigger, body, loop_node, end_node], edges);

        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["t1"], "shared-body 作为 Loop body 不应被 DAG 调度");
        assert!(
            !ready.contains(&"shared-body".to_string()),
            "shared-body 即使有入边也不应被 DAG 提前调度"
        );
    }
}
