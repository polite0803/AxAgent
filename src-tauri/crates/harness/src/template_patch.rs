// SPDX-License-Identifier: AGPL-3.0-only
//! TemplatePatch — 对 WorkflowTemplateData 的增量修改（diff + 应用）。
//!
//! # 设计目标
//!
//! 工作流优化（P1-2）和自动进化（P2）需要一种**增量修改**现有模板的机制：
//! trajectory 模块的 Optimizer 从运行轨迹分析得到优化建议（某节点可替换、
//! 可并行、可新增知识库检索等），转成 TemplatePatch，然后应用到模板上，
//! 版本号 +1。
//!
//! 比全量替换更轻量：
//! - 保留模板的元数据（名称、标签、图标、触发器等）
//! - 只改 nodes/edges 的增量部分
//! - 可审计（Patch 记录了具体的增删改操作）
//!
//! # Patch 操作
//!
//! | 操作 | 作用 |
//! |------|------|
//! | add_nodes | 新增节点（追加到 nodes 列表末尾） |
//! | remove_node_ids | 按 node_id 删除节点 |
//! | update_nodes | 按 node_id 替换节点（完整替换） |
//! | add_edges | 新增边 |
//! | remove_edge_ids | 按 edge_id 删除边 |
//!
//! 应用顺序：remove → update → add（先清理、再修改、最后新增，避免 ID 冲突）。
//! 删除节点时自动清理引用该节点的边。

use crate::workflow_types::{WorkflowEdge, WorkflowNode};
use serde::{Deserialize, Serialize};

/// 增量修改操作集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplatePatch {
    /// 新增节点
    #[serde(default)]
    pub add_nodes: Vec<WorkflowNode>,
    /// 删除节点（按 base.id 匹配）
    #[serde(default)]
    pub remove_node_ids: Vec<String>,
    /// 完整替换节点（按 base.id 匹配）
    #[serde(default)]
    pub update_nodes: Vec<WorkflowNode>,
    /// 新增边
    #[serde(default)]
    pub add_edges: Vec<WorkflowEdge>,
    /// 删除边（按 edge_id 匹配）
    #[serde(default)]
    pub remove_edge_ids: Vec<String>,
}

impl TemplatePatch {
    /// 是否为空 patch（什么都不做）。
    pub fn is_empty(&self) -> bool {
        self.add_nodes.is_empty()
            && self.remove_node_ids.is_empty()
            && self.update_nodes.is_empty()
            && self.add_edges.is_empty()
            && self.remove_edge_ids.is_empty()
    }

    /// 两个 patch 合并（self 先应用，other 后应用）。
    pub fn merge(self, other: TemplatePatch) -> TemplatePatch {
        TemplatePatch {
            add_nodes: [self.add_nodes, other.add_nodes].concat(),
            remove_node_ids: [self.remove_node_ids, other.remove_node_ids].concat(),
            update_nodes: [self.update_nodes, other.update_nodes].concat(),
            add_edges: [self.add_edges, other.add_edges].concat(),
            remove_edge_ids: [self.remove_edge_ids, other.remove_edge_ids].concat(),
        }
    }
}

/// 将 TemplatePatch 应用到 WorkflowTemplateData（workflow_types 版本，内存 DAG）。
///
/// 返回最终的 nodes 和 edges。版本号由调用方自行递增（这里只做 DAG 层面的修改）。
pub fn apply_template_patch(
    nodes: &[WorkflowNode],
    edges: &[WorkflowEdge],
    patch: &TemplatePatch,
) -> (Vec<WorkflowNode>, Vec<WorkflowEdge>) {
    if patch.is_empty() {
        return (nodes.to_vec(), edges.to_vec());
    }

    let mut nodes = nodes.to_vec();
    let mut edges = edges.to_vec();

    // 1. 删除节点 + 清理引用这些节点的边
    if !patch.remove_node_ids.is_empty() {
        let remove_ids: std::collections::HashSet<&String> = patch.remove_node_ids.iter().collect();

        nodes.retain(|n| {
            let id = node_id(n);
            !remove_ids.contains(&id)
        });

        // 同时清理引用这些节点的边
        edges.retain(|e| {
            let from = edge_from_id(e);
            let to = edge_to_id(e);
            !remove_ids.contains(&from) && !remove_ids.contains(&to)
        });
    }

    // 2. 删除边（按 edge_id）
    if !patch.remove_edge_ids.is_empty() {
        let remove_edge_ids: std::collections::HashSet<&String> =
            patch.remove_edge_ids.iter().collect();

        edges.retain(|e| {
            let eid = edge_id(e);
            !remove_edge_ids.contains(&eid)
        });
    }

    // 3. 更新节点（按 node_id 替换）
    if !patch.update_nodes.is_empty() {
        for update in &patch.update_nodes {
            let update_id = node_id(update);
            if let Some(pos) = nodes.iter().position(|n| node_id(n) == update_id) {
                nodes[pos] = update.clone();
            }
            // 找不到则跳过（不是 error，patch 里写了 update 但原模板没有这个节点）
        }
    }

    // 4. 新增节点
    nodes.extend(patch.add_nodes.iter().cloned());

    // 5. 新增边（自动去重——相同 from+to+type 的边只保留一条）
    let mut existing_edge_keys: std::collections::HashSet<(String, String)> =
        edges.iter().map(|e| (edge_from_id(e), edge_to_id(e))).collect();

    for new_edge in &patch.add_edges {
        let key = (edge_from_id(new_edge), edge_to_id(new_edge));
        if !existing_edge_keys.contains(&key) {
            existing_edge_keys.insert(key.clone());
            edges.push(new_edge.clone());
        }
    }

    (nodes, edges)
}

