// SPDX-License-Identifier: AGPL-3.0-only
//! 工作流图谱 — 三层路由树的分析层 DAG 索引
//!
//! # 核心概念：执行层 vs 分析层
//! 现有引擎已具备执行层的子工作流嵌套能力，本模块聚焦于分析层的元数据索引：
//!
//! | 维度 | 执行层（现有能力） | 分析层（本模块） |
//! |------|-------------------|-----------------|
//! | 目标 | 运行时嵌套执行子工作流 | 设计时全局路径规划与影响分析 |
//! | 机制 | SubWorkflowNode + SubWorkflowExecutor | 元数据图谱 + 邻接表索引 |
//! | 示例 | 主工作流调用安全网关子工作流 | 路由时从"交易域"定位到"退款处理"路径 |
//!
//! # 核心价值
//! 图谱让路由模型输出确定性路径地址（如 `/trade/refund/auto`），
//! 而非自然语言描述，大幅降低 LLM 记忆负担和错判概率。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 路由层级枚举 ──────────────────────────────────

/// 路由层级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteLevel {
    /// L1: 域层（约10个，对应 CapabilityDomain）
    Domain,
    /// L2: 能力簇层（每域约20个）
    Cluster,
    /// L3: 具体工作流（每簇约10个）
    Workflow,
}

impl RouteLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouteLevel::Domain => "L1",
            RouteLevel::Cluster => "L2",
            RouteLevel::Workflow => "L3",
        }
    }
}

// ── 边类型枚举 ────────────────────────────────────

/// 图谱边类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// 层级关系：域 → 簇 → 工作流
    Hierarchy,
    /// 跳转关系：退款 → 补发优惠券
    Transition,
    /// 兜底关系：失败 → 转人工
    Fallback,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Hierarchy => "hierarchy",
            EdgeType::Transition => "transition",
            EdgeType::Fallback => "fallback",
        }
    }
}

// ── 图谱节点 ──────────────────────────────────────

/// 工作流图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphNode {
    /// 路径标识（确定性 ID），如 "finance/stock_analysis"
    pub path: String,
    /// 显示名称
    pub display_name: String,
    /// 层级
    pub level: RouteLevel,
    /// 业务域
    pub domain: String,
    /// 所属集群（L2/L3 节点有值）
    pub cluster: Option<String>,
    /// 关联的工作流 ID（仅 L3 节点）
    pub workflow_id: Option<String>,
    /// 是否激活
    pub is_active: bool,
}

impl WorkflowGraphNode {
    /// 创建 L1 域节点
    pub fn domain_node(domain: &str, display_name: &str) -> Self {
        Self {
            path: domain.to_string(),
            display_name: display_name.to_string(),
            level: RouteLevel::Domain,
            domain: domain.to_string(),
            cluster: None,
            workflow_id: None,
            is_active: true,
        }
    }

    /// 创建 L2 集群节点
    pub fn cluster_node(domain: &str, cluster: &str, display_name: &str) -> Self {
        let path = format!("{}/{}", domain, cluster);
        Self {
            path: path.clone(),
            display_name: display_name.to_string(),
            level: RouteLevel::Cluster,
            domain: domain.to_string(),
            cluster: Some(cluster.to_string()),
            workflow_id: None,
            is_active: true,
        }
    }

    /// 创建 L3 工作流节点
    pub fn workflow_node(
        domain: &str,
        cluster: &str,
        workflow_id: &str,
        display_name: &str,
    ) -> Self {
        let path = format!("{}/{}/{}", domain, cluster, workflow_id);
        Self {
            path: path.clone(),
            display_name: display_name.to_string(),
            level: RouteLevel::Workflow,
            domain: domain.to_string(),
            cluster: Some(cluster.to_string()),
            workflow_id: Some(workflow_id.to_string()),
            is_active: true,
        }
    }

