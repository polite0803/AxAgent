use crate::AppState;
use axagent_core::rag::KnowledgeContainer;
use axagent_core::types::*;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn list_knowledge_bases(
    state: State<'_, AppState>,
) -> Result<Vec<KnowledgeBase>, String> {
    axagent_core::repo::knowledge::list_knowledge_bases(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_base(
    state: State<'_, AppState>,
    input: CreateKnowledgeBaseInput,
) -> Result<KnowledgeBase, String> {
    axagent_core::repo::knowledge::create_knowledge_base(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_knowledge_base(
    state: State<'_, AppState>,
    id: String,
    input: UpdateKnowledgeBaseInput,
) -> Result<KnowledgeBase, String> {
    axagent_core::repo::knowledge::update_knowledge_base(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_knowledge_base(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Delete vector collection (vec_kb_{id} and vec_kb_{id}_meta tables)
    let collection_id = format!("kb_{}", id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    axagent_core::repo::knowledge::delete_knowledge_base(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reorder_knowledge_bases(
    state: State<'_, AppState>,
    base_ids: Vec<String>,
) -> Result<(), String> {
    axagent_core::repo::knowledge::reorder_knowledge_bases(state.harness.db(), &base_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_knowledge_documents(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<KnowledgeDocument>, String> {
    axagent_core::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_knowledge_document(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    title: String,
    source_path: String,
    mime_type: String,
) -> Result<KnowledgeDocument, String> {
    let doc = axagent_core::repo::knowledge::add_document(
        state.harness.db(),
        &base_id,
        &title,
        &source_path,
        &mime_type,
        None, // doc_type defaults to "file"
    )
    .await
    .map_err(|e| e.to_string())?;

    // Spawn async indexing task
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    if kb.embedding_provider.is_some() {
        let container = axagent_core::rag::KnowledgeContainer::from_knowledge_base(&kb);
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let vector_store = state.vector_store.clone();
        let doc_id = doc.id.clone();
        let src_path = source_path.clone();
        let mime = mime_type.clone();
        let semaphore = state.indexing_semaphore.clone();

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await;
            let result = crate::indexing::index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                &doc_id,
                "",
                Some(&src_path),
                Some(&mime),
            )
            .await;

            if let Err(e) = &result {
                let err_msg = e.to_string();
                tracing::error!("Indexing failed for doc {}: {}", doc_id, err_msg);
                let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                    &db,
                    &doc_id,
                    "failed",
                    Some(&err_msg),
                )
                .await;
            }

            // Emit event to notify frontend
            let _ = app.emit(
                "knowledge-document-indexed",
                serde_json::json!({
                    "documentId": doc_id,
                    "success": result.is_ok(),
                    "error": result.err().map(|e| e.to_string()),
                }),
            );
        });
    }

    Ok(doc)
}

#[tauri::command]
pub async fn delete_knowledge_document(
    state: State<'_, AppState>,
    base_id: String,
    id: String,
) -> Result<(), String> {
    // Delete vector embeddings for this document
    let collection_id = format!("kb_{}", base_id);
    let _ = state
        .vector_store
        .delete_document_embeddings(&collection_id, &id)
        .await;

    axagent_core::repo::knowledge::delete_document(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_knowledge_base(
    state: State<'_, AppState>,
    base_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<axagent_core::vector_store::VectorSearchResult>, String> {
    let mut results = crate::indexing::search_knowledge(
        state.harness.db(),
        state.harness.master_key(),
        &state.vector_store,
        &base_id,
        &query,
        top_k.unwrap_or(5),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Apply distance threshold filter consistent with collect_rag_context
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;
    let default_max_distance = 2.0_f32;
    let threshold = kb.retrieval_threshold.unwrap_or(0.0);
    let effective_threshold = if threshold > 0.0 {
        threshold
    } else {
        default_max_distance
    };
    results.retain(|r| r.score <= effective_threshold);

    Ok(results)
}

#[tauri::command]
pub async fn rebuild_knowledge_index(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
) -> Result<(), String> {
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    let embedding_provider = kb
        .embedding_provider
        .ok_or("No embedding provider configured")?;

    let collection_id = format!("kb_{}", base_id);

    // Get all documents
    let docs = axagent_core::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    if docs.is_empty() {
        let _ = app.emit("knowledge-rebuild-complete", serde_json::json!({ "baseId": base_id }));
        return Ok(());
    }

    // Reset all document statuses to "indexing"
    for doc in &docs {
        let _ = axagent_core::repo::knowledge::update_document_status(
            state.harness.db(),
            &doc.id,
            "indexing",
        )
        .await;
    }

    // Clear only embeddings (vec0), keep _meta intact
    let _ = state.vector_store.clear_embeddings(&collection_id).await;

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();
    let ep = embedding_provider.clone();
    let provider_registry = state.harness.provider_registry().clone();

    tokio::spawn(async move {
        // Process each document individually so status updates per-doc
        for doc in &docs {
            let chunks = match vector_store
                .list_document_chunks_raw(&collection_id, &doc.id)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                        &db,
                        &doc.id,
                        "failed",
                        Some(&err_msg),
                    )
                    .await;
                    let _ = app.emit(
                        "knowledge-document-indexed",
                        serde_json::json!({
                            "documentId": doc.id,
                            "success": false,
                            "error": err_msg,
                        }),
                    );
                    continue;
                },
            };

            if chunks.is_empty() {
                let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                    &db, &doc.id, "ready", None,
                )
                .await;
                let _ = app.emit(
                    "knowledge-document-indexed",
                    serde_json::json!({ "documentId": doc.id, "success": true }),
                );
                continue;
            }

            let texts: Vec<String> = chunks
                .iter()
                .map(|(_, _, content)| content.clone())
                .collect();
            let rowids: Vec<i64> = chunks.iter().map(|(rid, _, _)| *rid).collect();

            match crate::indexing::generate_embeddings(
                &db,
                &master_key,
                &provider_registry,
                &ep,
                texts,
                None,
            )
            .await
            {
                Ok(embed_response) => {
                    let entries: Vec<(i64, Vec<f32>)> =
                        rowids.into_iter().zip(embed_response.embeddings).collect();

                    if let Err(e) = vector_store
                        .upsert_document_embeddings(&collection_id, entries)
                        .await
                    {
                        let err_msg = e.to_string();
                        tracing::error!(
                            "Failed to upsert embeddings for doc {}: {}",
                            doc.id,
                            err_msg
                        );
                        let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                            &db,
                            &doc.id,
                            "failed",
                            Some(&err_msg),
                        )
                        .await;
                        let _ = app.emit(
                            "knowledge-document-indexed",
                            serde_json::json!({
                                "documentId": doc.id,
                                "success": false,
                                "error": err_msg,
                            }),
                        );
                    } else {
                        let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                            &db, &doc.id, "ready", None,
                        )
                        .await;
                        let _ = app.emit(
                            "knowledge-document-indexed",
                            serde_json::json!({
                                "documentId": doc.id,
                                "success": true,
                            }),
                        );
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    tracing::error!("Failed to embed doc {} during rebuild: {}", doc.id, err_msg);
                    let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                        &db,
                        &doc.id,
                        "failed",
                        Some(&err_msg),
                    )
                    .await;
                    let _ = app.emit(
                        "knowledge-document-indexed",
                        serde_json::json!({
                            "documentId": doc.id,
                            "success": false,
                            "error": err_msg,
                        }),
                    );
                },
            }
        }

        let _ = app.emit("knowledge-rebuild-complete", serde_json::json!({ "baseId": base_id }));
    });

    Ok(())
}

#[tauri::command]
pub async fn list_knowledge_containers(
    state: State<'_, AppState>,
) -> Result<Vec<KnowledgeContainer>, String> {
    let mut containers = Vec::new();

    let kbs = axagent_core::repo::knowledge::list_knowledge_bases(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for kb in kbs {
        containers.push(KnowledgeContainer::from_knowledge_base(&kb));
    }

    let namespaces = axagent_core::repo::memory::list_namespaces(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for ns in namespaces {
        containers.push(KnowledgeContainer::from_memory_ns(&ns));
    }

    let wikis = axagent_core::repo::wiki::list_wikis(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for wiki in wikis {
        containers.push(KnowledgeContainer::from_wiki(&wiki));
    }

    containers.sort_by_key(|c| c.sort_order);

    Ok(containers)
}

#[tauri::command]
pub async fn list_knowledge_entities(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_core::types::KnowledgeEntity>, String> {
    axagent_core::repo::knowledge_graph::list_knowledge_entities(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_entity(
    state: State<'_, AppState>,
    input: axagent_core::types::CreateKnowledgeEntityInput,
) -> Result<axagent_core::types::KnowledgeEntity, String> {
    axagent_core::repo::knowledge_graph::create_knowledge_entity(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_knowledge_attributes(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<axagent_core::types::KnowledgeAttribute>, String> {
    axagent_core::repo::knowledge_graph::list_knowledge_attributes(state.harness.db(), &entity_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_attribute(
    state: State<'_, AppState>,
    input: axagent_core::types::CreateKnowledgeAttributeInput,
) -> Result<axagent_core::types::KnowledgeAttribute, String> {
    axagent_core::repo::knowledge_graph::create_knowledge_attribute(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_knowledge_relations(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_core::types::KnowledgeRelation>, String> {
    axagent_core::repo::knowledge_graph::list_knowledge_relations(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_relation(
    state: State<'_, AppState>,
    input: axagent_core::types::CreateKnowledgeRelationInput,
) -> Result<axagent_core::types::KnowledgeRelation, String> {
    axagent_core::repo::knowledge_graph::create_knowledge_relation(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_knowledge_flows(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_core::types::KnowledgeFlow>, String> {
    axagent_core::repo::knowledge_graph::list_knowledge_flows(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_flow(
    state: State<'_, AppState>,
    input: axagent_core::types::CreateKnowledgeFlowInput,
) -> Result<axagent_core::types::KnowledgeFlow, String> {
    axagent_core::repo::knowledge_graph::create_knowledge_flow(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_knowledge_interfaces(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_core::types::KnowledgeInterface>, String> {
    axagent_core::repo::knowledge_graph::list_knowledge_interfaces(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_knowledge_interface(
    state: State<'_, AppState>,
    input: axagent_core::types::CreateKnowledgeInterfaceInput,
) -> Result<axagent_core::types::KnowledgeInterface, String> {
    axagent_core::repo::knowledge_graph::create_knowledge_interface(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_knowledge_index(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<(), String> {
    let collection_id = format!("kb_{}", base_id);
    // Only clear embeddings (vec0), keep chunk metadata (_meta) intact
    state
        .vector_store
        .clear_embeddings(&collection_id)
        .await
        .map_err(|e| e.to_string())?;

    // Reset all documents to "pending"
    let docs = axagent_core::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    for doc in docs {
        let _ = axagent_core::repo::knowledge::update_document_status(
            state.harness.db(),
            &doc.id,
            "pending",
        )
        .await;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_knowledge_document_chunks(
    state: State<'_, AppState>,
    base_id: String,
    document_id: String,
) -> Result<Vec<axagent_core::vector_store::VectorSearchResult>, String> {
    let collection_id = format!("kb_{}", base_id);
    state
        .vector_store
        .list_document_chunks(&collection_id, &document_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_knowledge_chunk(
    state: State<'_, AppState>,
    base_id: String,
    chunk_id: String,
) -> Result<(), String> {
    let collection_id = format!("kb_{}", base_id);
    state
        .vector_store
        .delete_chunk(&collection_id, &chunk_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_knowledge_chunk(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    chunk_id: String,
    content: String,
) -> Result<(), String> {
    let collection_id = format!("kb_{}", base_id);
    state
        .vector_store
        .update_chunk_content(&collection_id, &chunk_id, &content)
        .await
        .map_err(|e| e.to_string())?;

    // Auto-reindex: re-embed the chunk with the updated content
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(embedding_provider) = kb.embedding_provider {
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let provider_registry = state.harness.provider_registry().clone();
        let vector_store = state.vector_store.clone();
        let cid = chunk_id.clone();
        let chunk_content = content.clone();

        tokio::spawn(async move {
            let result = async {
                let embed_response = crate::indexing::generate_embeddings(
                    &db,
                    &master_key,
                    &provider_registry,
                    &embedding_provider,
                    vec![chunk_content],
                    None,
                )
                .await?;

                if let Some(embedding) = embed_response.embeddings.into_iter().next() {
                    vector_store
                        .update_chunk_embedding(&collection_id, &cid, &embedding)
                        .await?;
                }
                Ok::<_, axagent_core::error::AxAgentError>(())
            }
            .await;

            if let Err(e) = &result {
                tracing::warn!("Auto-reindex failed for chunk {}: {}", cid, e);
            }

            let _ = app.emit(
                "knowledge-chunk-reindexed",
                serde_json::json!({
                    "chunkId": cid,
                    "success": result.is_ok(),
                    "error": result.err().map(|e| e.to_string()),
                }),
            );
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn add_knowledge_chunk(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    document_id: String,
    content: String,
) -> Result<String, String> {
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    let embedding_provider = kb
        .embedding_provider
        .ok_or_else(|| "No embedding provider configured".to_string())?;

    let collection_id = format!("kb_{}", base_id);
    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();
    let doc_id = document_id.clone();
    let chunk_content = content.clone();
    let provider_registry = state.harness.provider_registry().clone();

    let chunk_id_result = tokio::spawn(async move {
        let embed_response = crate::indexing::generate_embeddings(
            &db,
            &master_key,
            &provider_registry,
            &embedding_provider,
            vec![chunk_content.clone()],
            None,
        )
        .await?;

        let embedding = embed_response
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| {
                axagent_core::error::AxAgentError::Provider("No embedding returned".to_string())
            })?;

        let chunk_id = vector_store
            .add_single_chunk(&collection_id, &doc_id, &chunk_content, &embedding)
            .await?;

        let _ = app.emit(
            "knowledge-chunk-added",
            serde_json::json!({
                "baseId": base_id,
                "documentId": doc_id,
                "chunkId": chunk_id,
            }),
        );

        Ok::<String, axagent_core::error::AxAgentError>(chunk_id)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(chunk_id_result)
}

#[tauri::command]
pub async fn reindex_knowledge_chunk(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    chunk_id: String,
) -> Result<(), String> {
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    let embedding_provider = kb
        .embedding_provider
        .ok_or_else(|| "No embedding provider configured".to_string())?;

    let collection_id = format!("kb_{}", base_id);

    let chunk_content = {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};
        let name = format!("vec_kb_{}", base_id.replace('-', "_"));
        let row = state
            .harness
            .db()
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                format!("SELECT content FROM {name}_meta WHERE id = $1"),
                vec![chunk_id.clone().into()],
            ))
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Chunk {} not found", chunk_id))?;
        row.try_get::<String>("", "content")
            .map_err(|e| e.to_string())?
    };

    // Embed the single chunk
    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let provider_registry = state.harness.provider_registry().clone();
    let vector_store = state.vector_store.clone();
    let cid = chunk_id.clone();

    tokio::spawn(async move {
        let result = async {
            let embed_response = crate::indexing::generate_embeddings(
                &db,
                &master_key,
                &provider_registry,
                &embedding_provider,
                vec![chunk_content],
                None,
            )
            .await?;

            if let Some(embedding) = embed_response.embeddings.into_iter().next() {
                vector_store
                    .update_chunk_embedding(&collection_id, &cid, &embedding)
                    .await?;
            }
            Ok::<_, axagent_core::error::AxAgentError>(())
        }
        .await;

        let _ = app.emit(
            "knowledge-chunk-reindexed",
            serde_json::json!({
                "chunkId": cid,
                "success": result.is_ok(),
                "error": result.err().map(|e| e.to_string()),
            }),
        );
    });

    Ok(())
}

/// Rebuild the index for a single document (re-embed its chunks only).
#[tauri::command]
pub async fn rebuild_knowledge_document(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    document_id: String,
) -> Result<(), String> {
    let kb = axagent_core::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| e.to_string())?;

    let embedding_provider = kb
        .embedding_provider
        .ok_or("No embedding provider configured")?;

    let collection_id = format!("kb_{}", base_id);

    let chunks = state
        .vector_store
        .list_document_chunks_raw(&collection_id, &document_id)
        .await
        .map_err(|e| e.to_string())?;

    if chunks.is_empty() {
        let _ = app.emit(
            "knowledge-document-indexed",
            serde_json::json!({ "documentId": document_id, "success": true }),
        );
        return Ok(());
    }

    // Set document status to "indexing"
    let _ = axagent_core::repo::knowledge::update_document_status(
        state.harness.db(),
        &document_id,
        "indexing",
    )
    .await;

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();
    let ep = embedding_provider.clone();
    let doc_id = document_id.clone();
    let provider_registry = state.harness.provider_registry().clone();

    tokio::spawn(async move {
        let texts: Vec<String> = chunks
            .iter()
            .map(|(_, _, content)| content.clone())
            .collect();
        let rowids: Vec<i64> = chunks.iter().map(|(rid, _, _)| *rid).collect();

        let result = crate::indexing::generate_embeddings(
            &db,
            &master_key,
            &provider_registry,
            &ep,
            texts,
            None,
        )
        .await;

        match result {
            Ok(embed_response) => {
                let entries: Vec<(i64, Vec<f32>)> =
                    rowids.into_iter().zip(embed_response.embeddings).collect();

                if let Err(e) = vector_store
                    .upsert_document_embeddings(&collection_id, entries)
                    .await
                {
                    let err_msg = e.to_string();
                    tracing::error!("Failed to upsert embeddings for doc {}: {}", doc_id, err_msg);
                    let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                        &db,
                        &doc_id,
                        "failed",
                        Some(&err_msg),
                    )
                    .await;
                    let _ = app.emit(
                        "knowledge-document-indexed",
                        serde_json::json!({
                            "documentId": doc_id,
                            "success": false,
                            "error": err_msg,
                        }),
                    );
                } else {
                    let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                        &db, &doc_id, "ready", None,
                    )
                    .await;
                    let _ = app.emit(
                        "knowledge-document-indexed",
                        serde_json::json!({
                            "documentId": doc_id,
                            "success": true,
                        }),
                    );
                }
            },
            Err(e) => {
                let err_msg = e.to_string();
                tracing::error!("Failed to embed doc {}: {}", doc_id, err_msg);
                let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                    &db,
                    &doc_id,
                    "failed",
                    Some(&err_msg),
                )
                .await;
                let _ = app.emit(
                    "knowledge-document-indexed",
                    serde_json::json!({
                        "documentId": doc_id,
                        "success": false,
                        "error": err_msg,
                    }),
                );
            },
        }
    });

    Ok(())
}