// ── 辅助函数：从 WorkflowNode / WorkflowEdge 提取关键 ID ───────────────────────

fn node_id(n: &WorkflowNode) -> String {
    n.base().id.clone()
}

fn edge_id(e: &WorkflowEdge) -> String {
    e.id.clone()
}

fn edge_from_id(e: &WorkflowEdge) -> String {
    e.source.clone()
}

fn edge_to_id(e: &WorkflowEdge) -> String {
    e.target.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_types::*;

    fn make_base(id: &str) -> WorkflowNodeBase {
        WorkflowNodeBase {
            id: id.to_string(),
            title: format!("node_{id}"),
            description: None,
            position: Position::default(),
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            continue_on_fail: false,
            compensation: None,
        }
    }

    fn make_node(id: &str) -> WorkflowNode {
        WorkflowNode::End(EndNode {
            base: make_base(id),
            config: EndNodeConfig { output_var: None },
        })
    }

    fn make_edge(id: &str, from: &str, to: &str) -> WorkflowEdge {
        WorkflowEdge {
            id: id.to_string(),
            source: from.to_string(),
            source_handle: None,
            target: to.to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        }
    }

    #[test]
    fn test_empty_patch_noop() {
        let nodes = vec![make_node("a")];
        let edges = vec![make_edge("e1", "a", "a")];
        let empty = TemplatePatch::default();

        let (n_out, e_out) = apply_template_patch(&nodes, &edges, &empty);
        assert_eq!(n_out.len(), 1);
        assert_eq!(e_out.len(), 1);
    }

    #[test]
    fn test_add_nodes_and_edges() {
        let patch = TemplatePatch {
            add_nodes: vec![make_node("b"), make_node("c")],
            add_edges: vec![make_edge("e1", "a", "b"), make_edge("e2", "b", "c")],
            ..Default::default()
        };

        let (n_out, e_out) = apply_template_patch(&[make_node("a")], &[], &patch);
        assert_eq!(n_out.len(), 3);
        assert_eq!(e_out.len(), 2);
    }

    #[test]
    fn test_remove_node_cleans_up_edges() {
        let patch = TemplatePatch { remove_node_ids: vec!["b".to_string()], ..Default::default() };

        let nodes = vec![make_node("a"), make_node("b"), make_node("c")];
        let edges =
            vec![make_edge("e1", "a", "b"), make_edge("e2", "b", "c"), make_edge("e3", "a", "c")];

        let (n_out, e_out) = apply_template_patch(&nodes, &edges, &patch);
        assert_eq!(n_out.len(), 2);
        assert_eq!(e_out.len(), 1);
        assert!(e_out.iter().any(|e| e.id == "e3"));
    }

    #[test]
    fn test_edge_dedup_on_add() {
        let edges = vec![make_edge("e1", "a", "b")];
        let patch =
            TemplatePatch { add_edges: vec![make_edge("e2", "a", "b")], ..Default::default() };

        let (_, e_out) = apply_template_patch(&[make_node("a"), make_node("b")], &edges, &patch);
        assert_eq!(e_out.len(), 1);
    }

    #[test]
    fn test_patch_merge() {
        let p1 = TemplatePatch { add_nodes: vec![make_node("x")], ..Default::default() };
        let p2 = TemplatePatch { add_nodes: vec![make_node("y")], ..Default::default() };

        let merged = p1.merge(p2);
        assert_eq!(merged.add_nodes.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        assert!(TemplatePatch::default().is_empty());
        assert!(
            !TemplatePatch { add_nodes: vec![make_node("a")], ..Default::default() }.is_empty()
        );
    }
}
