//! Inference engine trait for local ML inference.
//!
//! The concrete implementation lives in `axagent-core` and uses `candle`/`tokenizers`.
//! Consumers (reranker, judge evaluator) use this trait from harness.

use async_trait::async_trait;
use crate::core_error::Result;

/// Local inference engine that runs GGUF models for reranking and judging.
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
}
