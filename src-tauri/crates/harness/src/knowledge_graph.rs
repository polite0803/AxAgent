// SPDX-License-Identifier: AGPL-3.0-only
//! 知识图谱契约
use async_trait::async_trait;
use crate::types::rag_voice_etc::{CreateKnowledgeEntityInput, KnowledgeEntity, KnowledgeRelation};

#[async_trait]
pub trait EntityGraphProvider: Send + Sync {
    async fn get_entities(&self, kb_id: &str) -> Result<Vec<KnowledgeEntity>, String>;
    async fn search_entities(&self, kb_id: &str, query: &str) -> Result<Vec<KnowledgeEntity>, String>;
    async fn create_entity(&self, kb_id: &str, input: CreateKnowledgeEntityInput) -> Result<KnowledgeEntity, String>;
    async fn delete_entity(&self, entity_id: &str) -> Result<(), String>;
    async fn get_relations(&self, entity_id: &str) -> Result<Vec<KnowledgeRelation>, String>;
    async fn create_relation(&self, source_id: &str, target_id: &str, rel_type: &str) -> Result<KnowledgeRelation, String>;
    async fn delete_relation(&self, relation_id: &str) -> Result<(), String>;
}
#[derive(Default)] pub struct NoopEntityGraphProvider;
#[async_trait] impl EntityGraphProvider for NoopEntityGraphProvider { async fn get_entities(&self, _: &str) -> Result<Vec<KnowledgeEntity>, String> { Ok(Vec::new()) } async fn search_entities(&self, _: &str, _: &str) -> Result<Vec<KnowledgeEntity>, String> { Ok(Vec::new()) } async fn create_entity(&self, _: &str, _: CreateKnowledgeEntityInput) -> Result<KnowledgeEntity, String> { Err("not configured".into()) } async fn delete_entity(&self, _: &str) -> Result<(), String> { Ok(()) } async fn get_relations(&self, _: &str) -> Result<Vec<KnowledgeRelation>, String> { Ok(Vec::new()) } async fn create_relation(&self, _: &str, _: &str, _: &str) -> Result<KnowledgeRelation, String> { Err("not configured".into()) } async fn delete_relation(&self, _: &str) -> Result<(), String> { Ok(()) } }

#[derive(Debug, Clone)] pub struct ExtractedEntity { pub name: String, pub entity_type: String, pub aliases: Vec<String>, pub description: String }
#[derive(Debug, Clone)] pub struct ExtractedRelation { pub source: String, pub target: String, pub relation_type: String }
#[async_trait]
pub trait EntityExtractor: Send + Sync { async fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>, String>; async fn extract_relations(&self, text: &str, entities: &[ExtractedEntity]) -> Result<Vec<ExtractedRelation>, String>; }
#[derive(Default)] pub struct NoopEntityExtractor;
#[async_trait] impl EntityExtractor for NoopEntityExtractor { async fn extract_entities(&self, _: &str) -> Result<Vec<ExtractedEntity>, String> { Ok(Vec::new()) } async fn extract_relations(&self, _: &str, _: &[ExtractedEntity]) -> Result<Vec<ExtractedRelation>, String> { Ok(Vec::new()) } }