    /// 构造路径字符串
    pub fn build_path(domain: &str, cluster: Option<&str>, workflow_id: Option<&str>) -> String {
        match (cluster, workflow_id) {
            (Some(c), Some(w)) => format!("{}/{}/{}", domain, c, w),
            (Some(c), None) => format!("{}/{}", domain, c),
            (None, _) => domain.to_string(),
        }
    }
}

// ── 图谱边 ────────────────────────────────────────

/// 工作流图谱边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphEdge {
    /// 源节点 path
    pub from: String,
    /// 目标节点 path
    pub to: String,
    /// 边类型
    pub edge_type: EdgeType,
    /// 优先级（数值越大越优先）
    pub priority: i32,
}

impl WorkflowGraphEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>, edge_type: EdgeType) -> Self {
        Self { from: from.into(), to: to.into(), edge_type, priority: 0 }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

// ── 工作流图谱 ────────────────────────────────────

/// 工作流图谱 — 三层路由树的 DAG 索引
///
/// # 数据结构
/// - `nodes`: 路径 → 节点信息的映射
/// - `edges`: 所有边列表
/// - `adjacency`: 邻接表，源路径 → 目标路径列表
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowGraph {
    /// 节点映射：path → WorkflowGraphNode
    #[serde(default)]
    pub nodes: HashMap<String, WorkflowGraphNode>,
    /// 边列表
    #[serde(default)]
    pub edges: Vec<WorkflowGraphEdge>,
    /// 邻接表：from_path → Vec<to_path>
    #[serde(default)]
    pub adjacency: HashMap<String, Vec<String>>,
}

impl WorkflowGraph {
    /// 创建空图谱
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加节点
    pub fn add_node(&mut self, node: WorkflowGraphNode) {
        self.nodes.insert(node.path.clone(), node);
    }

    /// 删除节点及其关联的边
    pub fn remove_node(&mut self, path: &str) {
        self.nodes.remove(path);
        self.edges.retain(|e| e.from != path && e.to != path);
        self.adjacency.remove(path);
        // 清理指向该节点的邻接表
        for neighbors in self.adjacency.values_mut() {
            neighbors.retain(|n| n != path);
        }
    }

    /// 添加边（自动更新邻接表）
    pub fn add_edge(&mut self, edge: WorkflowGraphEdge) {
        // 避免重复添加
        if !self.has_edge(&edge.from, &edge.to, Some(&edge.edge_type)) {
            self.adjacency.entry(edge.from.clone()).or_default().push(edge.to.clone());
            self.edges.push(edge);
        }
    }

    /// 删除边
    pub fn remove_edge(&mut self, from: &str, to: &str, edge_type: Option<&EdgeType>) {
        self.edges.retain(|e| {
            let type_match = match edge_type {
                Some(t) => &e.edge_type == t,
                None => true,
            };
            !(e.from == from && e.to == to && type_match)
        });

        if let Some(neighbors) = self.adjacency.get_mut(from) {
            neighbors.retain(|n| n != to);
        }
    }

    /// 检查边是否存在
    pub fn has_edge(&self, from: &str, to: &str, edge_type: Option<&EdgeType>) -> bool {
        self.edges.iter().any(|e| {
            let type_match = match edge_type {
                Some(t) => &e.edge_type == t,
                None => true,
            };
            e.from == from && e.to == to && type_match
        })
    }

