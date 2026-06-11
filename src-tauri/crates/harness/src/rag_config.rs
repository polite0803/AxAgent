// SPDX-License-Identifier: AGPL-3.0-only

//! RAG 相关配置类型
//!
//! 纯数据 DTO，不依赖重型实现模块。
//! 被 `axagent-core::types` re-export。

use serde::{Deserialize, Serialize};

/// Rerank 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankConfig {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub backend: String,
    pub cross_encoder_model: Option<String>,
    pub top_n: usize,
    pub candidate_k: usize,
    pub rule_filter_keep: usize,
    pub score_threshold: Option<f32>,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "rule".to_string(),
            cross_encoder_model: Some("bge-reranker-v2-m3.Q4_K_M.gguf".to_string()),
            top_n: 5,
            candidate_k: 30,
            rule_filter_keep: 15,
            score_threshold: None,
        }
    }
}

/// Self-RAG 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfRagConfig {
    pub enabled: bool,
    pub judge_model: String,
    pub ollama_endpoint: String,
    pub relevance_threshold: f32,
    pub quality_threshold: f32,
    pub max_retry_rounds: u8,
}

impl Default for SelfRagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            judge_model: "qwen2.5:0.5b".to_string(),
            ollama_endpoint: "http://localhost:11434".to_string(),
            relevance_threshold: 0.5,
            quality_threshold: 0.6,
            max_retry_rounds: 2,
        }
    }
}

/// 笔记数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub vault_id: String,
    pub title: String,
    pub file_path: String,
    pub content: String,
    pub content_hash: String,
    pub author: String,
    pub page_type: Option<String>,
    pub source_refs: Option<Vec<String>>,
    pub related_pages: Option<Vec<String>>,
    pub quality_score: Option<f64>,
    pub last_linted_at: Option<i64>,
    pub last_compiled_at: Option<i64>,
    pub compiled_source_hash: Option<String>,
    pub user_edited: bool,
    pub user_edited_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: bool,
}

/// 全局 RAG 管线配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RAGPipelineConfig {
    #[serde(default)]
    pub query_enhancement: crate::types::EnhancementConfig,
    #[serde(default)]
    pub rerank: RerankConfig,
    #[serde(default)]
    pub self_rag: SelfRagConfig,
}

/// 笔记检索结果（含完整 Note 对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSearchResult {
    pub note: Note,
    pub snippet: String,
    pub score: f64,
}
