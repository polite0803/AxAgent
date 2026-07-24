// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::hybrid_search::HybridSearchResult;
use axagent_harness::InferenceEngine;
use axagent_harness::core_error::{AxAgentError, Result};

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
    ) -> Result<Vec<(String, f32)>>;
}

// ── Rule backend (migrated from existing logic) ──────────────

pub struct RuleReranker;

#[async_trait]
impl RerankBackend for RuleReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> Result<Vec<(String, f32)>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> =
            query_lower.split_whitespace().filter(|w| w.len() > 1).collect();
        let mut scored: Vec<(String, f32)> = chunks
            .iter()
            .map(|(id, content, orig_score)| {
                let content_lower = content.to_lowercase();
                let exact_matches =
                    query_terms.iter().filter(|t| content_lower.contains(*t)).count() as f32;
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
        Self { model_filename, engine }
    }
}

#[async_trait]
impl RerankBackend for CrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        let documents: Vec<String> = chunks.iter().map(|(_, c, _)| c.clone()).collect();

        match self.engine.rerank(&self.model_filename, query, &documents).await {
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

// ── 云端 Rerank 公共逻辑 ──────────────────────────────────────
//
// 三家厂商（Cohere / Jina / Voyage）的接口语义高度相似：
//   POST {base}/v1/rerank
//   Authorization: Bearer {api_key}
//   body: { model, query, documents, top_n }
//   resp: { results: [{ index, relevance_score }] }（Voyage 用 data 字段）
//
// 通过 `cloud_rerank_request` helper 统一发请求 + 解析响应，
// 各厂商 struct 只负责定义 base URL、默认 model、body 字段名差异。

/// 云端 rerank API 请求结果：(候选 index, relevance_score)
type CloudRerankResponse = Vec<(usize, f32)>;

/// 调用云端 rerank API 并解析返回的 (index, score) 列表。
///
/// - 超时 30 秒
/// - 任何 HTTP / 解析错误均返回 `AxAgentError::Provider`，由上层降级处理
async fn cloud_rerank_request(
    url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> std::result::Result<CloudRerankResponse, AxAgentError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client 构建失败: {e}")))?;

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Rerank API 请求失败: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Provider(format!("Rerank API 返回非 2xx: {status} - {text}")));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Rerank API 响应解析失败: {e}")))?;

    // 兼容 Cohere/Jina 的 `results` 数组 与 Voyage 的 `data` 数组
    let arr = json
        .get("results")
        .or_else(|| json.get("data"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| AxAgentError::Provider("Rerank 响应缺少 results/data 数组".into()))?;

    let mut scored: Vec<(usize, f32)> = arr
        .iter()
        .map(|item| {
            let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let score = item
                .get("relevance_score")
                .or_else(|| item.get("score"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            (index, score)
        })
        .collect();

    // 按 score 降序排序（API 通常已排序，但保险起见再排一次）
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(scored)
}

/// 把云端 API 返回的 (index, score) 列表映射回 (chunk_id, score)，
/// 失败时降级为原分数排序（与 CrossEncoderReranker 失败回退策略一致）。
fn map_cloud_response(
    chunks: &[(String, String, f32)],
    scored: CloudRerankResponse,
) -> Vec<(String, f32)> {
    scored
        .iter()
        .filter_map(|(idx, score)| chunks.get(*idx).map(|(id, _, _)| (id.clone(), *score)))
        .collect()
}

/// 云端 reranker 失败时的统一降级：返回原分数排序结果
fn fallback_original_scores(chunks: &[(String, String, f32)]) -> Vec<(String, f32)> {
    let mut result: Vec<(String, f32)> = chunks.iter().map(|(id, _, s)| (id.clone(), *s)).collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

// ── Cohere 云端 Rerank ────────────────────────────────────────

/// Cohere Rerank API 客户端
///
/// 文档：https://docs.cohere.com/reference/rerank
/// 默认模型：`rerank-multilingual-v3.0`
pub struct CohereReranker {
    api_key: String,
    api_base: Option<String>,
    model: String,
}

impl CohereReranker {
    pub fn new(api_key: String, api_base: Option<String>, model: Option<String>) -> Self {
        Self {
            api_key,
            api_base,
            model: model.unwrap_or_else(|| "rerank-multilingual-v3.0".to_string()),
        }
    }

    fn url(&self) -> String {
        let base = self.api_base.as_deref().unwrap_or("https://api.cohere.ai");
        format!("{}/v1/rerank", base.trim_end_matches('/'))
    }
}

#[async_trait]
impl RerankBackend for CohereReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let documents: Vec<&str> = chunks.iter().map(|(_, c, _)| c.as_str()).collect();
        // 让 API 对所有候选打分，排序交给本地 top_n 截断逻辑
        let body = json!({
            "model": self.model,
            "query": query,
            "documents": documents,
            "top_n": chunks.len(),
        });

        match cloud_rerank_request(&self.url(), &self.api_key, body).await {
            Ok(scored) => Ok(map_cloud_response(chunks, scored)),
            Err(e) => {
                tracing::warn!("Cohere rerank 失败，降级到原分数排序: {}", e);
                Ok(fallback_original_scores(chunks))
            },
        }
    }
}

// ── Jina 云端 Rerank ──────────────────────────────────────────

/// Jina Rerank API 客户端
///
/// 文档：https://jina.ai/models/jina-reranker-v2-base-multilingual
/// 默认模型：`jina-reranker-v2-base-multilingual`
pub struct JinaReranker {
    api_key: String,
    api_base: Option<String>,
    model: String,
}

impl JinaReranker {
    pub fn new(api_key: String, api_base: Option<String>, model: Option<String>) -> Self {
        Self {
            api_key,
            api_base,
            model: model.unwrap_or_else(|| "jina-reranker-v2-base-multilingual".to_string()),
        }
    }

    fn url(&self) -> String {
        let base = self.api_base.as_deref().unwrap_or("https://api.jina.ai");
        format!("{}/v1/rerank", base.trim_end_matches('/'))
    }
}

#[async_trait]
impl RerankBackend for JinaReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let documents: Vec<&str> = chunks.iter().map(|(_, c, _)| c.as_str()).collect();
        let body = json!({
            "model": self.model,
            "query": query,
            "documents": documents,
            "top_n": chunks.len(),
        });

        match cloud_rerank_request(&self.url(), &self.api_key, body).await {
            Ok(scored) => Ok(map_cloud_response(chunks, scored)),
            Err(e) => {
                tracing::warn!("Jina rerank 失败，降级到原分数排序: {}", e);
                Ok(fallback_original_scores(chunks))
            },
        }
    }
}

