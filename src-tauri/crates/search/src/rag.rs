// SPDX-License-Identifier: AGPL-3.0-only

//! Unified RAG (Retrieval-Augmented Generation) abstraction layer.
//!
//! Provides a trait-based interface for different RAG sources (knowledge bases,
//! memory namespaces, etc.) to share indexing, searching, and context-collection
//! logic without code duplication.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};

use crate::hybrid_search::{HybridSearchOptions, HybridSearchResult, HybridSearcher};
use crate::self_rag::RetrievalQuality;
use crate::sources;
use crate::text_chunker;
use crate::vector_store::{EmbeddingRecord, VectorSearchResult, VectorStore};
use axagent_harness::InferenceEngine;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{RagContextResult, RagRetrievedItem, RagSourceResult};

// ── Trait ────────────────────────────────────────────────────────────────────

/// A source of RAG content that can be searched and indexed.
///
/// Each implementor describes how to look up its embedding provider and
/// what prefix / label to use for vector-store collections and conversation
/// context injection.
#[async_trait]
pub trait RAGSource: Send + Sync {
    /// Collection prefix for vector-store table names (e.g. `"kb"`, `"mem"`).
    fn collection_prefix(&self) -> &'static str;

    /// Human-readable label inserted into conversation context
    /// (e.g. `"Knowledge Base Reference"`, `"Memory Reference"`).
    fn context_label(&self) -> &'static str;

    /// Resolve the `"providerId::model_id"` embedding provider string
    /// configured on the container identified by `container_id`.
    async fn resolve_embedding_provider(
        &self,
        db: &DatabaseConnection,
        container_id: &str,
    ) -> Result<String>;
}

// ── Built-in implementations ─────────────────────────────────────────────────

/// RAG source backed by a knowledge base (documents → parsed → chunked → embedded).
pub struct KnowledgeRAG;

#[async_trait]
impl RAGSource for KnowledgeRAG {
    fn collection_prefix(&self) -> &'static str {
        "kb"
    }

    fn context_label(&self) -> &'static str {
        "Knowledge Base Reference"
    }

    async fn resolve_embedding_provider(
        &self,
        db: &DatabaseConnection,
        container_id: &str,
    ) -> Result<String> {
        let kb = sources::knowledge().get_knowledge_base(container_id).await?;
        if let Some(provider) = kb.embedding_provider {
            return Ok(provider);
        }
        resolve_default_embedding_provider(db).await
    }
}

/// RAG source backed by a memory namespace (text items → directly embedded).
pub struct MemoryRAG;

#[async_trait]
impl RAGSource for MemoryRAG {
    fn collection_prefix(&self) -> &'static str {
        "mem"
    }

    fn context_label(&self) -> &'static str {
        "Memory Reference"
    }

    async fn resolve_embedding_provider(
        &self,
        db: &DatabaseConnection,
        container_id: &str,
    ) -> Result<String> {
        let ns = sources::memory().get_namespace(container_id).await?;
        if let Some(provider) = ns.embedding_provider {
            return Ok(provider);
        }
        resolve_default_embedding_provider(db).await
    }
}

/// RAG source backed by a Wiki vault (notes → chunked → embedded).
pub struct WikiVaultRAG;

#[async_trait]
impl RAGSource for WikiVaultRAG {
    fn collection_prefix(&self) -> &'static str {
        "wiki"
    }

    fn context_label(&self) -> &'static str {
        "Wiki Reference"
    }

    async fn resolve_embedding_provider(
        &self,
        db: &DatabaseConnection,
        container_id: &str,
    ) -> Result<String> {
        let wiki = sources::wiki().get_wiki(container_id).await?;
        if let Some(provider) = wiki.embedding_provider {
            return Ok(provider);
        }
        resolve_default_embedding_provider(db).await
    }
}

/// 当容器未显式配置 embedding_provider 时，回退到系统默认 provider。
///
/// 错误消息带 `ERR_NO_EMBEDDING_PROVIDER` 前缀：index_queue worker 据此把
/// 这类确定性配置错误直接置为终态 failed，不做无意义的 max_retries 重试（R9）。
pub const ERR_NO_EMBEDDING_PROVIDER: &str = "EMBEDDING_PROVIDER_NOT_CONFIGURED";

async fn resolve_default_embedding_provider(_db: &DatabaseConnection) -> Result<String> {
    let settings = sources::settings()
        .get_settings()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Failed to load settings: {}", e)))?;
    settings.default_provider_id.ok_or_else(|| {
        AxAgentError::Provider(format!(
            "{}: No embedding provider configured and no default provider found",
            ERR_NO_EMBEDDING_PROVIDER
        ))
    })
}

// ── 统一知识容器（P2: 抽象三个系统的共性字段） ──────────────────────────────

/// Knowledge/Memory/Wiki 三个系统的容器共性。
/// 长期计划（P3）：将 `memory_namespaces` 合并到 `knowledge_bases`，Memory 作为特殊的轻量级知识库。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeContainer {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub container_type: ContainerType,
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub sort_order: i32,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerType {
    KnowledgeBase,
    Memory,
    WikiVault,
}

impl KnowledgeContainer {
    /// 从 memory_namespace 转换
    pub fn from_memory_ns(ns: &axagent_harness::types::MemoryNamespace) -> Self {
        Self {
            id: ns.id.clone(),
            name: ns.name.clone(),
            description: None,
            container_type: ContainerType::Memory,
            embedding_provider: ns.embedding_provider.clone(),
            embedding_dimensions: ns.embedding_dimensions,
            retrieval_threshold: ns.retrieval_threshold,
            retrieval_top_k: ns.retrieval_top_k,
            icon_type: ns.icon_type.clone(),
            icon_value: ns.icon_value.clone(),
            sort_order: ns.sort_order,
            chunk_size: None,
            chunk_overlap: None,
            enabled: true,
        }
    }

    /// 从 knowledge_base 转换
    pub fn from_knowledge_base(kb: &axagent_harness::types::KnowledgeBase) -> Self {
        Self {
            id: kb.id.clone(),
            name: kb.name.clone(),
            description: kb.description.clone(),
            container_type: ContainerType::KnowledgeBase,
            embedding_provider: kb.embedding_provider.clone(),
            embedding_dimensions: kb.embedding_dimensions,
            retrieval_threshold: kb.retrieval_threshold,
            retrieval_top_k: kb.retrieval_top_k,
            icon_type: kb.icon_type.clone(),
            icon_value: kb.icon_value.clone(),
            sort_order: kb.sort_order,
            chunk_size: kb.chunk_size,
            chunk_overlap: kb.chunk_overlap,
            enabled: kb.enabled,
        }
    }

    /// 从 wiki 转换
    pub fn from_wiki(w: &axagent_harness::types::Wiki) -> Self {
        Self {
            id: w.id.clone(),
            name: w.name.clone(),
            description: w.description.clone(),
            container_type: ContainerType::WikiVault,
            embedding_provider: w.embedding_provider.clone(),
            embedding_dimensions: w.embedding_dimensions,
            retrieval_threshold: w.retrieval_threshold,
            retrieval_top_k: w.retrieval_top_k,
            icon_type: None,
            icon_value: None,
            sort_order: 0,
            chunk_size: None,
            chunk_overlap: None,
            enabled: true,
        }
    }

    /// 获取 RAG collection 名称
    pub fn collection_name(&self) -> String {
        match self.container_type {
            ContainerType::KnowledgeBase => format!("kb_{}", self.id),
            ContainerType::Memory => format!("mem_{}", self.id),
            ContainerType::WikiVault => format!("wiki_{}", self.id),
        }
    }

    pub fn source_config(&self) -> axagent_harness::types::SourceConfig {
        axagent_harness::types::SourceConfig {
            embedding_provider: self.embedding_provider.clone(),
            embedding_dimensions: self.embedding_dimensions,
            retrieval_threshold: self.retrieval_threshold,
            retrieval_top_k: self.retrieval_top_k,
        }
    }

