// SPDX-License-Identifier: AGPL-3.0-only

//! 学习图数据模型 (P2-12)
//!
//! 借鉴 Hermes Agent 的学习可视化：
//! - LearningGraph: 学习图谱（节点和关系）
//! - LearnedItem: 学到的条目
//! - LearningStats: 学习统计
//! - GraphLayout: 图谱布局建议

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 学习图核心结构
// ---------------------------------------------------------------------------

/// 学习图
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningGraph {
    /// 图 ID
    pub id: String,
    /// 图名称
    pub name: String,
    /// 节点列表
    pub nodes: Vec<LearningNode>,
    /// 边列表
    pub edges: Vec<LearningEdge>,
    /// 布局信息
    pub layout: Option<GraphLayout>,
    /// 更新时间
    pub updated_at: String,
}

/// 学习节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningNode {
    /// 节点 ID
    pub id: String,
    /// 节点标签
    pub label: String,
    /// 节点类型
    pub node_type: LearningNodeType,
    /// 节点描述
    pub description: String,
    /// 关联的会话 ID
    pub source_session_id: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 置信度（0-1）
    pub confidence: f64,
    /// 关联的标签
    pub tags: Vec<String>,
    /// 节点元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 位置（如果有布局）
    pub position: Option<NodePosition>,
    /// 子节点 ID 列表
    pub children: Vec<String>,
}

/// 学习节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningNodeType {
    /// 技能
    Skill,
    /// 知识片段
    KnowledgeSnippet,
    /// 决策模式
    DecisionPattern,
    /// 工具使用模式
    ToolUsagePattern,
    /// 工作流程
    Workflow,
    /// 错误恢复策略
    ErrorRecoveryStrategy,
    /// 用户偏好
    UserPreference,
    /// 项目事实
    ProjectFact,
}

/// 学习边（关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningEdge {
    /// 边 ID
    pub id: String,
    /// 源节点 ID
    pub source: String,
    /// 目标节点 ID
    pub target: String,
    /// 边类型
    pub edge_type: LearningEdgeType,
    /// 权重
    pub weight: f64,
    /// 创建时间
    pub created_at: String,
}

/// 学习边类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningEdgeType {
    /// 包含关系
    Contains,
    /// 依赖关系
    DependsOn,
    /// 关联关系
    RelatedTo,
    /// 前置关系
    Precedes,
    /// 变体关系
    VariantOf,
    /// 优化关系
    Optimizes,
}

/// 节点位置（用于布局）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub group: Option<String>,
}

/// 图谱布局
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphLayout {
    /// 布局算法
    pub algorithm: LayoutAlgorithm,
    /// 布局参数
    pub params: HashMap<String, f64>,
    /// 节点位置映射
    pub positions: HashMap<String, NodePosition>,
    /// 计算时间
    pub computed_at: String,
}

/// 布局算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlgorithm {
    /// 力导向布局
    ForceDirected,
    /// 圆形布局
    Circular,
    /// 层级布局
    Hierarchical,
    /// 径向布局
    Radial,
    /// 网格布局
    Grid,
}

impl LearningGraph {
    /// 创建新的学习图
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("graph-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            layout: None,
            updated_at: now,
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: LearningNode) -> &LearningNode {
        self.nodes.push(node);
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.nodes.last().expect("集合为空")
    }

    /// 添加边
    pub fn add_edge(&mut self, edge: LearningEdge) {
        self.edges.push(edge);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// 获取节点
    pub fn get_node(&self, node_id: &str) -> Option<&LearningNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// 获取节点的所有邻居
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&LearningNode> {
        let neighbor_ids: Vec<String> = self
            .edges
            .iter()
            .filter(|e| e.source == node_id || e.target == node_id)
            .map(|e| {
                if e.source == node_id {
                    e.target.clone()
                } else {
                    e.source.clone()
                }
            })
            .collect();

        self.nodes.iter().filter(|n| neighbor_ids.contains(&n.id)).collect()
    }

    /// 按类型获取节点
    pub fn get_nodes_by_type(&self, node_type: LearningNodeType) -> Vec<&LearningNode> {
        self.nodes.iter().filter(|n| n.node_type == node_type).collect()
    }

    /// 图统计
    pub fn stats(&self) -> LearningGraphStats {
        let mut type_counts = HashMap::new();
        for node in &self.nodes {
            *type_counts.entry(format!("{:?}", node.node_type)).or_insert(0) += 1;
        }

        let mut edge_type_counts = HashMap::new();
        for edge in &self.edges {
            *edge_type_counts.entry(format!("{:?}", edge.edge_type)).or_insert(0) += 1;
        }

        LearningGraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            node_type_distribution: type_counts,
            edge_type_distribution: edge_type_counts,
        }
    }
}

/// 学习图统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_type_distribution: HashMap<String, usize>,
    pub edge_type_distribution: HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// 学习条目（简化版，用于快速展示）
// ---------------------------------------------------------------------------

