// SPDX-License-Identifier: AGPL-3.0-only
//! 文档索引契约
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub content: String,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
}
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub collection_name: String,
    pub overwrite: bool,
}
impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 64,
            collection_name: "knowledge".into(),
            overwrite: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexJobStatus {
    Pending,
    Running { progress: f64 },
    Completed { chunks_indexed: usize },
    Failed { error: String },
}

#[async_trait]
pub trait ChunkProvider: Send + Sync {
    async fn chunk(&self, text: &str, config: &IndexConfig) -> Result<Vec<DocumentChunk>, String>;
    async fn chunk_batch(
        &self,
        texts: &[(String, String)],
        config: &IndexConfig,
    ) -> Result<Vec<DocumentChunk>, String>;
}

#[async_trait]
pub trait DocumentIndexer: Send + Sync {
    async fn index_document(
        &self,
        source: &str,
        content: &str,
        config: &IndexConfig,
    ) -> Result<IndexJobStatus, String>;
    async fn index_batch(
        &self,
        docs: &[(String, String)],
        config: &IndexConfig,
    ) -> Result<IndexJobStatus, String>;
    async fn delete_index(&self, collection: &str) -> Result<(), String>;
    async fn get_stats(&self, collection: &str) -> Result<serde_json::Value, String>;
}