// ── Voyage AI 云端 Rerank ─────────────────────────────────────

/// Voyage AI Rerank API 客户端
///
/// 文档：https://docs.voyageai.com/reference/reranker
/// 默认模型：`rerank-2`
///
/// 注意：Voyage 的请求字段是 `top_k`（不是 `top_n`），响应字段是 `data`（不是 `results`）。
pub struct VoyageReranker {
    api_key: String,
    api_base: Option<String>,
    model: String,
}

impl VoyageReranker {
    pub fn new(api_key: String, api_base: Option<String>, model: Option<String>) -> Self {
        Self { api_key, api_base, model: model.unwrap_or_else(|| "rerank-2".to_string()) }
    }

    fn url(&self) -> String {
        let base = self.api_base.as_deref().unwrap_or("https://api.voyageai.com");
        format!("{}/v1/rerank", base.trim_end_matches('/'))
    }
}

#[async_trait]
impl RerankBackend for VoyageReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        // Voyage 接受 String 数组（而非 &str），先转换
        let documents: Vec<String> = chunks.iter().map(|(_, c, _)| c.clone()).collect();
        let body = json!({
            "query": query,
            "documents": documents,
            "model": self.model,
            "top_k": chunks.len(),
        });

        match cloud_rerank_request(&self.url(), &self.api_key, body).await {
            Ok(scored) => Ok(map_cloud_response(chunks, scored)),
            Err(e) => {
                tracing::warn!("Voyage rerank 失败，降级到原分数排序: {}", e);
                Ok(fallback_original_scores(chunks))
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
                let sa = score_map.get(a.id.as_str()).copied().unwrap_or(a.combined_score);
                let sb = score_map.get(b.id.as_str()).copied().unwrap_or(b.combined_score);
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

/// 创建 rerank pipeline。
///
/// - `engine`：本地 Cross-Encoder 推理引擎（仅 `cross_encoder`/`pipeline` backend 需要）
/// - `api_key`：云端 reranker 的实际 API Key（仅 `cohere`/`jina`/`voyage` backend 需要）。
///   该 key 应由 wiring 层根据 `RerankConfig.api_key_ref` 凭证引用名解析后注入；
///   当前调用方暂传 `None`（占位），后续 wiring 层接入后改为传入实际 key。
pub fn create_rerank_pipeline(
    config: &RerankConfig,
    engine: Option<Arc<dyn InferenceEngine>>,
    api_key: Option<String>,
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
        "cohere" => {
            if let Some(key) = api_key {
                pipeline.add_stage(Box::new(CohereReranker::new(
                    key,
                    config.api_base.clone(),
                    None,
                )));
            } else {
                tracing::warn!(
                    "Cohere rerank backend 未提供 api_key（api_key_ref 尚未由 wiring 层注入），降级到 rule reranker"
                );
                pipeline.add_stage(Box::new(RuleReranker));
            }
        },
        "jina" => {
            if let Some(key) = api_key {
                pipeline.add_stage(Box::new(JinaReranker::new(key, config.api_base.clone(), None)));
            } else {
                tracing::warn!(
                    "Jina rerank backend 未提供 api_key（api_key_ref 尚未由 wiring 层注入），降级到 rule reranker"
                );
                pipeline.add_stage(Box::new(RuleReranker));
            }
        },
        "voyage" => {
            if let Some(key) = api_key {
                pipeline.add_stage(Box::new(VoyageReranker::new(
                    key,
                    config.api_base.clone(),
                    None,
                )));
            } else {
                tracing::warn!(
                    "Voyage rerank backend 未提供 api_key（api_key_ref 尚未由 wiring 层注入），降级到 rule reranker"
                );
                pipeline.add_stage(Box::new(RuleReranker));
            }
        },
        unknown => {
            tracing::warn!("未知 rerank backend '{}'，降级到 rule reranker", unknown);
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
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None, None);
        let results = vec![
            make_result("1", "The quick brown fox", 0.5),
            make_result("2", "fox jumps over the lazy dog", 0.9),
        ];
        let reranked = pipeline.execute("lazy dog", results, &RerankConfig::default()).await;
        assert_eq!(reranked[0].id, "2");
    }

    #[tokio::test]
    async fn test_empty_results() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None, None);
        let reranked = pipeline.execute("test", vec![], &RerankConfig::default()).await;
        assert!(reranked.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let config = RerankConfig { enabled: false, ..Default::default() };
        let pipeline = create_rerank_pipeline(&config, None, None);
        let results = vec![make_result("1", "test content", 0.8)];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 1);
        assert_eq!(reranked[0].rerank_score, 0.8);
    }

    #[tokio::test]
    async fn test_top_n_limit() {
        let config = RerankConfig { top_n: 2, candidate_k: 5, ..Default::default() };
        let pipeline = create_rerank_pipeline(&config, None, None);
        let results = vec![
            make_result("1", "a", 0.3),
            make_result("2", "b", 0.5),
            make_result("3", "c", 0.9),
            make_result("4", "d", 0.7),
        ];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 2);
    }

    #[tokio::test]
    async fn test_cohere_backend_without_api_key_falls_back_to_rule() {
        let config = RerankConfig {
            backend: "cohere".to_string(),
            api_key_ref: Some("cohere_key".to_string()),
            ..Default::default()
        };
        // api_key=None 时应降级到 rule reranker，不 panic
        let pipeline = create_rerank_pipeline(&config, None, None);
        let results =
            vec![make_result("1", "machine learning", 0.3), make_result("2", "deep learning", 0.9)];
        let reranked = pipeline.execute("learning", results, &config).await;
        assert_eq!(reranked.len(), 2);
    }

    #[tokio::test]
    #[ignore = "需要外网访问真实 Voyage AI API；CI 环境下默认跳过，本地手动运行"]
    async fn test_voyage_backend_with_invalid_key_falls_back() {
        // 提供一个无效的 API key，HTTP 调用会失败，应降级到原分数排序而不抛错
        let config = RerankConfig {
            backend: "voyage".to_string(),
            api_key_ref: Some("voyage_key".to_string()),
            ..Default::default()
        };
        let pipeline =
            create_rerank_pipeline(&config, None, Some("invalid-test-key-not-real".to_string()));
        let results =
            vec![make_result("1", "rust programming", 0.3), make_result("2", "rust language", 0.9)];
        let reranked = pipeline.execute("rust", results, &config).await;
        // 失败降级后仍应返回所有候选（最多 top_n 条，这里只有 2 条候选）
        assert_eq!(reranked.len(), 2);
        // 降级到原分数排序时，分数高的应排在前
        assert_eq!(reranked[0].id, "2");
        assert_eq!(reranked[1].id, "1");
    }

    #[test]
    fn test_cloud_reranker_url_construction() {
        let r = CohereReranker::new("k".to_string(), None, None);
        assert_eq!(r.url(), "https://api.cohere.ai/v1/rerank");

        let r = CohereReranker::new(
            "k".to_string(),
            Some("https://gateway.example.com/".to_string()),
            None,
        );
        assert_eq!(r.url(), "https://gateway.example.com/v1/rerank");

        let r = JinaReranker::new("k".to_string(), None, None);
        assert_eq!(r.url(), "https://api.jina.ai/v1/rerank");

        let r = VoyageReranker::new("k".to_string(), None, None);
        assert_eq!(r.url(), "https://api.voyageai.com/v1/rerank");
    }
}
