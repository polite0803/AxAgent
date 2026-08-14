// SPDX-License-Identifier: AGPL-3.0-only
//! 能力索引器实现 — 复用 VectorStore + EmbeddingProvider
//!
//! 将 CapabilityPassport 向量化存储到本地向量库：
//! 1. 正向索引：description + tags 拼接嵌入
//! 2. 负向索引：negative_scenarios 逐条嵌入
//! 3. 元数据存储：完整 CapabilityPassportDto 持久化到 VectorStore 元数据表 + 内存索引
//!
//! # 架构位置
//! tools crate (hybrid) 实现 harness 层的 CapabilityIndexer trait，
//! 依赖 axagent-search::VectorStore 和 axagent-harness::EmbeddingProvider。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::rag_provider::EmbeddingProvider;
use axagent_harness::{
    CAPABILITY_COLLECTION, CAPABILITY_NEGATIVE_COLLECTION, CapabilityIndexStats, CapabilityIndexer,
    CapabilityPassportDto, IndexResult,
};
use axagent_search::vector_store::{EmbeddingRecord, VectorStore};
use tokio::sync::RwLock;

/// 元数据文档 ID 前缀，用于标识存储完整护照 JSON 的元数据行
const META_DOC_PREFIX: &str = "__CAPABILITY_META__:";

/// 能力索引器实现
///
/// 复用现有 `VectorStore`（SQLite vec0 + FTS5）存储向量，
/// 元数据同时持久化到 VectorStore 元数据表（跨进程恢复）和内存 HashMap（快速访问）。
#[derive(Clone)]
pub struct CapabilityIndexerImpl {
    vector_store: Arc<VectorStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// capability_id → CapabilityPassportDto 的元数据索引
    metadata: Arc<RwLock<HashMap<String, CapabilityPassportDto>>>,
    embedding_dimensions: usize,
}

impl CapabilityIndexerImpl {
    pub fn new(
        vector_store: Arc<VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        let dimensions = embedding_provider.dimension();
        Self {
            vector_store,
            embedding_provider,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            embedding_dimensions: dimensions,
        }
    }

    /// 从 VectorStore 恢复元数据索引（启动时调用，确保重启后元数据不丢失）
    pub async fn restore_metadata_from_store(&self) -> Result<(), String> {
        let rows = self
            .vector_store
            .list_all_metadata_rows(CAPABILITY_COLLECTION)
            .await
            .map_err(|e| format!("Failed to list metadata rows: {e}"))?;

        let mut restored = 0usize;
        for (_rowid, _id, doc_id, content) in &rows {
            if let Some(capability_id) = doc_id.strip_prefix(META_DOC_PREFIX) {
                match serde_json::from_str::<CapabilityPassportDto>(content) {
                    Ok(passport) => {
                        let mut meta = self.metadata.write().await;
                        meta.insert(passport.capability_id.clone(), passport);
                        restored += 1;
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize passport for {}: {}",
                            capability_id,
                            e
                        );
                    },
                }
            }
        }