    /// 批量添加层级边（L1→L2→L3 结构）
    pub fn add_hierarchy_edges(&mut self) {
        let paths: Vec<String> = self.nodes.keys().cloned().collect();

        for path in &paths {
            if let Some(node) = self.nodes.get(path) {
                match node.level {
                    RouteLevel::Domain => {
                        // 查找该域下的所有 L2 节点
                        let cluster_paths: Vec<String> = self
                            .nodes
                            .values()
                            .filter(|n| n.level == RouteLevel::Cluster && n.domain == node.domain)
                            .map(|n| n.path.clone())
                            .collect();

                        for cluster_path in cluster_paths {
                            self.add_edge(WorkflowGraphEdge::new(
                                path.clone(),
                                cluster_path,
                                EdgeType::Hierarchy,
                            ));
                        }
                    },
                    RouteLevel::Cluster => {
                        // 查找该集群下的所有 L3 节点
                        let workflow_paths: Vec<String> = self
                            .nodes
                            .values()
                            .filter(|n| {
                                n.level == RouteLevel::Workflow
                                    && n.domain == node.domain
                                    && n.cluster.as_deref() == node.cluster.as_deref()
                            })
                            .map(|n| n.path.clone())
                            .collect();

                        for workflow_path in workflow_paths {
                            self.add_edge(WorkflowGraphEdge::new(
                                path.clone(),
                                workflow_path,
                                EdgeType::Hierarchy,
                            ));
                        }
                    },
                    RouteLevel::Workflow => {},
                }
            }
        }
    }

    /// 获取指定节点的下游可达节点
    pub fn get_neighbors(&self, path: &str) -> Vec<&WorkflowGraphNode> {
        self.adjacency
            .get(path)
            .map(|neighbors| neighbors.iter().filter_map(|p| self.nodes.get(p)).collect())
            .unwrap_or_default()
    }

    /// 获取指定节点的下游路径列表
    pub fn get_neighbor_paths(&self, path: &str) -> Vec<String> {
        self.adjacency.get(path).cloned().unwrap_or_default()
    }

    /// 获取指定节点的某类型下游节点
    pub fn get_neighbors_by_type(
        &self,
        path: &str,
        edge_type: &EdgeType,
    ) -> Vec<&WorkflowGraphNode> {
        self.edges
            .iter()
            .filter(|e| e.from == path && e.edge_type == *edge_type)
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }

    /// 获取节点信息
    pub fn get_node(&self, path: &str) -> Option<&WorkflowGraphNode> {
        self.nodes.get(path)
    }

    /// 获取或创建节点
    pub fn get_or_create_node(&mut self, path: &str, level: RouteLevel) -> &mut WorkflowGraphNode {
        if !self.nodes.contains_key(path) {
            let node = match level {
                RouteLevel::Domain => WorkflowGraphNode::domain_node(path, path),
                RouteLevel::Cluster => {
                    let parts: Vec<&str> = path.split('/').collect();
                    let domain = parts.first().copied().unwrap_or("unknown");
                    let cluster = parts.get(1).copied().unwrap_or("unknown");
                    WorkflowGraphNode::cluster_node(domain, cluster, path)
                },
                RouteLevel::Workflow => {
                    let parts: Vec<&str> = path.split('/').collect();
                    let domain = parts.first().copied().unwrap_or("unknown");
                    let cluster = parts.get(1).copied().unwrap_or("unknown");
                    let workflow_id = parts.get(2).copied().unwrap_or("unknown");
                    WorkflowGraphNode::workflow_node(domain, cluster, workflow_id, path)
                },
            };
            self.nodes.insert(path.to_string(), node);
        }
        self.nodes.get_mut(path).unwrap()
    }

    /// 获取所有指定层级的节点
    pub fn get_nodes_by_level(&self, level: RouteLevel) -> Vec<&WorkflowGraphNode> {
        self.nodes.values().filter(|n| n.level == level).collect()
    }

    /// 获取指定域下的所有节点
    pub fn get_nodes_by_domain(&self, domain: &str) -> Vec<&WorkflowGraphNode> {
        self.nodes.values().filter(|n| n.domain == domain).collect()
    }

    /// 生成 LLM 友好的图谱摘要（仅邻接表）
    ///
    /// # 格式
    /// ```text
    /// 当前节点：finance/stock_analysis。下游可达节点：finance/stock_analysis/tech、finance/stock_analysis/fundamental。
    /// ```
    pub fn to_adjacency_summary(&self, current_path: &str) -> String {
        let neighbors = self.get_neighbors(current_path);
        if neighbors.is_empty() {
            return format!("当前节点：{}。无下游可达节点。", current_path);
        }

        let neighbor_list: Vec<String> = neighbors
            .iter()
            .map(|n| {
                if n.display_name == n.path {
                    n.path.clone()
                } else {
                    format!("{}（{}）", n.path, n.display_name)
                }
            })
            .collect();

        format!("当前节点：{}。下游可达节点：{}。", current_path, neighbor_list.join("、"))
    }