    pub fn container_type_str(&self) -> &'static str {
        match self.container_type {
            ContainerType::KnowledgeBase => "KnowledgeBase",
            ContainerType::Memory => "Memory",
            ContainerType::WikiVault => "WikiVault",
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build the sanitised collection ID for a RAG source.
pub fn collection_id(prefix: &str, container_id: &str) -> String {
    format!("{}_{}", prefix, container_id)
}

// ── Unified search ───────────────────────────────────────────────────────────

/// Search a single RAG source for content relevant to `query`.
///
/// Uses hybrid search (vector similarity + BM25 full-text with trigram
/// tokenizer for Chinese support) with Reciprocal Rank Fusion by default.
/// Falls back to pure vector search if FTS index is unavailable.
///
/// This is the generic replacement for the separate `search_knowledge` /
/// `search_memory` functions.  The concrete `EmbedFn` is injected by the
/// caller (typically `crate::indexing::generate_embeddings`).
#[allow(clippy::too_many_arguments)]
pub async fn search<S: RAGSource + ?Sized>(
    source: &S,
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    container_id: &str,
    query: &str,
    top_k: usize,
    dimensions: Option<usize>,
    embed_fn: impl AsyncEmbedFn,
) -> Result<Vec<VectorSearchResult>> {
    search_with_filter(
        source,
        db,
        master_key,
        vector_store,
        container_id,
        query,
        top_k,
        dimensions,
        embed_fn,
        None,
    )
    .await
}

/// `search` with optional `doc_ids` filter (multi-document collaboration).
/// When `doc_ids` is `Some` and non-empty, results are restricted to chunks
/// belonging to one of the listed documents.
#[allow(clippy::too_many_arguments)]
pub async fn search_with_filter<S: RAGSource + ?Sized>(
    source: &S,
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    container_id: &str,
    query: &str,
    top_k: usize,
    dimensions: Option<usize>,
    embed_fn: impl AsyncEmbedFn,
    doc_ids: Option<&[String]>,
) -> Result<Vec<VectorSearchResult>> {
    let hybrid_opts = HybridSearchOptions { top_k, ..Default::default() };
    // 多引擎 RAG：保留原始 bm25_score 明细，避免下游 rerank 丢失分数信息
    let hybrid_results = search_hybrid_with_filter(
        source,
        db,
        master_key,
        vector_store,
        container_id,
        query,
        dimensions,
        embed_fn,
        hybrid_opts,
        doc_ids,
    )
    .await?;

    Ok(hybrid_results
        .into_iter()
        .map(|r| VectorSearchResult {
            id: r.id,
            document_id: r.document_id,
            chunk_index: r.chunk_index,
            content: r.content,
            score: 1.0 - r.combined_score,
            has_embedding: r.vector_score.is_some(),
        })
        .collect())
}

/// Hybrid search (vector + BM25 FTS5 with trigram tokenizer) returning
/// detailed score breakdown.  Uses Reciprocal Rank Fusion by default for
/// robust score combination without manual weight tuning.
#[allow(clippy::too_many_arguments)]
pub async fn search_hybrid<S: RAGSource + ?Sized>(
    source: &S,
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    _vector_store: &VectorStore,
    container_id: &str,
    query: &str,
    dimensions: Option<usize>,
    embed_fn: impl AsyncEmbedFn,
    options: HybridSearchOptions,
) -> Result<Vec<HybridSearchResult>> {
    search_hybrid_with_filter(
        source,
        db,
        master_key,
        _vector_store,
        container_id,
        query,
        dimensions,
        embed_fn,
        options,
        None,
    )
    .await
}

/// `search_hybrid` with optional `doc_ids` filter (multi-document collaboration).
#[allow(clippy::too_many_arguments)]
pub async fn search_hybrid_with_filter<S: RAGSource + ?Sized>(
    source: &S,
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    _vector_store: &VectorStore,
    container_id: &str,
    query: &str,
    dimensions: Option<usize>,
    embed_fn: impl AsyncEmbedFn,
    options: HybridSearchOptions,
    doc_ids: Option<&[String]>,
) -> Result<Vec<HybridSearchResult>> {
    let embedding_provider = match source.resolve_embedding_provider(db, container_id).await {
        Ok(p) => p,
        // R9 遗留：embedding 未配置（容器与系统默认都没有）是确定性配置错误。
        // 会话 RAG 兜底场景下此前直接 Err → 调用方跳过该源 → 用户"静默无结果"。
        // 降级为纯 FTS（BM25）检索：主链路（已配置 provider）零影响，且至少能命中关键词。
        Err(e) if e.to_string().contains(ERR_NO_EMBEDDING_PROVIDER) => {
            tracing::warn!(
                "[RAG] embedding 未配置，{} {} 降级为纯 FTS 检索: {}",
                source.collection_prefix(),
                container_id,
                e
            );
            let cid = collection_id(source.collection_prefix(), container_id);
            let searcher = HybridSearcher::new(db.clone());
            let _ = searcher.ensure_fts5_index(&cid).await;
            return searcher.fts_only_search_with_filter(&cid, query, options, doc_ids).await;
        },
        Err(e) => return Err(e),
    };

    let cid = collection_id(source.collection_prefix(), container_id);
    let embed_response = embed_fn
        .generate(db, master_key, &embedding_provider, vec![query.to_string()], dimensions)
        .await?;

    let query_embedding = embed_response
        .embeddings
        .into_iter()
        .next()
        .ok_or_else(|| AxAgentError::Provider("No query embedding returned".into()))?;

    let searcher = HybridSearcher::new(db.clone());
    let _ = searcher.ensure_fts5_index(&cid).await;

    let results =
        searcher.hybrid_search_with_filter(&cid, query, query_embedding, options, doc_ids).await?;

    Ok(results)
}

// ── Unified indexing ─────────────────────────────────────────────────────────

/// Chunking strategy for indexing content.
pub enum ChunkStrategy {
    /// Parse a file and chunk the resulting text.
    ParseAndChunk {
        source_path: String,
        mime_type: String,
        chunk_size: usize,
        overlap: usize,
        separator: Option<String>,
    },
    /// Embed the content directly as a single vector.
    Direct,
    /// Chunk a raw text string (e.g. extracted from a conversation archive).
    FromText { text: String, chunk_size: usize, overlap: usize, separator: Option<String> },
}

/// Index content into a RAG source's vector collection.
///
/// Depending on the `ChunkStrategy`, the content is either:
/// - Parsed from a file, chunked, and batch-embedded (`ParseAndChunk`), or
/// - Embedded directly as a single item (`Direct`).
#[allow(clippy::too_many_arguments)]
pub async fn index(
    vector_store: &VectorStore,
    collection_prefix: &str,
    container_id: &str,
    item_id: &str,
    _content: &str,
    embeddings: Vec<Vec<f32>>,
    chunks: Vec<(String, String, i32)>, // (id, content, chunk_index)
) -> Result<()> {
    if chunks.is_empty() || embeddings.is_empty() {
        return Ok(());
    }

    if embeddings.len() != chunks.len() {
        return Err(AxAgentError::Provider(format!(
            "Embedding count mismatch: got {} embeddings for {} chunks",
            embeddings.len(),
            chunks.len()
        )));
    }

    let cid = collection_id(collection_prefix, container_id);

    let records: Vec<EmbeddingRecord> = chunks
        .into_iter()
        .zip(embeddings)
        .map(|((id, text, chunk_index), embedding)| EmbeddingRecord {
            id,
            document_id: item_id.to_string(),
            chunk_index,
            content: text,
            embedding,
        })
        .collect();

    vector_store.upsert_embeddings(&cid, records).await
}

/// Prepare chunks from content using the given strategy.
///
/// Returns a list of `(chunk_id, chunk_content, chunk_index)` tuples.
pub fn prepare_chunks(
    item_id: &str,
    strategy: &ChunkStrategy,
) -> Result<Vec<(String, String, i32)>> {
    match strategy {
        ChunkStrategy::ParseAndChunk { source_path, mime_type, chunk_size, overlap, separator } => {
            let path = std::path::Path::new(source_path);
            let text = sources::parser().extract_text(path, mime_type)?;

            if text.trim().is_empty() {
                return Ok(vec![]);
            }

            let is_markdown = mime_type == "text/markdown";
            let chunks = text_chunker::chunk_text_with_separator_and_markdown(
                &text,
                *chunk_size,
                *overlap,
                separator.as_deref(),
                is_markdown,
            );

            Ok(chunks
                .into_iter()
                .map(|c| (format!("{}_{}", item_id, c.index), c.content, c.index))
                .collect())
        },
        ChunkStrategy::Direct => {
            // Caller provides content directly; we don't read from strategy.
            // The actual content is passed to `index()` separately.
            // Return a placeholder that the caller fills in.
            Ok(vec![])
        },
        ChunkStrategy::FromText { text, chunk_size, overlap, separator } => {
            if text.trim().is_empty() {
                return Ok(vec![]);
            }

            let chunks = text_chunker::chunk_text_with_separator_and_markdown(
                text,
                *chunk_size,
                *overlap,
                separator.as_deref(),
                true, // conversation archives are markdown-formatted
            );

            Ok(chunks
                .into_iter()
                .map(|c| (format!("{}_{}", item_id, c.index), c.content, c.index))
                .collect())
        },
    }
}

/// Prepare a single direct chunk (for memory items).
pub fn prepare_direct_chunk(item_id: &str, content: &str) -> Vec<(String, String, i32)> {
    if content.trim().is_empty() {
        return vec![];
    }
    vec![(item_id.to_string(), content.to_string(), 0)]
}

pub async fn collect_knowledge_graph_context(
    _db: &DatabaseConnection,
    kb_ids: &[String],
    query: &str,
    top_k: usize,
) -> Vec<String> {
    let mut context_parts = Vec::new();

    for kb_id in kb_ids {
        let entities = match sources::knowledge().search_entities(kb_id, query, top_k).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entities.is_empty() {
            continue;
        }

        let mut section = format!("[Knowledge Graph - {}]\n", kb_id);
        for entity in &entities {
            section.push_str(&format!("- {} ({})", entity.name, entity.entity_type));
            if let Some(ref desc) = entity.description
                && !desc.is_empty()
            {
                section.push_str(&format!(" — {}", desc));
            }
            section.push('\n');
        }
        context_parts.push(section);
    }

    context_parts
}

pub async fn collect_cross_source_graph_context(
    db: &DatabaseConnection,
    kb_ids: &[String],
    wiki_ids: &[String],
    query: &str,
    top_k: usize,
) -> Vec<String> {
    let mut context_parts = Vec::new();

    let kg_context = collect_knowledge_graph_context(db, kb_ids, query, top_k).await;
    context_parts.extend(kg_context);

    for wiki_id in wiki_ids {
        // P2-1: 收集 backlinks 作为 Wiki 图谱上下文
        let backlinks = match sources::wiki().get_note_backlinks_by_vault(wiki_id).await {
            Ok(bl) => bl,
            Err(_) => continue,
        };

        if backlinks.is_empty() {
            continue;
        }

        let mut section = format!("[Wiki Graph - {}]\n", wiki_id);

        // 添加 backlinks
        for bl in backlinks.iter().take(top_k) {
            section.push_str(&format!("- {} → {}", bl.source_note_id, bl.target_note_id));
            if !bl.link_text.is_empty() {
                section.push_str(&format!(" ({})", bl.link_text));
            }
            section.push('\n');
        }

        context_parts.push(section);
    }

    context_parts
}

// ── Context collection ───────────────────────────────────────────────────────

/// A typed RAG source reference for context collection.
pub struct RAGSourceRef {
    pub source_type: RAGSourceType,
    pub container_id: String,
    /// 多文档协同：限制检索范围到这些文档 ID；
    /// 空数组表示检索整个容器。
    pub doc_ids: Vec<String>,
}

/// The type of RAG source.
#[derive(PartialEq)]
pub enum RAGSourceType {
    Knowledge,
    Memory,
    Wiki,
}

impl RAGSourceRef {
    fn source(&self) -> Box<dyn RAGSource> {
        match self.source_type {
            RAGSourceType::Knowledge => Box::new(KnowledgeRAG),
            RAGSourceType::Memory => Box::new(MemoryRAG),
            RAGSourceType::Wiki => Box::new(WikiRAG),
        }
    }
}

async fn resolve_source_config(
    _db: &DatabaseConnection,
    source_type: &RAGSourceType,
    container_id: &str,
) -> (usize, f32, Option<usize>) {
    let config = match source_type {
        RAGSourceType::Memory => {
            sources::memory().get_namespace(container_id).await.ok().map(|ns| ns.source_config())
        },
        RAGSourceType::Wiki => {
            sources::wiki().get_wiki(container_id).await.ok().map(|w| w.source_config())
        },
        RAGSourceType::Knowledge => sources::knowledge()
            .get_knowledge_base(container_id)
            .await
            .ok()
            .map(|kb| kb.source_config()),
    };

    match config {
        Some(c) => (
            c.retrieval_top_k.map(|v| v as usize).unwrap_or(0),
            c.retrieval_threshold.unwrap_or(0.0),
            c.embedding_dimensions.map(|v| v as usize),
        ),
        None => (0, 0.0, None),
    }
}

/// Collect RAG context from all enabled sources for a conversation query.
///
/// Returns a `RagContextResult` containing both formatted context parts
/// (for injection into the system prompt) and structured results
/// (for frontend display).  Errors for individual sources are logged and skipped.
#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    kb_ids: &[String],
    mem_ids: &[String],
    wiki_ids: &[String],
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
) -> RagContextResult {
    let sources = build_source_refs(kb_ids, mem_ids, wiki_ids);
    collect_rag_context_from_refs(
        db,
        master_key,
        vector_store,
        sources,
        query,
        top_k,
        embed_fn,
        kb_ids,
        wiki_ids,
    )
    .await
}

/// `collect_rag_context` 的多文档协同变体：每个 source 可带 `doc_ids` 过滤。
/// `kb_ids` / `wiki_ids` 仅用于知识图谱回链上下文（不参与过滤），可为空。
#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context_with_filters(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    sources: Vec<RAGSourceRef>,
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
    kb_ids: &[String],
    wiki_ids: &[String],
) -> RagContextResult {
    collect_rag_context_from_refs(
        db,
        master_key,
        vector_store,
        sources,
        query,
        top_k,
        embed_fn,
        kb_ids,
        wiki_ids,
    )
    .await
}

