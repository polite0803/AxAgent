use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::hybrid_search::HybridSearchResult;
use axagent_harness::InferenceEngine;

// ── Config ──────────────────────────────────────────────────

/// Rerank 配置（类型定义位于 axagent-harness）
pub use axagent_harness::rag_config::RerankConfig;

// ── Result types ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RerankedResult {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub original_score: f32,
    pub rerank_score: f32,
    pub rerank_reason: Option<String>,
}

// ── Pluggable backend trait ──────────────────────────────────

#[async_trait]
pub trait RerankBackend: Send + Sync {
    /// 对候选集重新排序，返回 (chunk_id, score) 列表
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)], // (id, content, original_score)
    ) -> axagent_harness::core_error::Result<Vec<(String, f32)>>;
}

// ── Rule backend (migrated from existing logic) ──────────────

pub struct RuleReranker;

#[async_trait]
impl RerankBackend for RuleReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> axagent_harness::core_error::Result<Vec<(String, f32)>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();
        let mut scored: Vec<(String, f32)> = chunks
            .iter()
            .map(|(id, content, orig_score)| {
                let content_lower = content.to_lowercase();
                let exact_matches = query_terms
                    .iter()
                    .filter(|t| content_lower.contains(*t))
                    .count() as f32;
                let exact_score = exact_matches / query_terms.len().max(1) as f32;
                let word_count = content.split_whitespace().count().max(1);
                let coverage = query_terms
                    .iter()
                    .filter(|t| content_lower.split_whitespace().any(|w| w.contains(*t)))
                    .count() as f32
                    / query_terms.len().max(1) as f32;
                let first_pos = content_lower
                    .find(&query_lower)
                    .map(|p| 1.0 - p as f32 / content_lower.len() as f32)
                    .unwrap_or(1.0);
                let len_penalty = {
                    let ratio = word_count as f32 / 100.0;
                    if ratio < 1.0 {
                        ratio
                    } else {
                        1.0 / ratio.sqrt()
                    }
                };
                let score = *orig_score * 0.3
                    + exact_score * 0.25
                    + coverage * 0.2
                    + first_pos * 0.15
                    + len_penalty * 0.1;
                (id.clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }
}

// ── Cross-Encoder backend (candle local inference) ───────────

pub struct CrossEncoderReranker {
    model_filename: String,
    engine: Arc<dyn InferenceEngine>,
}

impl CrossEncoderReranker {
    pub fn new(model_filename: String, engine: Arc<dyn InferenceEngine>) -> Self {
        Self {
            model_filename,
            engine,
        }
    }
}

#[async_trait]
impl RerankBackend for CrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> axagent_harness::core_error::Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        let documents: Vec<String> = chunks.iter().map(|(_, c, _)| c.clone()).collect();

        match self
            .engine
            .rerank(&self.model_filename, query, &documents)
            .await
        {
            Ok(scores) => {
                let mut result: Vec<(String, f32)> = chunks
                    .iter()
                    .zip(scores.iter())
                    .map(|((id, _, _), &s)| (id.clone(), s))
                    .collect();
                result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                Ok(result)
            },
            Err(e) => {
                tracing::warn!("Cross-encoder rerank failed, fallback: {}", e);
                Ok(chunks.iter().map(|(id, _, s)| (id.clone(), *s)).collect())
            },
        }
    }
}

// ── Pipeline orchestrator ────────────────────────────────────

pub struct RerankPipeline {
    stages: Vec<Box<dyn RerankBackend>>,
}