    /// 生成完整的图谱摘要（用于 System Prompt 注入）
    ///
    /// # 输出格式
    /// ```text
    /// ## 工作流图谱
    /// 当前节点：finance/stock_analysis
    ///
    /// ### 下游可达节点
    /// 1. finance/stock_analysis/tech — 技术面分析
    /// 2. finance/stock_analysis/fundamental — 基本面分析
    /// 3. finance/stock_analysis/news — 舆情分析
    /// ```
    pub fn to_graph_summary(&self, current_path: &str) -> String {
        let mut summary = String::new();

        summary.push_str("## 工作流图谱\n");
        summary.push_str(&format!("当前节点：{}\n\n", current_path));

        // 按边类型分组
        let neighbors = self.get_neighbors(current_path);
        if neighbors.is_empty() {
            summary.push_str("无下游可达节点。");
            return summary;
        }

        // 构建带编号的列表
        summary.push_str("### 下游可达节点\n");
        for (i, node) in neighbors.iter().enumerate() {
            let idx = i + 1;
            if node.display_name == node.path {
                summary.push_str(&format!("{}. {}\n", idx, node.path));
            } else {
                summary.push_str(&format!("{}. {} — {}\n", idx, node.path, node.display_name));
            }
        }

        summary
    }

    /// 计算节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 计算边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 广度优先搜索（BFS）获取从起始节点可达的所有路径
    pub fn bfs_reachable(&self, start_path: &str) -> Vec<String> {
        let mut visited = Vec::new();
        let mut queue = vec![start_path.to_string()];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current.clone());

            if let Some(neighbors) = self.adjacency.get(&current) {
                for next in neighbors {
                    if !visited.contains(next) {
                        queue.push(next.clone());
                    }
                }
            }
        }

        visited
    }

    /// 检查路径是否存在
    pub fn path_exists(&self, path: &str) -> bool {
        self.nodes.contains_key(path)
    }

    /// 检查路径是否为系统路径（需要特殊处理）
    pub fn is_system_path(path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.contains("system")
            || path_lower.contains("orchestrator")
            || path_lower.contains("cognitive_router")
    }
}

// ── 图谱同步器 ────────────────────────────────────

/// 图谱同步器 — 工作流创建/更新时自动维护图谱
pub struct WorkflowGraphSync;

impl WorkflowGraphSync {
    /// 同步工作流到图谱
    ///
    /// # 参数
    /// - `graph`: 工作流图谱
    /// - `domain`: 业务域
    /// - `cluster`: 集群名称
    /// - `workflow_id`: 工作流 ID
    /// - `display_name`: 显示名称
    pub fn sync_workflow(
        graph: &mut WorkflowGraph,
        domain: &str,
        cluster: &str,
        workflow_id: &str,
        display_name: &str,
    ) {
        // 1. 确保 L1 域节点存在
        let domain_node = WorkflowGraphNode::domain_node(
            domain, domain, // 使用 domain 作为默认 display_name
        );
        graph.add_node(domain_node);

        // 2. 确保 L2 集群节点存在
        let cluster_node = WorkflowGraphNode::cluster_node(domain, cluster, display_name);
        graph.add_node(cluster_node);

        // 3. 确保 L3 工作流节点存在
        let workflow_node =
            WorkflowGraphNode::workflow_node(domain, cluster, workflow_id, display_name);
        graph.add_node(workflow_node);

        // 4. 更新层级边
        graph.add_hierarchy_edges();
    }

    /// 批量同步工作流
    pub fn sync_batch(graph: &mut WorkflowGraph, workflows: &[(&str, &str, &str, &str)]) {
        for (domain, cluster, workflow_id, display_name) in workflows {
            Self::sync_workflow(graph, domain, cluster, workflow_id, display_name);
        }
    }

