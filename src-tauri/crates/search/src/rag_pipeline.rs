// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::hybrid_search::HybridSearchResult;
use crate::rag::{self, AsyncEmbedFn};
use crate::reranker::{self, RerankPipeline};
use crate::self_rag::{RetrievalQuality, SelfRagGate};
use crate::vector_store::VectorStore;
use axagent_harness::InferenceEngine;
use axagent_harness::core_error::Result;
use axagent_harness::types::*;

/// RAG 管线 —— 编排 检索 → 重排序 → 质检 → 图增强检索
pub struct RAGPipeline {
    rerank_pipeline: RerankPipeline,
    self_rag_gate: SelfRagGate,
    /// 可选的实体图谱提供者，用于 Graph RAG 增强检索
    entity_graph_provider: Option<Arc<dyn axagent_harness::EntityGraphProvider>>,
}

impl RAGPipeline {
    /// 创建 RAG 管线。
    ///
    /// - `engine`：本地 Cross-Encoder 推理引擎（仅 `cross_encoder`/`pipeline` backend 需要）。
    ///   传 `None` 时 cross_encoder 会降级到 rule reranker。
    /// - `api_key`：云端 reranker（cohere/jina/voyage）的实际 API Key，
    ///   应由 wiring 层根据 `RerankConfig.api_key_ref` 凭证引用名解析后注入；
    ///   当前调用方暂传 `None`（占位），后续 wiring 层接入后改为传入实际 key。
    /// - `entity_graph_provider`：可选的实体图谱提供者，用于执行图增强检索。
    pub fn new(
        config: &RAGPipelineConfig,
        engine: Option<Arc<dyn InferenceEngine>>,
        api_key: Option<String>,
        entity_graph_provider: Option<Arc<dyn axagent_harness::EntityGraphProvider>>,
    ) -> Self {
        Self {
            rerank_pipeline: reranker::create_rerank_pipeline(&config.rerank, engine, api_key),
            self_rag_gate: SelfRagGate::new(config.self_rag.clone()),
            entity_graph_provider,
        }
    }

    /// 完整管线：检索 → 重排序 → 质检 → 返回上下文
    #[allow(clippy::too_many_arguments)]
    pub async fn execute<S: rag::RAGSource + ?Sized>(
        &self,
        source: &S,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        vector_store: &VectorStore,
        container_id: &str,
        query: &str,
        top_k: usize,
        dimensions: Option<usize>,
        embed_fn: impl AsyncEmbedFn,
        rerank_config: &reranker::RerankConfig,
    ) -> Result<PipelineOutput> {
        self.execute_with_filter(
            source,
            db,
            master_key,
            vector_store,
            container_id,
            query,
            top_k,
            dimensions,
            embed_fn,
            rerank_config,
            None,
            None,
        )
        .await
    }

    /// `execute` 的多文档协同变体：透传 `doc_ids` 过滤到底层检索。
    ///
    /// `precomputed_embedding`：调用方已为 `query` 计算好的 query embedding
    /// （须与该源 resolve 出的 embedding provider / dims 一致），
    /// 传 `Some` 时跳过检索阶段的重复 embed 调用。
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_with_filter<S: rag::RAGSource + ?Sized>(
        &self,
        source: &S,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        vector_store: &VectorStore,
        container_id: &str,
        query: &str,
        top_k: usize,
        dimensions: Option<usize>,
        embed_fn: impl AsyncEmbedFn,
        rerank_config: &reranker::RerankConfig,
        doc_ids: Option<&[String]>,
        precomputed_embedding: Option<Vec<f32>>,
    ) -> Result<PipelineOutput> {
        // 阶段 1：检索（使用 rag::search_with_filter 透传 doc_ids）
        let raw_results = rag::search_with_filter(
            source,
            db,
            master_key,
            vector_store,
            container_id,
            query,
            top_k.max(rerank_config.candidate_k),
            dimensions,
            embed_fn,
            doc_ids,
            precomputed_embedding,
        )
        .await?;

        if raw_results.is_empty() {
            return Ok(PipelineOutput {
                results: vec![],
                quality: RetrievalQuality::Poor("No results from search".to_string()),
                retries: 0,
                graph_context: None,
            });
        }

        // 转换为 HybridSearchResult
        let hybrid_results: Vec<HybridSearchResult> = raw_results
            .iter()
            .map(|r| HybridSearchResult {
                id: r.id.clone(),
                document_id: r.document_id.clone(),
                chunk_index: r.chunk_index,
                content: r.content.clone(),
                vector_score: Some(1.0 - (r.score / 20.0).min(1.0)),
                bm25_score: None,
                sparse_score: None,
                combined_score: 1.0 - (r.score / 20.0).min(1.0),
            })
            .collect();

        // 阶段 2：重排序
        let reranked = self.rerank_pipeline.execute(query, hybrid_results, rerank_config).await;

        // 阶段 3：质检
        let chunks: Vec<(String, String)> =
            reranked.iter().map(|r| (r.id.clone(), r.content.clone())).collect();

        let judgments = self.self_rag_gate.judge_chunks(query, &chunks).await?;
        let quality = self.self_rag_gate.evaluate_quality(&judgments);

        // 阶段 4：图增强检索（如果启用了 EntityGraphProvider）
        let graph_context = if let Some(provider) = &self.entity_graph_provider {
            let graph_input = axagent_harness::GraphEnhancedSearchInput {
                knowledge_base_id: container_id.to_string(),
                query: query.to_string(),
                entity_type_filters: vec![],
                relation_type_filters: vec![],
                top_k: Some(5),
                include_neighbors: Some(true),
            };
            match provider.graph_enhanced_search(graph_input).await {
                Ok(result) => Some(result),
                Err(e) => {
                    tracing::warn!("Graph enhanced search failed: {}", e);
                    None
                },
            }
        } else {
            None
        };

        // 过滤不相关 chunk（引用追溯：保留 chunk_index 字段）
        let filtered: Vec<RerankedChunk> = reranked
            .iter()
            .zip(judgments.iter())
            .filter(|(_, j)| j.relevant)
            .map(|(r, j)| RerankedChunk {
                id: r.id.clone(),
                document_id: r.document_id.clone(),
                chunk_index: r.chunk_index,
                content: r.content.clone(),
                score: r.rerank_score,
                relevance: j.score,
                reason: j.reason.clone(),
            })
            .collect();

        Ok(PipelineOutput { results: filtered, quality, retries: 0, graph_context })
    }
}

#[derive(Debug, Clone)]
pub struct RerankedChunk {
    pub id: String,
    pub document_id: String,
    /// 引用追溯：chunk 在文档内的顺序索引（从 0 开始）
    pub chunk_index: i32,
    pub content: String,
    pub score: f32,
    pub relevance: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub results: Vec<RerankedChunk>,
    pub quality: RetrievalQuality,
    pub retries: u8,
    /// 图增强检索结果（如果启用了 EntityGraphProvider）
    pub graph_context: Option<axagent_harness::GraphEnhancedSearchResult>,
}