impl Default for RerankPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl RerankPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, backend: Box<dyn RerankBackend>) {
        self.stages.push(backend);
    }

    pub async fn execute(
        &self,
        query: &str,
        results: Vec<HybridSearchResult>,
        config: &RerankConfig,
    ) -> Vec<RerankedResult> {
        if !config.enabled || results.is_empty() {
            return results
                .into_iter()
                .map(|r| RerankedResult {
                    id: r.id,
                    document_id: r.document_id,
                    chunk_index: r.chunk_index,
                    content: r.content,
                    original_score: r.combined_score,
                    rerank_score: r.combined_score,
                    rerank_reason: None,
                })
                .collect();
        }

        let mut current: Vec<HybridSearchResult> =
            results.into_iter().take(config.candidate_k).collect();

        for stage in &self.stages {
            let chunks: Vec<(String, String, f32)> = current
                .iter()
                .map(|r| (r.id.clone(), r.content.clone(), r.combined_score))
                .collect();

            let scored = match stage.rerank(query, &chunks).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Rerank stage failed: {}", e);
                    continue;
                },
            };

            let score_map: std::collections::HashMap<&str, f32> =
                scored.iter().map(|(id, s)| (id.as_str(), *s)).collect();

            current.sort_by(|a, b| {
                let sa = score_map
                    .get(a.id.as_str())
                    .copied()
                    .unwrap_or(a.combined_score);
                let sb = score_map
                    .get(b.id.as_str())
                    .copied()
                    .unwrap_or(b.combined_score);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });

            current = current.into_iter().take(config.rule_filter_keep).collect();
        }

        current
            .into_iter()
            .take(config.top_n)
            .enumerate()
            .map(|(i, r)| RerankedResult {
                id: r.id,
                document_id: r.document_id,
                chunk_index: r.chunk_index,
                content: r.content,
                original_score: r.combined_score,
                rerank_score: r.combined_score,
                rerank_reason: Some(format!("Ranked #{}", i + 1)),
            })
            .collect()
    }
}

// ── Factory ──────────────────────────────────────────────────

pub fn create_rerank_pipeline(
    config: &RerankConfig,
    engine: Option<Arc<dyn InferenceEngine>>,
) -> RerankPipeline {
    let mut pipeline = RerankPipeline::new();
    match config.backend.as_str() {
        "rule" => {
            pipeline.add_stage(Box::new(RuleReranker));
        },
        "cross_encoder" => {
            if let Some(eng) = engine {
                let model = config
                    .cross_encoder_model
                    .clone()
                    .unwrap_or_else(|| "bge-reranker-v2-m3.Q4_K_M.gguf".to_string());
                pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, eng)));
            } else {
                tracing::warn!("No InferenceEngine, falling back to rule reranker");
                pipeline.add_stage(Box::new(RuleReranker));
            }
        },
        "pipeline" => {
            pipeline.add_stage(Box::new(RuleReranker));
            if let Some(eng) = engine {
                let model = config
                    .cross_encoder_model
                    .clone()
                    .unwrap_or_else(|| "bge-reranker-v2-m3.Q4_K_M.gguf".to_string());
                pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, eng)));
            }
        },
        _ => {
            pipeline.add_stage(Box::new(RuleReranker));
        },
    }
    pipeline
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, content: &str, score: f32) -> HybridSearchResult {
        HybridSearchResult {
            id: id.to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: content.to_string(),
            vector_score: Some(score),
            bm25_score: None,
            combined_score: score,
        }
    }

    #[tokio::test]
    async fn test_rule_reranker_sorts_by_relevance() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None);
        let results = vec![
            make_result("1", "The quick brown fox", 0.5),
            make_result("2", "fox jumps over the lazy dog", 0.9),
        ];
        let reranked = pipeline
            .execute("lazy dog", results, &RerankConfig::default())
            .await;
        assert_eq!(reranked[0].id, "2");
    }

    #[tokio::test]
    async fn test_empty_results() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None);
        let reranked = pipeline
            .execute("test", vec![], &RerankConfig::default())
            .await;
        assert!(reranked.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let mut config = RerankConfig::default();
        config.enabled = false;
        let pipeline = create_rerank_pipeline(&config, None);
        let results = vec![make_result("1", "test content", 0.8)];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 1);
        assert_eq!(reranked[0].rerank_score, 0.8);
    }

    #[tokio::test]
    async fn test_top_n_limit() {
        let mut config = RerankConfig::default();
        config.top_n = 2;
        config.candidate_k = 5;
        let pipeline = create_rerank_pipeline(&config, None);
        let results = vec![
            make_result("1", "a", 0.3),
            make_result("2", "b", 0.5),
            make_result("3", "c", 0.9),
            make_result("4", "d", 0.7),
        ];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 2);
    }
}
