// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_dao::repo::index_jobs as jobs;
use axagent_harness::types::*;
use axagent_search::rag::KnowledgeContainer;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

/// 目录导入结果（单文档批量导入的汇总）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDirectoryError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDirectoryResult {
    pub base_id: String,
    pub imported_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub imported: Vec<KnowledgeDocument>,
    pub skipped: Vec<String>,
    pub errors: Vec<ImportDirectoryError>,
}

/// document-parser 支持解析的扩展名；目录导入仅收录这些类型。
fn is_supported_knowledge_ext(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "txt"
            | "md"
            | "markdown"
            | "csv"
            | "html"
            | "htm"
            | "xml"
            | "json"
            | "pdf"
            | "docx"
            | "xlsx"
            | "pptx"
    )
}

/// 收集目录下的可导入文件，跳过隐藏文件/目录与不支持的扩展名。
/// `extensions` 指定时仅收录该白名单内的扩展名，否则使用 [`is_supported_knowledge_ext`]。
fn collect_importable_files(
    dir: &std::path::Path,
    recursive: bool,
    extensions: &Option<Vec<String>>,
    files: &mut Vec<PathBuf>,
    skipped: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        // 跳过隐藏项（如 .git / .DS_Store）
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if file_type.is_dir() {
            if recursive {
                collect_importable_files(&path, recursive, extensions, files, skipped)?;
            }
        } else if file_type.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase());
            let allowed = match extensions {
                Some(exts) => ext
                    .as_ref()
                    .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
                    .unwrap_or(false),
                None => ext.as_deref().map(is_supported_knowledge_ext).unwrap_or(false),
            };
            if allowed {
                files.push(path);
            } else {
                skipped.push(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn list_knowledge_bases(
    state: State<'_, AppState>,
) -> Result<Vec<KnowledgeBase>, String> {
    axagent_dao::repo::knowledge::list_knowledge_bases(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn create_knowledge_base(
    state: State<'_, AppState>,
    input: CreateKnowledgeBaseInput,
) -> Result<KnowledgeBase, String> {
    axagent_dao::repo::knowledge::create_knowledge_base(state.harness.db(), input).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn update_knowledge_base(
    state: State<'_, AppState>,
    id: String,
    input: UpdateKnowledgeBaseInput,
) -> Result<KnowledgeBase, String> {
    axagent_dao::repo::knowledge::update_knowledge_base(state.harness.db(), &id, input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn delete_knowledge_base(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Delete vector collection (vec_kb_{id} and vec_kb_{id}_meta tables)
    let collection_id = format!("kb_{}", id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    axagent_dao::repo::knowledge::delete_knowledge_base(state.harness.db(), &id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn reorder_knowledge_bases(
    state: State<'_, AppState>,
    base_ids: Vec<String>,
) -> Result<(), String> {
    axagent_dao::repo::knowledge::reorder_knowledge_bases(state.harness.db(), &base_ids)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn list_knowledge_documents(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<KnowledgeDocument>, String> {
    axagent_dao::repo::knowledge::list_documents(state.harness.db(), &base_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
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
    let doc = axagent_dao::repo::knowledge::add_document(
        state.harness.db(),
        &base_id,
        &title,
        &source_path,
        &mime_type,
        None, // doc_type defaults to "file"
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 将文档状态标记为pending（等待队列处理）
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if kb.embedding_provider.is_some() {
        let _ = axagent_dao::repo::knowledge::update_document_status(
            state.harness.db(),
            &doc.id,
            "pending",
        )
        .await;
        crate::index_queue::enqueue_job_sync(
            &state,
            &app,
            jobs::JOB_TYPE_INDEX_DOCUMENT,
            "kb",
            &base_id,
            &doc.id,
            None,
            None,
        )
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    Ok(doc)
}

/// 批量导入一个目录下的文档到指定知识库。
///
/// - `directory_path`：要导入的目录绝对路径
/// - `recursive`：是否递归子目录（默认 false）
/// - `extensions`：可选扩展名白名单（不含点，如 `["md", "txt"]`），未指定则使用支持的类型集
///
/// 仅收录 document-parser 支持的类型；其余文件计入 `skipped`。
/// 若知识库配置了 embedding 提供方，每个文档会被标记为 pending 并入队索引任务。
#[tauri::command]
pub async fn import_knowledge_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    directory_path: String,
    recursive: Option<bool>,
    extensions: Option<Vec<String>>,
) -> Result<ImportDirectoryResult, String> {
    let dir = PathBuf::from(&directory_path);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("路径不存在或不是目录: {directory_path}"));
    }

    let recursive = recursive.unwrap_or(false);

    let mut files = Vec::new();
    let mut skipped = Vec::new();
    collect_importable_files(&dir, recursive, &extensions, &mut files, &mut skipped)
        .map_err(|e| format!("读取目录失败 {directory_path}: {e}"))?;

    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let has_embedding = kb.embedding_provider.is_some();

    let mut result = ImportDirectoryResult {
        base_id: base_id.clone(),
        imported_count: 0,
        skipped_count: 0,
        error_count: 0,
        imported: Vec::new(),
        skipped,
        errors: Vec::new(),
    };

    for path in files {
        let abs = path.to_string_lossy().to_string();
        let mime = axagent_document_parser::mime_from_extension(&path).to_string();

        // 递归导入时用相对路径作为标题，避免重名；非递归用文件名
        let title = if recursive {
            path.strip_prefix(&dir).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(
                |_| path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            )
        } else {
            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        };

        match axagent_dao::repo::knowledge::add_document(
            state.harness.db(),
            &base_id,
            &title,
            &abs,
            &mime,
            None,
        )
        .await
        {
            Ok(doc) => {
                if has_embedding {
                    let _ = axagent_dao::repo::knowledge::update_document_status(
                        state.harness.db(),
                        &doc.id,
                        "pending",
                    )
                    .await;
                    if let Err(e) = crate::index_queue::enqueue_job_sync(
                        &state,
                        &app,
                        jobs::JOB_TYPE_INDEX_DOCUMENT,
                        "kb",
                        &base_id,
                        &doc.id,
                        None,
                        None,
                    ) {
                        tracing::warn!("[knowledge] 目录导入入队索引失败 {}: {}", doc.id, e);
                    }
                }
                result.imported_count += 1;
                result.imported.push(doc);
            },
            Err(e) => {
                result.error_count += 1;
                result.errors.push(ImportDirectoryError { path: abs, error: e.to_string() });
            },
        }
    }

    result.skipped_count = result.skipped.len();

    Ok(result)
}

#[tauri::command]
pub async fn delete_knowledge_document(
    state: State<'_, AppState>,
    base_id: String,
    id: String,
) -> Result<(), String> {
    // Delete vector embeddings for this document
    let collection_id = format!("kb_{}", base_id);
    let _ = state.vector_store.delete_document_embeddings(&collection_id, &id).await;

    axagent_dao::repo::knowledge::delete_document(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn search_knowledge_base(
    state: State<'_, AppState>,
    base_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<axagent_search::vector_store::VectorSearchResult>, String> {
    let mut results = crate::indexing::search_knowledge(
        state.harness.db(),
        state.harness.master_key(),
        &state.vector_store,
        &base_id,
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

    // Apply distance threshold filter consistent with collect_rag_context
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
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
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let embedding_provider = kb.embedding_provider.ok_or("No embedding provider configured")?;

    let collection_id = format!("kb_{}", base_id);

    // Get all documents
    let docs = axagent_dao::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if docs.is_empty() {
        let _ = app.emit("knowledge-rebuild-complete", serde_json::json!({ "baseId": base_id }));
        return Ok(());
    }

    // Reset all document statuses to "indexing"
    for doc in &docs {
        let _ = axagent_dao::repo::knowledge::update_document_status(
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

    tokio::spawn(catch_unwind_logged("knowledge.batch_index_docs", async move {
        for doc in &docs {
            let chunks = match vector_store.list_document_chunks_raw(&collection_id, &doc.id).await
            {
                Ok(c) => c,
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
                let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
                    &db, &doc.id, "ready", None,
                )
                .await;
                let _ = app.emit(
                    "knowledge-document-indexed",
                    serde_json::json!({ "documentId": doc.id, "success": true }),
                );
                continue;
            }

            let texts: Vec<String> = chunks.iter().map(|(_, _, content)| content.clone()).collect();
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

                    if let Err(e) =
                        vector_store.upsert_document_embeddings(&collection_id, entries).await
                    {
                        let err_msg = e.to_string();
                        tracing::error!(
                            "Failed to upsert embeddings for doc {}: {}",
                            doc.id,
                            err_msg
                        );
                        let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
                        let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
                    let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
    }));

    Ok(())
}

#[tauri::command]
pub async fn list_knowledge_containers(
    state: State<'_, AppState>,
) -> Result<Vec<KnowledgeContainer>, String> {
    let mut containers = Vec::new();

    let kbs = axagent_dao::repo::knowledge::list_knowledge_bases(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    for kb in kbs {
        containers.push(KnowledgeContainer::from_knowledge_base(&kb));
    }

    let namespaces =
        axagent_dao::repo::memory::list_namespaces(state.harness.db()).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    for ns in namespaces {
        containers.push(KnowledgeContainer::from_memory_ns(&ns));
    }

    let wikis = axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
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
) -> Result<Vec<axagent_harness::types::KnowledgeEntity>, String> {
    axagent_dao::repo::knowledge_graph::list_knowledge_entities(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn create_knowledge_entity(
    state: State<'_, AppState>,
    input: axagent_harness::types::CreateKnowledgeEntityInput,
) -> Result<axagent_harness::types::KnowledgeEntity, String> {
    axagent_dao::repo::knowledge_graph::create_knowledge_entity(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn list_knowledge_attributes(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<axagent_harness::types::KnowledgeAttribute>, String> {
    axagent_dao::repo::knowledge_graph::list_knowledge_attributes(state.harness.db(), &entity_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn create_knowledge_attribute(
    state: State<'_, AppState>,
    input: axagent_harness::types::CreateKnowledgeAttributeInput,
) -> Result<axagent_harness::types::KnowledgeAttribute, String> {
    axagent_dao::repo::knowledge_graph::create_knowledge_attribute(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn list_knowledge_relations(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_harness::types::KnowledgeRelation>, String> {
    axagent_dao::repo::knowledge_graph::list_knowledge_relations(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn create_knowledge_relation(
    state: State<'_, AppState>,
    input: axagent_harness::types::CreateKnowledgeRelationInput,
) -> Result<axagent_harness::types::KnowledgeRelation, String> {
    axagent_dao::repo::knowledge_graph::create_knowledge_relation(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn list_knowledge_flows(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_harness::types::KnowledgeFlow>, String> {
    axagent_dao::repo::knowledge_graph::list_knowledge_flows(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn create_knowledge_flow(
    state: State<'_, AppState>,
    input: axagent_harness::types::CreateKnowledgeFlowInput,
) -> Result<axagent_harness::types::KnowledgeFlow, String> {
    axagent_dao::repo::knowledge_graph::create_knowledge_flow(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn list_knowledge_interfaces(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<Vec<axagent_harness::types::KnowledgeInterface>, String> {
    axagent_dao::repo::knowledge_graph::list_knowledge_interfaces(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn create_knowledge_interface(
    state: State<'_, AppState>,
    input: axagent_harness::types::CreateKnowledgeInterfaceInput,
) -> Result<axagent_harness::types::KnowledgeInterface, String> {
    axagent_dao::repo::knowledge_graph::create_knowledge_interface(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn clear_knowledge_index(
    state: State<'_, AppState>,
    base_id: String,
) -> Result<(), String> {
    let collection_id = format!("kb_{}", base_id);
    // Only clear embeddings (vec0), keep chunk metadata (_meta) intact
    state.vector_store.clear_embeddings(&collection_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // Reset all documents to "pending"
    let docs = axagent_dao::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    for doc in docs {
        let _ = axagent_dao::repo::knowledge::update_document_status(
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
) -> Result<Vec<axagent_search::vector_store::VectorSearchResult>, String> {
    let collection_id = format!("kb_{}", base_id);
    state.vector_store.list_document_chunks(&collection_id, &document_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn delete_knowledge_chunk(
    state: State<'_, AppState>,
    base_id: String,
    chunk_id: String,
) -> Result<(), String> {
    let collection_id = format!("kb_{}", base_id);
    state.vector_store.delete_chunk(&collection_id, &chunk_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
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
    state.vector_store.update_chunk_content(&collection_id, &chunk_id, &content).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;

    // Auto-reindex: re-embed the chunk with the updated content
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if let Some(embedding_provider) = kb.embedding_provider {
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let provider_registry = state.harness.provider_registry().clone();
        let vector_store = state.vector_store.clone();
        let cid = chunk_id.clone();
        let chunk_content = content.clone();

        tokio::spawn(catch_unwind_logged("knowledge.auto_reindex_chunk", async move {
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
                    vector_store.update_chunk_embedding(&collection_id, &cid, &embedding).await?;
                }
                Ok::<_, axagent_harness::core_error::AxAgentError>(())
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
        }));
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
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let embedding_provider =
        kb.embedding_provider.ok_or_else(|| "No embedding provider configured".to_string())?;

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

        let embedding = embed_response.embeddings.into_iter().next().ok_or_else(|| {
            axagent_harness::core_error::AxAgentError::Provider("No embedding returned".to_string())
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

        Ok::<String, axagent_harness::core_error::AxAgentError>(chunk_id)
    })
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(chunk_id_result)
}

#[tauri::command]
pub async fn reindex_knowledge_chunk(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    chunk_id: String,
) -> Result<(), String> {
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let embedding_provider =
        kb.embedding_provider.ok_or_else(|| "No embedding provider configured".to_string())?;

    // Whitelist check: base_id must only contain alphanumeric chars and hyphens (for safe table name usage)
    if !base_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!(
            "Invalid base_id: '{base_id}' — only ASCII alphanumeric and hyphens allowed"
        ));
    }

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
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
            .ok_or_else(|| format!("Chunk {} not found", chunk_id))?;
        row.try_get::<String>("", "content").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
    };

    // Embed the single chunk
    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let provider_registry = state.harness.provider_registry().clone();
    let vector_store = state.vector_store.clone();
    let cid = chunk_id.clone();

    tokio::spawn(catch_unwind_logged("knowledge.reindex_chunk", async move {
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
                vector_store.update_chunk_embedding(&collection_id, &cid, &embedding).await?;
            }
            Ok::<_, axagent_harness::core_error::AxAgentError>(())
        }
        .await;

        if let Err(ref e) = result {
            tracing::warn!("[knowledge] 重索引单块失败 (chunk={}): {}", cid, e);
        }

        let _ = app.emit(
            "knowledge-chunk-reindexed",
            serde_json::json!({
                "chunkId": cid,
                "success": result.is_ok(),
                "error": result.err().map(|e| e.to_string()),
            }),
        );
    }));

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
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let embedding_provider = kb.embedding_provider.ok_or("No embedding provider configured")?;

    let collection_id = format!("kb_{}", base_id);

    let chunks =
        state.vector_store.list_document_chunks_raw(&collection_id, &document_id).await.map_err(
            |e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            },
        )?;

    if chunks.is_empty() {
        let _ = app.emit(
            "knowledge-document-indexed",
            serde_json::json!({ "documentId": document_id, "success": true }),
        );
        return Ok(());
    }

    // Set document status to "indexing"
    let _ = axagent_dao::repo::knowledge::update_document_status(
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

    tokio::spawn(catch_unwind_logged("knowledge.rebuild_doc", async move {
        let texts: Vec<String> = chunks.iter().map(|(_, _, content)| content.clone()).collect();
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

                if let Err(e) =
                    vector_store.upsert_document_embeddings(&collection_id, entries).await
                {
                    let err_msg = e.to_string();
                    tracing::error!("Failed to upsert embeddings for doc {}: {}", doc_id, err_msg);
                    let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
                    let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
                let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
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
    }));

    Ok(())
}

// ── lemonhu 开源股票知识库导入 ─────────────────────────────

/// 从 knowledge-sources/lemonhu/ 导入全部知识图谱数据
///
/// 导入 CSV（stock/concept/industry/executive + 关系）和 wiki_pages 到 DB。
/// 幂等：已存在的记录会被跳过。
#[tauri::command]
pub async fn import_lemonhu_knowledge(
    state: State<'_, AppState>,
    knowledge_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::{
        knowledge_bases, knowledge_documents, knowledge_entities, knowledge_relations,
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

    let db = state.harness.db();
    let kb_id = "lemonhu_knowledge_graph";
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 确定知识库目录
    let knowledge_dir = match knowledge_dir {
        Some(d) => PathBuf::from(d),
        None => {
            let cwd = std::env::current_dir().map_err(|e| format!("获取 cwd 失败: {e}"))?;
            let candidate = cwd.parent().unwrap_or(&cwd).join("knowledge-sources").join("lemonhu");
            if candidate.exists() {
                candidate
            } else {
                cwd.join("knowledge-sources").join("lemonhu")
            }
        },
    };
    if !knowledge_dir.exists() {
        return Err(format!("知识库目录不存在: {}", knowledge_dir.display()));
    }

    // 确保 knowledge_bases 存在
    let kb_exists = knowledge_bases::Entity::find_by_id(kb_id)
        .one(db)
        .await
        .map_err(|e| format!("查 knowledge_bases 失败: {e}"))?
        .is_some();
    if !kb_exists {
        knowledge_bases::ActiveModel {
            id: Set(kb_id.to_string()),
            name: Set("开源股票知识库(lemonhu)".into()),
            description: Set(Some(
                "由开源项目 lemonhu 构建的 A 股知识图谱，含概念/行业/公司/高管关系及百科文档"
                    .into(),
            )),
            embedding_provider: Set(None),
            enabled: Set(1),
            icon_type: Set(Some("book".into())),
            icon_value: Set(None),
            sort_order: Set(0),
            embedding_dimensions: Set(None),
            retrieval_threshold: Set(None),
            retrieval_top_k: Set(None),
            chunk_size: Set(None),
            chunk_overlap: Set(None),
            separator: Set(None),
        }
        .insert(db)
        .await
        .map_err(|e| format!("创建 knowledge_bases 失败: {e}"))?;
    }

    let raw_dir = knowledge_dir.join("raw");
    let mut entity_count = 0usize;
    let mut rel_count = 0usize;

    // ── 收集 entities ──
    // (id, name, entity_type, source_path)
    let mut entity_data: Vec<(String, String, &str, &str)> = Vec::new();

    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("stock.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(4, ',').collect();
            if fields.len() < 3 {
                continue;
            }
            entity_data.push((
                fields[0].to_string(),
                fields[1].to_string(),
                "company",
                "raw/stock.csv",
            ));
        }
    }
    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("concept.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, ',').collect();
            if fields.len() < 2 {
                continue;
            }
            entity_data.push((
                fields[0].to_string(),
                fields[1].to_string(),
                "concept",
                "raw/concept.csv",
            ));
        }
    }
    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("industry.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, ',').collect();
            if fields.len() < 2 {
                continue;
            }
            entity_data.push((
                fields[0].to_string(),
                fields[1].to_string(),
                "industry",
                "raw/industry.csv",
            ));
        }
    }
    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("executive.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(5, ',').collect();
            if fields.len() < 2 {
                continue;
            }
            entity_data.push((
                fields[0].to_string(),
                fields[1].to_string(),
                "person",
                "raw/executive.csv",
            ));
        }
    }

    for (id, name, etype, source) in entity_data {
        let exists = knowledge_entities::Entity::find_by_id(&id)
            .one(db)
            .await
            .map(|o| o.is_some())
            .unwrap_or(false);
        if exists {
            continue;
        }
        let active = knowledge_entities::ActiveModel {
            id: Set(id),
            knowledge_base_id: Set(kb_id.to_string()),
            name: Set(name),
            entity_type: Set(etype.into()),
            description: Set(None),
            source_path: Set(source.into()),
            source_language: Set(None),
            properties: Set(serde_json::json!({})),
            lifecycle: Set(None),
            behaviors: Set(None),
            metadata: Set(None),
            // v101: trajectory entity fields
            aliases: Set(String::new()),
            mention_count: Set(0),
            confidence: Set(0.0),
            first_seen_at: Set(None),
            last_seen_at: Set(None),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        };
        if active.insert(db).await.is_ok() {
            entity_count += 1;
        }
    }

    // ── 收集 relations ──
    // (id, source, target, relation_type)
    let mut rel_data: Vec<(String, String, String, String)> = Vec::new();

    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("stock_concept.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, ',').collect();
            if fields.len() < 3 {
                continue;
            }
            let src = fields[0].to_string();
            let tgt = fields[1].to_string();
            rel_data.push((format!("{src}_has_concept_{tgt}"), src, tgt, "has_concept".into()));
        }
    }
    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("stock_industry.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, ',').collect();
            if fields.len() < 3 {
                continue;
            }
            let src = fields[0].to_string();
            let tgt = fields[1].to_string();
            rel_data.push((format!("{src}_in_industry_{tgt}"), src, tgt, "in_industry".into()));
        }
    }
    if let Ok(csv) = std::fs::read_to_string(raw_dir.join("executive_stock.csv")) {
        for line in csv.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.splitn(4, ',').collect();
            if fields.len() < 4 {
                continue;
            }
            let src = fields[0].to_string();
            let position = fields[1].replace('/', "_");
            let tgt = fields[2].to_string();
            let rel_type = format!("employ_{position}");
            rel_data.push((format!("{src}_{rel_type}_{tgt}"), src, tgt, rel_type));
        }
    }

    for (id, src, tgt, rtype) in rel_data {
        let exists = knowledge_relations::Entity::find_by_id(&id)
            .one(db)
            .await
            .map(|o| o.is_some())
            .unwrap_or(false);
        if exists {
            continue;
        }
        let active = knowledge_relations::ActiveModel {
            id: Set(id),
            knowledge_base_id: Set(kb_id.to_string()),
            source_entity_id: Set(src),
            target_entity_id: Set(tgt),
            relation_type: Set(rtype),
            description: Set(None),
            properties: Set(None),
            metadata: Set(None),
            // v101: trajectory relationship weight
            weight: Set(0.0),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        };
        if active.insert(db).await.is_ok() {
            rel_count += 1;
        }
    }

    // ── 导入 wiki_pages ──
    let mut doc_count = 0usize;
    let wiki_dir = knowledge_dir.join("wiki_pages");
    if wiki_dir.exists() {
        let existing_docs = knowledge_documents::Entity::find()
            .filter(knowledge_documents::Column::KnowledgeBaseId.eq(kb_id))
            .count(db)
            .await
            .unwrap_or(0);
        if existing_docs == 0 {
            if let Ok(mut reader) = std::fs::read_dir(&wiki_dir) {
                while let Ok(Some(entry)) = reader.next().transpose() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("md") {
                        continue;
                    }
                    let content = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let file_name =
                        path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                    let title = content
                        .lines()
                        .find(|l| !l.trim().is_empty())
                        .map(|l| l.trim().chars().take(80).collect::<String>())
                        .unwrap_or_else(|| file_name.clone());
                    let active = knowledge_documents::ActiveModel {
                        id: Set(uuid::Uuid::new_v4().to_string()),
                        knowledge_base_id: Set(kb_id.to_string()),
                        title: Set(title),
                        source_path: Set(path.to_string_lossy().to_string()),
                        mime_type: Set("text/markdown".into()),
                        size_bytes: Set(content.len() as i64),
                        indexing_status: Set("pending".into()),
                        doc_type: Set("markdown".into()),
                        index_error: Set(None),
                        source_conversation_id: Set(None),
                        created_at: Set(now_ms),
                        updated_at: Set(now_ms),
                    };
                    if active.insert(db).await.is_ok() {
                        doc_count += 1;
                    }
                }
            }
        } else {
            tracing::info!("[lemonhu] DB 已有 {existing_docs} 篇文档，跳过 wiki_pages 导入");
        }
    }

    tracing::info!(
        "[lemonhu] 导入完成: {entity_count} 节点 + {rel_count} 关系 + {doc_count} 文档 (kb={kb_id})"
    );

    Ok(serde_json::json!({
        "knowledgeBaseId": kb_id,
        "entityCount": entity_count,
        "relationCount": rel_count,
        "documentCount": doc_count,
    }))
}