    /// 同步集群（不包含具体工作流）
    pub fn sync_cluster(
        graph: &mut WorkflowGraph,
        domain: &str,
        cluster: &str,
        display_name: &str,
    ) {
        // 确保 L1 域节点存在
        let domain_node = WorkflowGraphNode::domain_node(domain, domain);
        graph.add_node(domain_node);

        // 确保 L2 集群节点存在
        let cluster_node = WorkflowGraphNode::cluster_node(domain, cluster, display_name);
        graph.add_node(cluster_node);

        // 更新层级边
        graph.add_hierarchy_edges();
    }

    /// 移除工作流（保留域和集群节点）
    pub fn remove_workflow(graph: &mut WorkflowGraph, workflow_id: &str) {
        // 查找包含该 workflow_id 的节点
        let path_to_remove: Option<String> = graph
            .nodes
            .iter()
            .find(|(_, node)| node.workflow_id.as_deref() == Some(workflow_id))
            .map(|(path, _)| path.clone());

        if let Some(path) = path_to_remove {
            graph.remove_node(&path);
        }
    }

    /// 从能力护照批量同步
    pub fn sync_from_passports<F>(
        graph: &mut WorkflowGraph,
        passports: &[(String, String, String, String)],
        _get_display_name: F,
    ) where
        F: Fn(&str) -> String,
    {
        for (domain, cluster, workflow_id, _) in passports {
            Self::sync_workflow(graph, domain, cluster, workflow_id, workflow_id);
        }
    }
}

// ── 图谱路由 ──────────────────────────────────────

/// 图谱路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRouteResult {
    /// 选中的路径
    pub selected_path: String,
    /// 置信度
    pub confidence: f64,
    /// 可用的候选路径列表
    pub available_paths: Vec<String>,
    /// 兜底路径（如果主路径失败）
    pub fallback_path: Option<String>,
}

/// 图谱路由器 — 基于 DAG 结构进行路径选择
pub struct WorkflowGraphRouter;

impl WorkflowGraphRouter {
    /// 构建图谱路由 Prompt
    ///
    /// # 格式
    /// ```text
    /// 你是一个工作流路由器。根据当前节点和下游可达节点，选择最佳路径。
    ///
    /// ## 当前节点
    /// finance/stock_analysis
    ///
    /// ## 下游可达节点
    /// 1. finance/stock_analysis/tech — 技术面分析
    /// 2. finance/stock_analysis/fundamental — 基本面分析
    /// ...
    ///
    /// ## 用户输入
    /// {user_input}
    ///
    /// ## 输出格式（只能输出路径地址）
    /// {
    ///   "path": "finance/stock_analysis/tech",
    ///   "confidence": 0.98
    /// }
    /// ```
    pub fn build_route_prompt(
        graph: &WorkflowGraph,
        current_path: &str,
        user_input: &str,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("你是一个工作流路由器。根据当前节点和下游可达节点，选择最佳路径。\n\n");

        prompt.push_str("## 当前节点\n");
        prompt.push_str(current_path);
        prompt.push_str("\n\n");

        prompt.push_str("## 下游可达节点\n");
        let neighbors = graph.get_neighbors(current_path);
        if neighbors.is_empty() {
            prompt.push_str("无下游可达节点。\n");
        } else {
            for (i, node) in neighbors.iter().enumerate() {
                let idx = i + 1;
                prompt.push_str(&format!("{}. {} — {}\n", idx, node.path, node.display_name));
            }
        }

        prompt.push_str("\n## 用户输入\n");
        prompt.push_str(user_input);
        prompt.push_str("\n\n## 输出格式（只能输出路径地址）\n");
        prompt
            .push_str("{\n  \"path\": \"finance/stock_analysis/tech\",\n  \"confidence\": 0.98\n}");

        prompt
    }

