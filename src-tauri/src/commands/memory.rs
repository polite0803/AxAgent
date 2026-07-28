// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use crate::AppState;
use axagent_dao::repo::index_jobs as jobs;
use axagent_harness::types::*;
use axagent_harness::{
    ProviderAdapter, ProviderRequestContext, url_utils::resolve_base_url_for_type,
};
use axagent_kit::prompts::PromptLang;
use sea_orm::ActiveModelTrait;
use tauri::{AppHandle, State};

/// 校验容器 ID（namespace_id / item_id 等）格式，防止 SQL 注入和路径穿越。
/// 规则：1-128 字符，仅允许字母数字、连字符、下划线。
fn validate_container_id(id: &str, field_name: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || id.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(String::from(crate::commands::error::ErrorResponse::from_error(
            format!(
                "Invalid {field_name}: must be 1-128 alphanumeric/hyphen/underscore characters"
            ),
            crate::commands::error::ErrorCategory::Unrecoverable,
        )));
    }
    Ok(())
}

fn provider_type_to_registry_key(pt: &ProviderType) -> &'static str {
    match pt {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    }
}

/// 解析默认 provider 的完整上下文
struct ResolvedProvider {
    model_id: String,
    ctx: ProviderRequestContext,
    adapter: Arc<dyn ProviderAdapter>,
}

async fn resolve_default_provider(state: &AppState) -> Result<ResolvedProvider, String> {
    let settings =
        axagent_dao::repo::settings::get_settings(state.harness.db()).await.unwrap_or_default();
    let provider_id =
        settings.default_provider_id.as_deref().ok_or("No default provider configured")?;
    let model_id = settings.default_model_id.as_deref().ok_or("No default model configured")?;

    let provider = axagent_dao::repo::provider::get_provider(state.harness.db(), provider_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let key_row = axagent_dao::repo::provider::get_active_key(state.harness.db(), provider_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let api_key = axagent_crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let proxy = axagent_harness::types::provider_model::resolve_provider_proxy(
        &provider.proxy_config,
        &settings,
    );
    let ctx = ProviderRequestContext {
        api_key: api_key.clone(),
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: proxy,
        custom_headers: provider.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
        crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::provider::ADAPTER_NOT_FOUND,
            format!("Unsupported provider type: {registry_key}"),
        )
    })?;

    Ok(ResolvedProvider { model_id: model_id.to_string(), ctx, adapter })
}

