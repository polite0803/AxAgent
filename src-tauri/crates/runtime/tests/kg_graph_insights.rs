// SPDX-License-Identifier: AGPL-3.0-only

//! 端到端验证：知识库 CSV → link_graph.json → LinkGraph → Louvain → GraphInsights
//!
//! 证明「开源股票知识库」可作为 AxInvest 的 Wiki 冷启动种子，并直接产出图洞察
//! （社区发现 / 桥节点识别 / 意外关联），对应知识层价值定位中的 #3。
//!
//! 运行：`cargo test -p axagent-runtime --test kg_graph_insights`

use std::collections::HashMap;

use axagent_agent::graph_insights::GraphInsightAnalyzer;
use axagent_dao::repo::louvain::detect_communities;
use axagent_harness::graph_dtos::{GraphData, LinkGraph};

#[test]
fn kg_sample_graph_insights_end_to_end() {
    let json = include_str!("../../../../knowledge-sources/sample/link_graph.json");
    let data: GraphData = serde_json::from_str(json)
        .expect("link_graph.json 应可反序列化为 harness::graph_dtos::GraphData");

    let graph = LinkGraph::from_graph_data(data);
    assert!(graph.node_count() >= 10, "样例应有足够节点以形成社区");

    // 真实 Louvain 社区发现（与运行时 wiring 层一致）
    let louvain = detect_communities(graph.clone());
    assert!(louvain.num_communities >= 2, "Louvain 应分出至少 2 个社区（银行 / 半导体）");

    // 组装图洞察分析器（source_map 为空：种子图无来源重叠信息）
    let analyzer = GraphInsightAnalyzer::new(graph, louvain, HashMap::new());
    let insights = analyzer.analyze();

    // 价值验证：桥节点识别（如中国平安连接银行与保险两个社区）
    assert!(!insights.bridge_nodes.is_empty(), "应识别出桥节点（连接多个知识社区的枢纽实体）");

    println!(
        "[kg_graph_insights] communities={} bridge_nodes={:?} surprising={} gaps={} isolated={}",
        insights.stats.num_communities,
        insights.bridge_nodes.iter().map(|b| b.node_title.clone()).collect::<Vec<_>>(),
        insights.surprising_connections.len(),
        insights.knowledge_gaps.len(),
        insights.isolated_pages.len(),
    );
}
