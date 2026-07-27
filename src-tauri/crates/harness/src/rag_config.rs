// SPDX-License-Identifier: AGPL-3.0-only

//! RAG 相关配置类型
//!
//! 纯数据 DTO，不依赖重型实现模块。
//! 被 `axagent-core::types` re-export。

pub use crate::note_dtos::Note;
use serde::{Deserialize, Serialize};

/// Rerank 配置
///
/// `backend` 字段支持的取值：
/// - `rule` —— 基于关键词匹配的规则排序（默认，零依赖）
/// - `cross_encoder` —— 本地 candle 推理的 Cross-Encoder 模型
/// - `pipeline` —— 规则 + Cross-Encoder 级联
/// - `cohere` —— 云端 Cohere Rerank API（需配合 `api_key_ref`）
/// - `jina` —— 云端 Jina Rerank API（需配合 `api_key_ref`）
/// - `voyage` —— 云端 Voyage AI Rerank API（需配合 `api_key_ref`）
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
    /// 云端 reranker（cohere/jina/voyage）的 API Key 凭证引用名，
    /// 由 wiring 层（credential store）解析后注入实际 key。
    /// 本地 backend（rule/cross_encoder/pipeline）忽略此字段。
    #[serde(default)]
    pub api_key_ref: Option<String>,
    /// 自定义云端 rerank API base URL（可选，覆盖各厂商默认域名）。
    /// 例如自建 Cohere 兼容网关或私有化部署时使用。
    #[serde(default)]
    pub api_base: Option<String>,
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
            api_key_ref: None,
            api_base: None,
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

/// 多引擎 RAG 混合检索配置
///
/// 控制三路融合（dense + sparse + BM25）的权重与算法。
/// 权重会被归一化（总和不要求为 1，按比例分配）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HybridConfig {
    /// 是否启用混合检索（false 时仅走 dense vector）
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// dense vector 检索权重（0.0~1.0，默认 0.7）
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,
    /// BM25 关键词检索权重（0.0~1.0，默认 0.3）
    #[serde(default = "default_bm25_weight")]
    pub bm25_weight: f32,
    /// sparse neural 检索权重（0.0~1.0，默认 0.0）。
    /// 当前未接入 sparse encoder，保持 0 即可；后续接入 SPLADE/BGE-M3 时可调高。
    #[serde(default)]
    pub sparse_weight: f32,
    /// 融合算法：`rrf`（默认）或 `weighted`
    #[serde(default = "default_fusion")]
    pub fusion: String,
    /// RRF 算法的 k 参数（默认 60.0）
    #[serde(default = "default_rrf_k")]
    pub rrf_k: f32,
}

fn default_true() -> bool {
    true
}
fn default_vector_weight() -> f32 {
    0.7
}
fn default_bm25_weight() -> f32 {
    0.3
}
fn default_fusion() -> String {
    "rrf".to_string()
}
fn default_rrf_k() -> f32 {
    60.0
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vector_weight: 0.7,
            bm25_weight: 0.3,
            sparse_weight: 0.0,
            fusion: "rrf".to_string(),
            rrf_k: 60.0,
        }
    }
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
    /// 多引擎 RAG：混合检索权重与融合算法配置
    #[serde(default)]
    pub hybrid: HybridConfig,
}

/// 笔记检索结果（含完整 Note 对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSearchResult {
    pub note: Note,
    pub snippet: String,
    pub score: f64,
}
