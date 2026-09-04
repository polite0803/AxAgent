// SPDX-License-Identifier: AGPL-3.0-only

//! `graph_enhanced_search` 关系查询的回归测试
//!
//! 防两类泄露：
//! 1. 因果边（`relation_type = "causes"`，行为统计而非文档知识）混入 RAG 检索
//! 2. 跨知识库关系泄露（旧实现 `(kb AND source_in) OR target_in` 的 OR 分支丢掉了 kb 约束）

use axagent_dao::db::create_test_pool;
use axagent_dao::repo::knowledge_graph as kg;
use axagent_harness::types::rag_voice_etc::{
    CreateKnowledgeEntityInput, CreateKnowledgeRelationInput,
};
use serde_json::json;

const CAUSAL_RELATION_TYPE: &str = axagent_harness::knowledge_graph::CAUSAL_RELATION_TYPE;

async fn seed_entity(db: &sea_orm::DatabaseConnection, kb: &str, name: &str) -> String {
    let e = kg::create_knowledge_entity(
        db,
        CreateKnowledgeEntityInput {
            knowledge_base_id: kb.to_string(),
            name: name.to_string(),
            entity_type: "module".to_string(),
            description: Some(name.to_string()),
            source_path: String::new(),
            source_language: None,
            properties: json!({}),
            lifecycle: None,
            behaviors: None,
            metadata: None,
        },
    )
    .await
    .expect("create entity");
    e.id
}

async fn seed_relation(
    db: &sea_orm::DatabaseConnection,
    kb: &str,
    source: &str,
    target: &str,
    rel_type: &str,
) {
    kg::create_knowledge_relation(
        db,
        CreateKnowledgeRelationInput {
            knowledge_base_id: kb.to_string(),
            source_entity_id: source.to_string(),
            target_entity_id: target.to_string(),
            relation_type: rel_type.to_string(),
            description: None,
            properties: None,
            metadata: None,
        },
    )
    .await
    .expect("create relation");
}

#[tokio::test]
async fn graph_search_excludes_causal_and_cross_kb_relations() {
    let handle = create_test_pool().await.expect("test pool");
    let db = &handle.conn;
    let kb_main = "kb_main";
    let kb_other = "kb_other";

    let a = seed_entity(db, kb_main, "auth module").await;
    let b = seed_entity(db, kb_main, "login flow").await;
    seed_relation(db, kb_main, &a, &b, "follows").await;

    // 对抗样本 1：同 KB 的因果边——行为统计，必须被类型过滤排除
    seed_relation(db, kb_main, &a, &b, CAUSAL_RELATION_TYPE).await;

    // 对抗样本 2：跨 KB 关系，target 撞上 seed——必须被 kb 分组排除
    let c = seed_entity(db, kb_other, "other thing").await;
    seed_relation(db, kb_other, &c, &a, "mentions").await;

    let chunks = kg::graph_enhanced_search(db, kb_main, "auth", 10, true).await.expect("search");

    assert_eq!(chunks.len(), 1, "query 'auth' 只应命中 auth module");
    let chunk = &chunks[0];
    assert_eq!(chunk.entity_name, "auth module");

    let types: Vec<&str> = chunk.relations.iter().map(|r| r.relation_type.as_str()).collect();
    assert_eq!(types, vec!["follows"], "只允许本 KB 的文档关系，实际: {types:?}");
    assert_eq!(chunk.relations[0].target_entity_name, "login flow");

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn graph_search_without_neighbors_has_no_relations() {
    let handle = create_test_pool().await.expect("test pool");
    let db = &handle.conn;

    let a = seed_entity(db, "kb_x", "auth module").await;
    let b = seed_entity(db, "kb_x", "login flow").await;
    seed_relation(db, "kb_x", &a, &b, "follows").await;

    let chunks = kg::graph_enhanced_search(db, "kb_x", "auth", 10, false).await.expect("search");
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].relations.is_empty(), "include_neighbors=false 不得携带关系");

    std::fs::remove_file(&handle.path).ok();
}
