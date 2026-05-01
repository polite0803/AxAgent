//! Unified RAG (Retrieval-Augmented Generation) abstraction layer.
//!
//! Provides a trait-based interface for different RAG sources (knowledge bases,
//! memory namespaces, etc.) to share indexing, searching, and context-collection
//! logic without code duplication.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};

use crate::error::{AxAgentError, Result};
use crate::types::{RagContextResult, RagRetrievedItem, RagSourceResult};
use crate::vector_store::{EmbeddingRecord, VectorSearchResult, VectorStore};
use crate::{document_parser, text_chunker};

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
        let kb = crate::repo::knowledge::get_knowledge_base(db, container_id).await?;
        kb.embedding_provider.ok_or_else(|| {
            AxAgentError::Provider(
                "Knowledge base has no embedding provider configured".to_string(),
            )
        })
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
        let ns = crate::repo::memory::get_namespace(db, container_id).await?;
        ns.embedding_provider.ok_or_else(|| {
            AxAgentError::Provider(
                "Memory namespace has no embedding provider configured".to_string(),
            )
        })
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
/// This is the generic replacement for the separate `search_knowledge` /
/// `search_memory` functions.  The concrete `EmbedFn` is injected by the
/// caller (typically `crate::indexing::generate_embeddings`).
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
    let embedding_provider = source.resolve_embedding_provider(db, container_id).await?;
    let cid = collection_id(source.collection_prefix(), container_id);

    let embed_response = embed_fn
        .generate(
            db,
            master_key,
            &embedding_provider,
            vec![query.to_string()],
            dimensions,
        )
        .await?;

    let query_embedding = embed_response
        .embeddings
        .into_iter()
        .next()
        .ok_or_else(|| AxAgentError::Provider("No query embedding returned".into()))?;

    let results = vector_store.search(&cid, query_embedding, top_k).await?;
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
    FromText {
        text: String,
        chunk_size: usize,
        overlap: usize,
        separator: Option<String>,
    },
}

