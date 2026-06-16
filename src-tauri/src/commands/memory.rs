// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_core::prompts::PromptLang;
use axagent_harness::types::*;
use axagent_harness::{ProviderRequestContext, url_utils::resolve_base_url_for_type};
use sea_orm::ActiveModelTrait;
use tauri::{AppHandle, Emitter, State};

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

#[tauri::command]
pub async fn list_memory_namespaces(
    state: State<'_, AppState>,
) -> Result<Vec<MemoryNamespace>, String> {
    axagent_core::repo::memory::list_namespaces(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_memory_namespace(
    state: State<'_, AppState>,
    input: CreateMemoryNamespaceInput,
) -> Result<MemoryNamespace, String> {
    axagent_core::repo::memory::create_namespace(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_memory_namespace(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_core::repo::memory::delete_namespace(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_memory_namespace(
    state: State<'_, AppState>,
    id: String,
    input: UpdateMemoryNamespaceInput,
) -> Result<MemoryNamespace, String> {
    axagent_core::repo::memory::update_namespace(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_memory_items(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<Vec<MemoryItem>, String> {
    // Validate namespace_id format (prevent injection)
    if namespace_id.is_empty()
        || namespace_id.len() > 128
        || namespace_id.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(
            "Invalid namespace_id: must be 1-128 alphanumeric/hyphen/underscore characters"
                .to_string(),
        );
    }
    // Verify namespace exists before accessing its items
    let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = ns; // Namespace exists, proceed
    axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_memory_item(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateMemoryItemInput,
) -> Result<MemoryItem, String> {
    let item = axagent_core::repo::memory::add_item(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())?;

    // Spawn async embedding task if namespace has an embedding provider
    let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &item.namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    if ns.embedding_provider.is_some() {
        let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
        let _ = axagent_core::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "indexing",
            None,
        )
        .await;

        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let vector_store = state.vector_store.clone();
        let item_id = item.id.clone();
        let content = item.content.clone();

        tokio::spawn(async move {
            let result = crate::indexing::index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                &item_id,
                &content,
                None,
                None,
            )
            .await;

            let (status, err_msg) = match &result {
                Ok(_) => ("ready", None),
                Err(e) => {
                    tracing::error!("Memory embedding failed for item {}: {}", item_id, e);
                    ("failed", Some(e.to_string()))
                },
            };
            let _ = axagent_core::repo::memory::update_item_index_status(
                &db,
                &item_id,
                status,
                err_msg.as_deref(),
            )
            .await;

            let _ = app.emit(
                "memory-item-indexed",
                serde_json::json!({
                    "itemId": item_id,
                    "success": result.is_ok(),
                    "status": status,
                    "error": err_msg,
                }),
            );
        });

        // Return item with "indexing" status
        Ok(MemoryItem {
            index_status: "indexing".to_string(),
            ..item
        })
    } else {
        // No embedding provider — mark as skipped
        let _ = axagent_core::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "skipped",
            None,
        )
        .await;
        Ok(MemoryItem {
            index_status: "skipped".to_string(),
            ..item
        })
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
    let _ = state
        .vector_store
        .delete_document_embeddings(&collection_id, &id)
        .await;

    axagent_core::repo::memory::delete_item(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
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
    let item = axagent_core::repo::memory::update_item(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())?;

    // Re-index if content changed and namespace has embedding provider
    if content_changed {
        let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
            .await
            .map_err(|e| e.to_string())?;

        if ns.embedding_provider.is_some() {
            let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
            let _ = axagent_core::repo::memory::update_item_index_status(
                state.harness.db(),
                &id,
                "indexing",
                None,
            )
            .await;

            let db = state.harness.db().clone();
            let master_key = state.harness.master_key_owned();
            let vector_store = state.vector_store.clone();
            let item_id = item.id.clone();
            let content = item.content.clone();

            tokio::spawn(async move {
                let collection_id = format!("mem_{}", container.id);
                let _ = vector_store
                    .delete_document_embeddings(&collection_id, &item_id)
                    .await;

                let result = crate::indexing::index_source(
                    &db,
                    &master_key,
                    &vector_store,
                    &container,
                    &item_id,
                    &content,
                    None,
                    None,
                )
                .await;

                let (status, err_msg) = match &result {
                    Ok(_) => ("ready", None),
                    Err(e) => {
                        tracing::error!("Memory re-embedding failed for item {}: {}", item_id, e);
                        ("failed", Some(e.to_string()))
                    },
                };
                let _ = axagent_core::repo::memory::update_item_index_status(
                    &db,
                    &item_id,
                    status,
                    err_msg.as_deref(),
                )
                .await;

                let _ = app.emit(
                    "memory-item-indexed",
                    serde_json::json!({
                        "itemId": item_id,
                        "success": result.is_ok(),
                        "status": status,
                        "error": err_msg,
                    }),
                );
            });

            return Ok(MemoryItem {
                index_status: "indexing".to_string(),
                ..item
            });
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
) -> Result<Vec<axagent_core::vector_store::VectorSearchResult>, String> {
    // Validate namespace_id format (prevent injection)
    if namespace_id.is_empty()
        || namespace_id.len() > 128
        || namespace_id.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(
            "Invalid namespace_id: must be 1-128 alphanumeric/hyphen/underscore characters"
                .to_string(),
        );
    }
    // Verify namespace exists before searching
    let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = ns; // Namespace exists, proceed
    crate::indexing::search_memory(
        state.harness.db(),
        state.harness.master_key(),
        &state.vector_store,
        &namespace_id,
        &query,
        top_k.unwrap_or(5),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebuild_memory_index(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<(), String> {
    let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    let _embedding_provider = ns
        .embedding_provider
        .as_ref()
        .ok_or("No embedding provider configured")?;

    let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);

    let collection_id = format!("mem_{}", namespace_id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    let items = axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    for item in &items {
        let _ = axagent_core::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "indexing",
            None,
        )
        .await;
    }

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();

    tokio::spawn(async move {
        for item in items {
            let result = crate::indexing::index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                &item.id,
                &item.content,
                None,
                None,
            )
            .await;

            let (status, err_msg) = match &result {
                Ok(_) => ("ready", None),
                Err(e) => {
                    tracing::error!("Memory re-indexing failed for item {}: {}", item.id, e);
                    ("failed", Some(e.to_string()))
                },
            };
            let _ = axagent_core::repo::memory::update_item_index_status(
                &db,
                &item.id,
                status,
                err_msg.as_deref(),
            )
            .await;

            // Emit per-item event for real-time progress
            let _ = app.emit(
                "memory-item-indexed",
                serde_json::json!({
                    "itemId": item.id,
                    "success": result.is_ok(),
                    "status": status,
                    "error": err_msg,
                    "isRebuild": true,
                }),
            );
        }

        let _ =
            app.emit("memory-rebuild-complete", serde_json::json!({ "namespaceId": namespace_id }));
    });

    Ok(())
}

#[tauri::command]
pub async fn auto_extract_incremental_memories(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    namespace_id: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_core::entity::conversations;
    use sea_orm::EntityTrait;

    let conv = conversations::Entity::find_by_id(&conversation_id)
        .one(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;

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
            match axagent_core::repo::memory::get_namespace(state.harness.db(), provided_id).await {
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
            let default_ns = axagent_core::repo::memory::list_namespaces(state.harness.db())
                .await
                .map_err(|e| e.to_string())?
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

    let messages = axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    if messages.len() < 4 {
        return Ok(serde_json::json!({"skipped": true, "reason": "not enough messages"}));
    }

    let last_extracted = conv.last_memory_extracted_at.as_deref();
    let new_messages: Vec<axagent_harness::types::Message> = if let Some(_last_ts) = last_extracted
    {
        let recent: Vec<_> = messages.into_iter().rev().take(6).collect();
        recent.into_iter().rev().collect()
    } else {
        let recent: Vec<_> = messages.into_iter().rev().take(20).collect();
        recent.into_iter().rev().collect()
    };

    let (provider, key_row, model_id, settings) = {
        let settings = axagent_core::repo::settings::get_settings(state.harness.db())
            .await
            .unwrap_or_default();
        let provider_id = settings
            .default_provider_id
            .as_deref()
            .ok_or("No default provider configured")?;
        let model_id = settings
            .default_model_id
            .as_deref()
            .ok_or("No default model configured")?;

        let provider = axagent_core::repo::provider::get_provider(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        let key_row = axagent_core::repo::provider::get_active_key(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;

        (provider, key_row, model_id.to_string(), settings)
    };

    let api_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    let proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &settings);
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| format!("Unsupported provider type: {}", registry_key))?;

    let result = crate::memory_extract::extract_incremental_memories(
        &new_messages,
        &conversation_id,
        adapter.as_ref(),
        &ctx,
        &model_id,
        PromptLang::ZhCN,
    )
    .await?;

    if result.items.is_empty() {
        return Ok(serde_json::json!({"extracted": 0, "skipped": false}));
    }

    let ns_config = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .ok();
    let can_vector_dedup = ns_config
        .as_ref()
        .and_then(|ns| ns.embedding_provider.clone())
        .is_some();

    let existing_items = axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;
    let existing_contents: std::collections::HashSet<String> = existing_items
        .iter()
        .map(|item| item.content.to_lowercase().trim().to_string())
        .collect();

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
        };
        if let Ok(mem_item) = axagent_core::repo::memory::add_item(state.harness.db(), input).await
        {
            let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                .await
                .ok();
            if let Some(ns) = ns {
                if ns.embedding_provider.is_some() {
                    let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
                    let _ = axagent_core::repo::memory::update_item_index_status(
                        state.harness.db(),
                        &mem_item.id,
                        "indexing",
                        None,
                    )
                    .await;

                    let db = state.harness.db().clone();
                    let master_key = state.harness.master_key_owned();
                    let vector_store = state.vector_store.clone();
                    let item_id = mem_item.id.clone();
                    let content = mem_item.content.clone();
                    let app_clone = app.clone();

                    tokio::spawn(async move {
                        let res = crate::indexing::index_source(
                            &db,
                            &master_key,
                            &vector_store,
                            &container,
                            &item_id,
                            &content,
                            None,
                            None,
                        )
                        .await;

                        let (status, err_msg) = match &res {
                            Ok(_) => ("ready", None),
                            Err(e) => ("failed", Some(e.to_string())),
                        };
                        let _ = axagent_core::repo::memory::update_item_index_status(
                            &db,
                            &item_id,
                            status,
                            err_msg.as_deref(),
                        )
                        .await;
                        let _ = app_clone.emit(
                            "memory-item-indexed",
                            serde_json::json!({
                                "itemId": item_id,
                                "success": res.is_ok(),
                            }),
                        );
                    });
                }
            }
            saved_count += 1;

            {
                let ms = state.memory_service.read().await;
                let _ = ms.add_memory_advanced(axagent_trajectory::AddMemoryRequest {
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
                });
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
    state
        .vector_store
        .delete_collection(&collection_id)
        .await
        .map_err(|e| e.to_string())?;

    // Reset all items to "pending"
    let items = axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    for item in items {
        let _ = axagent_core::repo::memory::update_item_index_status(
            state.harness.db(),
            &item.id,
            "pending",
            None,
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
    let ns = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
    if container.embedding_provider.is_none() {
        return Err("No embedding provider configured".to_string());
    }

    let items = axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;

    let item = items
        .into_iter()
        .find(|i| i.id == item_id)
        .ok_or("Item not found")?;

    let _ = axagent_core::repo::memory::update_item_index_status(
        state.harness.db(),
        &item_id,
        "indexing",
        None,
    )
    .await;

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();
    let iid = item_id.clone();
    let content = item.content.clone();

    tokio::spawn(async move {
        let collection_id = format!("mem_{}", container.id);
        let _ = vector_store
            .delete_document_embeddings(&collection_id, &iid)
            .await;

        let result = crate::indexing::index_source(
            &db,
            &master_key,
            &vector_store,
            &container,
            &iid,
            &content,
            None,
            None,
        )
        .await;

        let (status, err_msg) = match &result {
            Ok(_) => ("ready", None),
            Err(e) => {
                tracing::error!("Memory reindex failed for item {}: {}", iid, e);
                ("failed", Some(e.to_string()))
            },
        };
        let _ = axagent_core::repo::memory::update_item_index_status(
            &db,
            &iid,
            status,
            err_msg.as_deref(),
        )
        .await;

        let _ = app.emit(
            "memory-item-indexed",
            serde_json::json!({
                "itemId": iid,
                "success": result.is_ok(),
                "status": status,
                "error": err_msg,
            }),
        );
    });

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
        ms.get_all_entries_for_sync()
    };

    if entries.is_empty() {
        return Ok(0);
    }

    let mut synced = 0;
    for (id, content, mem_type) in &entries {
        let title = format!("[working-memory][{}] {}", mem_type, &content[..content.len().min(50)]);
        let input = CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content: content.clone(),
            source: Some("auto_extract".to_string()),
        };
        match axagent_core::repo::memory::add_item(state.harness.db(), input).await {
            Ok(mem_item) => {
                let ns =
                    axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                        .await
                        .ok();
                if let Some(ns) = ns {
                    if ns.embedding_provider.is_some() {
                        let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
                        let _ = axagent_core::repo::memory::update_item_index_status(
                            state.harness.db(),
                            &mem_item.id,
                            "indexing",
                            None,
                        )
                        .await;

                        let db = state.harness.db().clone();
                        let master_key = state.harness.master_key_owned();
                        let vector_store = state.vector_store.clone();
                        let item_id = mem_item.id.clone();
                        let item_content = mem_item.content.clone();
                        let app_clone = app.clone();

                        tokio::spawn(async move {
                            let res = crate::indexing::index_source(
                                &db,
                                &master_key,
                                &vector_store,
                                &container,
                                &item_id,
                                &item_content,
                                None,
                                None,
                            )
                            .await;

                            let (status, err_msg) = match &res {
                                Ok(_) => ("ready", None),
                                Err(e) => {
                                    tracing::warn!(
                                        "Sync working memory embedding failed for {}: {}",
                                        item_id,
                                        e
                                    );
                                    ("failed", Some(e.to_string()))
                                },
                            };
                            let _ = axagent_core::repo::memory::update_item_index_status(
                                &db,
                                &item_id,
                                status,
                                err_msg.as_deref(),
                            )
                            .await;

                            let _ = app_clone.emit(
                                "memory-item-indexed",
                                serde_json::json!({
                                    "itemId": item_id,
                                    "success": res.is_ok(),
                                    "status": status,
                                    "error": err_msg,
                                }),
                            );
                        });
                    } else {
                        let _ = axagent_core::repo::memory::update_item_index_status(
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
    axagent_core::repo::memory::reorder_namespaces(state.harness.db(), &namespace_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn promote_memory_entry(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.promote_memory(&memory_id);
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn demote_memory_entry(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.demote_memory(&memory_id);
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn add_memory_with_dedup(
    state: State<'_, AppState>,
    target: String,
    content: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.add_memory_with_dedup(&target, &content);
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn apply_memory_decay_tick(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let evicted = ms.apply_decay_tick();
    Ok(serde_json::json!({ "evicted_count": evicted }))
}

#[tauri::command]
pub async fn search_working_memories(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.search_memories(&query, limit.unwrap_or(10));
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn update_memory_importance(
    state: State<'_, AppState>,
    memory_id: String,
    delta: f64,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.update_importance(&memory_id, delta);
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memory_tier_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let usage = ms.get_memory_usage();
    Ok(serde_json::to_value(usage).unwrap_or_default())
}

#[tauri::command]
pub async fn extract_conversation_entities(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<serde_json::Value, String> {
    let messages = axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    if messages.len() < 2 {
        return Ok(serde_json::json!({"entities": [], "relations": []}));
    }

    let (provider, key_row, model_id, settings) = {
        let settings = axagent_core::repo::settings::get_settings(state.harness.db())
            .await
            .unwrap_or_default();
        let provider_id = settings
            .default_provider_id
            .as_deref()
            .ok_or("No default provider configured")?;
        let model_id = settings
            .default_model_id
            .as_deref()
            .ok_or("No default model configured")?;
        let provider = axagent_core::repo::provider::get_provider(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        let key_row = axagent_core::repo::provider::get_active_key(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        (provider, key_row, model_id.to_string(), settings)
    };

    let api_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    let proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &settings);
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| format!("Unsupported provider type: {}", registry_key))?;

    let result = crate::memory_extract::extract_entities_from_messages(
        &messages,
        adapter.as_ref(),
        &ctx,
        &model_id,
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

        let existing_entities = storage
            .search_entities(&ext_entity.name, 5)
            .unwrap_or_default();
        let already_exists = existing_entities.iter().any(|e| {
            e.name.to_lowercase() == ext_entity.name.to_lowercase()
                && e.entity_type
                    == axagent_trajectory::EntityType::from(ext_entity.entity_type.as_str())
        });

        if !already_exists {
            if let Err(e) = storage.save_entity(&entity) {
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
                if !updated
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase() == alias.to_lowercase())
                {
                    updated.aliases.push(alias.clone());
                }
            }
            let _ = storage.save_entity(&updated);
        }
    }

    let storage = {
        let ms = state.memory_service.read().await;
        ms.storage()
    };

    for ext_rel in &result.relations {
        let source_entities = storage
            .search_entities(&ext_rel.source_name, 3)
            .unwrap_or_default();
        let target_entities = storage
            .search_entities(&ext_rel.target_name, 3)
            .unwrap_or_default();

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

        if let Err(e) = storage.save_relationship(&rel) {
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
    let results = ms.graph_enhanced_search(&query, limit.unwrap_or(10));
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn disambiguate_memory_entities(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let result = ms.disambiguate_entities();
    Ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn list_knowledge_graph(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let storage = ms.storage();
    let entities = storage.get_all_entities().map_err(|e| e.to_string())?;
    let relationships = storage.get_all_relationships().map_err(|e| e.to_string())?;
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
    let results = ms.search_memories_by_time_range(start_ts, end_ts, limit.unwrap_or(50));
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memories_time_grouped(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let groups = ms.get_memories_grouped_by_time();
    Ok(serde_json::to_value(groups).unwrap_or_default())
}

#[tauri::command]
pub async fn search_memories_explained(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let results = ms.search_memories_explained(&query, limit.unwrap_or(10));
    Ok(serde_json::to_value(results).unwrap_or_default())
}

#[tauri::command]
pub async fn get_memory_provenance(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let mem = ms.get_working_memory();
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
        None => Err("Memory not found".to_string()),
    }
}

#[tauri::command]
pub async fn find_memory_clusters(
    state: State<'_, AppState>,
    similarity_threshold: Option<f64>,
) -> Result<serde_json::Value, String> {
    let ms = state.memory_service.read().await;
    let clusters = ms.find_similar_clusters(similarity_threshold.unwrap_or(0.5));
    Ok(serde_json::to_value(clusters).unwrap_or_default())
}

#[tauri::command]
pub async fn consolidate_memory_cluster(
    _app: AppHandle,
    state: State<'_, AppState>,
    memory_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    if memory_ids.len() < 2 {
        return Err("Need at least 2 memories to consolidate".to_string());
    }

    let ms = state.memory_service.read().await;
    let mem = ms.get_working_memory();

    let contents: Vec<String> = memory_ids
        .iter()
        .filter_map(|id| mem.entries.get(id).map(|e| e.content.clone()))
        .collect();

    if contents.len() < 2 {
        return Err("Could not find enough memories for consolidation".to_string());
    }

    drop(ms);

    let (provider, key_row, model_id, settings) = {
        let settings = axagent_core::repo::settings::get_settings(state.harness.db())
            .await
            .unwrap_or_default();
        let provider_id = settings
            .default_provider_id
            .as_deref()
            .ok_or("No default provider configured")?;
        let model_id = settings
            .default_model_id
            .as_deref()
            .ok_or("No default model configured")?;
        let provider = axagent_core::repo::provider::get_provider(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        let key_row = axagent_core::repo::provider::get_active_key(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        (provider, key_row, model_id.to_string(), settings)
    };

    let api_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    let proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &settings);
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| format!("Unsupported provider type: {}", registry_key))?;

    let consolidated = crate::memory_extract::consolidate_memories(
        &contents,
        adapter.as_ref(),
        &ctx,
        &model_id,
        PromptLang::ZhCN,
    )
    .await?;

    let ms = state.memory_service.read().await;
    let result = ms.add_memory_advanced(axagent_trajectory::AddMemoryRequest {
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
    });

    if result.success {
        for id in &memory_ids {
            let ms2 = state.memory_service.read().await;
            let _ = ms2.remove_memory("memory", id);
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
    let result = ms.apply_user_feedback(&memory_id, &feedback);
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
    let search_results = state
        .vector_store
        .search(&collection_name, query_embedding, 3)
        .await
        .ok()?;

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
    let messages = axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    let (provider, key_row, model_id, settings) = {
        let settings = axagent_core::repo::settings::get_settings(state.harness.db())
            .await
            .unwrap_or_default();
        let provider_id = settings
            .default_provider_id
            .as_deref()
            .ok_or("No default provider configured")?;
        let model_id = settings
            .default_model_id
            .as_deref()
            .ok_or("No default model configured")?;

        let provider = axagent_core::repo::provider::get_provider(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;
        let key_row = axagent_core::repo::provider::get_active_key(state.harness.db(), provider_id)
            .await
            .map_err(|e| e.to_string())?;

        (provider, key_row, model_id.to_string(), settings)
    };

    let api_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    let proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &settings);
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| format!("Unsupported provider type: {}", registry_key))?;

    let result = crate::memory_extract::extract_memories_from_messages(
        &messages,
        &conversation_id,
        adapter.as_ref(),
        &ctx,
        &model_id,
        PromptLang::ZhCN,
    )
    .await?;

    let existing_items = axagent_core::repo::memory::list_items(state.harness.db(), &namespace_id)
        .await
        .map_err(|e| e.to_string())?;
    let existing_contents: std::collections::HashSet<String> = existing_items
        .iter()
        .map(|item| item.content.to_lowercase().trim().to_string())
        .collect();

    let ns_config = axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
        .await
        .ok();
    let can_vector_dedup = ns_config
        .as_ref()
        .and_then(|ns| ns.embedding_provider.clone())
        .is_some();

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
        };
        match axagent_core::repo::memory::add_item(state.harness.db(), input).await {
            Ok(mem_item) => {
                let ns =
                    axagent_core::repo::memory::get_namespace(state.harness.db(), &namespace_id)
                        .await
                        .ok();
                if let Some(ns) = ns {
                    if ns.embedding_provider.is_some() {
                        let container = axagent_core::rag::KnowledgeContainer::from_memory_ns(&ns);
                        let _ = axagent_core::repo::memory::update_item_index_status(
                            state.harness.db(),
                            &mem_item.id,
                            "indexing",
                            None,
                        )
                        .await;

                        let db = state.harness.db().clone();
                        let master_key = state.harness.master_key_owned();
                        let vector_store = state.vector_store.clone();
                        let item_id = mem_item.id.clone();
                        let content = mem_item.content.clone();
                        let app_clone = app.clone();

                        tokio::spawn(async move {
                            let res = crate::indexing::index_source(
                                &db,
                                &master_key,
                                &vector_store,
                                &container,
                                &item_id,
                                &content,
                                None,
                                None,
                            )
                            .await;

                            let (status, err_msg) = match &res {
                                Ok(_) => ("ready", None),
                                Err(e) => {
                                    tracing::warn!(
                                        "Auto-extract memory embedding failed for {}: {}",
                                        item_id,
                                        e
                                    );
                                    ("failed", Some(e.to_string()))
                                },
                            };
                            let _ = axagent_core::repo::memory::update_item_index_status(
                                &db,
                                &item_id,
                                status,
                                err_msg.as_deref(),
                            )
                            .await;

                            let _ = app_clone.emit(
                                "memory-item-indexed",
                                serde_json::json!({
                                    "itemId": item_id,
                                    "success": res.is_ok(),
                                    "status": status,
                                    "error": err_msg,
                                }),
                            );
                        });

                        saved.push(MemoryItem {
                            index_status: "indexing".to_string(),
                            ..mem_item
                        });

                        {
                            let ms = state.memory_service.read().await;
                            let _ = ms.add_memory_advanced(axagent_trajectory::AddMemoryRequest {
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
                                namespace_id: None,
                            });
                        }

                        continue;
                    }
                }

                let _ = axagent_core::repo::memory::update_item_index_status(
                    state.harness.db(),
                    &mem_item.id,
                    "skipped",
                    None,
                )
                .await;
                saved.push(MemoryItem {
                    index_status: "skipped".to_string(),
                    ..mem_item
                });

                {
                    let ms = state.memory_service.read().await;
                    let _ = ms.add_memory_advanced(axagent_trajectory::AddMemoryRequest {
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
                        namespace_id: None,
                    });
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
    use axagent_core::entity::conversations;
    use sea_orm::{EntityTrait, Set};

    let conv = conversations::Entity::find_by_id(conversation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

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
            (current, "extracted") if !current.starts_with("extract") => "extracted",
            (current, "archived") if !current.starts_with("archiv") => "archived",
            _ => &current_status,
        };
        am.memory_status = Set(new_status.to_string());
        am.last_memory_extracted_at = Set(Some(chrono::Utc::now().to_rfc3339()));
        am.update(db).await.map_err(|e| e.to_string())?;
    }

    Ok(())
}
