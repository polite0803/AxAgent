// SPDX-License-Identifier: AGPL-3.0-only

//! Inference engine trait for local ML inference.
//!
//! The concrete implementation lives in `axagent-search` and uses `candle`/`tokenizers`.
//! Consumers (reranker, judge evaluator, sparse encoder) use this trait from harness.

use crate::core_error::Result;
use async_trait::async_trait;

/// Sparse vector entry: (token_id, weight).
///
/// BGE-M3 sparse 输出格式：仅保留非零项，token_id 对应模型词表中的 token，
/// weight 为激活强度（经 sigmoid + 归一化后的分数）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SparseVectorEntry {
    pub token_id: u32,
    pub weight: f32,
}

/// Local inference engine that runs GGUF models for reranking, judging, and sparse encoding.
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Rerank a list of documents given a query.
    /// Returns a score for each document.
    async fn rerank(
        &self,
        model_filename: &str,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<f32>>;

    /// Compute sparse neural representation for a text.
    ///
    /// 用于多引擎 RAG 的 sparse neural 检索路径（如 BGE-M3 的 sparse 输出、
    /// SPLADE 风格的 lexical expanded 表示）。
    ///
    /// 实现：
    /// - 通过 `model_filename` 查找已注册的 sparse encoder（如 BGE-M3 GGUF）
    /// - 返回非零 `(token_id, weight)` 列表，长度由模型决定
    /// - 若模型未加载，返回空 Vec（调用方可回退到 BM25 或 dense 检索）
    async fn embed_sparse(
        &self,
        model_filename: &str,
        text: &str,
    ) -> Result<Vec<SparseVectorEntry>>;
}