/// Index content into a RAG source's vector collection.
///
/// Depending on the `ChunkStrategy`, the content is either:
/// - Parsed from a file, chunked, and batch-embedded (`ParseAndChunk`), or
/// - Embedded directly as a single item (`Direct`).
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
        ChunkStrategy::ParseAndChunk {
            source_path,
            mime_type,
            chunk_size,
            overlap,
            separator,
        } => {
            let path = std::path::Path::new(source_path);
            let text = document_parser::extract_text(path, mime_type)?;

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
        ChunkStrategy::FromText {
            text,
            chunk_size,
            overlap,
            separator,
        } => {
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

// ── Context collection ───────────────────────────────────────────────────────

/// A typed RAG source reference for context collection.
pub struct RAGSourceRef {
    pub source_type: RAGSourceType,
    pub container_id: String,
}

/// The type of RAG source.
#[derive(PartialEq)]
pub enum RAGSourceType {
    Knowledge,
    Memory,
}

impl RAGSourceRef {
    fn source(&self) -> Box<dyn RAGSource> {
        match self.source_type {
            RAGSourceType::Knowledge => Box::new(KnowledgeRAG),
            RAGSourceType::Memory => Box::new(MemoryRAG),
        }
    }
}

/// Collect RAG context from all enabled sources for a conversation query.
///
/// Returns a `RagContextResult` containing both formatted context parts
/// (for injection into the system prompt) and structured results
/// (for frontend display).  Errors for individual sources are logged and skipped.
pub async fn collect_rag_context(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    kb_ids: &[String],
    mem_ids: &[String],
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
) -> RagContextResult {
    let mut sources: Vec<RAGSourceRef> = Vec::new();

    for id in kb_ids {
        sources.push(RAGSourceRef {
            source_type: RAGSourceType::Knowledge,
            container_id: id.clone(),
        });
    }
    for id in mem_ids {
        sources.push(RAGSourceRef {
            source_type: RAGSourceType::Memory,
            container_id: id.clone(),
        });
    }

    let mut context_parts = Vec::new();
    let mut source_results = Vec::new();

    for src_ref in &sources {
        let source = src_ref.source();

        // Resolve per-source search parameters (top_k, threshold, dimensions)
        let (source_top_k, threshold, dims) = if src_ref.source_type == RAGSourceType::Memory {
            match crate::repo::memory::get_namespace(db, &src_ref.container_id).await {
                Ok(ns) => (
                    ns.retrieval_top_k.map(|v| v as usize).unwrap_or(top_k),
                    ns.retrieval_threshold.unwrap_or(0.0),
                    ns.embedding_dimensions.map(|v| v as usize),
                ),
                Err(_) => (top_k, 0.0, None),
            }
        } else {
            match crate::repo::knowledge::get_knowledge_base(db, &src_ref.container_id).await {
                Ok(kb) => (
                    kb.retrieval_top_k.map(|v| v as usize).unwrap_or(top_k),
                    kb.retrieval_threshold.unwrap_or(0.0),
                    kb.embedding_dimensions.map(|v| v as usize),
                ),
                Err(_) => (top_k, 0.0, None),
            }
        };

        let result = search(
            source.as_ref(),
            db,
            master_key,
            vector_store,
            &src_ref.container_id,
            query,
            source_top_k,
            dims,
            embed_fn.clone(),
        )
        .await;

        match result {
            Ok(raw_results) if !raw_results.is_empty() => {
                // Apply distance threshold filter.
                // score is L2 distance (lower = more similar).
                // When threshold > 0, keep only results within the distance threshold.
                // When threshold == 0 (default), apply a reasonable default threshold
                // to filter out completely irrelevant results.
                let default_max_distance = 2.0; // L2 distance threshold for relevance
                let effective_threshold = if threshold > 0.0 {
                    threshold
                } else {
                    default_max_distance
                };
                let results: Vec<_> = raw_results
                    .into_iter()
                    .filter(|r| r.score <= effective_threshold)
                    .collect();
                if results.is_empty() {
                    continue;
                }

                let items: Vec<RagRetrievedItem> = results
                    .iter()
                    .map(|r| RagRetrievedItem {
                        content: r.content.clone(),
                        score: r.score,
                        document_id: r.document_id.clone(),
                        id: r.id.clone(),
                        document_name: None,
                    })
                    .collect();

                let snippets: Vec<String> = results.iter().map(|r| r.content.clone()).collect();
                context_parts.push(format!(
                    "[{}]\n{}",
                    source.context_label(),
                    snippets.join("\n---\n")
                ));

                let source_type_str = match src_ref.source_type {
                    RAGSourceType::Knowledge => "knowledge",
                    RAGSourceType::Memory => "memory",
                };
                source_results.push(RagSourceResult {
                    source_type: source_type_str.to_string(),
                    container_id: src_ref.container_id.clone(),
                    items,
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

    // Batch-lookup document titles for knowledge sources
    {
        let kb_doc_ids: Vec<String> = source_results
            .iter()
            .filter(|s| s.source_type == "knowledge")
            .flat_map(|s| s.items.iter().map(|it| it.document_id.clone()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !kb_doc_ids.is_empty() {
            match crate::repo::knowledge::get_document_titles(db, &kb_doc_ids).await {
                Ok(titles) => {
                    for src in source_results
                        .iter_mut()
                        .filter(|s| s.source_type == "knowledge")
                    {
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
    }

    RagContextResult {
        context_parts,
        source_results,
    }
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
    ) -> Result<crate::types::EmbedResponse>;
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
        let wiki = crate::repo::wiki::get_wiki(db, container_id).await?;
        let embedding_provider = wiki.embedding_provider.ok_or_else(|| {
            AxAgentError::Provider("Wiki has no embedding provider configured".to_string())
        })?;
        Ok(embedding_provider)
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
    let wiki = crate::repo::wiki::get_wiki(db, vault_id).await?;

    let collection_name = collection_id("wiki", vault_id);
    let current_count = count_collection_items(db, &collection_name).await?;

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

async fn count_collection_items(db: &DatabaseConnection, collection_name: &str) -> Result<usize> {
    let sanitized = collection_name.replace(['-', '\'', '"', ';'], "_");
    let table_name = format!("vec_{}_meta", sanitized);
    let count: i64 = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            format!(
                "SELECT COUNT(*) as cnt FROM \"{}\"",
                table_name
            ),
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
    let _wiki = crate::repo::wiki::get_wiki(db, vault_id).await?;
    let collection_name = collection_id("wiki", vault_id);
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
    let sanitized = collection_name.replace(['-', '\'', '"', ';'], "_");
    let table_name = format!("vec_{}_meta", sanitized);
    let result = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            format!(
                "SELECT created_at FROM \"{}\" ORDER BY created_at ASC LIMIT 1",
                table_name
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
    let fn_patterns = [
        "fn ",
        "def ",
        "function ",
        "class ",
        "impl ",
        "pub fn ",
        "pub struct ",
    ];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_surrounding_lines() {
        let source = "line1\nline2\nline3\nMATCH\nline5\nline6\nline7";
        let result = extract_surrounding_lines(source, "MATCH", 2);
        assert!(result.is_some());
        let result = result.unwrap();
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
}