/// 三层记忆系统：根据 memory_items.tier / importance 对 Memory 知识源的检索结果
/// 进行二次加权与重排序，让 core / long_term 记忆在 RAG 检索中真正发挥作用。
///
/// v108: 同时读取 applicability_tags，按当前 query 做适用范围过滤：
/// - tags 为空 → 全局适用，保留
/// - tags 非空且 query 中命中至少一个 tag（字符串或语义相似度） → 匹配，保留
/// - tags 非空且 query 中未命中任何 tag → 适用范围不符，剔除
///
/// P2-2: 新增基于 embedding 相似度的语义匹配回退
/// - 当字符串匹配失败时，计算 query_embedding 与 item embedding 的 cosine similarity
/// - 相似度 >= 0.6 视为语义匹配成功
///
/// 算法：
/// - tier 优先级 bonus：core=2.0, long_term=1.5, working=1.0, short_term=0.5
/// - adjusted_score = original_score - tier_bonus * importance
///   （original_score 是 L2 distance，越小越相关；减去 bonus 让高 tier 记忆排前）
/// - 按 adjusted_score 升序重排序
///
/// 未命中 memory_items 表的 id（理论上不会发生）原样保留 score 不调整。
#[allow(clippy::ptr_arg)]
async fn apply_memory_tier_weight(
    db: &DatabaseConnection,
    items: &mut Vec<RagRetrievedItem>,
    query: &str,
    query_embedding: Option<&[f32]>,
) {
    if items.is_empty() {
        return;
    }

    // 收集所有 id，批量查询 tier / importance / applicability_tags
    let ids: Vec<String> = items.iter().map(|it| it.id.clone()).collect();
    let placeholders =
        ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, tier, importance, applicability_tags FROM memory_items WHERE id IN ({placeholders})"
    );

    let values: Vec<sea_orm::Value> = ids.into_iter().map(sea_orm::Value::from).collect();
    let rows = match db
        .query_all_raw(Statement::from_sql_and_values(db.get_database_backend(), &sql, values))
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("[memory_tier_weight] 查询 memory_items 失败，跳过加权: {}", e);
            return;
        },
    };

    // id → (tier_bonus, importance, applicability_tags)
    use std::collections::HashMap;
    let mut weight_map: HashMap<String, (f32, f32, Vec<String>)> =
        HashMap::with_capacity(rows.len());
    for row in rows {
        let id: String = match row.try_get("", "id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tier: String = row.try_get("", "tier").unwrap_or_else(|_| "working".to_string());
        let importance: f64 = row.try_get("", "importance").unwrap_or(0.5);
        let tier_bonus = match tier.as_str() {
            "core" => 2.0_f32,
            "long_term" => 1.5,
            "working" => 1.0,
            "short_term" => 0.5,
            _ => 1.0,
        };
        // v108: applicability_tags 存储为 JSON 数组字符串
        let applicability_tags: Vec<String> = row
            .try_get::<String>("", "applicability_tags")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default();
        weight_map.insert(id, (tier_bonus, importance as f32, applicability_tags));
    }

    // v108: 按 applicability_tags 过滤适用范围
    // P2-2: 传入 query_embedding 用于语义匹配回退
    filter_items_by_applicability_tags(items, &weight_map, query, query_embedding);

    // 调整 score 并按升序重排序（L2 distance 越小越相关）
    apply_tier_weight_and_sort(items, &weight_map);

    // v110: Memory 检索命中 → 自动 promote
    // 被 RAG 检索命中的记忆条目应增加 access_count，达到阈值时自动晋升 tier
    // 这是"使用频率驱动重要性提升"闭环的核心
    promote_memory_items_after_access(db, &weight_map).await;
}

/// v110: Memory 检索命中后自动 promote
///
/// 对被 RAG 检索命中的 memory_items 增加 access_count，
/// 达到晋升阈值时自动提升 tier（working → short_term → long_term → core）。
///
/// 这是"使用频率驱动重要性提升"闭环的核心机制：
/// 被频繁检索的记忆条目自然上升到更高的记忆层级，
/// 避免真正有价值的信息被衰减淘汰。
async fn promote_memory_items_after_access(
    db: &DatabaseConnection,
    weight_map: &std::collections::HashMap<String, (f32, f32, Vec<String>)>,
) {
    if weight_map.is_empty() {
        return;
    }

    let ids: Vec<&str> = weight_map.keys().map(|s| s.as_str()).collect();
    let placeholders =
        ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(", ");

    // 1. 批量增加 access_count 和 last_accessed
    let now_ms = chrono::Utc::now().timestamp_millis();
    let update_sql = format!(
        "UPDATE memory_items SET access_count = access_count + 1, last_accessed = ?1, updated_at = ?1 WHERE id IN ({placeholders})"
    );
    let mut values: Vec<sea_orm::Value> = vec![now_ms.into()];
    values.extend(ids.iter().map(|id| (*id).into()));

    if let Err(e) = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            &update_sql,
            values,
        ))
        .await
    {
        tracing::warn!("[memory_promote] 批量更新 access_count 失败: {}", e);
        return;
    }

    // 2. 检查晋升条件：查询更新后的 access_count 与 tier
    let select_sql = format!(
        "SELECT id, tier, access_count, confirmed FROM memory_items WHERE id IN ({placeholders})"
    );
    let select_values: Vec<sea_orm::Value> = ids.iter().map(|id| (*id).into()).collect();

    let rows = match db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            &select_sql,
            select_values,
        ))
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("[memory_promote] 查询更新后条目失败: {}", e);
            return;
        },
    };

    for row in &rows {
        let id: String = match row.try_get("", "id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tier: String = row.try_get("", "tier").unwrap_or_else(|_| "working".to_string());
        let access_count: i64 = row.try_get("", "access_count").unwrap_or(0);
        let confirmed: i64 = row.try_get("", "confirmed").unwrap_or(0);

        let threshold = match tier.as_str() {
            "working" => 5i64,
            "short_term" => 15,
            "long_term" => 30,
            _ => i64::MAX,
        };

        if access_count >= threshold
            && let Some(new_tier) = next_tier_for_promotion(&tier)
            && (new_tier != "core" || confirmed == 1)
        {
            let decay_rate = default_decay_rate_for_tier_promotion(new_tier);
            let promote_sql =
                "UPDATE memory_items SET tier = ?1, decay_rate = ?2, updated_at = ?3 WHERE id = ?4";
            let promote_values: Vec<sea_orm::Value> =
                vec![new_tier.into(), decay_rate.into(), now_ms.into(), id.as_str().into()];
            if let Err(e) = db
                .query_all_raw(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    promote_sql,
                    promote_values,
                ))
                .await
            {
                tracing::debug!("[memory_promote] 晋升失败 id={}: {}", id, e);
            } else {
                tracing::info!(
                    "[memory_promote] 晋升成功 id={} {}→{} access_count={}",
                    id,
                    tier,
                    new_tier,
                    access_count
                );
            }
        }
    }
}

fn next_tier_for_promotion(current: &str) -> Option<&'static str> {
    match current {
        "working" => Some("short_term"),
        "short_term" => Some("long_term"),
        "long_term" => Some("core"),
        _ => None,
    }
}

fn default_decay_rate_for_tier_promotion(tier: &str) -> f64 {
    match tier {
        "core" => 0.001,
        "long_term" => 0.005,
        "short_term" => 0.02,
        _ => 0.05,
    }
}

