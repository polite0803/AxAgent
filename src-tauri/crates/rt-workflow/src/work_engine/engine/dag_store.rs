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
    ///
    /// 支持 `continue_on_fail` 容错机制：当上游节点 Failed 且下游节点
    /// 配置了 `continue_on_fail = true` 时，该依赖不阻塞下游节点执行。
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

        // 构建节点 continue_on_fail 索引（用于快速查找 target 是否允许容错）
        let continue_on_fail_map: HashMap<&str, bool> =
            workflow.nodes.iter().map(|n| (n.base_id(), n.base_continue_on_fail())).collect();

        // 构建 Failed 状态节点集合
        let failed_nodes: HashSet<&str> = workflow
            .node_states
            .iter()
            .filter(|(_, s)| matches!(s.status, NodeStatus::Failed))
            .map(|(id, _)| id.as_str())
            .collect();

        // ====== 第一遍扫描：标记"被激活控制边选中的 target" ======
        // 这些 target 已经被某个条件分支或 switch 路径"选中"，
        // 它们剩余的 Direct 边可能来自互斥路径（如 fallback 分支），不应阻塞。
        let mut selected_targets: HashSet<&str> = HashSet::new();
        for edge in &workflow.edges {
            let is_conditional = matches!(
                edge.edge_type,
                EdgeType::ConditionTrue | EdgeType::ConditionFalse
            );
            let is_switch = workflow
                .nodes
                .iter()
                .any(|n| n.base_id() == edge.source && matches!(n, WorkflowNode::Switch(_)));

            if !is_conditional && !is_switch {
                continue;
            }
            // source 还没完成 → 条件还不能判断，跳过
            if !done_or_skipped.contains(edge.source.as_str()) {
                continue;
            }

            let should_follow = if is_conditional {
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
                (branch == "true" && result) || (branch == "false" && !result)
            } else {
                // Switch 边
                let switch_output = workflow.results.get(edge.source.as_str());
                let matched = switch_output
                    .and_then(|o| o.get("matched_label"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("");
                let label = edge.source_handle.as_deref().unwrap_or("");
                label.is_empty() || matched == label
            };

            tracing::info!(
                "[compute_ready] 控制边 {}({:?}) → {} should_follow={}",
                edge.source,
                edge.edge_type,
                edge.target,
                should_follow,
            );

            if should_follow {
                selected_targets.insert(edge.target.as_str());
            }
        }

        // ====== 第二遍扫描：计算 remaining_deps，跳过已选中 target 的互斥 Direct 边 ======
        // 计算每个未完成节点的"未完成依赖数"
        let mut remaining_deps: HashMap<&str, usize> = HashMap::new();
        for node in &workflow.nodes {
            remaining_deps.entry(node.base_id()).or_insert(0);
        }
        for edge in &workflow.edges {
            // source 未完成 → 检查是否因容错而跳过
            if !done_or_skipped.contains(edge.source.as_str()) {
                // ========== 互斥路径短路 ==========
                // 如果 target 已经被某个激活的控制边选中，说明互斥路径上的
                // Direct 边（如 l1_fallback_normalize → call_l2）不应再阻塞。
                let target_id = edge.target.as_str();
                if selected_targets.contains(target_id)
                    && !matches!(
                        edge.edge_type,
                        EdgeType::ConditionTrue | EdgeType::ConditionFalse
                    )
                {
                    tracing::info!(
                        "[compute_ready] 互斥短路: {} → {} (target 已被控制边选中)",
                        edge.source,
                        target_id,
                    );
                    continue;
                }

                // 如果 source Failed 且 target 允许容错，则不计入依赖
                let source_failed = failed_nodes.contains(edge.source.as_str());
                let target_continue_on_fail =
                    continue_on_fail_map.get(target_id).copied().unwrap_or(false);

                if source_failed && target_continue_on_fail {
                    // 容错：上游失败不阻塞下游
                    tracing::debug!(
                        source = edge.source.as_str(),
                        target = target_id,
                        "continue_on_fail: 上游节点失败但下游允许容错，跳过依赖"
                    );
                } else {
                    *remaining_deps.entry(target_id).or_insert(0) += 1;
                }
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
                tracing::info!(
                    "[compute_ready] cond edge {}({:?}) → {} result={:?} branch={} should_follow={}",
                    edge.source,
                    edge.edge_type,
                    edge.target,
                    result,
                    branch,
                    should_follow,
                );
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

        let ready: Vec<String> = workflow
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
            .collect();

        // [DIAG] 当 ready 为空但存在 pending 节点时，打印 remaining_deps 精确诊断
        if ready.is_empty() {
            let pending_ids: Vec<String> = workflow
                .nodes
                .iter()
                .filter_map(|n| {
                    let st = workflow.node_states.get(n.base_id());
                    (st.is_none_or(|s| matches!(s.status, NodeStatus::Pending | NodeStatus::Ready)))
                        .then(|| n.base_id().to_string())
                })
                .collect();
            if !pending_ids.is_empty() {
                let deps_dump: Vec<String> = pending_ids
                    .iter()
                    .map(|id| {
                        let deps = remaining_deps.get(id.as_str()).copied().unwrap_or(0);
                        let node_state = workflow
                            .node_states
                            .get(id)
                            .map(|s| format!("{:?}", s.status))
                            .unwrap_or_else(|| "NO_STATE".to_string());
                        let in_edges: Vec<String> = workflow
                            .edges
                            .iter()
                            .filter(|e| e.target == *id)
                            .map(|e| {
                                let src_done = done_or_skipped.contains(e.source.as_str());
                                format!(
                                    "{}(src_done={}, type={:?})",
                                    e.source, src_done, e.edge_type
                                )
                            })
                            .collect();
                        format!(
                            "  {} state={} remaining={} in_edges=[{}]",
                            id,
                            node_state,
                            deps,
                            in_edges.join(", ")
                        )
                    })
                    .collect();
                tracing::warn!(
                    workflow_id = %workflow.id,
                    pending_nodes = ?pending_ids,
                    deps_dump = ?deps_dump,
                    "🔍 [DIAG] compute_ready_nodes 返回空 — 诊断 remaining_deps"
                );
            }
        }

        ready
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

        // ====== 第一遍扫描：标记"被激活控制边选中的 target" ======
        let mut selected_targets: HashSet<&str> = HashSet::new();
        for edge in &workflow.edges {
            let is_conditional = matches!(
                edge.edge_type,
                EdgeType::ConditionTrue | EdgeType::ConditionFalse
            );
            let is_switch = workflow
                .nodes
                .iter()
                .any(|n| n.base_id() == edge.source && matches!(n, WorkflowNode::Switch(_)));

            if !is_conditional && !is_switch {
                continue;
            }
            if !done_or_skipped.contains(edge.source.as_str()) {
                continue;
            }

            let should_follow = if is_conditional {
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
                (branch == "true" && result) || (branch == "false" && !result)
            } else {
                let switch_output = workflow.results.get(edge.source.as_str());
                let matched = switch_output
                    .and_then(|o| o.get("matched_label"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("");
                let label = edge.source_handle.as_deref().unwrap_or("");
                label.is_empty() || matched == label
            };

            if should_follow {
                selected_targets.insert(edge.target.as_str());
            }
        }

        // ====== 第二遍扫描：计算 remaining_deps，跳过已选中 target 的互斥 Direct 边 ======
        // 计算每个未完成节点的"未完成依赖数"
        let mut remaining_deps: HashMap<&str, usize> = HashMap::new();
        for node in &workflow.nodes {
            remaining_deps.entry(node.base_id()).or_insert(0);
        }
        for edge in &workflow.edges {
            if !done_or_skipped.contains(edge.source.as_str()) {
                // 互斥路径短路：target 已被激活的控制边选中 → 来自互斥分支的 Direct 边不再阻塞
                let target_id = edge.target.as_str();
                if selected_targets.contains(target_id)
                    && !matches!(
                        edge.edge_type,
                        EdgeType::ConditionTrue | EdgeType::ConditionFalse
                    )
                {
                    continue;
                }
                *remaining_deps.entry(target_id).or_insert(0) += 1;
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
        let prev_state = workflow.node_states.get(node_id).cloned()?;

        // 只有 Ready 状态才能转换为 Running
        if !matches!(prev_state.status, NodeStatus::Ready) {
            return None;
        }

        // 使用 Typestate 公共 API 进行状态转移
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();

        // 同步回 Workflow，保留旧状态的 attempts 等运行时计数
        // （Typestate 从 Pending 重建会把 runtime_state 重置为 default，
        // 丢掉 reset_node_for_retry 递增的 attempts，导致重试死循环）
        let mut new_state = running.runtime_state().clone();
        new_state.attempts = prev_state.attempts;
        new_state.error = None;
        workflow.node_states.insert(node_id.to_string(), new_state);

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
///
/// **汇合点保护（2026-08-28 修复）**：此前直接递归标记整棵下游子树，
/// 当分支在下游重新汇合时（如认知编排主 DAG 中 `call_l2` 同时有
/// `l1_low_conf(ConditionFalse)` 活跃边和 `l1_fallback_normalize` 跳过边），
/// 汇合点会被跳过分支连坐标记为 Skipped，导致活跃路径也中断。
///
/// 现改为两阶段算法：
///   1. 先收集被跳过分支的子树节点集合（不改状态）；
///   2. 不动点收敛：子树内节点若存在任一「活跃入边」（来自子树外的
///      follow 分支条件边 / 已完成或未决定状态的非条件上游），则视为
///      分支汇合点，移出跳过集合并级联重估其下游；
///   3. 最后统一把仍在集合内且处于 Pending/Ready 的节点标记 Skipped。
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
    let follow_branch = if result { "true" } else { "false" };

    // ── 阶段 1：收集跳过分支子树（不改状态，terminal 节点不展开）──
    let mut skip_set: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::with_capacity(16);
    for edge in edges {
        if edge.source != cond_node_id {
            continue;
        }
        if edge.edge_type != EdgeType::ConditionTrue && edge.edge_type != EdgeType::ConditionFalse {
            continue;
        }
        let actual_branch = edge.source_handle.as_deref().unwrap_or(match edge.edge_type {
            EdgeType::ConditionTrue => "true",
            _ => "false",
        });
        if actual_branch == skip_branch {
            stack.push(edge.target.clone());
        }
    }
    // 防御性上限：单次标记最多处理 MAX_NODES 个节点，避免恶意/畸形 DAG 触发 DoS
    const MAX_NODES: usize = 10_000;
    let mut processed: usize = 0;
    while let Some(node_id) = stack.pop() {
        if processed >= MAX_NODES {
            tracing::error!(
                processed,
                "skip_disabled_branch_nodes: 超过 MAX_NODES={MAX_NODES}，停止展开以防 DoS"
            );
            break;
        }
        processed += 1;
        // 已 terminal 状态 → 不收集不再展开
        if let Some(state) = workflow.node_states.get(&node_id)
            && matches!(
                state.status,
                NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped
            )
        {
            continue;
        }
        if !skip_set.insert(node_id.clone()) {
            continue;
        }
        for edge in edges {
            if edge.source == node_id {
                stack.push(edge.target.clone());
            }
        }
    }

    // ── 阶段 2：汇合点保护（不动点收敛）──
    loop {
        let mut changed = false;
        for node in skip_set.clone() {
            // 活跃入边：来自子树外、且不会被执行跳过的上游
            let has_active_incoming = edges.iter().any(|e| {
                if e.target != node {
                    return false;
                }
                if skip_set.contains(&e.source) {
                    return false;
                }
                match e.edge_type {
                    EdgeType::ConditionTrue | EdgeType::ConditionFalse => {
                        // 条件边：仅 follow 分支视为活跃
                        let branch = e.source_handle.as_deref().unwrap_or(match e.edge_type {
                            EdgeType::ConditionTrue => "true",
                            _ => "false",
                        });
                        branch == follow_branch
                    },
                    _ => {
                        // 非条件边：source 非 Skipped 即视为活跃
                        // （Completed 直连 / Pending 待执行，保守保护汇合点）
                        workflow
                            .node_states
                            .get(&e.source)
                            .map(|s| !matches!(s.status, NodeStatus::Skipped))
                            .unwrap_or(true)
                    },
                }
            });
            if has_active_incoming {
                skip_set.remove(&node);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ── 阶段 3：统一应用 Skipped（仅 Pending/Ready，不覆盖 terminal 状态）──
    for node_id in skip_set {
        let state = workflow
            .node_states
            .entry(node_id)
            .or_insert_with(|| NodeRuntimeState {
                status: NodeStatus::Skipped,
                attempts: 0,
                error: None,
                started_at: None,
                completed_at: Some(current_timestamp() as i64),
            });
        if matches!(state.status, NodeStatus::Pending | NodeStatus::Ready) {
            state.status = NodeStatus::Skipped;
            state.completed_at = Some(current_timestamp() as i64);
        }
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建一个支持 continue_on_fail 的 Tool 节点
    fn make_tool_node_with_failover(
        id: &str,
        enabled: bool,
        continue_on_fail: bool,
    ) -> WorkflowNode {
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
                continue_on_fail,
            },
            config: ToolNodeConfig {
                tool_name: format!("tool_{id}"),
                input_mapping: HashMap::new(),
                output_var: format!("out_{id}"),
            },
        })
    }

    #[test]
    fn continue_on_fail_unblocks_downstream() {
        // 上游节点 a Failed，下游节点 b 配置 continue_on_fail = true
        let a = make_tool_node("a", true);
        let b = make_tool_node_with_failover("b", true, true); // continue_on_fail = true
        let edges = vec![make_edge("a", "b")];
        let mut wf = make_workflow("wf_failover", vec![a, b], edges);

        // 标记 a 为 Failed
        wf.node_states.get_mut("a").unwrap().status = NodeStatus::Failed;

        // b 应该就绪，因为它允许容错
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["b"], "continue_on_fail=true 的节点应在上游失败时就绪");
    }

    #[test]
    fn continue_on_fail_false_blocks_downstream() {
        // 上游节点 a Failed，下游节点 b 配置 continue_on_fail = false
        let a = make_tool_node("a", true);
        let b = make_tool_node_with_failover("b", true, false); // continue_on_fail = false
        let edges = vec![make_edge("a", "b")];
        let mut wf = make_workflow("wf_no_failover", vec![a, b], edges);

        // 标记 a 为 Failed
        wf.node_states.get_mut("a").unwrap().status = NodeStatus::Failed;

        // b 不应该就绪，因为它不允许容错
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert!(ready.is_empty(), "continue_on_fail=false 的节点在上游失败时不应就绪");
    }

    #[test]
    fn multi_upstream_with_mixed_failover() {
        // 多个上游节点，部分 Failed，部分 Completed
        // 下游节点配置 continue_on_fail = true
        let a = make_tool_node("a", true);
        let b = make_tool_node("b", true);
        let c = make_tool_node_with_failover("c", true, true); // continue_on_fail = true
        let edges = vec![make_edge("a", "c"), make_edge("b", "c")];
        let mut wf = make_workflow("wf_multi", vec![a, b, c], edges);

        // 标记 a 为 Failed，b 为 Completed
        wf.node_states.get_mut("a").unwrap().status = NodeStatus::Failed;
        wf.node_states.get_mut("b").unwrap().status = NodeStatus::Completed;

        // c 应该就绪，因为它允许容错（虽然 a 失败了，但 b 完成了）
        let ready = WorkEngine::compute_ready_nodes(&wf);
        assert_eq!(ready, vec!["c"], "混合状态下 continue_on_fail=true 节点应就绪");
    }

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