        if restored > 0 {
            tracing::info!("Restored {} capability metadata entries from vector store", restored);
        }
        Ok(())
    }

    /// 构建元数据文档 ID
    fn meta_doc_id(capability_id: &str) -> String {
        format!("{}{}", META_DOC_PREFIX, capability_id)
    }

    /// 构建正向索引文本（description + tags）
    fn build_positive_text(passport: &CapabilityPassportDto) -> String {
        let mut parts = Vec::new();
        if !passport.description.is_empty() {
            parts.push(passport.description.clone());
        }
        if !passport.tags.is_empty() {
            parts.push(passport.tags.join(" "));
        }
        parts.join("\n")
    }

    /// 计算索引耗时（毫秒）
    fn elapsed_ms(start: std::time::Instant) -> u64 {
        start.elapsed().as_millis() as u64
    }

    /// AxAgentError → String 转换
    fn err_to_string<E: std::fmt::Display>(e: E) -> String {
        e.to_string()
    }

    /// 生成嵌入向量（正向 + 所有负向场景）
    async fn generate_embeddings(
        &self,
        passport: &CapabilityPassportDto,
    ) -> Result<Vec<Vec<f32>>, String> {
        let mut texts = Vec::new();

        // 正向文本
        texts.push(Self::build_positive_text(passport));

        // 负向文本
        for scenario in &passport.negative_scenarios {
            texts.push(scenario.clone());
        }

        if texts.is_empty() {
            return Ok(Vec::new());
        }

        self.embedding_provider.embed_batch(&texts).await
    }

    /// 持久化元数据到 VectorStore + 内存索引
    async fn store_metadata(&self, passport: &CapabilityPassportDto) {
        // 1. 持久化到 VectorStore（跨进程恢复）
        if let Ok(json) = serde_json::to_string(passport) {
            let doc_id = Self::meta_doc_id(&passport.capability_id);
            let chunk_id = doc_id.clone();
            if let Err(e) = self
                .vector_store
                .insert_metadata_only_chunk(CAPABILITY_COLLECTION, &doc_id, &chunk_id, &json)
                .await
            {
                tracing::warn!("Failed to persist metadata for {}: {}", passport.capability_id, e);
            }
        } else {
            tracing::warn!(
                "Failed to serialize passport for {}: {}",
                passport.capability_id,
                "serialization error"
            );
        }

        // 2. 写入内存索引
        let mut meta = self.metadata.write().await;
        meta.insert(passport.capability_id.clone(), passport.clone());
    }

    /// 从元数据索引获取护照
    pub async fn get_passport(&self, capability_id: &str) -> Option<CapabilityPassportDto> {
        let meta = self.metadata.read().await;
        meta.get(capability_id).cloned()
    }

    /// 获取所有已索引护照的 ID 列表
    pub async fn list_capability_ids(&self) -> Vec<String> {
        let meta = self.metadata.read().await;
        meta.keys().cloned().collect()
    }

    /// 一次性获取所有护照（单次读锁，避免 N 次逐个获取的性能瓶颈）
    pub async fn list_passports(&self) -> Vec<CapabilityPassportDto> {
        let meta = self.metadata.read().await;
        meta.values().cloned().collect()
    }
}

