// SPDX-License-Identifier: AGPL-3.0-only

//! EntityGraphProvider 的数据库实现
//!
//! 该结构体实现了 axagent-harness 中定义的 EntityGraphProvider trait，
//! 通过调用 dao::repo::knowledge_graph 中的函数来提供知识图谱的查询能力。

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::{
    EntityGraphProvider, GraphEnhancedSearchInput, GraphEnhancedSearchResult,
    types::{CreateKnowledgeEntityInput, KnowledgeEntity, KnowledgeRelation},
};

use crate::repo::knowledge_graph as kg_repo;

/// 基于数据库的实体图谱提供者实现
pub struct KnowledgeGraphProvider {
    db: Arc<DatabaseConnection>,
}

impl KnowledgeGraphProvider {
    pub fn new(db: &DatabaseConnection) -> Self {
        Self { db: Arc::new(db.clone()) }
    }

    pub fn from_arc(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EntityGraphProvider for KnowledgeGraphProvider {
    async fn get_entities(&self, kb_id: &str) -> std::result::Result<Vec<KnowledgeEntity>, String> {
        kg_repo::list_knowledge_entities(&self.db, kb_id).await.map_err(|e| e.to_string())
    }

    async fn search_entities(
        &self,
        kb_id: &str,
        query: &str,
    ) -> std::result::Result<Vec<KnowledgeEntity>, String> {
        kg_repo::search_entities(&self.db, kb_id, query, 10).await.map_err(|e| e.to_string())
    }

    async fn create_entity(
        &self,
        _kb_id: &str,
        _input: CreateKnowledgeEntityInput,
    ) -> std::result::Result<KnowledgeEntity, String> {
        Err("create_entity not yet implemented".to_string())
    }

    async fn delete_entity(&self, _entity_id: &str) -> std::result::Result<(), String> {
        Err("delete_entity not yet implemented".to_string())
    }

    async fn get_relations(
        &self,
        _entity_id: &str,
    ) -> std::result::Result<Vec<KnowledgeRelation>, String> {
        Ok(Vec::new())
    }

    async fn create_relation(
        &self,
        _source_id: &str,
        _target_id: &str,
        _rel_type: &str,
    ) -> std::result::Result<KnowledgeRelation, String> {
        Err("create_relation not yet implemented".to_string())
    }

    async fn delete_relation(&self, _relation_id: &str) -> std::result::Result<(), String> {
        Err("delete_relation not yet implemented".to_string())
    }

    async fn graph_enhanced_search(
        &self,
        input: GraphEnhancedSearchInput,
    ) -> std::result::Result<GraphEnhancedSearchResult, String> {
        let chunks = kg_repo::graph_enhanced_search(
            &self.db,
            &input.knowledge_base_id,
            &input.query,
            input.top_k.unwrap_or(10),
            input.include_neighbors.unwrap_or(true),
        )
        .await
        .map_err(|e| e.to_string())?;

        let filtered_chunks = if input.entity_type_filters.is_empty() {
            chunks
        } else {
            chunks
                .into_iter()
                .filter(|c| input.entity_type_filters.contains(&c.entity_type))
                .collect()
        };

        let final_chunks = if input.relation_type_filters.is_empty() {
            filtered_chunks
        } else {
            filtered_chunks
                .into_iter()
                .map(|mut c| {
                    c.relations.retain(|r| input.relation_type_filters.contains(&r.relation_type));
                    c
                })
                .collect()
        };

        let context_text =
            kg_repo::build_graph_context_text(&input.knowledge_base_id, &final_chunks);
        let total_hits = final_chunks.len();

        Ok(GraphEnhancedSearchResult { entities: final_chunks, context_text, total_hits })
    }
}