/// 学习条目（从会话中提取的知识点）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedItem {
    /// 条目 ID
    pub id: String,
    /// 标题
    pub title: String,
    /// 内容摘要
    pub summary: String,
    /// 条目类型
    pub item_type: LearningNodeType,
    /// 来源会话 ID
    pub source_session_id: String,
    /// 学习时间
    pub learned_at: String,
    /// 相关标签
    pub tags: Vec<String>,
    /// 重要性分数（0-1）
    pub importance: f64,
    /// 关联条目 ID
    pub related_item_ids: Vec<String>,
}

impl LearnedItem {
    /// 转换为学习节点
    pub fn to_node(&self) -> LearningNode {
        LearningNode {
            id: self.id.clone(),
            label: self.title.clone(),
            node_type: self.item_type,
            description: self.summary.clone(),
            source_session_id: Some(self.source_session_id.clone()),
            created_at: self.learned_at.clone(),
            confidence: self.importance,
            tags: self.tags.clone(),
            metadata: HashMap::new(),
            position: None,
            children: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// 学习统计（用于仪表盘）
// ---------------------------------------------------------------------------

/// 学习统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningStats {
    /// 总学习条目数
    pub total_items: u64,
    /// 按类型统计
    pub items_by_type: HashMap<String, u64>,
    /// 按日期统计（最近 30 天）
    pub items_by_day: HashMap<String, u64>,
    /// 按标签统计
    pub top_tags: Vec<GraphTagCount>,
    /// 最近学习的条目
    pub recent_items: Vec<LearnedItem>,
    /// 学习活跃度（每天平均条目数）
    pub avg_daily_items: f64,
    /// 最活跃的类别
    pub most_active_category: String,
    /// 学习增长率（本周 vs 上周）
    pub growth_rate: f64,
}

/// 标签计数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTagCount {
    pub tag: String,
    pub count: u64,
}

impl LearningStats {
    /// 计算学习增长率
    pub fn growth_rate(&self) -> f64 {
        self.growth_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(id: &str, label: &str, node_type: LearningNodeType) -> LearningNode {
        LearningNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type,
            description: format!("Description for {}", label),
            source_session_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            confidence: 0.8,
            tags: Vec::new(),
            metadata: HashMap::new(),
            position: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn test_learning_graph() {
        let mut graph = LearningGraph::new("测试学习图");
        assert_eq!(graph.nodes.len(), 0);

        let node1 = create_test_node("1", "技能 A", LearningNodeType::Skill);
        let node2 = create_test_node("2", "知识 B", LearningNodeType::KnowledgeSnippet);

        graph.add_node(node1);
        graph.add_node(node2);
        assert_eq!(graph.nodes.len(), 2);

        let edge = LearningEdge {
            id: "e1".to_string(),
            source: "1".to_string(),
            target: "2".to_string(),
            edge_type: LearningEdgeType::RelatedTo,
            weight: 1.0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        graph.add_edge(edge);
        assert_eq!(graph.edges.len(), 1);

        let node = graph.get_node("1");
        assert!(node.is_some());
        assert_eq!(node.expect("测试应成功").label, "技能 A");
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = LearningGraph::new("测试图");

        graph.add_node(create_test_node("1", "Skill1", LearningNodeType::Skill));
        graph.add_node(create_test_node("2", "Skill2", LearningNodeType::Skill));
        graph.add_node(create_test_node("3", "Knowledge1", LearningNodeType::KnowledgeSnippet));

        let stats = graph.stats();
        assert_eq!(stats.total_nodes, 3);
        assert!(stats.node_type_distribution.contains_key("Skill"));
        assert_eq!(stats.node_type_distribution.get("Skill").expect("测试：键应存在"), &2);
    }

    #[test]
    fn test_learned_item() {
        let item = LearnedItem {
            id: "item-1".to_string(),
            title: "新技能学习".to_string(),
            summary: "学习了如何使用 Docker".to_string(),
            item_type: LearningNodeType::Skill,
            source_session_id: "session-1".to_string(),
            learned_at: chrono::Utc::now().to_rfc3339(),
            tags: vec!["docker".to_string()],
            importance: 0.9,
            related_item_ids: Vec::new(),
        };

        let node = item.to_node();
        assert_eq!(node.label, "新技能学习");
        assert_eq!(node.node_type, LearningNodeType::Skill);
    }

    #[test]
    fn test_neighbors() {
        let mut graph = LearningGraph::new("测试图");

        graph.add_node(create_test_node("1", "Node1", LearningNodeType::Skill));
        graph.add_node(create_test_node("2", "Node2", LearningNodeType::KnowledgeSnippet));
        graph.add_node(create_test_node("3", "Node3", LearningNodeType::Workflow));

        graph.add_edge(LearningEdge {
            id: "e1".to_string(),
            source: "1".to_string(),
            target: "2".to_string(),
            edge_type: LearningEdgeType::DependsOn,
            weight: 1.0,
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        let neighbors = graph.get_neighbors("1");
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].id, "2");
    }
}
