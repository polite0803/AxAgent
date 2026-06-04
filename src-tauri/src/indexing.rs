//! Indexing pipeline for knowledge base documents and memory items.
//!
//! Provides functions to:
//! - Parse an `embedding_provider` string ("providerId::model_id")
//! - Build a `ProviderRequestContext` for embedding API calls
//! - Generate embeddings via provider adapters
//! - Index knowledge base documents and memory items via the unified RAG layer
//! - Search knowledge base / memory vectors via the unified RAG layer
//! - Collect RAG context for conversation injection

use sea_orm::DatabaseConnection;

use std::sync::Arc;

use axagent_core::error::{AxAgentError, Result};
use axagent_core::rag::{self, ChunkStrategy, KnowledgeRAG, LlmCallFn, MemoryRAG};
use axagent_harness::types::*;
use axagent_core::vector_store::{VectorSearchResult, VectorStore};

use axagent_providers::{ProviderAdapter, ProviderRequestContext, resolve_base_url_for_type};

// ── AsyncEmbedFn implementation ──────────────────────────────────────────────

/// Concrete implementation of `AsyncEmbedFn` that uses provider adapters.
///
/// 通过 `DEFAULT_PROVIDER_REGISTRY`（OnceLock 缓存）共享 provider registry，
/// 避免每次 embed 都 `RuntimeHarness::new`（之前会丢弃 adapter cache）。
#[derive(Clone)]
pub struct ProviderEmbedFn;

static DEFAULT_PROVIDER_REGISTRY: std::sync::OnceLock<
    std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>,
> = std::sync::OnceLock::new();

fn default_provider_registry()
-> &'static std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry> {
    DEFAULT_PROVIDER_REGISTRY.get_or_init(|| {
        std::sync::Arc::new(axagent_providers::registry::ProviderRegistry::create_default())
            as std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>
    })
}

#[async_trait::async_trait]
impl rag::AsyncEmbedFn for ProviderEmbedFn {
    async fn generate(
        &self,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        embedding_provider: &str,
        texts: Vec<String>,
        dimensions: Option<usize>,
    ) -> Result<EmbedResponse> {
        generate_embeddings(
            db,
            master_key,
            default_provider_registry(),
            embedding_provider,
            texts,
            dimensions,
        )
        .await
    }
}

// ── RAG Pipeline LLM helper ───────────────────────────────────────────────────

/// Build a `LlmCallFn` from the first enabled provider in the DB.
/// Used by the RAG pipeline for query enhancement LLM calls.
pub async fn build_rag_llm_fn(db: &DatabaseConnection, master_key: &[u8; 32]) -> Option<LlmCallFn> {
    let bridge = axagent_runtime::llm_bridge::build_llm_bridge_from_db(db, master_key).await?;

    Some(Arc::new(move |prompt: String| {
        let bridge = bridge.clone();
        Box::pin(async move {
            bridge
                .call_llm("", &prompt)
                .await
                .map_err(AxAgentError::Provider)
        })
    }))
}

// ── Low-level embedding utilities ────────────────────────────────────────────