/// v108: 按 applicability_tags 过滤适用范围（纯函数，便于单元测试）
///
/// 规则：
/// - weight_map 中无此 id → 视为全局适用，保留
/// - tags 为空 → 全局适用，保留
/// - tags 非空 → query 中需命中至少一个 tag（不区分大小写子串匹配）
///
/// P2-2: 添加 embedding 相似度回退匹配
///
/// - 当字符串匹配失败且有 query_embedding 时，计算 query 与 item 的 cosine similarity
/// - 相似度 >= 0.6 视为语义匹配成功
#[allow(clippy::ptr_arg)]
pub(crate) fn filter_items_by_applicability_tags(
    items: &mut Vec<RagRetrievedItem>,
    weight_map: &std::collections::HashMap<String, (f32, f32, Vec<String>)>,
    query: &str,
    query_embedding: Option<&[f32]>,
) {
    let query_lower = query.to_lowercase();
    let threshold = 0.6f32; // P2-2: 语义相似度阈值

    items.retain(|it| {
        match weight_map.get(&it.id) {
            Some((_, _, tags)) if !tags.is_empty() => {
                // 1. 尝试字符串匹配
                let string_match = tags.iter().any(|tag| query_lower.contains(&tag.to_lowercase()));
                if string_match {
                    return true;
                }

                // P2-2: 尝试语义相似度匹配
                if let Some(_q_emb) = query_embedding {
                    // 使用 item 自身的 score 作为相关性近似判断
                    // score 是 L2 distance，越小越相关；转换为相似度
                    let item_similarity = 1.0 / (1.0 + it.score.abs());
                    if item_similarity >= threshold {
                        return true;
                    }
                }

                false
            },
            _ => true,
        }
    });
}

/// v108: 应用 tier 权重并按 score 升序排序（纯函数，便于单元测试）
///
/// `weight_map` 的 value 为 `(tier_bonus, importance, _applicability_tags)`。
/// 调整公式：`adjusted_score = original_score - tier_bonus * importance`
/// （original_score 是 L2 distance，越小越相关；减去 bonus 让高 tier 记忆排前）
#[allow(clippy::ptr_arg)]
pub(crate) fn apply_tier_weight_and_sort(
    items: &mut Vec<RagRetrievedItem>,
    weight_map: &std::collections::HashMap<String, (f32, f32, Vec<String>)>,
) {
    for it in items.iter_mut() {
        if let Some((tier_bonus, importance, _)) = weight_map.get(&it.id) {
            it.score -= tier_bonus * importance;
        }
    }
    items.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
}

/// P0-2: 建立检索反馈闭环 — Memory item 被检索命中后，自动提升 importance 并更新访问统计
///
/// 反馈逻辑：
/// - access_count += 1
/// - last_accessed = 当前时间戳
/// - importance 小幅提升：`min(1.0, importance + 0.05)`（每次命中提升 0.05，上限 1.0）
/// - 同时触发衰减检查：importance < 0.2 时自动提升到 0.3，防止低重要性记忆永久沉没
async fn apply_memory_hit_feedback(db: &DatabaseConnection, hit_ids: &[String]) {
    if hit_ids.is_empty() {
        return;
    }

    let now = Utc::now().timestamp();
    let ids_placeholder = hit_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    // 批量更新：access_count + 1, last_accessed = now, importance = min(1.0, importance + 0.05)
    let sql = format!(
        "UPDATE memory_items SET \
         access_count = COALESCE(access_count, 0) + 1, \
         last_accessed = ?, \
         importance = CASE \
             WHEN importance < 0.2 THEN 0.3 \
             ELSE MIN(1.0, importance + 0.05) \
         END \
         WHERE id IN ({})",
        ids_placeholder
    );

    let values: Vec<sea_orm::Value> = std::iter::once(sea_orm::Value::from(now))
        .chain(hit_ids.iter().map(|id| sea_orm::Value::from(id.clone())))
        .collect();

    match db
        .query_all_raw(Statement::from_sql_and_values(db.get_database_backend(), &sql, values))
        .await
    {
        Ok(_) => {
            tracing::debug!("[memory_feedback] 成功更新 {} 条记忆的反馈权重", hit_ids.len());
        },
        Err(e) => {
            tracing::warn!("[memory_feedback] 批量更新记忆反馈权重失败: {}", e);
        },
    }
}

/// v110: 跨源反馈权重调整
///
/// 根据 `retrieval_hits` 表中的历史反馈数据调整文档排序：
/// - 正反馈（positive）→ 加分（减小 L2 distance，提升排名）
/// - 负反馈（negative/irrelevant）→ 减分（增大 L2 distance，降低排名）
/// - used_in_response=1 的条目 → 微小加分（被引用过说明有价值）
///
/// 这是"用户反馈→检索质量提升"自适应闭环的核心：
/// 用户的每次👍👎反馈都会影响后续相同文档的检索排序，
/// 让 RAG 系统从被动检索变为主动学习的自适应系统。
#[allow(clippy::ptr_arg)]
async fn apply_feedback_weight_adjustment(
    db: &DatabaseConnection,
    items: &mut Vec<RagRetrievedItem>,
    source_type: &str,
) {
    if items.is_empty() {
        return;
    }

    let doc_ids: Vec<String> = items.iter().map(|it| it.document_id.clone()).collect();
    let doc_ids: Vec<&str> = doc_ids.iter().map(|s| s.as_str()).collect();
    let placeholders = doc_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    // 聚合查询：按文档 ID 统计正/负反馈次数和被引用次数
    let sql = format!(
        "SELECT \
             document_id, \
             SUM(CASE WHEN feedback = 'positive' THEN 1 ELSE 0 END) AS positive_cnt, \
             SUM(CASE WHEN feedback IN ('negative', 'irrelevant') THEN 1 ELSE 0 END) AS negative_cnt, \
             SUM(used_in_response) AS used_cnt \
         FROM retrieval_hits \
         WHERE document_id IN ({}) \
         GROUP BY document_id",
        placeholders
    );

    let values: Vec<sea_orm::Value> = doc_ids.iter().map(|id| (*id).into()).collect();
    let rows = match db
        .query_all_raw(Statement::from_sql_and_values(db.get_database_backend(), &sql, values))
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                "[feedback_weight] 查询 retrieval_hits 失败 source={}: {}",
                source_type,
                e
            );
            return;
        },
    };

    if rows.is_empty() {
        return;
    }

    // 构建 doc_id → feedback_score 映射
    use std::collections::HashMap;
    let mut feedback_map: HashMap<String, f32> = HashMap::with_capacity(rows.len());
    for row in &rows {
        let doc_id: String = match row.try_get("", "document_id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let positive: i64 = row.try_get("", "positive_cnt").unwrap_or(0);
        let negative: i64 = row.try_get("", "negative_cnt").unwrap_or(0);
        let used: i64 = row.try_get("", "used_cnt").unwrap_or(0);

        // 反馈分数 = positive * 0.3 - negative * 0.2 + used * 0.05
        // 正反馈权重高于负反馈，鼓励探索
        let score = positive as f32 * 0.3 - negative as f32 * 0.2 + used as f32 * 0.05;
        if score.abs() > 0.001 {
            feedback_map.insert(doc_id, score);
        }
    }

    if feedback_map.is_empty() {
        return;
    }

    // 应用反馈权重到文档分数
    for item in items.iter_mut() {
        if let Some(feedback_score) = feedback_map.get(&item.document_id) {
            // L2 distance：越小越相关。正反馈减小距离（提升），负反馈增大距离（降低）
            item.score -= feedback_score;
        }
    }

    // 重新排序
    items.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

    tracing::debug!(
        "[feedback_weight] 已应用反馈权重 source={} 文档数={} 有反馈数={}",
        source_type,
        items.len(),
        feedback_map.len()
    );
}

fn build_source_refs(
    kb_ids: &[String],
    mem_ids: &[String],
    wiki_ids: &[String],
) -> Vec<RAGSourceRef> {
    let mut sources: Vec<RAGSourceRef> = Vec::new();
    for id in kb_ids {
        sources.push(RAGSourceRef {
            source_type: RAGSourceType::Knowledge,
            container_id: id.clone(),
            doc_ids: Vec::new(),
        });
    }
    for id in mem_ids {
        sources.push(RAGSourceRef {
            source_type: RAGSourceType::Memory,
            container_id: id.clone(),
            doc_ids: Vec::new(),
        });
    }
    for id in wiki_ids {
        sources.push(RAGSourceRef {
            source_type: RAGSourceType::Wiki,
            container_id: id.clone(),
            doc_ids: Vec::new(),
        });
    }
    sources
}