#[tauri::command]
pub async fn list_memory_namespaces(
    state: State<'_, AppState>,
) -> Result<Vec<MemoryNamespace>, String> {
    axagent_dao::repo::memory::list_namespaces(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn create_memory_namespace(
    state: State<'_, AppState>,
    input: CreateMemoryNamespaceInput,
) -> Result<MemoryNamespace, String> {
    axagent_dao::repo::memory::create_namespace(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn delete_memory_namespace(state: State<'_, AppState>, id: String) -> Result<(), String> {
    validate_container_id(&id, "namespace_id")?;

    // 先查询该 namespace 下所有 item，用于清理 FTS5 索引
    let items =
        axagent_dao::repo::memory::list_items(state.harness.db(), &id).await.unwrap_or_default();

    // Delete the entire vector collection for this namespace
    let collection_name = format!("mem_{}", id);
    if let Err(e) = state.vector_store.delete_collection(&collection_name).await {
        tracing::warn!("Failed to delete vector collection {}: {}", collection_name, e);
    }

    // 清理 FTS5 全文搜索索引（与 delete_memory_item 保持一致）
    if !items.is_empty() {
        let ms = state.memory_service.read().await;
        let storage = ms.storage();
        for item in &items {
            if let Err(e) = storage.delete_memory_fts(&item.id).await {
                tracing::warn!(
                    "Failed to remove memory {} from FTS5 index during namespace deletion: {}",
                    item.id,
                    e
                );
            }
        }
        drop(ms);
    }

    axagent_dao::repo::memory::delete_namespace(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn update_memory_namespace(
    state: State<'_, AppState>,
    id: String,
    input: UpdateMemoryNamespaceInput,
) -> Result<MemoryNamespace, String> {
    axagent_dao::repo::memory::update_namespace(state.harness.db(), &id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn list_memory_items(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<Vec<MemoryItem>, String> {
    validate_container_id(&namespace_id, "namespace_id")?;
    // Verify namespace exists before accessing its items
    let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let _ = ns; // Namespace exists, proceed
    axagent_dao::repo::memory::list_items(state.harness.db(), &namespace_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn add_memory_item(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateMemoryItemInput,
) -> Result<MemoryItem, String> {
    let item =
        axagent_dao::repo::memory::add_item(state.harness.db(), input).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // Spawn async embedding task if namespace has an embedding provider
    let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &item.namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if ns.embedding_provider.is_some() {
        let _ = axagent_dao::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "pending",
            None,
        )
        .await;

        // 入队失败时回滚 index_status 到 "skipped"，避免记忆永久卡在 pending 状态
        if let Err(e) = crate::index_queue::enqueue_job_sync(
            &state,
            &app,
            jobs::JOB_TYPE_INDEX_MEMORY,
            "mem",
            &item.namespace_id,
            &item.id,
            None,
            None,
        ) {
            let _ = axagent_dao::repo::memory::update_item_index_status(
                state.harness.db(),
                &item.id,
                "skipped",
                Some(&format!("enqueue failed: {e}")),
            )
            .await;
            return Err(String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            )));
        }

        Ok(MemoryItem { index_status: "pending".to_string(), ..item })
    } else {
        // No embedding provider — mark as skipped
        let _ = axagent_dao::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "skipped",
            None,
        )
        .await;
        Ok(MemoryItem { index_status: "skipped".to_string(), ..item })
    }
}

#[tauri::command]
pub async fn delete_memory_item(
    state: State<'_, AppState>,
    namespace_id: String,
    id: String,
) -> Result<(), String> {
    // Delete vector embedding for this item
    let collection_id = format!("mem_{}", namespace_id);
    let _ = state.vector_store.delete_document_embeddings(&collection_id, &id).await;

    // Also delete from FTS5 full-text search index
    let ms = state.memory_service.read().await;
    let storage = ms.storage();
    if let Err(e) = storage.delete_memory_fts(&id).await {
        tracing::warn!("Failed to remove memory from FTS5 index: {}", e);
    }
    drop(ms);

    axagent_dao::repo::memory::delete_item(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn update_memory_item(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace_id: String,
    id: String,
    input: UpdateMemoryItemInput,
) -> Result<MemoryItem, String> {
    let content_changed = input.content.is_some();
    let item = axagent_dao::repo::memory::update_item(state.harness.db(), &id, input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // Re-index if content changed and namespace has embedding provider
    if content_changed {
        let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

        if ns.embedding_provider.is_some() {
            let _ = axagent_dao::repo::memory::update_item_index_status(
                state.harness.db(),
                &id,
                "pending",
                None,
            )
            .await;

            if let Err(e) = crate::index_queue::enqueue_job_sync(
                &state,
                &app,
                jobs::JOB_TYPE_REINDEX_DOCUMENT,
                "mem",
                &namespace_id,
                &id,
                None,
                None,
            ) {
                // 入队失败时回滚状态到 "skipped"，避免条目永久卡在 pending
                let err_msg = format!("enqueue failed: {e}");
                let _ = axagent_dao::repo::memory::update_item_index_status(
                    state.harness.db(),
                    &id,
                    "skipped",
                    Some(&err_msg),
                )
                .await;
                return Err(String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                )));
            }

            return Ok(MemoryItem { index_status: "pending".to_string(), ..item });
        }
    }

    Ok(item)
}

#[tauri::command]
pub async fn search_memory(
    state: State<'_, AppState>,
    namespace_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<axagent_search::vector_store::VectorSearchResult>, String> {
    validate_container_id(&namespace_id, "namespace_id")?;
    // Verify namespace exists before searching
    let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let mut results = crate::indexing::search_memory(
        state.harness.db(),
        state.harness.master_key(),
        &state.vector_store,
        &namespace_id,
        &query,
        top_k.unwrap_or(5),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 应用与 collect_rag_context_from_refs 一致的距离阈值过滤
    // score 是 L2 距离（越小越相似），threshold > 0 时使用用户配置，否则用默认阈值 20.0
    let default_max_distance = 20.0_f32;
    let threshold = ns.retrieval_threshold.unwrap_or(0.0);
    let effective_threshold = if threshold > 0.0 {
        threshold
    } else {
        default_max_distance
    };
    results.retain(|r| r.score <= effective_threshold);

    Ok(results)
}

#[tauri::command]
pub async fn rebuild_memory_index(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<(), String> {
    let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if ns.embedding_provider.is_none() {
        return Err(crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        ));
    }

    let collection_id = format!("mem_{}", namespace_id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    let items = axagent_dao::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    for item in &items {
        let _ = axagent_dao::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "pending",
            None,
        )
        .await;

        crate::index_queue::enqueue_job_sync(
            &state,
            &app,
            jobs::JOB_TYPE_REBUILD_CONTAINER,
            "mem",
            &namespace_id,
            &item.id,
            Some(10),
            None,
        )
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    Ok(())
}

#[tauri::command]
pub async fn auto_extract_incremental_memories(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    namespace_id: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::conversations;
    use sea_orm::EntityTrait;

    let conv = conversations::Entity::find_by_id(&conversation_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let conv = match conv {
        Some(c) => c,
        None => {
            return Ok(serde_json::json!({"skipped": true, "reason": "conversation not found"}));
        },
    };

    match conv.memory_status.as_str() {
        "archived" | "both" => {
            return Ok(serde_json::json!({
                "skipped": true,
                "reason": format!("conversation already {} - skipping to avoid duplicate", conv.memory_status)
            }));
        },
        _ => {},
    }

    // 优先使用用户选择的 namespace_id，回退到 name 含 "auto"/"default" 的
    let namespace_id = if let Some(ref provided_id) = namespace_id {
        if provided_id.is_empty() {
            None
        } else {
            match axagent_dao::repo::memory::get_namespace(state.harness.db(), provided_id).await {
                Ok(_) => Some(provided_id.clone()),
                Err(_) => {
                    return Ok(serde_json::json!({
                        "skipped": true,
                        "reason": format!("specified namespace '{}' not found", provided_id)
                    }));
                },
            }
        }
    } else {
        None
    };

    let namespace_id = match namespace_id {
        Some(id) => id,
        None => {
            // 回退：找 name 含 "auto" 或 "default" 的 namespace
            let default_ns = axagent_dao::repo::memory::list_namespaces(state.harness.db())
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?
                .into_iter()
                .find(|ns| {
                    ns.name.to_lowercase().contains("auto")
                        || ns.name.to_lowercase().contains("default")
                });
            match default_ns {
                Some(ns) => ns.id,
                None => {
                    return Ok(serde_json::json!({
                        "skipped": true,
                        "reason": "no default/auto memory namespace found"
                    }));
                },
            }
        },
    };

    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if messages.len() < 4 {
        return Ok(serde_json::json!({"skipped": true, "reason": "not enough messages"}));
    }

    let last_extracted = conv.last_memory_extracted_at.as_deref();
    // 按时间戳过滤增量消息：只提取上次抽取之后的新消息
    let new_messages: Vec<axagent_harness::types::Message> = if let Some(last_ts) = last_extracted {
        // 解析上次抽取时间（RFC3339），失败时回退到取最近 6 条
        let cutoff_ts = chrono::DateTime::parse_from_rfc3339(last_ts)
            .ok()
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        let filtered: Vec<_> = messages.into_iter().filter(|m| m.created_at > cutoff_ts).collect();
        if filtered.is_empty() {
            return Ok(serde_json::json!({
                "extracted": 0,
                "skipped": true,
                "reason": "no new messages since last extraction"
            }));
        }
        filtered
    } else {
        // 首次抽取：取最近 20 条
        let recent: Vec<_> = messages.into_iter().rev().take(20).collect();
        recent.into_iter().rev().collect()
    };

    let resolved = resolve_default_provider(&state).await?;

    let result = crate::memory_extract::extract_incremental_memories(
        &new_messages,
        &conversation_id,
        resolved.adapter.as_ref(),
        &resolved.ctx,
        &resolved.model_id,
        PromptLang::ZhCN,
    )
    .await?;

    if result.items.is_empty() {
        return Ok(serde_json::json!({"extracted": 0, "skipped": false}));
    }

    let ns_config =
        axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id).await.ok();
    let can_vector_dedup =
        ns_config.as_ref().and_then(|ns| ns.embedding_provider.clone()).is_some();

    let existing_items = axagent_dao::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let existing_contents: std::collections::HashSet<String> =
        existing_items.iter().map(|item| item.content.to_lowercase().trim().to_string()).collect();

    let mut saved_count = 0usize;
    for item in &result.items {
        let content_lower = item.content.to_lowercase().trim().to_string();
        if existing_contents.contains(&content_lower) {
            continue;
        }

        if can_vector_dedup {
            let similar =
                check_semantic_duplicate(&state, &namespace_id, &item.content, ns_config.as_ref())
                    .await;
            if let Some(dup_id) = similar {
                tracing::info!(
                    "Auto-extract: skipping semantic duplicate (similar to {}): {}",
                    dup_id,
                    item.title
                );
                continue;
            }
        }

        let title = format!("[{}][auto] {}", item.category.as_str(), item.title);
        let input = CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content: item.content.clone(),
            source: Some("auto_extract".to_string()),
            tier: None,
            importance: None,
            memory_nature: None,
            tags: None,
            decay_rate: None,
            expires_at: None,
            applicability_tags: None,
            confirmed: None,
            // v109: 经验溯源字段（命令层不追踪来源，留空）
            source_conversation_id: None,
            source_message_id: None,
        };
        if let Ok(mem_item) = axagent_dao::repo::memory::add_item(state.harness.db(), input).await {
            let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                .await
                .ok();
            if let Some(ns) = ns {
                if ns.embedding_provider.is_some() {
                    let _ = axagent_dao::repo::memory::update_item_index_status(
                        state.harness.db(),
                        &mem_item.id,
                        "pending",
                        None,
                    )
                    .await;

                    let _ = crate::index_queue::enqueue_job_sync(
                        &state,
                        &app,
                        jobs::JOB_TYPE_INDEX_MEMORY,
                        "mem",
                        &namespace_id,
                        &mem_item.id,
                        None,
                        None,
                    );
                }
            }
            saved_count += 1;

            {
                let ms = state.memory_service.read().await;
                let _ = ms
                    .add_memory_advanced(axagent_trajectory::AddMemoryRequest {
                        target: "memory".to_string(),
                        content: item.content.clone(),
                        tier: axagent_trajectory::MemoryTier::Working,
                        importance: item.importance,
                        nature: match item.nature {
                            crate::memory_extract::ExtractedNature::Episodic => {
                                axagent_trajectory::MemoryNature::Episodic
                            },
                            crate::memory_extract::ExtractedNature::Semantic => {
                                axagent_trajectory::MemoryNature::Semantic
                            },
                        },
                        provenance: Some(axagent_trajectory::MemoryProvenance {
                            conversation_id: Some(conversation_id.clone()),
                            message_id: None,
                            extraction_method: "auto_incremental".to_string(),
                        }),
                        tags: item.tags.clone(),
                        expires_at: None,
                        namespace_id: Some(namespace_id.clone()),
                    })
                    .await;
            }
        }
    }

    if saved_count > 0 {
        let _ =
            update_conversation_memory_status(state.harness.db(), &conversation_id, "extracted")
                .await;
    }

    Ok(serde_json::json!({
        "extracted": saved_count,
        "skipped": false,
        "namespace_id": namespace_id,
    }))
}

#[tauri::command]
pub async fn clear_memory_index(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<(), String> {
    let collection_id = format!("mem_{}", namespace_id);
    state.vector_store.delete_collection(&collection_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 清空索引后把条目状态重置为 "skipped"（而非 "pending"），
    // 避免条目永久卡在 pending 但无索引任务可执行。
    // 用户如需重新索引，可调用 rebuild_memory_index。
    let items = axagent_dao::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    for item in items {
        let _ = axagent_dao::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "skipped",
            Some("index cleared by user"),
        )
        .await;
    }

    Ok(())
}

#[tauri::command]
pub async fn reindex_memory_item(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace_id: String,
    item_id: String,
) -> Result<(), String> {
    let ns = axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if ns.embedding_provider.is_none() {
        return Err(crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        ));
    }

    let _ = axagent_dao::repo::memory::update_item_index_status(
        state.harness.db(),
        &item_id,
        "pending",
        None,
    )
    .await;

    crate::index_queue::enqueue_job_sync(
        &state,
        &app,
        jobs::JOB_TYPE_REINDEX_DOCUMENT,
        "mem",
        &namespace_id,
        &item_id,
        None,
        None,
    )
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(())
}

#[tauri::command]
pub async fn sync_working_memory_to_namespace(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<usize, String> {
    let entries = {
        let ms = state.memory_service.read().await;
        ms.get_all_entries_for_sync().await
    };

    if entries.is_empty() {
        return Ok(0);
    }

    let mut synced = 0;
    for (id, content, mem_type) in &entries {
        // SAFETY: 使用 truncate_to_char_boundary 避免 UTF-8 字符边界 panic
        let title = format!(
            "[working-memory][{}] {}",
            mem_type,
            axagent_harness::util_fns::truncate_to_char_boundary(content, 50)
        );
        let input = CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content: content.clone(),
            source: Some("auto_extract".to_string()),
            tier: None,
            importance: None,
            memory_nature: None,
            tags: None,
            decay_rate: None,
            expires_at: None,
            applicability_tags: None,
            confirmed: None,
            // v109: 经验溯源字段（命令层不追踪来源，留空）
            source_conversation_id: None,
            source_message_id: None,
        };
        match axagent_dao::repo::memory::add_item(state.harness.db(), input).await {
            Ok(mem_item) => {
                let ns =
                    axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                        .await
                        .ok();
                if let Some(ns) = ns {
                    if ns.embedding_provider.is_some() {
                        let _ = axagent_dao::repo::memory::update_item_index_status(
                            state.harness.db(),
                            &mem_item.id,
                            "pending",
                            None,
                        )
                        .await;

                        let _ = crate::index_queue::enqueue_job_sync(
                            &state,
                            &app,
                            jobs::JOB_TYPE_INDEX_MEMORY,
                            "mem",
                            &namespace_id,
                            &mem_item.id,
                            None,
                            None,
                        );
                    } else {
                        let _ = axagent_dao::repo::memory::update_item_index_status(
                            state.harness.db(),
                            &mem_item.id,
                            "skipped",
                            None,
                        )
                        .await;
                    }
                }

                synced += 1;
                tracing::info!("Synced working memory entry {} to namespace {}", id, namespace_id);
            },
            Err(e) => {
                tracing::warn!("Failed to sync working memory entry {}: {}", id, e);
            },
        }
    }

    Ok(synced)
}

#[tauri::command]
pub async fn reorder_memory_namespaces(
    state: State<'_, AppState>,
    namespace_ids: Vec<String>,
) -> Result<(), String> {
    axagent_dao::repo::memory::reorder_namespaces(state.harness.db(), &namespace_ids).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn promote_memory_entry(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.promote_memory(&memory_id).await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn demote_memory_entry(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.demote_memory(&memory_id).await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn add_memory_with_dedup(
    state: State<'_, AppState>,
    target: String,
    content: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.add_memory_with_dedup(&target, &content).await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn apply_memory_decay_tick(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let evicted = ms.apply_decay_tick().await;
    Ok(serde_json::json!({ "evicted_count": evicted }))
}

#[tauri::command]
pub async fn search_working_memories(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.search_memories(&query, limit.unwrap_or(10)).await;
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn update_memory_importance(
    state: State<'_, AppState>,
    memory_id: String,
    delta: f64,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.update_importance(&memory_id, delta).await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memory_tier_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let usage = ms.get_memory_usage().await;
    Ok(serde_json::to_value(usage).unwrap_or_default())
}

#[tauri::command]
pub async fn extract_conversation_entities(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<serde_json::Value, String> {
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if messages.len() < 2 {
        return Ok(serde_json::json!({"entities": [], "relations": []}));
    }

    let resolved = resolve_default_provider(&state).await?;

    let result = crate::memory_extract::extract_entities_from_messages(
        &messages,
        resolved.adapter.as_ref(),
        &resolved.ctx,
        &resolved.model_id,
        PromptLang::ZhCN,
    )
    .await?;

    let mut saved_entities = 0usize;
    let mut saved_relations = 0usize;

    for ext_entity in &result.entities {
        let now = chrono::Utc::now();
        let entity = axagent_trajectory::Entity {
            id: format!("ent_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]),
            name: ext_entity.name.clone(),
            entity_type: axagent_trajectory::EntityType::from(ext_entity.entity_type.as_str()),
            properties: ext_entity.properties.clone(),
            aliases: ext_entity.aliases.clone(),
            first_seen_at: now,
            last_seen_at: now,
            mention_count: 1,
            confidence: ext_entity.confidence,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let existing = state.memory_service.read().await;
        let storage = {
            let ms = &existing;
            ms.storage()
        };
        drop(existing);

        let existing_entities =
            storage.search_entities(&ext_entity.name, 5).await.unwrap_or_default();
        let already_exists = existing_entities.iter().any(|e| {
            e.name.to_lowercase() == ext_entity.name.to_lowercase()
                && e.entity_type
                    == axagent_trajectory::EntityType::from(ext_entity.entity_type.as_str())
        });

        if !already_exists {
            if let Err(e) = storage.save_entity(&entity).await {
                tracing::warn!("Failed to save entity {}: {}", ext_entity.name, e);
            } else {
                saved_entities += 1;
            }
        } else if let Some(existing) = existing_entities
            .iter()
            .find(|e| e.name.to_lowercase() == ext_entity.name.to_lowercase())
        {
            let mut updated = existing.clone();
            updated.mention_count += 1;
            updated.last_seen_at = now;
            updated.confidence = updated.confidence.max(ext_entity.confidence);
            for alias in &ext_entity.aliases {
                if !updated.aliases.iter().any(|a| a.to_lowercase() == alias.to_lowercase()) {
                    updated.aliases.push(alias.clone());
                }
            }
            if let Err(e) = storage.save_entity(&updated).await {
                tracing::warn!("Failed to save updated entity {}: {}", updated.name, e);
            }
        }
    }

    let storage = {
        let ms = state.memory_service.read().await;
        ms.storage()
    };

    for ext_rel in &result.relations {
        let source_entities =
            storage.search_entities(&ext_rel.source_name, 3).await.unwrap_or_default();
        let target_entities =
            storage.search_entities(&ext_rel.target_name, 3).await.unwrap_or_default();

        let source_id = match source_entities.first() {
            Some(e) => e.id.clone(),
            None => continue,
        };
        let target_id = match target_entities.first() {
            Some(e) => e.id.clone(),
            None => continue,
        };

        let rel = axagent_trajectory::Relationship {
            id: format!("rel_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]),
            source_id,
            target_id,
            relation_type: axagent_trajectory::RelationshipType::from(
                ext_rel.relation_type.as_str(),
            ),
            properties: ext_rel.properties.clone(),
            weight: ext_rel.weight,
            created_at: chrono::Utc::now(),
        };

        if let Err(e) = storage.save_relationship(&rel).await {
            tracing::warn!("Failed to save relationship: {}", e);
        } else {
            saved_relations += 1;
        }
    }

    Ok(serde_json::json!({
        "entities_extracted": result.entities.len(),
        "relations_extracted": result.relations.len(),
        "entities_saved": saved_entities,
        "relations_saved": saved_relations,
    }))
}

#[tauri::command]
pub async fn graph_search_memories(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.graph_enhanced_search(&query, limit.unwrap_or(10)).await;
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn disambiguate_memory_entities(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.disambiguate_entities().await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn list_knowledge_graph(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let storage = ms.storage();
    let entities = storage.get_all_entities().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let relationships = storage.get_all_relationships().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(serde_json::json!({
        "entities": entities,
        "relationships": relationships,
    }))
}

#[tauri::command]
pub async fn search_memories_by_time(
    state: State<'_, AppState>,
    start_ts: i64,
    end_ts: i64,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.search_memories_by_time_range(start_ts, end_ts, limit.unwrap_or(50)).await;
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memories_time_grouped(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let groups = ms.get_memories_grouped_by_time().await;
    Ok(serde_json::to_value(groups).unwrap_or_default())
}

#[tauri::command]
pub async fn search_memories_explained(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.search_memories_explained(&query, limit.unwrap_or(10)).await;
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memory_provenance(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let mem = ms.get_working_memory().await;
    match mem.entries.get(&memory_id) {
        Some(entry) => Ok(serde_json::json!({
            "id": entry.id,
            "content": entry.content,
            "tier": entry.tier.as_str(),
            "importance": entry.importance,
            "nature": entry.nature.as_str(),
            "provenance": entry.provenance,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at,
            "access_count": entry.access_count,
            "last_accessed": entry.last_accessed,
            "effective_score": entry.effective_score(),
            "tags": entry.tags,
        })),
        None => Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::memory::NOT_FOUND,
            "Memory not found",
        )),
    }
}

#[tauri::command]
pub async fn find_memory_clusters(
    state: State<'_, AppState>,
    similarity_threshold: Option<f64>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let clusters = ms.find_similar_clusters(similarity_threshold.unwrap_or(0.5)).await;
    Ok(serde_json::to_value(clusters).unwrap_or_default())
}

#[tauri::command]
pub async fn consolidate_memory_cluster(
    _app: AppHandle,
    state: State<'_, AppState>,
    memory_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    if memory_ids.len() < 2 {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::memory::CONSOLIDATION_INSUFFICIENT,
            "Need at least 2 memories to consolidate",
        ));
    }

    let ms = state.memory_service.read().await;
    let mem = ms.get_working_memory().await;

    let contents: Vec<String> =
        memory_ids.iter().filter_map(|id| mem.entries.get(id).map(|e| e.content.clone())).collect();

    if contents.len() < 2 {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::memory::CONSOLIDATION_INSUFFICIENT,
            "Could not find enough memories for consolidation",
        ));
    }

    drop(ms);

    let resolved = resolve_default_provider(&state).await?;

    let consolidated = crate::memory_extract::consolidate_memories(
        &contents,
        resolved.adapter.as_ref(),
        &resolved.ctx,
        &resolved.model_id,
        PromptLang::ZhCN,
    )
    .await?;

    let ms = state.memory_service.read().await;
    let result = ms
        .add_memory_advanced(axagent_trajectory::AddMemoryRequest {
            target: "memory".to_string(),
            content: consolidated.content,
            tier: axagent_trajectory::MemoryTier::Working,
            importance: consolidated.importance,
            nature: axagent_trajectory::MemoryNature::Semantic,
            provenance: Some(axagent_trajectory::MemoryProvenance {
                conversation_id: None,
                message_id: None,
                extraction_method: "consolidation".to_string(),
            }),
            tags: consolidated.tags,
            expires_at: None,
            namespace_id: None,
        })
        .await;

    if result.success {
        for id in &memory_ids {
            let ms2 = state.memory_service.read().await;
            let rm_result = ms2.remove_memory("memory", id).await;
            if !rm_result.success {
                tracing::warn!("Failed to remove old memory {}: {}", id, rm_result.message);
            }
        }
    }

    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn submit_memory_feedback(
    state: State<'_, AppState>,
    memory_id: String,
    feedback: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.apply_user_feedback(&memory_id, &feedback).await;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

const SEMANTIC_DEDUP_DISTANCE_THRESHOLD: f32 = 5.0;

async fn check_semantic_duplicate(
    state: &AppState,
    namespace_id: &str,
    content: &str,
    ns_config: Option<&MemoryNamespace>,
) -> Option<String> {
    let ns = ns_config?;
    let embedding_provider = ns.embedding_provider.as_ref()?;
    let dimensions = ns.embedding_dimensions.map(|d| d as usize);

    let embed_result = crate::indexing::generate_embeddings(
        state.harness.db(),
        state.harness.master_key(),
        state.harness.provider_registry(),
        embedding_provider,
        vec![content.to_string()],
        dimensions,
    )
    .await
    .ok()?;

    let query_embedding = embed_result.embeddings.into_iter().next()?;

    let collection_name = format!("mem_{}", namespace_id);
    let search_results =
        state.vector_store.search(&collection_name, query_embedding, 3).await.ok()?;

    for result in &search_results {
        if result.score <= SEMANTIC_DEDUP_DISTANCE_THRESHOLD {
            return Some(result.id.clone());
        }
    }

    None
}

#[tauri::command]
pub async fn extract_conversation_memories(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    namespace_id: String,
) -> Result<Vec<MemoryItem>, String> {
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let resolved = resolve_default_provider(&state).await?;

    let result = crate::memory_extract::extract_memories_from_messages(
        &messages,
        &conversation_id,
        resolved.adapter.as_ref(),
        &resolved.ctx,
        &resolved.model_id,
        PromptLang::ZhCN,
    )
    .await?;

    let existing_items = axagent_dao::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let existing_contents: std::collections::HashSet<String> =
        existing_items.iter().map(|item| item.content.to_lowercase().trim().to_string()).collect();

    let ns_config =
        axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id).await.ok();
    let can_vector_dedup =
        ns_config.as_ref().and_then(|ns| ns.embedding_provider.clone()).is_some();

    let mut saved = Vec::new();
    for item in &result.items {
        let content_lower = item.content.to_lowercase().trim().to_string();
        if existing_contents.contains(&content_lower) {
            tracing::info!("Skipping exact duplicate memory: {}", item.title);
            continue;
        }

        if can_vector_dedup {
            let similar =
                check_semantic_duplicate(&state, &namespace_id, &item.content, ns_config.as_ref())
                    .await;
            if let Some(dup_id) = similar {
                tracing::info!(
                    "Skipping semantic duplicate memory (similar to {}): {}",
                    dup_id,
                    item.title
                );
                continue;
            }
        }

        let title = format!("[{}] {}", item.category.as_str(), item.title);
        let input = CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content: item.content.clone(),
            source: Some("auto_extract".to_string()),
            tier: None,
            importance: None,
            memory_nature: None,
            tags: None,
            decay_rate: None,
            expires_at: None,
            applicability_tags: None,
            confirmed: None,
            // v109: 经验溯源字段（命令层不追踪来源，留空）
            source_conversation_id: None,
            source_message_id: None,
        };
        match axagent_dao::repo::memory::add_item(state.harness.db(), input).await {
            Ok(mem_item) => {
                let ns =
                    axagent_dao::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                        .await
                        .ok();
                if let Some(ns) = ns {
                    if ns.embedding_provider.is_some() {
                        let _ = axagent_dao::repo::memory::update_item_index_status(
                            state.harness.db(),
                            &mem_item.id,
                            "pending",
                            None,
                        )
                        .await;

                        let _ = crate::index_queue::enqueue_job_sync(
                            &state,
                            &app,
                            jobs::JOB_TYPE_INDEX_MEMORY,
                            "mem",
                            &namespace_id,
                            &mem_item.id,
                            None,
                            None,
                        );

                        saved.push(MemoryItem { index_status: "pending".to_string(), ..mem_item });

                        {
                            let ms = state.memory_service.read().await;
                            let _ = ms
                                .add_memory_advanced(axagent_trajectory::AddMemoryRequest {
                                    target: "memory".to_string(),
                                    content: item.content.clone(),
                                    tier: axagent_trajectory::MemoryTier::Working,
                                    importance: item.importance,
                                    nature: match item.nature {
                                        crate::memory_extract::ExtractedNature::Episodic => {
                                            axagent_trajectory::MemoryNature::Episodic
                                        },
                                        crate::memory_extract::ExtractedNature::Semantic => {
                                            axagent_trajectory::MemoryNature::Semantic
                                        },
                                    },
                                    provenance: Some(axagent_trajectory::MemoryProvenance {
                                        conversation_id: Some(conversation_id.clone()),
                                        message_id: None,
                                        extraction_method: "manual_extract".to_string(),
                                    }),
                                    tags: item.tags.clone(),
                                    expires_at: None,
                                    namespace_id: Some(namespace_id.clone()),
                                })
                                .await;
                        }

                        continue;
                    }
                }

                let _ = axagent_dao::repo::memory::update_item_index_status(
                    state.harness.db(),
                    &mem_item.id,
                    "skipped",
                    None,
                )
                .await;
                saved.push(MemoryItem { index_status: "skipped".to_string(), ..mem_item });

                {
                    let ms = state.memory_service.read().await;
                    let _ = ms
                        .add_memory_advanced(axagent_trajectory::AddMemoryRequest {
                            target: "memory".to_string(),
                            content: item.content.clone(),
                            tier: axagent_trajectory::MemoryTier::Working,
                            importance: item.importance,
                            nature: match item.nature {
                                crate::memory_extract::ExtractedNature::Episodic => {
                                    axagent_trajectory::MemoryNature::Episodic
                                },
                                crate::memory_extract::ExtractedNature::Semantic => {
                                    axagent_trajectory::MemoryNature::Semantic
                                },
                            },
                            provenance: Some(axagent_trajectory::MemoryProvenance {
                                conversation_id: Some(conversation_id.clone()),
                                message_id: None,
                                extraction_method: "manual_extract".to_string(),
                            }),
                            tags: item.tags.clone(),
                            expires_at: None,
                            namespace_id: Some(namespace_id.clone()),
                        })
                        .await;
                }
            },
            Err(e) => {
                tracing::warn!("Failed to save extracted memory: {}", e);
            },
        }
    }

    if !saved.is_empty() {
        let _ =
            update_conversation_memory_status(state.harness.db(), &conversation_id, "extracted")
                .await;
    }

    Ok(saved)
}

async fn update_conversation_memory_status(
    db: &sea_orm::DatabaseConnection,
    conversation_id: &str,
    action: &str,
) -> Result<(), String> {
    use axagent_entities::conversations;
    use sea_orm::{EntityTrait, Set};

    let conv = conversations::Entity::find_by_id(conversation_id).one(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if let Some(model) = conv {
        let mut am: conversations::ActiveModel = model.into();
        let current_status: String = match &am.memory_status {
            sea_orm::ActiveValue::Set(v) => v.clone(),
            sea_orm::ActiveValue::Unchanged(v) => v.clone(),
            _ => "none".to_string(),
        };
        let new_status = match (current_status.as_str(), action) {
            ("none", "extracted") => "extracted",
            ("archived", "extracted") => "both",
            ("none", "archived") => "archived",
            ("extracted", "archived") => "both",
            // "both" 状态已包含两种标记，任意动作都保持 "both"，避免状态丢失
            ("both", _) => "both",
            (current, "extracted") if !current.starts_with("extract") => "extracted",
            (current, "archived") if !current.starts_with("archiv") => "archived",
            _ => &current_status,
        };
        am.memory_status = Set(new_status.to_string());
        am.last_memory_extracted_at = Set(Some(chrono::Utc::now().to_rfc3339()));
        am.update(db).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 共享记忆命令（从 agent 模块迁移）
// ---------------------------------------------------------------------------

/// 列出命名空间中的所有共享记忆条目
#[tauri::command]
pub async fn shared_memory_list(
    app_state: State<'_, AppState>,
    namespace: String,
) -> Result<Vec<serde_json::Value>, String> {
    let mem = app_state.shared_memory.read().await;
    let entries = mem.list(&namespace);
    Ok(entries.iter().filter_map(|e| serde_json::to_value(e).ok()).collect())
}

/// 获取指定共享记忆条目
#[tauri::command]
pub async fn shared_memory_get(
    app_state: State<'_, AppState>,
    key: String,
    namespace: String,
) -> Result<serde_json::Value, String> {
    let mem = app_state.shared_memory.read().await;
    let entry = mem.get(&key, &namespace).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    serde_json::to_value(entry).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取共享记忆统计信息
#[tauri::command]
pub async fn shared_memory_stats(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mem = app_state.shared_memory.read().await;
    let stats = mem.stats();
    serde_json::to_value(stats).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 手动刷新记忆（前端触发）
#[tauri::command]
pub async fn memory_flush(
    app_state: State<'_, AppState>,
    content: String,
    target: Option<String>,
    category: Option<String>,
) -> Result<serde_json::Value, String> {
    let valid_target = target.as_deref().unwrap_or("memory");
    let _valid_category = category.as_deref().unwrap_or("insight");

    // 使用 MemoryService 持久化记忆
    let ms = app_state.memory_service.read().await;
    let result = ms.add_memory(valid_target, &content).await;
    serde_json::to_value(result).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── 三层记忆系统：用户 namespace item 的晋升/降级/衰减 ────────────────────

/// 三层记忆系统：晋升用户 namespace 中的 memory item 到下一 tier。
/// 与 trajectory 的 promote_memory_entry 区别：本命令操作 memory_items 表，覆盖所有 namespace。
#[tauri::command]
pub async fn promote_user_memory_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<MemoryItem, String> {
    axagent_dao::repo::memory::promote_item(state.harness.db(), &item_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 三层记忆系统：降级用户 namespace 中的 memory item 到下一 tier。
#[tauri::command]
pub async fn demote_user_memory_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<MemoryItem, String> {
    axagent_dao::repo::memory::demote_item(state.harness.db(), &item_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 三层记忆系统：记录访问并可能触发自动晋升。
#[tauri::command]
pub async fn record_user_memory_access(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<MemoryItem, String> {
    axagent_dao::repo::memory::record_access_and_maybe_promote(state.harness.db(), &item_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

/// v108: 自进化闭环 — 确认记忆项（设置 confirmed=1）。
///
/// Reflector 自动沉淀的经验默认未确认（confirmed=0），
/// 用户审核后调用此命令标记为已确认，之后才能晋升到 core 层。
#[tauri::command]
pub async fn confirm_memory_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<MemoryItem, String> {
    axagent_dao::repo::memory::confirm_item(state.harness.db(), &item_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 三层记忆系统：手动触发一次全表衰减 tick（通常由定时器调用，此命令供管理员/调试用）。
/// 返回 { expiredDeleted, lowScoreDeleted, capacityEvicted }
#[tauri::command]
pub async fn apply_user_memory_decay_tick(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (expired, low_score, capacity) =
        axagent_dao::repo::memory::apply_decay_tick(state.harness.db()).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    Ok(serde_json::json!({
        "expiredDeleted": expired,
        "lowScoreDeleted": low_score,
        "capacityEvicted": capacity,
    }))
}

/// v108: 自进化闭环 — 把 DB memory_items 导出到文件级 ProjectMemory。
///
/// 遍历所有 namespace，把 `tier ∈ {core, long_term}` 且 `confirmed=1` 的记忆
/// 导出到 `.axagent/memory/{user,project}/` 下的 .md 文件，并更新 MEMORY.md 索引。
///
/// 导出后，文件级 `scan_relevant_files` 检索即可取用到这些经验，
/// 即使向量嵌入未配置也能通过 TF 关键词匹配检索。
#[tauri::command]
pub async fn export_memories_to_project(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use crate::commands::error::{ErrorCategory, ErrorResponse};

    // 1. 获取 workspace_dir
    let settings =
        axagent_dao::repo::settings::get_settings(state.harness.db()).await.unwrap_or_default();
    let workspace_dir = match settings.default_workspace_dir.as_ref() {
        Some(d) if !d.is_empty() => d.clone(),
        _ => {
            return Err(ErrorResponse::err_with_detail(
                crate::commands::error_code::common::INVALID_INPUT,
                "default_workspace_dir not configured",
            ));
        },
    };

    // 2. 遍历所有 namespace，收集 memory_items
    let namespaces = axagent_dao::repo::memory::list_namespaces(state.harness.db())
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;

    let mut all_items: Vec<MemoryItem> = Vec::new();
    for ns in &namespaces {
        let items =
            axagent_dao::repo::memory::list_items(state.harness.db(), &ns.id).await.map_err(
                |e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)),
            )?;
        // 仅导出已确认的高价值记忆
        for it in items {
            if it.confirmed == 1 && (it.tier == "core" || it.tier == "long_term") {
                all_items.push(it);
            }
        }
    }

    if all_items.is_empty() {
        return Ok(serde_json::json!({
            "exported": 0,
            "message": "无可导出的记忆（需 confirmed=1 且 tier ∈ {core, long_term}）",
        }));
    }

    // 3. 调用 ProjectMemory 导出到文件
    let pm = axagent_agent::ProjectMemory::new(std::path::PathBuf::from(workspace_dir));
    let exported = pm.export_memory_items(&all_items).await.map_err(|e| {
        String::from(ErrorResponse::from_error(
            axagent_harness::core_error::AxAgentError::Internal(format!(
                "导出到 ProjectMemory 失败: {}",
                e
            )),
            ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(serde_json::json!({
        "exported": exported,
        "totalCandidates": all_items.len(),
    }))
}
