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

/// RAG 管线 —— 编排 检索 → 重排序 → 质检
pub struct RAGPipeline {
    rerank_pipeline: RerankPipeline,
    self_rag_gate: SelfRagGate,
}

impl RAGPipeline {
    /// 创建 RAG 管线。
    ///
    /// - `engine`：本地 Cross-Encoder 推理引擎（仅 `cross_encoder`/`pipeline` backend 需要）。
    ///   传 `None` 时 cross_encoder 会降级到 rule reranker。
    /// - `api_key`：云端 reranker（cohere/jina/voyage）的实际 API Key，
    ///   应由 wiring 层根据 `RerankConfig.api_key_ref` 凭证引用名解析后注入；
    ///   当前调用方暂传 `None`（占位），后续 wiring 层接入后改为传入实际 key。
    pub fn new(
        config: &RAGPipelineConfig,
        engine: Option<Arc<dyn InferenceEngine>>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            // 修复：原本硬编码传 None 给 engine，导致 cross_encoder 永远降级到 rule。
            // 现在改为根据 config + 注入的 engine/api_key 创建合适的 rerank pipeline，
            // 让 rule / cross_encoder / pipeline / cohere / jina / voyage 各 backend
            // 都能被正确实例化。
            rerank_pipeline: reranker::create_rerank_pipeline(&config.rerank, engine, api_key),
            self_rag_gate: SelfRagGate::new(config.self_rag.clone()),
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
        // 阶段 1：检索（使用现有 rag::search）
        let raw_results = rag::search(
            source,
            db,
            master_key,
            vector_store,
            container_id,
            query,
            top_k.max(rerank_config.candidate_k),
            dimensions,
            embed_fn,
        )
        .await?;

        if raw_results.is_empty() {
            return Ok(PipelineOutput {
                results: vec![],
                quality: RetrievalQuality::Poor("No results from search".to_string()),
                retries: 0,
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

        // 过滤不相关 chunk
        let filtered: Vec<RerankedChunk> = reranked
            .iter()
            .zip(judgments.iter())
            .filter(|(_, j)| j.relevant)
            .map(|(r, j)| RerankedChunk {
                id: r.id.clone(),
                document_id: r.document_id.clone(),
                content: r.content.clone(),
                score: r.rerank_score,
                relevance: j.score,
                reason: j.reason.clone(),
            })
            .collect();

        Ok(PipelineOutput { results: filtered, quality, retries: 0 })
    }
}

#[derive(Debug, Clone)]
pub struct RerankedChunk {
    pub id: String,
    pub document_id: String,
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
}