    /// 验证路径是否为合法的业务路径（禁止系统路径）
    pub fn validate_path(path: &str) -> Result<(), String> {
        if WorkflowGraph::is_system_path(path) {
            return Err(format!("禁止选择系统路径：{path}。系统路径仅供内部使用。"));
        }
        Ok(())
    }

    /// 验证路径是否存在于图谱中
    pub fn validate_path_exists(graph: &WorkflowGraph, path: &str) -> Result<(), String> {
        if !graph.path_exists(path) {
            return Err(format!("路径不存在：{path}"));
        }
        Ok(())
    }

    /// 解析 LLM 输出的路由结果
    pub fn parse_route_result(json_str: &str) -> Result<GraphRouteResult, String> {
        let value: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("JSON 解析失败：{}", e))?;

        let path = value
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 'path' 字段".to_string())?
            .to_string();

        let confidence = value.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);

        Ok(GraphRouteResult {
            selected_path: path,
            confidence,
            available_paths: Vec::new(),
            fallback_path: None,
        })
    }

    /// 基于关键词匹配选择最优下游路径（无需 LLM 的快速路径）
    ///
    /// # 算法
    /// 1. 获取当前节点的所有下游可达节点
    /// 2. 计算每个节点 display_name 与用户输入的关键词匹配得分
    /// 3. 选择得分最高的节点
    pub fn select_best_path(
        graph: &WorkflowGraph,
        current_path: &str,
        user_input: &str,
        candidate_ids: &[String],
    ) -> Option<GraphRouteResult> {
        let neighbors = graph.get_neighbors(current_path);
        if neighbors.is_empty() {
            return None;
        }

        let user_lower = user_input.to_lowercase();
        let mut scored_paths: Vec<(&&WorkflowGraphNode, f64)> = neighbors
            .iter()
            .map(|node| {
                let score = compute_path_match_score(node, &user_lower, candidate_ids);
                (node, score)
            })
            .collect();

        // 按得分降序排序
        scored_paths.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((best_node, best_score)) = scored_paths.first() {
            // 验证路径合法性
            if Self::validate_path(&best_node.path).is_ok() {
                return Some(GraphRouteResult {
                    selected_path: best_node.path.clone(),
                    confidence: best_score.min(1.0),
                    available_paths: neighbors.iter().map(|n| n.path.clone()).collect(),
                    fallback_path: if scored_paths.len() > 1 {
                        Some(scored_paths[1].0.path.clone())
                    } else {
                        None
                    },
                });
            }
        }

        None
    }

    /// 从候选列表中选择最优路径（简化版，直接匹配候选 ID）
    pub fn select_from_candidates(
        graph: &WorkflowGraph,
        current_path: &str,
        candidate_ids: &[String],
    ) -> Option<GraphRouteResult> {
        let neighbors = graph.get_neighbors(current_path);

        // 从候选 ID 中找到第一个在图谱中的节点
        for candidate_id in candidate_ids {
            for node in &neighbors {
                if node.workflow_id.as_deref() == Some(candidate_id.as_str())
                    && Self::validate_path(&node.path).is_ok()
                {
                    return Some(GraphRouteResult {
                        selected_path: node.path.clone(),
                        confidence: 0.95,
                        available_paths: neighbors.iter().map(|n| n.path.clone()).collect(),
                        fallback_path: None,
                    });
                }
            }
        }

        None
    }
}

/// 计算路径与用户输入的匹配得分
fn compute_path_match_score(
    node: &WorkflowGraphNode,
    user_lower: &str,
    candidate_ids: &[String],
) -> f64 {
    let mut score = 0.0;

    // 名称匹配（高权重）
    let name_lower = node.display_name.to_lowercase();
    if user_lower.contains(&name_lower) || name_lower.contains(user_lower) {
        score += 0.5;
    }

    // 候选 ID 匹配（高权重）
    if let Some(ref workflow_id) = node.workflow_id
        && candidate_ids.iter().any(|id| id == workflow_id)
    {
        score += 0.4;
    }

    // 路径关键词匹配
    let path_lower = node.path.to_lowercase();
    let keywords: Vec<&str> = user_lower.split_whitespace().collect();
    let keyword_matches = keywords.iter().filter(|kw| path_lower.contains(**kw)).count();
    score += (keyword_matches as f64 / keywords.len().max(1) as f64) * 0.3;

    score.min(1.0)
}