#[async_trait]
impl CapabilityIndexer for CapabilityIndexerImpl {
    async fn index_passport(
        &self,
        passport: &CapabilityPassportDto,
    ) -> Result<IndexResult, String> {
        let start = std::time::Instant::now();
        let cid = passport.capability_id.clone();

        // 1. 生成嵌入向量
        let embeddings = match self.generate_embeddings(passport).await {
            Ok(emb) if !emb.is_empty() => emb,
            Ok(_) => {
                // 无嵌入向量时仍持久化元数据
                self.store_metadata(passport).await;
                return Ok(IndexResult {
                    capability_id: cid,
                    success: true,
                    vector_dimensions: self.embedding_dimensions,
                    indexed_at_ms: Self::elapsed_ms(start),
                    error: None,
                });
            },
            Err(e) => {
                // 嵌入失败时仍持久化元数据，确保元数据索引不缺失（后续可重新索引生成嵌入）
                self.store_metadata(passport).await;
                return Ok(IndexResult {
                    capability_id: cid,
                    success: false,
                    vector_dimensions: self.embedding_dimensions,
                    indexed_at_ms: Self::elapsed_ms(start),
                    error: Some(format!("Embedding generation failed: {e}")),
                });
            },
        };

        // 2. 构建记录并写入正向 collection
        let pos_records: Vec<EmbeddingRecord> = embeddings
            .first()
            .map(|emb| {
                vec![EmbeddingRecord {
                    id: format!("{}_pos", passport.capability_id),
                    document_id: passport.capability_id.clone(),
                    chunk_index: 0,
                    content: Self::build_positive_text(passport),
                    embedding: emb.clone(),
                }]
            })
            .unwrap_or_default();

        if !pos_records.is_empty()
            && let Err(e) =
                self.vector_store.upsert_embeddings(CAPABILITY_COLLECTION, pos_records).await
        {
            return Ok(IndexResult {
                capability_id: cid,
                success: false,
                vector_dimensions: self.embedding_dimensions,
                indexed_at_ms: Self::elapsed_ms(start),
                error: Some(format!("Vector store upsert failed: {e}")),
            });
        }

        // 3. 写入负向 collection
        let neg_records: Vec<EmbeddingRecord> = embeddings
            .iter()
            .skip(1)
            .enumerate()
            .filter_map(|(i, emb)| {
                if i < passport.negative_scenarios.len() {
                    Some(EmbeddingRecord {
                        id: format!("{}_neg_{}", passport.capability_id, i),
                        document_id: passport.capability_id.clone(),
                        chunk_index: (i as i32) + 1,
                        content: passport.negative_scenarios[i].clone(),
                        embedding: emb.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        if !neg_records.is_empty()
            && let Err(e) = self
                .vector_store
                .upsert_embeddings(CAPABILITY_NEGATIVE_COLLECTION, neg_records)
                .await
        {
            tracing::warn!(
                "Negative scenario indexing failed for {}: {}",
                passport.capability_id,
                e
            );
        }

        // 4. 持久化元数据（VectorStore + 内存）
        self.store_metadata(passport).await;

        Ok(IndexResult {
            capability_id: cid,
            success: true,
            vector_dimensions: self.embedding_dimensions,
            indexed_at_ms: Self::elapsed_ms(start),
            error: None,
        })
    }

    async fn index_batch(&self, passports: &[CapabilityPassportDto]) -> Vec<IndexResult> {
        let mut results = Vec::with_capacity(passports.len());
        for passport in passports {
            match self.index_passport(passport).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(IndexResult {
                        capability_id: passport.capability_id.clone(),
                        success: false,
                        vector_dimensions: self.embedding_dimensions,
                        indexed_at_ms: 0,
                        error: Some(e),
                    });
                },
            }
        }
        results
    }

    async fn remove_index(&self, capability_id: &str) -> Result<(), String> {
        // 删除正向记录（按 document_id 匹配）
        if let Err(e) =
            self.vector_store.delete_document_embeddings(CAPABILITY_COLLECTION, capability_id).await
        {
            tracing::warn!("Failed to delete positive index for {}: {}", capability_id, e);
        }

        // 同时删除元数据持久化记录
        let meta_doc_id = Self::meta_doc_id(capability_id);
        if let Err(e) =
            self.vector_store.delete_document_embeddings(CAPABILITY_COLLECTION, &meta_doc_id).await
        {
            tracing::warn!("Failed to delete metadata record for {}: {}", capability_id, e);
        }

        // 删除负向记录（按 document_id 匹配）
        if let Err(e) = self
            .vector_store
            .delete_document_embeddings(CAPABILITY_NEGATIVE_COLLECTION, capability_id)
            .await
        {
            tracing::warn!("Failed to delete negative index for {}: {}", capability_id, e);
        }

        // 移除内存元数据
        let mut meta = self.metadata.write().await;
        meta.remove(capability_id);

        Ok(())
    }

    async fn clear_all(&self) -> Result<(), String> {
        self.vector_store
            .delete_collection(CAPABILITY_COLLECTION)
            .await
            .map_err(Self::err_to_string)?;
        self.vector_store
            .delete_collection(CAPABILITY_NEGATIVE_COLLECTION)
            .await
            .map_err(Self::err_to_string)?;

        let mut meta = self.metadata.write().await;
        meta.clear();

        Ok(())
    }

    async fn get_stats(&self) -> Result<CapabilityIndexStats, String> {
        let meta = self.metadata.read().await;
        let total = meta.len() as u64;

        let mut positive_vectors = 0u64;
        let mut negative_vectors = 0u64;

        for passport in meta.values() {
            positive_vectors += 1;
            negative_vectors += passport.negative_scenarios.len() as u64;
        }

        Ok(CapabilityIndexStats {
            total_capabilities: total,
            total_vectors: positive_vectors + negative_vectors,
            positive_vectors,
            negative_vectors,
            last_indexed_at: None,
        })
    }

    async fn list_capability_ids(&self) -> Vec<String> {
        let meta = self.metadata.read().await;
        meta.keys().cloned().collect()
    }

    async fn get_passport(&self, capability_id: &str) -> Option<CapabilityPassportDto> {
        let meta = self.metadata.read().await;
        meta.get(capability_id).cloned()
    }

    async fn list_passports(&self) -> Vec<CapabilityPassportDto> {
        let meta = self.metadata.read().await;
        meta.values().cloned().collect()
    }
}