/// Parse an embedding_provider string like "providerId::model_id" into (provider_id, model_id).
pub fn parse_embedding_provider(embedding_provider: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = embedding_provider.splitn(2, "::").collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(AxAgentError::Provider(format!(
            "Invalid embedding_provider format '{}'. Expected 'providerId::model_id'",
            embedding_provider
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Build a ProviderRequestContext for an embedding provider.
pub async fn build_embed_context(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_id: &str,
) -> Result<(ProviderRequestContext, ProviderConfig)> {
    let provider = axagent_core::repo::provider::get_provider(db, provider_id).await?;
    let key_row = axagent_core::repo::provider::get_active_key(db, provider_id).await?;
    let decrypted_key = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, master_key)?;

    let global_settings = axagent_core::repo::settings::get_settings(db)
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: None,
        proxy_config: resolved_proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Ok((ctx, provider))
}

/// Maximum number of texts per embedding API call batch.
/// OpenAI embed API limits to 2048 inputs per request; 256 is a conservative
/// default that works across all providers and keeps request sizes manageable.
const EMBED_BATCH_SIZE: usize = 256;

/// Maximum number of retry attempts for a single embedding API call.
const EMBED_MAX_RETRIES: u32 = 3;

/// Base delay in milliseconds for exponential backoff on retry.
const EMBED_RETRY_BASE_DELAY_MS: u64 = 500;

/// Generate embeddings for a list of texts using the specified provider.
///
/// Texts are sent in batches of `EMBED_BATCH_SIZE` to avoid exceeding API limits.
/// Each batch is retried up to `EMBED_MAX_RETRIES` times with exponential backoff.
///
/// `provider_registry` 由调用方传入（通常来自 `state.harness.provider_registry()`），
/// 不再内部 `RuntimeHarness::new` 重复创建（之前会丢弃 adapter cache）。
pub async fn generate_embeddings(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_registry: &std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>,
    embedding_provider: &str,
    texts: Vec<String>,
    dimensions: Option<usize>,
) -> Result<EmbedResponse> {
    let (provider_id, model_id) = parse_embedding_provider(embedding_provider)?;
    let (ctx, provider_config) = build_embed_context(db, master_key, &provider_id).await?;

    let registry_key = provider_config.provider_type.registry_key();
    let adapter = provider_registry.get(registry_key).ok_or_else(|| {
        AxAgentError::Provider(format!("Unsupported provider type: {}", registry_key))
    })?;

    // If texts fit in a single batch, use the simple path
    if texts.len() <= EMBED_BATCH_SIZE {
        let request = EmbedRequest {
            model: model_id,
            input: texts,
            dimensions,
        };
        return embed_with_retry(&*adapter, &ctx, request).await;
    }

    // Batch path: split texts into chunks and embed each batch
    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    let mut first_dimensions: Option<usize> = None;

    for batch in texts.chunks(EMBED_BATCH_SIZE) {
        let request = EmbedRequest {
            model: model_id.clone(),
            input: batch.to_vec(),
            dimensions,
        };
        let response = embed_with_retry(&*adapter, &ctx, request).await?;

        if first_dimensions.is_none() {
            first_dimensions = Some(response.dimensions);
        }
        all_embeddings.extend(response.embeddings);
    }

    Ok(EmbedResponse {
        embeddings: all_embeddings,
        dimensions: first_dimensions.unwrap_or(0),
    })
}

/// Execute a single embedding request with retry and exponential backoff.
async fn embed_with_retry(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: EmbedRequest,
) -> Result<EmbedResponse> {
    let mut last_err_msg = String::new();

    for attempt in 0..EMBED_MAX_RETRIES {
        match adapter.embed(ctx, request.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                last_err_msg = e.to_string();
                if attempt + 1 < EMBED_MAX_RETRIES {
                    let delay = EMBED_RETRY_BASE_DELAY_MS
                        .saturating_mul(2u64.checked_pow(attempt).unwrap_or(u64::MAX / 2))
                        .min(60_000); // Cap at 60 seconds to prevent excessive wait
                    tracing::warn!(
                        "Embedding attempt {}/{} failed, retrying in {}ms: {}",
                        attempt + 1,
                        EMBED_MAX_RETRIES,
                        delay,
                        e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                } else {
                    tracing::error!("Embedding failed after {} attempts: {}", EMBED_MAX_RETRIES, e);
                }
            },
        }
    }

    Err(AxAgentError::Provider(format!(
        "Embedding failed after {} retries: {}",
        EMBED_MAX_RETRIES, last_err_msg
    )))
}

// ── Document / item indexing (delegates to rag::index) ───────────────────────

/// Index a single knowledge base document: parse → chunk → embed → store.
///
/// Updates document status to "indexing" then "ready" or "failed".
#[allow(clippy::too_many_arguments)]
pub async fn index_knowledge_document(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    document_id: &str,
    source_path: &str,
    mime_type: &str,
    embedding_provider: &str,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    separator: Option<String>,
) -> Result<()> {
    axagent_core::repo::knowledge::update_document_status(db, document_id, "indexing").await?;

    // H6: resolve embedding dimensions from knowledge base config
    let dimensions = axagent_core::repo::knowledge::get_knowledge_base(db, knowledge_base_id)
        .await
        .ok()
        .and_then(|kb| kb.embedding_dimensions)
        .map(|d| d as usize);

    let result = run_indexing(
        db,
        master_key,
        vector_store,
        knowledge_base_id,
        document_id,
        source_path,
        mime_type,
        embedding_provider,
        chunk_size,
        chunk_overlap,
        separator,
        dimensions,
    )
    .await;

    match result {
        Ok(()) => {
            axagent_core::repo::knowledge::update_document_status(db, document_id, "ready").await?;
            Ok(())
        },
        Err(e) => {
            // H5: set status to failed so the user can retry
            let _ =
                axagent_core::repo::knowledge::update_document_status(db, document_id, "failed")
                    .await;
            Err(e)
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_indexing(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    document_id: &str,
    source_path: &str,
    mime_type: &str,
    embedding_provider: &str,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    separator: Option<String>,
    dimensions: Option<usize>,
) -> Result<()> {
    let is_conversation = source_path.starts_with("conversation://");

    let strategy = if is_conversation {
        let conv_id = source_path.strip_prefix("conversation://").unwrap_or("");
        let text =
            axagent_core::repo::conversation::get_conversation_archive_text(db, conv_id).await?;

        ChunkStrategy::FromText {
            text,
            chunk_size: chunk_size
                .map(|v| v as usize)
                .unwrap_or(axagent_core::text_chunker::DEFAULT_CHUNK_SIZE),
            overlap: chunk_overlap
                .map(|v| v as usize)
                .unwrap_or(axagent_core::text_chunker::DEFAULT_OVERLAP),
            separator,
        }
    } else {
        ChunkStrategy::ParseAndChunk {
            source_path: source_path.to_string(),
            mime_type: mime_type.to_string(),
            chunk_size: chunk_size
                .map(|v| v as usize)
                .unwrap_or(axagent_core::text_chunker::DEFAULT_CHUNK_SIZE),
            overlap: chunk_overlap
                .map(|v| v as usize)
                .unwrap_or(axagent_core::text_chunker::DEFAULT_OVERLAP),
            separator,
        }
    };

    let chunks = rag::prepare_chunks(document_id, &strategy)?;

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    rag::index(
        vector_store,
        "kb",
        knowledge_base_id,
        document_id,
        "",
        embed_response.embeddings,
        chunks,
    )
    .await
}

/// Index a single memory item: embed content → store in vector DB.
#[allow(clippy::too_many_arguments)]
pub async fn index_memory_item(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    namespace_id: &str,
    item_id: &str,
    content: &str,
    embedding_provider: &str,
    dimensions: Option<usize>,
) -> Result<()> {
    let chunks = rag::prepare_direct_chunk(item_id, content);

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    rag::index(
        vector_store,
        "mem",
        namespace_id,
        item_id,
        content,
        embed_response.embeddings,
        chunks,
    )
    .await
}

/// Index a single wiki note: chunk text → embed → store in vector DB.
///
/// Uses the `FromText` chunk strategy since note content is already plain text.
/// The collection name follows the pattern `wiki_{wiki_id}`.
#[allow(clippy::too_many_arguments)]
pub async fn index_wiki_note(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    wiki_id: &str,
    note_id: &str,
    content: &str,
    embedding_provider: &str,
    dimensions: Option<usize>,
) -> Result<()> {
    let strategy = ChunkStrategy::FromText {
        text: content.to_string(),
        chunk_size: axagent_core::text_chunker::DEFAULT_CHUNK_SIZE,
        overlap: axagent_core::text_chunker::DEFAULT_OVERLAP,
        separator: None,
    };

    let chunks = rag::prepare_chunks(note_id, &strategy)?;

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    rag::index(
        vector_store,
        "wiki",
        wiki_id,
        note_id,
        content,
        embed_response.embeddings,
        chunks,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn index_source(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    container: &axagent_core::rag::KnowledgeContainer,
    item_id: &str,
    content: &str,
    source_path: Option<&str>,
    mime_type: Option<&str>,
) -> Result<()> {
    let config = container.source_config();
    let embedding_provider = match &config.embedding_provider {
        Some(p) => p.clone(),
        None => {
            return Err(axagent_core::error::AxAgentError::Provider(format!(
                "{} '{}' has no embedding provider configured",
                container.container_type_str(),
                container.id
            )));
        },
    };
    let dimensions = config.embedding_dimensions.map(|d| d as usize);

    match container.container_type {
        axagent_core::rag::ContainerType::KnowledgeBase => {
            let chunk_size = container.chunk_size;
            let chunk_overlap = container.chunk_overlap;
            let separator = None;

            if let (Some(sp), Some(mt)) = (source_path, mime_type) {
                index_knowledge_document(
                    db,
                    master_key,
                    vector_store,
                    &container.id,
                    item_id,
                    sp,
                    mt,
                    &embedding_provider,
                    chunk_size,
                    chunk_overlap,
                    separator,
                )
                .await
            } else {
                Err(axagent_core::error::AxAgentError::Provider(
                    "KnowledgeBase indexing requires source_path and mime_type".to_string(),
                ))
            }
        },
        axagent_core::rag::ContainerType::Memory => {
            index_memory_item(
                db,
                master_key,
                vector_store,
                &container.id,
                item_id,
                content,
                &embedding_provider,
                dimensions,
            )
            .await
        },
        axagent_core::rag::ContainerType::WikiVault => {
            index_wiki_note(
                db,
                master_key,
                vector_store,
                &container.id,
                item_id,
                content,
                &embedding_provider,
                dimensions,
            )
            .await
        },
    }
}

// ── Search (delegates to rag::search) ────────────────────────────────────────

/// Search knowledge base vectors for relevant content.
pub async fn search_knowledge(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<VectorSearchResult>> {
    rag::search(
        &KnowledgeRAG,
        db,
        master_key,
        vector_store,
        knowledge_base_id,
        query,
        top_k,
        None,
        ProviderEmbedFn,
    )
    .await
}

/// Search memory namespace vectors for relevant content.
pub async fn search_memory(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    namespace_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<VectorSearchResult>> {
    // Look up namespace settings for dimensions
    let dims = axagent_core::repo::memory::get_namespace(db, namespace_id)
        .await
        .ok()
        .and_then(|ns| ns.embedding_dimensions.map(|v| v as usize));
    rag::search(
        &MemoryRAG,
        db,
        master_key,
        vector_store,
        namespace_id,
        query,
        top_k,
        dims,
        ProviderEmbedFn,
    )
    .await
}

// ── Context collection (delegates to rag::collect_rag_context) ───────────────

/// Collect RAG context from all enabled sources for a conversation query.
///
/// Returns a `RagContextResult` with formatted context parts and structured results.
/// RAG query cache: avoids repeated vector searches for the same query
static RAG_CACHE: std::sync::LazyLock<
    tokio::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, RagContextResult)>>,
> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));

const RAG_CACHE_TTL_SECS: u64 = 30;

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
) -> RagContextResult {
    if kb_ids.is_empty() && mem_ids.is_empty() && wiki_ids.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
        };
    }

    // Read pipeline config from global settings
    let pipeline_config = axagent_core::repo::settings::get_settings(db)
        .await
        .map(|s| s.rag_pipeline_config)
        .unwrap_or_default();

    let use_pipeline = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || pipeline_config
            .get("rerank")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || pipeline_config
            .get("self_rag")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    if !use_pipeline {
        // Fast path: no pipeline features enabled, use existing cached search
        let cache_key = format!("{:?}|{:?}|{:?}|{}", kb_ids, mem_ids, wiki_ids, query);
        {
            let cache = RAG_CACHE.lock().await;
            if let Some((timestamp, result)) = cache.get(&cache_key) {
                if timestamp.elapsed().as_secs() < RAG_CACHE_TTL_SECS {
                    return result.clone();
                }
            }
        }

        let result = rag::collect_rag_context(
            db,
            master_key,
            vector_store,
            kb_ids,
            mem_ids,
            wiki_ids,
            query,
            top_k,
            ProviderEmbedFn,
        )
        .await;

        let mut cache = RAG_CACHE.lock().await;
        cache.insert(cache_key, (std::time::Instant::now(), result.clone()));
        cache.retain(|_, (ts, _)| ts.elapsed().as_secs() < 300);
        return result;
    }

    // Pipeline path: build LLM function if query enhancement is enabled
    let qe_enabled = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rerank_enabled = pipeline_config
        .get("rerank")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let sr_enabled = pipeline_config
        .get("self_rag")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    tracing::info!(
        "RAG pipeline active: enhancement={}, rerank={}, self_rag={}",
        qe_enabled,
        rerank_enabled,
        sr_enabled,
    );

    let llm_fn = if qe_enabled {
        build_rag_llm_fn(db, master_key).await
    } else {
        None
    };

    // Pipeline results are not cached (involve LLM calls whose outputs vary)
    let pipeline_cfg: axagent_harness::types::RAGPipelineConfig =
        serde_json::from_value(pipeline_config.clone()).unwrap_or_default();
    rag::collect_rag_context_with_pipeline(
        db,
        master_key,
        vector_store,
        kb_ids,
        mem_ids,
        wiki_ids,
        query,
        top_k,
        ProviderEmbedFn,
        &pipeline_cfg,
        llm_fn,
    )
    .await
}
