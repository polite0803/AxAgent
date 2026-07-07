// SPDX-License-Identifier: AGPL-3.0-only
//! RAG 提供者契约
use async_trait::async_trait;
use crate::types::rag_voice_etc::{RagContextResult, RagRetrievedItem};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
    fn dimension(&self) -> usize;
}
#[derive(Default)] pub struct NoopEmbeddingProvider;
#[async_trait] impl EmbeddingProvider for NoopEmbeddingProvider { async fn embed(&self, _: &str) -> Result<Vec<f32>, String> { Err("not configured".into()) } async fn embed_batch(&self, _: &[String]) -> Result<Vec<Vec<f32>>, String> { Err("not configured".into()) } fn dimension(&self) -> usize { 0 } }

#[derive(Debug, Clone)] pub struct VectorQueryResult { pub id: String, pub score: f64, pub content: String, pub metadata: Option<serde_json::Value> }
#[async_trait]
pub trait VectorStoreProvider: Send + Sync {
    async fn search(&self, collection: &str, query: &[f32], top_k: usize) -> Result<Vec<VectorQueryResult>, String>;
    async fn upsert(&self, collection: &str, id: &str, vector: &[f32], content: &str) -> Result<(), String>;
    async fn delete(&self, collection: &str, id: &str) -> Result<(), String>;
    async fn clear_collection(&self, collection: &str) -> Result<(), String>;
}
#[derive(Default)] pub struct NoopVectorStoreProvider;
#[async_trait] impl VectorStoreProvider for NoopVectorStoreProvider { async fn search(&self, _: &str, _: &[f32], _: usize) -> Result<Vec<VectorQueryResult>, String> { Ok(Vec::new()) } async fn upsert(&self, _: &str, _: &str, _: &[f32], _: &str) -> Result<(), String> { Err("not configured".into()) } async fn delete(&self, _: &str, _: &str) -> Result<(), String> { Ok(()) } async fn clear_collection(&self, _: &str) -> Result<(), String> { Ok(()) } }

#[async_trait]
pub trait RerankProvider: Send + Sync { async fn rerank(&self, query: &str, items: &[RagRetrievedItem], top_k: usize) -> Result<Vec<RagRetrievedItem>, String>; }
#[derive(Default)] pub struct NoopRerankProvider;
#[async_trait] impl RerankProvider for NoopRerankProvider { async fn rerank(&self, _: &str, items: &[RagRetrievedItem], _: usize) -> Result<Vec<RagRetrievedItem>, String> { Ok(items.to_vec()) } }

#[derive(Debug, Clone, PartialEq, Eq)] pub enum RetrievalQuality { Good, Partial, Poor }
#[async_trait]
pub trait SelfRagProvider: Send + Sync { async fn judge_chunks(&self, query: &str, chunks: &[String]) -> Result<RetrievalQuality, String>; async fn refine_query(&self, query: &str, context: &str) -> Result<String, String>; }
#[derive(Default)] pub struct NoopSelfRagProvider;
#[async_trait] impl SelfRagProvider for NoopSelfRagProvider { async fn judge_chunks(&self, _: &str, _: &[String]) -> Result<RetrievalQuality, String> { Ok(RetrievalQuality::Good) } async fn refine_query(&self, query: &str, _: &str) -> Result<String, String> { Ok(query.to_string()) } }

#[derive(Debug, Clone)] pub struct RAGQuery { pub query: String, pub top_k: usize, pub collections: Vec<String>, pub use_rerank: bool, pub use_self_rag: bool }
impl Default for RAGQuery { fn default() -> Self { Self { query: String::new(), top_k: 5, collections: vec!["knowledge".into(),"memory".into()], use_rerank: true, use_self_rag: false } } }

#[async_trait]
pub trait RAGProvider: Send + Sync {
    async fn retrieve(&self, query: &RAGQuery) -> Result<RagContextResult, String>;
    async fn hybrid_search(&self, query: &RAGQuery) -> Result<Vec<RagRetrievedItem>, String>;
    fn available_collections(&self) -> Vec<String>;
}
#[derive(Default)] pub struct NoopRAGProvider;
#[async_trait] impl RAGProvider for NoopRAGProvider { async fn retrieve(&self, _: &RAGQuery) -> Result<RagContextResult, String> { Err("not configured".into()) } async fn hybrid_search(&self, _: &RAGQuery) -> Result<Vec<RagRetrievedItem>, String> { Ok(Vec::new()) } fn available_collections(&self) -> Vec<String> { Vec::new() } }