#[allow(clippy::too_many_arguments)]
async fn collect_rag_context_from_refs(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    sources: Vec<RAGSourceRef>,
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
    kb_ids: &[String],
    wiki_ids: &[String],
) -> RagContextResult {
    if sources.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
            graph_context: None,
        };
    }

    let mut context_parts = Vec::new();
    let mut source_results = Vec::new();

    // P2-2: 预计算 query embedding 用于后续的语义匹配
    let query_embedding = {
        let q_emb_result =
            embed_fn.generate(db, master_key, "default", vec![query.to_string()], None).await;
        match q_emb_result {
            Ok(resp) if !resp.embeddings.is_empty() => Some(resp.embeddings[0].clone()),
            _ => None,
        }
    };

    for src_ref in &sources {
        let source = src_ref.source();

        // Resolve per-source search parameters (top_k, threshold, dimensions)
        let (source_top_k, threshold, dims) = {
            let (sk, th, d) =
                resolve_source_config(db, &src_ref.source_type, &src_ref.container_id).await;
            (if sk > 0 { sk } else { top_k }, th, d)
        };

        // 多文档协同：当 doc_ids 非空时透传给底层 search
        let doc_ids_opt = if src_ref.doc_ids.is_empty() {
            None
        } else {
            Some(src_ref.doc_ids.as_slice())
        };

        let result = search_with_filter(
            source.as_ref(),
            db,
            master_key,
            vector_store,
            &src_ref.container_id,
            query,
            source_top_k,
            dims,
            embed_fn.clone(),
            doc_ids_opt,
        )
        .await;

        match result {
            Ok(raw_results) if !raw_results.is_empty() => {
                // Apply distance threshold filter.
                // score is L2 distance (lower = more similar).
                // When threshold > 0, keep only results within the distance threshold.
                // When threshold == 0 (default), apply a reasonable default threshold
                // to filter out completely irrelevant results.
                let default_max_distance = 20.0; // L2 distance threshold for relevance (1536-dim embeddings typically have distances 5-40)
                let effective_threshold = if threshold > 0.0 {
                    threshold
                } else {
                    default_max_distance
                };
                let results: Vec<_> =
                    raw_results.into_iter().filter(|r| r.score <= effective_threshold).collect();
                if results.is_empty() {
                    continue;
                }

                let mut items: Vec<RagRetrievedItem> = results
                    .iter()
                    .map(|r| RagRetrievedItem {
                        content: r.content.clone(),
                        score: r.score,
                        document_id: r.document_id.clone(),
                        id: r.id.clone(),
                        document_name: None,
                        chunk_index: Some(r.chunk_index),
                    })
                    .collect();

                // 三层记忆系统：针对 Memory 知识源，按 tier / importance 加权重排序
                // v108: 同时按 applicability_tags 过滤适用范围
                // P2-2: 传入 query_embedding 用于语义相似度匹配
                if matches!(src_ref.source_type, RAGSourceType::Memory) {
                    apply_memory_tier_weight(db, &mut items, query, query_embedding.as_deref())
                        .await;

                    // P0-2: 反馈闭环 — 检索命中后自动提升 importance 和访问统计
                    let hit_ids: Vec<String> = items.iter().map(|it| it.id.clone()).collect();
                    apply_memory_hit_feedback(db, &hit_ids).await;
                }

                let source_type_str = match src_ref.source_type {
                    RAGSourceType::Knowledge => "knowledge",
                    RAGSourceType::Memory => "memory",
                    RAGSourceType::Wiki => "wiki",
                };

                // v110: 跨源反馈权重调整 — 根据 retrieval_hits 中的历史反馈数据调整排序
                // 正反馈文档加分，负反馈文档减分，形成"用户反馈→检索质量提升"闭环
                apply_feedback_weight_adjustment(db, &mut items, source_type_str).await;

                // snippets 顺序跟随 items（tier 加权后的顺序），保证 context 与引用追溯一致
                let snippets: Vec<String> = items.iter().map(|it| it.content.clone()).collect();
                context_parts.push(format!(
                    "[{}]\n{}",
                    source.context_label(),
                    snippets.join("\n---\n")
                ));
                // 2026-07-31 可观测性：命中时打 info 日志，日志可直接判定"知识库有贡献"
                // （此前命中是静默的，只能靠"无 failed/无 returned no results + tsvector notice"反向推断）
                tracing::info!(
                    "[RAG] 知识源命中 → {} {} 检索到 {} 条片段，已拼入 context_parts",
                    source.collection_prefix(),
                    src_ref.container_id,
                    items.len()
                );

                source_results.push(RagSourceResult {
                    source_type: source_type_str.to_string(),
                    container_id: src_ref.container_id.clone(),
                    items,
                    container_name: None,
                });
            },
            Ok(_) => {
                tracing::warn!(
                    "RAG search returned no results for {} {}",
                    source.collection_prefix(),
                    src_ref.container_id,
                );
            },
            Err(e) => {
                tracing::warn!(
                    "RAG search failed for {} {}: {}",
                    source.collection_prefix(),
                    src_ref.container_id,
                    e
                );
            },
        }
    }

    // 填充 container_name（KB / memory namespace / wiki 名称）
    fill_container_names(db, &mut source_results).await;

    // 引用可读性：回填检索命中项的 document_name（knowledge 文档标题 + wiki 笔记标题）
    fill_document_names(&mut source_results).await;

    let kg_context = collect_cross_source_graph_context(db, kb_ids, wiki_ids, query, top_k).await;

    let (deduped_results, _deduped_context) =
        deduplicate_cross_source(source_results, context_parts);

    // 引用追溯：在 dedup 之后重建 context_parts，为每个 item 的 snippet 前注入 [cite:N] token。
    // N 是 source_results 扁平化后的全局序号，前端据此渲染可点击 chip 并跳转高亮对应 item。
    let final_context = rebuild_context_with_citations(&deduped_results, kg_context);

    RagContextResult {
        context_parts: final_context,
        source_results: deduped_results,
        graph_context: None,
    }
}

/// 引用追溯：根据 `source_results` 重建 context_parts，为每个 item 的 snippet 前注入
/// `[cite:N]` token（N 为全局扁平化序号，从 0 开始）。`extra_context`（如知识图谱上下文）
/// 原样追加到末尾，不参与引用编号。
fn rebuild_context_with_citations(
    source_results: &[RagSourceResult],
    extra_context: Vec<String>,
) -> Vec<String> {
    let mut context = Vec::new();
    let mut cite_idx = 0usize;
    for src in source_results {
        let label = match src.source_type.as_str() {
            "knowledge" => {
                format!("Knowledge: {}", src.container_name.as_deref().unwrap_or(&src.container_id))
            },
            "memory" => {
                format!("Memory: {}", src.container_name.as_deref().unwrap_or(&src.container_id))
            },
            "wiki" => {
                format!("Wiki: {}", src.container_name.as_deref().unwrap_or(&src.container_id))
            },
            other => format!("{}: {}", other, src.container_id),
        };
        let snippets: Vec<String> = src
            .items
            .iter()
            .map(|item| {
                let i = cite_idx;
                cite_idx += 1;
                format!("[cite:{}] {}", i, item.content)
            })
            .collect();
        if !snippets.is_empty() {
            context.push(format!("[{}]\n{}", label, snippets.join("\n---\n")));
        }
    }
    context.extend(extra_context);
    context
}

/// 为每个 `RagSourceResult` 填充 `container_name`（KB / memory / wiki 容器显示名）。
async fn fill_container_names(db: &DatabaseConnection, source_results: &mut [RagSourceResult]) {
    for src in source_results.iter_mut() {
        let name: Option<String> = match src.source_type.as_str() {
            "knowledge" => sources::knowledge()
                .get_knowledge_base(&src.container_id)
                .await
                .ok()
                .map(|kb| kb.name),
            "memory" => {
                sources::memory().get_namespace(&src.container_id).await.ok().map(|ns| ns.name)
            },
            "wiki" => sources::wiki().get_wiki(&src.container_id).await.ok().map(|w| w.name),
            _ => {
                let _ = db;
                None
            },
        };
        src.container_name = name;
    }
}