// ── 测试 ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_graph_creation() {
        let mut graph = WorkflowGraph::new();

        // 添加节点
        graph.add_node(WorkflowGraphNode::domain_node("finance", "金融投资"));
        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock_analysis", "股票分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));

        assert_eq!(graph.node_count(), 3);
        assert!(graph.path_exists("finance"));
        assert!(graph.path_exists("finance/stock_analysis"));
        assert!(graph.path_exists("finance/stock_analysis/tech"));
    }

    #[test]
    fn test_hierarchy_edges() {
        let mut graph = WorkflowGraph::new();

        // 添加节点
        graph.add_node(WorkflowGraphNode::domain_node("finance", "金融投资"));
        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock_analysis", "股票分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));

        // 自动构建层级边
        graph.add_hierarchy_edges();

        assert!(graph.has_edge("finance", "finance/stock_analysis", Some(&EdgeType::Hierarchy)));
        assert!(graph.has_edge(
            "finance/stock_analysis",
            "finance/stock_analysis/tech",
            Some(&EdgeType::Hierarchy)
        ));

        // 检查邻接表
        let neighbors = graph.get_neighbors("finance");
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].path, "finance/stock_analysis");
    }

    #[test]
    fn test_custom_edges() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "fundamental",
            "基本面分析",
        ));

        // 添加兜底边
        graph.add_edge(WorkflowGraphEdge::new(
            "finance/stock_analysis/tech",
            "finance/stock_analysis/fundamental",
            EdgeType::Fallback,
        ));

        let neighbors =
            graph.get_neighbors_by_type("finance/stock_analysis/tech", &EdgeType::Fallback);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].path, "finance/stock_analysis/fundamental");
    }

    #[test]
    fn test_adjacency_summary() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock_analysis", "股票分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "fundamental",
            "基本面分析",
        ));

        graph.add_edge(WorkflowGraphEdge::new(
            "finance/stock_analysis",
            "finance/stock_analysis/tech",
            EdgeType::Hierarchy,
        ));
        graph.add_edge(WorkflowGraphEdge::new(
            "finance/stock_analysis",
            "finance/stock_analysis/fundamental",
            EdgeType::Hierarchy,
        ));

        let summary = graph.to_adjacency_summary("finance/stock_analysis");
        assert!(summary.contains("finance/stock_analysis"));
        assert!(summary.contains("finance/stock_analysis/tech"));
        assert!(summary.contains("finance/stock_analysis/fundamental"));
    }

    #[test]
    fn test_graph_summary_for_prompt() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock_analysis", "股票分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));

        graph.add_edge(WorkflowGraphEdge::new(
            "finance/stock_analysis",
            "finance/stock_analysis/tech",
            EdgeType::Hierarchy,
        ));

        let summary = graph.to_graph_summary("finance/stock_analysis");
        assert!(summary.contains("## 工作流图谱"));
        assert!(summary.contains("finance/stock_analysis"));
        assert!(summary.contains("1. finance/stock_analysis/tech — 技术面分析"));
    }

    #[test]
    fn test_path_validation() {
        // 系统路径应该被拒绝
        assert!(WorkflowGraphRouter::validate_path("system_cognitive_router").is_err());
        assert!(WorkflowGraphRouter::validate_path("finance/stock_analysis/tech").is_ok());
    }

    #[test]
    fn test_graph_sync() {
        let mut graph = WorkflowGraph::new();

        WorkflowGraphSync::sync_workflow(
            &mut graph,
            "finance",
            "stock_analysis",
            "wf_tech",
            "技术面分析",
        );

        assert!(graph.path_exists("finance"));
        assert!(graph.path_exists("finance/stock_analysis"));
        assert!(graph.path_exists("finance/stock_analysis/wf_tech"));

        // 检查层级边
        assert!(graph.has_edge("finance", "finance/stock_analysis", Some(&EdgeType::Hierarchy)));
        assert!(graph.has_edge(
            "finance/stock_analysis",
            "finance/stock_analysis/wf_tech",
            Some(&EdgeType::Hierarchy)
        ));
    }

    #[test]
    fn test_batch_sync() {
        let mut graph = WorkflowGraph::new();

        let workflows = vec![
            ("finance", "stock", "wf_tech", "技术面分析"),
            ("finance", "stock", "wf_fundamental", "基本面分析"),
            ("finance", "fund", "wf_compare", "基金对比"),
        ];

        WorkflowGraphSync::sync_batch(&mut graph, &workflows);

        assert!(graph.path_exists("finance"));
        assert!(graph.path_exists("finance/stock"));
        assert!(graph.path_exists("finance/fund"));
        assert!(graph.path_exists("finance/stock/wf_tech"));
        assert!(graph.path_exists("finance/stock/wf_fundamental"));
        assert!(graph.path_exists("finance/fund/wf_compare"));
    }

    #[test]
    fn test_bfs_reachable() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::domain_node("finance", "投资"));
        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock", "股票"));
        graph.add_node(WorkflowGraphNode::workflow_node("finance", "stock", "tech", "技术分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock",
            "fundamental",
            "基本面分析",
        ));

        graph.add_hierarchy_edges();

        let reachable = graph.bfs_reachable("finance");
        assert!(reachable.contains(&"finance".to_string()));
        assert!(reachable.contains(&"finance/stock".to_string()));
        assert!(reachable.contains(&"finance/stock/tech".to_string()));
        assert!(reachable.contains(&"finance/stock/fundamental".to_string()));
    }

    #[test]
    fn test_build_route_prompt() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock_analysis", "股票分析"));
        graph.add_node(WorkflowGraphNode::workflow_node(
            "finance",
            "stock_analysis",
            "tech",
            "技术面分析",
        ));

        graph.add_edge(WorkflowGraphEdge::new(
            "finance/stock_analysis",
            "finance/stock_analysis/tech",
            EdgeType::Hierarchy,
        ));

        let prompt = WorkflowGraphRouter::build_route_prompt(
            &graph,
            "finance/stock_analysis",
            "分析301302股票",
        );

        assert!(prompt.contains("你是一个工作流路由器"));
        assert!(prompt.contains("finance/stock_analysis"));
        assert!(prompt.contains("finance/stock_analysis/tech"));
        assert!(prompt.contains("技术面分析"));
        assert!(prompt.contains("分析301302股票"));
    }

    #[test]
    fn test_parse_route_result() {
        let json = r#"{"path":"finance/stock_analysis/tech","confidence":0.95}"#;
        let result = WorkflowGraphRouter::parse_route_result(json).unwrap();

        assert_eq!(result.selected_path, "finance/stock_analysis/tech");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_parse_route_result_invalid() {
        let result = WorkflowGraphRouter::parse_route_result("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_node() {
        let mut graph = WorkflowGraph::new();

        graph.add_node(WorkflowGraphNode::domain_node("finance", "投资"));
        graph.add_node(WorkflowGraphNode::cluster_node("finance", "stock", "股票"));

        graph.add_hierarchy_edges();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        graph.remove_node("finance/stock");

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.path_exists("finance/stock"));
    }

    #[test]
    fn test_is_system_path() {
        assert!(WorkflowGraph::is_system_path("system_cognitive_router"));
        assert!(WorkflowGraph::is_system_path("finance/orchestrator"));
        assert!(WorkflowGraph::is_system_path("finance/cognitive_router"));
        assert!(!WorkflowGraph::is_system_path("finance/stock_analysis"));
        assert!(!WorkflowGraph::is_system_path("finance/stock_analysis/tech"));
    }
}