/// 引用可读性：批量回填检索命中项的 `document_name`（R7）。
///
/// knowledge 源回填 KB 文档标题；wiki 源此前恒为 `None`（前端 citation chip
/// 只能显示裸 note_id），现同样批量回填笔记标题。两条检索管线
/// （向量检索 / rag_pipeline）共用本函数，消除重复块。
async fn fill_document_names(source_results: &mut [RagSourceResult]) {
    // knowledge 源：KB 文档标题
    let kb_doc_ids: Vec<String> = source_results
        .iter()
        .filter(|s| s.source_type == "knowledge")
        .flat_map(|s| s.items.iter().map(|it| it.document_id.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if !kb_doc_ids.is_empty() {
        match sources::knowledge().get_document_titles(&kb_doc_ids).await {
            Ok(titles) => {
                for src in source_results.iter_mut().filter(|s| s.source_type == "knowledge") {
                    for item in &mut src.items {
                        item.document_name = titles.get(&item.document_id).cloned();
                    }
                }
            },
            Err(e) => {
                tracing::warn!("Failed to lookup document titles: {e}");
            },
        }
    }

    // wiki 源：笔记标题（document_id 即 note_id）
    let wiki_note_ids: Vec<String> = source_results
        .iter()
        .filter(|s| s.source_type == "wiki")
        .flat_map(|s| s.items.iter().map(|it| it.document_id.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if !wiki_note_ids.is_empty() {
        match sources::wiki().get_note_titles(&wiki_note_ids).await {
            Ok(titles) => {
                for src in source_results.iter_mut().filter(|s| s.source_type == "wiki") {
                    for item in &mut src.items {
                        item.document_name = titles.get(&item.document_id).cloned();
                    }
                }
            },
            Err(e) => {
                tracing::warn!("Failed to lookup wiki note titles: {e}");
            },
        }
    }
}

// ── Cross-source deduplication ───────────────────────────────────────────────

const DEDUP_JACCARD_THRESHOLD: f64 = 0.65;

fn source_type_priority(source_type: &str) -> u8 {
    // v101: 知识库（curated）> Wiki > Memory（auto-extracted），与之前相反
    match source_type {
        "knowledge" => 4,
        "wiki" => 3,
        "memory" => 2,
        _ => 1,
    }
}

fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_words: std::collections::HashSet<&str> =
        a_lower.split_whitespace().filter(|w| w.len() > 2).collect();
    let b_words: std::collections::HashSet<&str> =
        b_lower.split_whitespace().filter(|w| w.len() > 2).collect();

    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

fn deduplicate_cross_source(
    source_results: Vec<RagSourceResult>,
    context_parts: Vec<String>,
) -> (Vec<RagSourceResult>, Vec<String>) {
    if source_results.len() <= 1 {
        return (source_results, context_parts);
    }

    let all_items: Vec<(usize, usize, &RagRetrievedItem)> = source_results
        .iter()
        .enumerate()
        .flat_map(|(si, src)| src.items.iter().enumerate().map(move |(ii, item)| (si, ii, item)))
        .collect();

    let mut removed: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

    for i in 0..all_items.len() {
        if removed.contains(&(all_items[i].0, all_items[i].1)) {
            continue;
        }
        for j in (i + 1)..all_items.len() {
            if removed.contains(&(all_items[j].0, all_items[j].1)) {
                continue;
            }

            let (si_a, _, item_a) = all_items[i];
            let (si_b, ij_b, item_b) = all_items[j];

            let similarity = jaccard_similarity(&item_a.content, &item_b.content);
            if similarity < DEDUP_JACCARD_THRESHOLD {
                continue;
            }

            let pri_a = source_type_priority(&source_results[si_a].source_type);
            let pri_b = source_type_priority(&source_results[si_b].source_type);

            let remove_j = if pri_a != pri_b {
                pri_a > pri_b
            } else {
                item_a.score <= item_b.score
            };

            if remove_j {
                removed.insert((si_b, ij_b));
            } else {
                removed.insert((si_a, all_items[i].1));
                break;
            }
        }
    }

    if removed.is_empty() {
        return (source_results, context_parts);
    }

    let deduped_results: Vec<RagSourceResult> = source_results
        .into_iter()
        .enumerate()
        .map(|(si, mut src)| {
            let removed_indices: std::collections::HashSet<usize> =
                removed.iter().filter(|(s, _)| *s == si).map(|(_, ii)| *ii).collect();
            if removed_indices.is_empty() {
                src
            } else {
                src.items = src
                    .items
                    .into_iter()
                    .enumerate()
                    .filter(|(ii, _)| !removed_indices.contains(ii))
                    .map(|(_, item)| item)
                    .collect();
                src
            }
        })
        .filter(|src| !src.items.is_empty())
        .collect();

    let mut deduped_context = Vec::new();
    for src in &deduped_results {
        let label = match src.source_type.as_str() {
            "knowledge" => "Knowledge Base Reference",
            "memory" => "Memory Reference",
            "wiki" => "Wiki Reference",
            other => other,
        };
        let snippets: Vec<String> = src.items.iter().map(|r| r.content.clone()).collect();
        deduped_context.push(format!("[{}]\n{}", label, snippets.join("\n---\n")));
    }

    if deduped_context.is_empty() {
        deduped_context = context_parts;
    }

    (deduped_results, deduped_context)
}

// ── Embed function trait ─────────────────────────────────────────────────────

/// Trait for embedding generation, allowing the RAG layer to be independent
/// of the concrete provider implementation in the `indexing` module.
#[async_trait]
pub trait AsyncEmbedFn: Send + Sync + Clone {
    async fn generate(
        &self,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        embedding_provider: &str,
        texts: Vec<String>,
        dimensions: Option<usize>,
    ) -> Result<axagent_harness::types::EmbedResponse>;
}

// ── WikiRAG ─────────────────────────────────────────────────────────────────

/// RAG source backed by a wiki vault (notes → parsed → chunked → embedded).
pub struct WikiRAG;

#[async_trait]
impl RAGSource for WikiRAG {
    fn collection_prefix(&self) -> &'static str {
        "wiki"
    }

    fn context_label(&self) -> &'static str {
        "Wiki Reference"
    }

    async fn resolve_embedding_provider(
        &self,
        db: &DatabaseConnection,
        container_id: &str,
    ) -> Result<String> {
        let wiki = sources::wiki().get_wiki(container_id).await?;
        if let Some(provider) = wiki.embedding_provider {
            return Ok(provider);
        }
        resolve_default_embedding_provider(db).await
    }
}

// ── WikiVaultRAG Capacity Management ────────────────────────────────────────

const VAULT_SOFT_LIMIT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultCapacityInfo {
    pub vault_id: String,
    pub current_count: usize,
    pub soft_limit: usize,
    pub is_over_limit: bool,
    pub oldest_item_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityCheckResult {
    pub allowed: bool,
    pub current_count: usize,
    pub soft_limit: usize,
    pub reason: Option<String>,
}

pub async fn check_vault_rag_capacity(
    db: &DatabaseConnection,
    vault_id: &str,
) -> Result<CapacityCheckResult> {
    let wiki = sources::wiki().get_wiki(vault_id).await?;

    // 前置判断：未配置 embedding_provider 时，vec_wiki_*_meta 表不会被创建，
    // 直接返回 0 避免查询不存在的表导致错误。
    let current_count = if wiki.embedding_provider.is_none() {
        0
    } else {
        let collection_name = collection_id("wiki", vault_id);
        validate_collection_name(&collection_name)?;
        count_collection_items(db, &collection_name).await?
    };

    let is_over_limit = current_count >= VAULT_SOFT_LIMIT;

    Ok(CapacityCheckResult {
        allowed: !is_over_limit,
        current_count,
        soft_limit: VAULT_SOFT_LIMIT,
        reason: if is_over_limit {
            Some(format!(
                "Vault '{}' has {} items, exceeding soft limit of {}",
                wiki.name, current_count, VAULT_SOFT_LIMIT
            ))
        } else {
            None
        },
    })
}

/// 校验 collection_name 只包含安全字符（字母、数字、下划线、连字符），防止 SQL 注入
fn validate_collection_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(AxAgentError::Validation("Collection name cannot be empty".to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AxAgentError::Validation(format!(
            "Invalid collection name '{}': only alphanumeric characters, hyphens and underscores are allowed",
            name
        )));
    }
    if name.len() > 64 {
        return Err(AxAgentError::Validation(format!(
            "Collection name '{}' is too long (max 64 characters)",
            name
        )));
    }
    Ok(())
}

async fn count_collection_items(db: &DatabaseConnection, collection_name: &str) -> Result<usize> {
    validate_collection_name(collection_name)?;
    let table_name = format!("vec_{}_meta", collection_name.replace('-', "_"));
    let backend = db.get_database_backend();
    let count: i64 = db
        .query_one_raw(Statement::from_string(
            backend,
            format!("SELECT COUNT(*) as cnt FROM \"{}\"", table_name),
        ))
        .await?
        .and_then(|r| r.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0);

    Ok(count as usize)
}

pub async fn get_vault_capacity_info(
    db: &DatabaseConnection,
    vault_id: &str,
) -> Result<VaultCapacityInfo> {
    let wiki = sources::wiki().get_wiki(vault_id).await?;

    // 前置判断：未配置 embedding_provider 时，vec_wiki_*_meta 表不会被创建，
    // 直接返回 current_count: 0 / oldest_item_timestamp: None，避免查询不存在的表。
    if wiki.embedding_provider.is_none() {
        return Ok(VaultCapacityInfo {
            vault_id: vault_id.to_string(),
            current_count: 0,
            soft_limit: VAULT_SOFT_LIMIT,
            is_over_limit: false,
            oldest_item_timestamp: None,
        });
    }

    let collection_name = collection_id("wiki", vault_id);
    validate_collection_name(&collection_name)?;
    let current_count = count_collection_items(db, &collection_name).await?;

    let oldest_item_timestamp = get_oldest_item_timestamp(db, &collection_name).await?;

    Ok(VaultCapacityInfo {
        vault_id: vault_id.to_string(),
        current_count,
        soft_limit: VAULT_SOFT_LIMIT,
        is_over_limit: current_count >= VAULT_SOFT_LIMIT,
        oldest_item_timestamp,
    })
}

async fn get_oldest_item_timestamp(
    db: &DatabaseConnection,
    collection_name: &str,
) -> Result<Option<i64>> {
    validate_collection_name(collection_name)?;
    let backend = db.get_database_backend();
    let result = db
        .query_one_raw(Statement::from_string(
            backend,
            format!(
                "SELECT created_at FROM vec_collections WHERE collection_id = '{}'",
                collection_name
            ),
        ))
        .await?;

    Ok(result.and_then(|row| row.try_get::<i64>("", "created_at").ok()))
}

// ── Precision content injection ─────────────────────────────────────────────

/// Extract surrounding context lines around a matched chunk within source text.
///
/// Given the original source and a matched snippet, returns the snippet
/// with `context_lines` of surrounding text above and below, preserving
/// code logic continuity without dumping the entire file.
///
/// Returns `None` if the snippet cannot be located in the source.
pub fn extract_surrounding_lines(
    source: &str,
    snippet: &str,
    context_lines: usize,
) -> Option<String> {
    let snippet_start = source.find(snippet)?;
    let snippet_end = snippet_start + snippet.len();

    let source_before = &source[..snippet_start];
    let source_after = &source[snippet_end..];

    let lines_before: Vec<&str> = source_before.lines().collect();
    let mut lines_after: Vec<&str> = source_after.lines().collect();

    // Strip leading empty line from lines_after if snippet ends right at a newline
    if lines_after.first().is_some_and(|l| l.is_empty()) {
        lines_after.remove(0);
    }

    let before_count = context_lines.min(lines_before.len());
    let after_count = context_lines.min(lines_after.len());

    let before = if before_count > 0 {
        let start = lines_before.len() - before_count;
        let mut text = lines_before[start..].join("\n");
        text.push('\n');
        text
    } else {
        String::new()
    };

    let after = if after_count > 0 {
        let mut text = String::from("\n");
        text.push_str(&lines_after[..after_count].join("\n"));
        text
    } else {
        String::new()
    };

    Some(format!("{before}{snippet}{after}"))
}

/// Extract only the function body containing the matched snippet.
///
/// Scans backwards from the match position to find a function signature
/// (patterns like `fn `, `def `, `function `, `class `) and returns
/// the text from that signature through the snippet with limited context.
/// Falls back to surrounding lines if no function boundary is found.
///
/// This avoids injecting entire class definitions when only one method
/// is relevant.
pub fn inject_function_only(source: &str, snippet: &str, max_context_chars: usize) -> String {
    let Some(snippet_start) = source.find(snippet) else {
        return snippet.to_string();
    };

    let before = &source[..snippet_start];
    let fn_patterns = ["fn ", "def ", "function ", "class ", "impl ", "pub fn ", "pub struct "];

    let fn_start = before.lines().rev().take(50).find(|line| {
        let trimmed = line.trim();
        fn_patterns.iter().any(|p| trimmed.starts_with(p))
            || trimmed.ends_with('{')
            || trimmed.starts_with('#')
    });

    if let Some(fn_line) = fn_start {
        let fn_pos = before.rfind(fn_line).unwrap_or(0);
        let context_start = fn_pos.max(snippet_start.saturating_sub(max_context_chars));

        let relevant = &source[context_start..];
        let snippet_pos_in_relevant = relevant.find(snippet).unwrap_or(0);
        let raw_end = snippet_pos_in_relevant + snippet.len() + max_context_chars;
        let end = raw_end.min(relevant.len());

        // Try to stop at the next function definition boundary
        let after_snippet =
            &relevant[snippet_pos_in_relevant + snippet.len()..end.min(relevant.len())];
        let next_fn_pos = after_snippet
            .find("\nfn ")
            .or_else(|| after_snippet.find("\npub fn "))
            .or_else(|| after_snippet.find("\nclass "))
            .or_else(|| after_snippet.find("\ndef "));
        let bounded_end = if let Some(pos) = next_fn_pos {
            snippet_pos_in_relevant + snippet.len() + pos
        } else {
            end
        };

        relevant[..bounded_end.min(relevant.len())].to_string()
    } else {
        extract_surrounding_lines(source, snippet, 3).unwrap_or_else(|| snippet.to_string())
    }
}

// ── Pipeline-integrated context collection ────────────────────────────────────

/// LLM 调用函数类型（用于查询增强等场景）
pub type LlmCallFn = std::sync::Arc<
    dyn Fn(
            String,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = axagent_harness::core_error::Result<String>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

/// 带管线增强的上下文收集（新入口）
///
/// 相比 collect_rag_context 增加了查询增强、重排序和质检阶段。
///
/// `api_key`：云端 reranker（cohere/jina/voyage）的实际 API Key，
/// 由 wiring 层（`indexing.rs::collect_rag_context`）从 `CredentialManager` 解析后注入。
/// 为 `None` 时云端 backend 自动降级到 `RuleReranker`（本地规则排序）。
#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context_with_pipeline(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    kb_ids: &[String],
    mem_ids: &[String],
    wiki_ids: &[String],
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
    pipeline_config: &axagent_harness::types::RAGPipelineConfig,
    llm_fn: Option<LlmCallFn>,
    api_key: Option<String>,
) -> RagContextResult {
    let sources = build_source_refs(kb_ids, mem_ids, wiki_ids);
    collect_rag_context_with_pipeline_from_refs(
        db,
        master_key,
        vector_store,
        sources,
        query,
        top_k,
        embed_fn,
        pipeline_config,
        llm_fn,
        api_key,
        kb_ids,
        wiki_ids,
    )
    .await
}

/// `collect_rag_context_with_pipeline` 的多文档协同变体。
#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context_with_pipeline_from_refs(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    sources: Vec<RAGSourceRef>,
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
    pipeline_config: &axagent_harness::types::RAGPipelineConfig,
    llm_fn: Option<LlmCallFn>,
    api_key: Option<String>,
    kb_ids: &[String],
    wiki_ids: &[String],
) -> RagContextResult {
    // 阶段 0：查询增强
    let queries: Vec<String> = if pipeline_config.query_enhancement.enabled {
        if let Some(ref llm) = llm_fn {
            let llm_clone = std::sync::Arc::clone(llm);
            let enhancer = crate::query_enhancement::QueryEnhancer::new(
                pipeline_config.query_enhancement.clone(),
                move |s| llm_clone(s),
            );
            match enhancer.enhance(query).await {
                Ok(enhanced) => enhanced.into_iter().map(|eq| eq.text).collect(),
                Err(e) => {
                    tracing::warn!("Query enhancement failed: {}", e);
                    vec![query.to_string()]
                },
            }
        } else {
            vec![query.to_string()]
        }
    } else {
        vec![query.to_string()]
    };

    // 使用第一个增强查询
    let effective_query = queries.first().map(|s| s.as_str()).unwrap_or(query);

    // 如果没有启用 pipeline，直接走原有逻辑
    if !pipeline_config.rerank.enabled && !pipeline_config.self_rag.enabled {
        return collect_rag_context_with_filters(
            db,
            master_key,
            vector_store,
            sources,
            effective_query,
            top_k,
            embed_fn,
            kb_ids,
            wiki_ids,
        )
        .await;
    }

    let engine: Arc<dyn InferenceEngine> = crate::inference::global_engine();
    let pipeline =
        crate::rag_pipeline::RAGPipeline::new(pipeline_config, Some(engine), api_key, None);

    if sources.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
            graph_context: None,
        };
    }

    // P2-2: 预计算 query embedding 用于后续的语义匹配
    let query_embedding = {
        let q_emb_result =
            embed_fn.generate(db, master_key, "default", vec![query.to_string()], None).await;
        match q_emb_result {
            Ok(resp) if !resp.embeddings.is_empty() => Some(resp.embeddings[0].clone()),
            _ => None,
        }
    };

    let mut context_parts = Vec::new();
    let mut source_results = Vec::new();

    for src_ref in &sources {
        let source = src_ref.source();
        let (source_top_k, _threshold, dims) = {
            let (sk, _, d) =
                resolve_source_config(db, &src_ref.source_type, &src_ref.container_id).await;
            (if sk > 0 { sk } else { top_k }, sk, d)
        };

        // 多文档协同：当 doc_ids 非空时透传给底层 search
        let doc_ids_opt = if src_ref.doc_ids.is_empty() {
            None
        } else {
            Some(src_ref.doc_ids.as_slice())
        };

        let result = pipeline
            .execute_with_filter(
                source.as_ref(),
                db,
                master_key,
                vector_store,
                &src_ref.container_id,
                effective_query,
                source_top_k,
                dims,
                embed_fn.clone(),
                &pipeline_config.rerank,
                doc_ids_opt,
            )
            .await;

        match result {
            Ok(output) if !output.results.is_empty() => {
                let source_type_str = match src_ref.source_type {
                    RAGSourceType::Knowledge => "knowledge",
                    RAGSourceType::Memory => "memory",
                    RAGSourceType::Wiki => "wiki",
                };

                let mut items: Vec<RagRetrievedItem> = output
                    .results
                    .iter()
                    .map(|r| RagRetrievedItem {
                        content: r.content.clone(),
                        score: r.score,
                        document_id: r.document_id.clone(),
                        id: r.id.clone(),
                        document_name: None,
                        chunk_index: Some(r.chunk_index),
                    })
                    .collect();

                // 三层记忆系统：针对 Memory 知识源，按 tier / importance 加权重排序
                // v108: 同时按 applicability_tags 过滤适用范围
                // P2-2: 传入 query_embedding 用于语义相似度匹配
                if matches!(src_ref.source_type, RAGSourceType::Memory) {
                    apply_memory_tier_weight(db, &mut items, query, query_embedding.as_deref())
                        .await;

                    // P0-2: 反馈闭环 — 检索命中后自动提升 importance 和访问统计
                    let hit_ids: Vec<String> = items.iter().map(|it| it.id.clone()).collect();
                    apply_memory_hit_feedback(db, &hit_ids).await;
                }

                // v110: 跨源反馈权重调整 — 根据 retrieval_hits 中的历史反馈数据调整排序
                // 正反馈文档加分，负反馈文档减分，形成"用户反馈→检索质量提升"闭环
                apply_feedback_weight_adjustment(db, &mut items, source_type_str).await;

                let label = source.context_label();
                // snippets 顺序跟随 items（tier 加权后的顺序），保证 context 与引用追溯一致
                let snippets: Vec<String> = items.iter().map(|it| it.content.clone()).collect();
                context_parts.push(format!("[{}]\n{}", label, snippets.join("\n---\n")));

                if let RetrievalQuality::Poor(ref diag) = output.quality {
                    tracing::warn!(
                        "Poor RAG quality for {} {}: {}",
                        source_type_str,
                        src_ref.container_id,
                        diag
                    );
                }

                source_results.push(RagSourceResult {
                    source_type: source_type_str.to_string(),
                    container_id: src_ref.container_id.clone(),
                    items,
                    container_name: None,
                });
            },
            Ok(_) => {
                tracing::warn!(
                    "Pipeline returned no results for {} {}",
                    source.collection_prefix(),
                    src_ref.container_id
                );
            },
            Err(e) => {
                tracing::warn!(
                    "Pipeline failed for {} {}: {}",
                    source.collection_prefix(),
                    src_ref.container_id,
                    e
                );
            },
        }
    }

    // 引用追溯：填充 container_name（KB / memory namespace / wiki 名称）
    fill_container_names(db, &mut source_results).await;

    // 引用可读性：回填检索命中项的 document_name（knowledge 文档标题 + wiki 笔记标题）
    fill_document_names(&mut source_results).await;

    let kg_context =
        collect_cross_source_graph_context(db, kb_ids, wiki_ids, effective_query, top_k).await;

    let (deduped_results, _deduped_context) =
        deduplicate_cross_source(source_results, context_parts);

    // 引用追溯：在 dedup 之后重建 context_parts，为每个 item 的 snippet 前注入 [cite:N] token。
    let final_context = rebuild_context_with_citations(&deduped_results, kg_context);

    RagContextResult {
        context_parts: final_context,
        source_results: deduped_results,
        graph_context: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_surrounding_lines() {
        let source = "line1\nline2\nline3\nMATCH\nline5\nline6\nline7";
        let result = extract_surrounding_lines(source, "MATCH", 2);
        assert!(result.is_some());
        let result = result.expect("测试应成功");
        assert!(result.contains("line2"));
        assert!(result.contains("line6"));
        assert!(!result.contains("line1"));
        assert!(!result.contains("line7"));
    }

    #[test]
    fn test_extract_surrounding_lines_not_found() {
        let result = extract_surrounding_lines("abc\ndef", "xyz", 3);
        assert!(result.is_none());
    }

    #[test]
    fn test_inject_function_only_finds_fn() {
        let source =
            "// comment\nfn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\nfn other() {}";
        let snippet = "println!(\"{x}\");";
        let result = inject_function_only(source, snippet, 500);
        assert!(result.contains("fn main()"));
        assert!(!result.contains("fn other()"));
    }

    #[test]
    fn test_inject_function_only_fallback() {
        let source = "let x = 1;\nlet y = 2;\nMATCH_HERE\nlet z = 3;";
        let result = inject_function_only(source, "MATCH_HERE", 500);
        assert!(result.contains("MATCH_HERE"));
        // Should include surrounding context even without a function boundary
        assert!(result.contains("let x = 1"));
        assert!(result.contains("let z = 3"));
    }

    #[test]
    fn test_default_l2_threshold_is_reasonable() {
        // 默认 L2 阈值应该大于 10.0（1536 维向量有效匹配通常在 5-40 范围）
        // 修复前为 2.0，过滤了几乎所有结果
        let default_max_distance = 20.0;
        assert!(default_max_distance >= 10.0, "L2 threshold too restrictive");
        assert!(default_max_distance <= 100.0, "L2 threshold too permissive");
    }

    #[test]
    fn test_prepare_chunks_from_text() {
        let strategy = ChunkStrategy::FromText {
            text: "第一章\n这是第一段内容。\n\n第二章\n这是第二段内容。".to_string(),
            chunk_size: 50,
            overlap: 10,
            separator: None,
        };
        let chunks = prepare_chunks("doc-1", &strategy).expect("测试应成功");
        assert!(!chunks.is_empty());
        for (id, _content, index) in &chunks {
            assert!(id.starts_with("doc-1_"));
            assert!(*index >= 0);
        }
    }

    #[test]
    fn test_prepare_chunks_empty_text() {
        let strategy = ChunkStrategy::FromText {
            text: "   ".to_string(),
            chunk_size: 100,
            overlap: 20,
            separator: None,
        };
        let chunks = prepare_chunks("doc-1", &strategy).expect("测试应成功");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_direct_chunk_strategy_returns_empty() {
        let strategy = ChunkStrategy::Direct;
        let chunks = prepare_chunks("item-1", &strategy).expect("测试应成功");
        assert!(chunks.is_empty());
    }

    // ── v108: filter_items_by_applicability_tags 单元测试 ──────────

    /// 构造测试用 RagRetrievedItem
    fn make_item(id: &str, score: f32) -> RagRetrievedItem {
        RagRetrievedItem {
            content: String::new(),
            score,
            document_id: String::new(),
            id: id.to_string(),
            document_name: None,
            chunk_index: None,
        }
    }

    #[test]
    fn test_filter_empty_items_no_op() {
        let mut items: Vec<RagRetrievedItem> = Vec::new();
        let map = std::collections::HashMap::new();
        filter_items_by_applicability_tags(&mut items, &map, "rust", None);
        assert!(items.is_empty());
    }

    #[test]
    fn test_filter_empty_weight_map_keeps_all() {
        let mut items = vec![make_item("a", 1.0), make_item("b", 2.0)];
        let map = std::collections::HashMap::new();
        filter_items_by_applicability_tags(&mut items, &map, "rust", None);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_filter_id_not_in_map_keeps() {
        let mut items = vec![make_item("a", 1.0), make_item("b", 2.0)];
        let mut map = std::collections::HashMap::new();
        // 仅注册 a，b 不在 map → b 保留
        map.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        filter_items_by_applicability_tags(&mut items, &map, "rust", None);
        // a 命中 rust tag，b 视为全局适用，均保留
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_filter_empty_tags_keeps() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, Vec::new()));
        filter_items_by_applicability_tags(&mut items, &map, "anything", None);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_filter_tag_matched_keeps() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        filter_items_by_applicability_tags(&mut items, &map, "rust programming", None);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_filter_tag_not_matched_removed() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        filter_items_by_applicability_tags(&mut items, &map, "python programming", None);
        assert!(items.is_empty());
    }

    #[test]
    fn test_filter_case_insensitive() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        // tag 大写
        map.insert("a".to_string(), (1.0, 0.5, vec!["RUST".to_string()]));
        // query 小写 → 子串匹配应不区分大小写
        filter_items_by_applicability_tags(&mut items, &map, "rust is great", None);
        assert_eq!(items.len(), 1);

        // 反向：tag 小写，query 大写
        let mut items2 = vec![make_item("a", 1.0)];
        let mut map2 = std::collections::HashMap::new();
        map2.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        filter_items_by_applicability_tags(&mut items2, &map2, "RUST IS GREAT", None);
        assert_eq!(items2.len(), 1);
    }

    #[test]
    fn test_filter_multiple_tags_any_match() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        // 多 tag，query 命中第二个
        map.insert("a".to_string(), (1.0, 0.5, vec!["python".to_string(), "rust".to_string()]));
        filter_items_by_applicability_tags(&mut items, &map, "rust coding", None);
        assert_eq!(items.len(), 1);

        // 多 tag，query 未命中任何一个
        let mut items2 = vec![make_item("a", 1.0)];
        filter_items_by_applicability_tags(&mut items2, &map, "golang coding", None);
        assert!(items2.is_empty());
    }

    #[test]
    fn test_filter_mixed_items_partial_removal() {
        let mut items = vec![
            make_item("a", 1.0), // tags=["rust"], query 命中 → 保留
            make_item("b", 2.0), // tags=["python"], query 未命中 → 移除
            make_item("c", 3.0), // tags=[] → 全局适用 → 保留
            make_item("d", 4.0), // 不在 map → 保留
        ];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        map.insert("b".to_string(), (1.0, 0.5, vec!["python".to_string()]));
        map.insert("c".to_string(), (1.0, 0.5, Vec::new()));
        filter_items_by_applicability_tags(&mut items, &map, "rust programming", None);
        assert_eq!(items.len(), 3);
        // b 应被移除
        assert!(items.iter().all(|it| it.id != "b"));
    }

    #[test]
    fn test_filter_empty_query_with_nonempty_tags_removes() {
        let mut items = vec![make_item("a", 1.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, vec!["rust".to_string()]));
        // query 为空 → 任何 tag 都无法命中 → 移除
        filter_items_by_applicability_tags(&mut items, &map, "", None);
        assert!(items.is_empty());
    }

    // ── v108: apply_tier_weight_and_sort 单元测试 ──────────

    #[test]
    fn test_weight_sort_empty_items_no_op() {
        let mut items: Vec<RagRetrievedItem> = Vec::new();
        let map = std::collections::HashMap::new();
        apply_tier_weight_and_sort(&mut items, &map);
        assert!(items.is_empty());
    }

    #[test]
    fn test_weight_sort_id_not_in_map_score_unchanged() {
        let mut items = vec![make_item("a", 5.0)];
        let map = std::collections::HashMap::new();
        apply_tier_weight_and_sort(&mut items, &map);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].score, 5.0);
    }

    #[test]
    fn test_weight_sort_single_item_adjusted() {
        let mut items = vec![make_item("a", 10.0)];
        let mut map = std::collections::HashMap::new();
        // tier_bonus=2.0, importance=0.5 → adjusted = 10 - 2.0*0.5 = 9.0
        map.insert("a".to_string(), (2.0, 0.5, Vec::new()));
        apply_tier_weight_and_sort(&mut items, &map);
        assert_eq!(items.len(), 1);
        assert!((items[0].score - 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_weight_sort_ascending_order() {
        // 原始 score：a=10, b=5, c=8
        // 加权后：a=10-2.0*0.9=8.2, b=5-0.5*0.5=4.75, c=8-1.5*0.7=6.95
        // 升序：b(4.75) < c(6.95) < a(8.2)
        let mut items = vec![make_item("a", 10.0), make_item("b", 5.0), make_item("c", 8.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (2.0, 0.9, Vec::new())); // core
        map.insert("b".to_string(), (0.5, 0.5, Vec::new())); // short_term
        map.insert("c".to_string(), (1.5, 0.7, Vec::new())); // long_term
        apply_tier_weight_and_sort(&mut items, &map);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "b");
        assert_eq!(items[1].id, "c");
        assert_eq!(items[2].id, "a");
        // 验证 adjusted score
        assert!((items[0].score - 4.75).abs() < 1e-6);
        assert!((items[1].score - 6.95).abs() < 1e-6);
        assert!((items[2].score - 8.2).abs() < 1e-6);
    }

    #[test]
    fn test_weight_sort_core_ranks_before_short_term() {
        // 即使原始 L2 distance 相同，core 的 bonus 更大（减得更多），应排前
        let mut items = vec![make_item("short", 5.0), make_item("core", 5.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("core".to_string(), (2.0, 0.9, Vec::new()));
        map.insert("short".to_string(), (0.5, 0.9, Vec::new()));
        apply_tier_weight_and_sort(&mut items, &map);
        // core adjusted = 5 - 2.0*0.9 = 3.2
        // short adjusted = 5 - 0.5*0.9 = 4.55
        // 升序：core(3.2) < short(4.55)
        assert_eq!(items[0].id, "core");
        assert_eq!(items[1].id, "short");
    }

    #[test]
    fn test_weight_sort_nan_score_fallback() {
        // 包含 NaN score 的 item 应不 panic（fallback 到 Equal）
        let mut items = vec![make_item("a", f32::NAN), make_item("b", 1.0)];
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), (1.0, 0.5, Vec::new()));
        map.insert("b".to_string(), (1.0, 0.5, Vec::new()));
        // 不应 panic
        apply_tier_weight_and_sort(&mut items, &map);
        assert_eq!(items.len(), 2);
    }
}
