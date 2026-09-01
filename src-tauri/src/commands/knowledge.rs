// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::index_jobs as jobs;
use axagent_entities::{
    knowledge_bases, knowledge_documents, knowledge_entities, knowledge_relations,
};
use axagent_harness::types::*;
use axagent_search::rag::KnowledgeContainer;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::collections::{HashMap, HashSet};
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
    pub entity_count: usize,   // 知识图谱实体导入数
    pub relation_count: usize, // 知识图谱关系导入数
    /// 实际使用的嵌入模型 provider（None 表示未配置，向量检索不可用）
    pub embedding_provider: Option<String>,
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识库")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识库")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "更新知识库")]
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

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateOnly, description = "删除知识库")]
#[tauri::command]
pub async fn delete_knowledge_base(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // 校验 base_id 格式，防止 SQL 注入（与 list_memory_items 一致的规则）
    if id.is_empty()
        || id.len() > 128
        || id.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(String::from(crate::commands::error::ErrorResponse::from_error(
            "Invalid base_id: must be 1-128 alphanumeric/hyphen/underscore characters",
            crate::commands::error::ErrorCategory::Unrecoverable,
        )));
    }

    // Delete vector collection (vec_kb_{id} and vec_kb_{id}_meta tables)
    let collection_id = format!("kb_{}", id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    // 若为 ConnectedVault 类型，注销全局 VaultRegistry 中的绑定
    axagent_tools::tools::obsidian::unregister_vault(&id);

    axagent_dao::repo::knowledge::delete_knowledge_base(state.harness.db(), &id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

/// 将已有 KB 转换为 ConnectedVault 类型，并绑定 Obsidian vault 路径
///
/// 用法场景：用户先创建了一个普通 KB，后来决定让它指向 Obsidian vault。
/// 转换后该 KB 不再走 RAG 索引，agent 通过 9 个 `obsidian_*` 工具直接读写。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "连接Obsidian Vault")]
#[tauri::command]
pub async fn kb_connect_vault(
    state: State<'_, AppState>,
    id: String,
    vault_path: String,
) -> Result<KnowledgeBase, String> {
    let path = std::path::Path::new(&vault_path);
    if !path.is_absolute() {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            "vault_path must be an absolute path",
        ));
    }
    if !path.is_dir() {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            format!("vault_path is not a directory: {vault_path}"),
        ));
    }

    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // 直接更新 kind/vault_path 字段（通过 update_knowledge_base 走 DAO）
    let updated = axagent_dao::repo::knowledge::set_vault_binding(
        state.harness.db(),
        &id,
        axagent_harness::KbKind::ConnectedVault,
        Some(vault_path.clone()),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 注册到全局 VaultRegistry
    if let Err(e) =
        axagent_tools::tools::obsidian::register_vault(&id, std::path::PathBuf::from(&vault_path))
    {
        tracing::warn!(kb_id = %id, error = %e, "Failed to register Obsidian vault after connect");
    }

    let _ = kb; // 保留原始 KB 引用便于未来审计
    Ok(updated)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "断开Obsidian Vault")]
/// 解除 KB 的 Obsidian vault 绑定，转换回默认 Indexed 类型
#[tauri::command]
pub async fn kb_disconnect_vault(
    state: State<'_, AppState>,
    id: String,
) -> Result<KnowledgeBase, String> {
    let updated = axagent_dao::repo::knowledge::set_vault_binding(
        state.harness.db(),
        &id,
        axagent_harness::KbKind::Indexed,
        None,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    axagent_tools::tools::obsidian::unregister_vault(&id);
    Ok(updated)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "重排序知识库")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识库文档")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "添加知识库文档")]
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
            // 入队失败时回滚状态到 "skipped"，避免文档永久卡在 pending
            let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
                state.harness.db(),
                &doc.id,
                "skipped",
                Some(&format!("enqueue failed: {e}")),
            )
            .await;
            return Err(String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            )));
        }
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
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "导入目录到知识库")]
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
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            format!("路径不存在或不是目录: {directory_path}"),
        ));
    }

    let recursive = recursive.unwrap_or(false);

    let mut files = Vec::new();
    let mut skipped = Vec::new();
    collect_importable_files(&dir, recursive, &extensions, &mut files, &mut skipped).map_err(
        |e| {
            crate::commands::error::ErrorResponse::err_with_detail(
                crate::commands::error_code::knowledge::IMPORT_DIR_FAILED,
                format!("读取目录失败 {directory_path}: {e}"),
            )
        },
    )?;

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
        entity_count: 0,
        relation_count: 0,
        embedding_provider: None,
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
                        // 入队失败时回滚状态到 "skipped"，避免文档永久卡在 pending
                        let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
                            state.harness.db(),
                            &doc.id,
                            "skipped",
                            Some(&format!("enqueue failed: {e}")),
                        )
                        .await;
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

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateOnly, description = "删除知识库文档")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "搜索知识库")]
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

    // Apply distance threshold filter consistent with collect_rag_context_from_refs.
    // score 是 L2 距离（越小越相似）。threshold > 0 时使用用户配置；
    // threshold == 0（默认）时使用与 rag.rs 一致的默认阈值 20.0，避免前端搜索与 Agent RAG 行为不一致。
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let default_max_distance = 20.0_f32; // 必须与 crates/search/src/rag.rs 中 default_max_distance 保持一致
    let threshold = kb.retrieval_threshold.unwrap_or(0.0);
    let effective_threshold = if threshold > 0.0 {
        threshold
    } else {
        default_max_distance
    };
    results.retain(|r| r.score <= effective_threshold);

    Ok(results)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "重建知识库索引")]
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

    let embedding_provider = kb.embedding_provider.ok_or_else(|| {
        crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        )
    })?;

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

        // 兜底：把本 KB 下所有仍处于 "indexing" 状态的文档标记为 "failed"，
        // 防止中途 panic / 任务取消导致状态永久卡死。
        if let Ok(stuck_docs) = axagent_dao::repo::knowledge::list_documents(&db, &base_id).await {
            for doc in &stuck_docs {
                if doc.indexing_status == "indexing" {
                    let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
                        &db,
                        &doc.id,
                        "failed",
                        Some("rebuild task terminated unexpectedly"),
                    )
                    .await;
                }
            }
        }

        let _ = app.emit("knowledge-rebuild-complete", serde_json::json!({ "baseId": base_id }));
    }));

    Ok(())
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识库容器")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识图谱实体")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识图谱实体")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识图谱属性")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识图谱属性")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识图谱关系")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识图谱关系")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识图谱流程")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识图谱流程")]
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

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识图谱接口")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "创建知识图谱接口")]
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

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateOnly, description = "清空知识库索引")]
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

    // 清空索引后把文档状态重置为 "skipped"（而非 "pending"），
    // 避免文档永久卡在 pending 但无索引任务可执行。
    // 用户如需重新索引，可调用 rebuild_knowledge_index。
    let docs = axagent_dao::repo::knowledge::list_documents(state.harness.db(), &base_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    for doc in docs {
        let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
            state.harness.db(),
            &doc.id,
            "skipped",
            Some("index cleared by user"),
        )
        .await;
    }

    Ok(())
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识文档分块")]
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

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateOnly, description = "删除知识分块")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "更新知识分块")]
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "添加知识分块")]
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

    let embedding_provider = kb.embedding_provider.ok_or_else(|| {
        crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        )
    })?;

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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "重索引知识分块")]
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

    let embedding_provider = kb.embedding_provider.ok_or_else(|| {
        crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        )
    })?;

    // Whitelist check: base_id must only contain alphanumeric chars and hyphens (for safe table name usage)
    if !base_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            format!("Invalid base_id: '{base_id}' — only ASCII alphanumeric and hyphens allowed"),
        ));
    }

    let collection_id = format!("kb_{}", base_id);

    let chunk_content = {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};
        // 2026-07-31 修复：原 SQL 用 $1（PG 风格）却标 DbBackend::Sqlite（反向标记）。
        // SQLite 模式 `$1` 占位符不合法（需 `?`）→ 该查询在 SQLite 下必炸，PG 恰好能跑。
        // 统一按 backend 分支。
        let db = state.harness.db();
        let is_pg = db.get_database_backend() == DbBackend::Postgres;
        let backend = if is_pg {
            DbBackend::Postgres
        } else {
            DbBackend::Sqlite
        };
        let name = format!("vec_kb_{}", base_id.replace('-', "_"));
        let sql = if is_pg {
            format!("SELECT content FROM {name}_meta WHERE id = $1")
        } else {
            format!("SELECT content FROM {name}_meta WHERE id = ?")
        };
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                backend,
                sql,
                vec![chunk_id.clone().into()],
            ))
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
            .ok_or_else(|| {
                crate::commands::error::ErrorResponse::err_with_detail(
                    crate::commands::error_code::knowledge::DOCUMENT_NOT_FOUND,
                    format!("Chunk {chunk_id} not found"),
                )
            })?;
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

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "重建知识文档索引")]
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

    let embedding_provider = kb.embedding_provider.ok_or_else(|| {
        crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        )
    })?;

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
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "导入lemonhu开源股票知识库")]
#[tauri::command]
pub async fn import_lemonhu_knowledge(
    state: State<'_, AppState>,
    knowledge_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();
    let dir = knowledge_dir.map(std::path::PathBuf::from);
    ensure_lemonhu_knowledge_imported(db, dir).await
}

/// 确保 lemonhu 开源股票知识库已导入（幂等：KB 已存在且数据已导入则跳过重导）。
///
/// 供 `import_lemonhu_knowledge` 命令与启动初始化（`seed_concept_index::ensure_concept_index`）
/// 共用，避免启动时重复实现目录解析/KB 创建逻辑。
pub(crate) async fn ensure_lemonhu_knowledge_imported(
    db: &sea_orm::DatabaseConnection,
    knowledge_dir: Option<std::path::PathBuf>,
) -> Result<serde_json::Value, String> {
    let kb_id = "lemonhu_knowledge_graph";

    // 确定知识库目录
    let knowledge_dir = match knowledge_dir {
        Some(d) => d,
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
            kind: Set("indexed".into()),
            vault_path: Set(None),
        }
        .insert(db)
        .await
        .map_err(|e| format!("创建 knowledge_bases 失败: {e}"))?;
    }

    let (entity_count, rel_count, doc_count) =
        import_lemonhu_graph(db, kb_id, &knowledge_dir, false).await;

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

/// 导入 lemonhu 开源知识图谱的实体、关系及文档到指定知识库。
///
/// 由 [`import_lemonhu_knowledge`] 和 [`import_project_knowledge_sources`] 共用。
/// 读取 `{lemonhu_dir}/raw/*.csv` 解析 entity/relation，读取 `{lemonhu_dir}/wiki_pages/*.md` 导入文档。
///
/// - `force_reimport_wiki_pages`：true 时即使 KB 已有文档也重新导入 wiki_pages（用于 update 模式）。
///   实体/关系始终按 id 幂等（已存在则跳过）。
async fn import_lemonhu_graph(
    db: &sea_orm::DatabaseConnection,
    kb_id: &str,
    lemonhu_dir: &std::path::Path,
    force_reimport_wiki_pages: bool,
) -> (usize, usize, usize) {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut entity_count = 0usize;
    let mut rel_count = 0usize;
    let mut doc_count = 0usize;
    let mut skipped_entities = 0usize;
    let mut skipped_relations = 0usize;

    // 实体/关系 id 加 KB 前缀，保证全局唯一，支持跨 KB 复用
    // （旧实现用全局 id，导致同名 KB 重导时实体被错误跳过且 knowledge_base_id 仍指向旧 KB）
    let prefix = format!("{}_", kb_id);

    // ── 收集 entities：优先标准格式 nodes.csv，回退到历史 raw/*.csv ──
    let raw_dir = lemonhu_dir.join("raw");
    let nodes_path = lemonhu_dir.join("nodes.csv");
    let mut entity_data: Vec<(String, String, String)> = Vec::new();

    if nodes_path.exists() {
        // 标准格式：id,title,type,tags
        if let Ok(csv) = std::fs::read_to_string(&nodes_path) {
            let mut lines = 0usize;
            for line in csv.lines().skip(1) {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                lines += 1;
                let fields: Vec<&str> = line.splitn(4, ',').collect();
                if fields.len() < 3 {
                    continue;
                }
                let id = fields[0].trim_matches('"').to_string();
                let title = fields[1].trim_matches('"').to_string();
                let etype = fields[2].trim_matches('"').to_string();
                if id.is_empty() || title.is_empty() {
                    continue;
                }
                entity_data.push((id, title, etype));
            }
            tracing::info!(
                "[graph_import] nodes.csv 读取 {lines} 行 → {} 条实体 (path={})",
                entity_data.len(),
                nodes_path.display()
            );
        } else {
            tracing::warn!("[graph_import] nodes.csv 读取失败: {}", nodes_path.display());
        }
    }
    if raw_dir.exists() {
        // 补充加载 raw/*.csv 中的行业/概念/高管实体（即使 nodes.csv 存在）
        // edges.csv 中引用的行业 hash ID 来自这些文件，不导入则关系指向"空气"
        tracing::info!("[graph_import] 从 raw/*.csv 补充实体");
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
                entity_data.push((fields[0].to_string(), fields[1].to_string(), "company".into()));
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
                entity_data.push((fields[0].to_string(), fields[1].to_string(), "concept".into()));
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
                entity_data.push((fields[0].to_string(), fields[1].to_string(), "industry".into()));
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
                entity_data.push((fields[0].to_string(), fields[1].to_string(), "person".into()));
            }
        }
        tracing::info!("[graph_import] raw/*.csv 读取完成 → {} 条实体", entity_data.len());
    } else {
        tracing::warn!(
            "[graph_import] 未找到 nodes.csv 或 raw/*.csv，实体跳过 (lemonhu_dir={})",
            lemonhu_dir.display()
        );
    }

    for (id, name, etype) in entity_data {
        let prefixed_id = format!("{}{}", prefix, id);
        let exists = knowledge_entities::Entity::find_by_id(&prefixed_id)
            .one(db)
            .await
            .map(|o| o.is_some())
            .unwrap_or(false);
        if exists {
            skipped_entities += 1;
            continue;
        }
        let active = knowledge_entities::ActiveModel {
            id: Set(prefixed_id),
            knowledge_base_id: Set(kb_id.to_string()),
            name: Set(name),
            entity_type: Set(etype),
            description: Set(None),
            source_path: Set("nodes.csv".into()),
            source_language: Set(None),
            properties: Set(serde_json::json!({})),
            lifecycle: Set(None),
            behaviors: Set(None),
            metadata: Set(None),
            aliases: Set(String::new()),
            mention_count: Set(0),
            confidence: Set(0.0),
            first_seen_at: Set(None),
            last_seen_at: Set(None),
            source_type: Set(String::from("knowledge_base")),
            source_id: Set(String::new()),
            node_type: Set(String::from("entity")),
            external_id: Set(None),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        };
        if active.insert(db).await.is_ok() {
            entity_count += 1;
        }
    }

    // ── 收集 relations：优先标准 edges.csv，回退到历史 raw/*.csv ──
    let edges_path = lemonhu_dir.join("edges.csv");
    let mut rel_data: Vec<(String, String, String, String)> = Vec::new();

    if edges_path.exists() {
        // 标准格式：source,target,type
        if let Ok(csv) = std::fs::read_to_string(&edges_path) {
            let mut lines = 0usize;
            for line in csv.lines().skip(1) {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                lines += 1;
                let fields: Vec<&str> = line.splitn(3, ',').collect();
                if fields.len() < 3 {
                    continue;
                }
                let src = fields[0].trim_matches('"').to_string();
                let tgt = fields[1].trim_matches('"').to_string();
                let rtype = fields[2].trim_matches('"').to_string();
                if src.is_empty() || tgt.is_empty() || rtype.is_empty() {
                    continue;
                }
                rel_data.push((format!("{src}_{rtype}_{tgt}"), src, tgt, rtype));
            }
            tracing::info!(
                "[graph_import] edges.csv 读取 {lines} 行 → {} 条关系 (path={})",
                rel_data.len(),
                edges_path.display()
            );
        } else {
            tracing::warn!("[graph_import] edges.csv 读取失败: {}", edges_path.display());
        }
    } else if raw_dir.exists() {
        // 历史兼容：raw/stock_concept.csv 等
        tracing::info!("[graph_import] edges.csv 不存在，回退到 raw/*.csv 路径");
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
        tracing::info!("[graph_import] raw/*.csv 关系文件读取完成 → {} 条关系", rel_data.len());
    } else {
        tracing::warn!(
            "[graph_import] 未找到 edges.csv 或 raw/*.csv，关系跳过 (lemonhu_dir={})",
            lemonhu_dir.display()
        );
    }

    for (id, src, tgt, rtype) in rel_data {
        let prefixed_id = format!("{}{}", prefix, id);
        let prefixed_src = format!("{}{}", prefix, src);
        let prefixed_tgt = format!("{}{}", prefix, tgt);
        let exists = knowledge_relations::Entity::find_by_id(&prefixed_id)
            .one(db)
            .await
            .map(|o| o.is_some())
            .unwrap_or(false);
        if exists {
            skipped_relations += 1;
            continue;
        }
        let active = knowledge_relations::ActiveModel {
            id: Set(prefixed_id),
            knowledge_base_id: Set(kb_id.to_string()),
            source_entity_id: Set(prefixed_src),
            target_entity_id: Set(prefixed_tgt),
            relation_type: Set(rtype),
            description: Set(None),
            properties: Set(None),
            metadata: Set(None),
            weight: Set(0.0),
            source_type: Set(String::from("knowledge_base")),
            source_id: Set(String::new()),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        };
        if active.insert(db).await.is_ok() {
            rel_count += 1;
        }
    }

    // ── 导入 wiki_pages ──
    let wiki_dir = lemonhu_dir.join("wiki_pages");
    if wiki_dir.exists() {
        let existing_docs = knowledge_documents::Entity::find()
            .filter(knowledge_documents::Column::KnowledgeBaseId.eq(kb_id))
            .count(db)
            .await
            .unwrap_or(0);
        // update 模式（force_reimport_wiki_pages=true）或 KB 为空时执行导入
        if existing_docs == 0 || force_reimport_wiki_pages {
            if force_reimport_wiki_pages && existing_docs > 0 {
                tracing::info!(
                    "[graph_import] update 模式：删除 KB={} 下 {} 条现有文档以重新导入 wiki_pages",
                    kb_id,
                    existing_docs
                );
                let _ = knowledge_documents::Entity::delete_many()
                    .filter(knowledge_documents::Column::KnowledgeBaseId.eq(kb_id))
                    .exec(db)
                    .await;
            }
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
                    let file_stem =
                        path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                    // 标题优先取 frontmatter title 字段，回退到首行非空行（去掉 # 前缀）
                    let title = extract_frontmatter_title(&content)
                        .or_else(|| {
                            content.lines().find(|l| !l.trim().is_empty()).map(|l| {
                                l.trim()
                                    .trim_start_matches('#')
                                    .trim()
                                    .chars()
                                    .take(80)
                                    .collect::<String>()
                            })
                        })
                        .unwrap_or_else(|| file_stem.clone());
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
            tracing::info!("[graph_import] DB 已有 {existing_docs} 篇文档，跳过 wiki_pages 导入");
        }
    }

    tracing::info!(
        "[graph_import] 导入知识图谱: {entity_count} 节点 (+{skipped_entities} 跳过) + {rel_count} 关系 (+{skipped_relations} 跳过) + {doc_count} 文档 (kb={kb_id})"
    );

    (entity_count, rel_count, doc_count)
}

/// 从 markdown frontmatter 中提取 `title:` 字段值。
/// 仅处理 YAML frontmatter（首行 `---` 开头的块），简单按行扫描避免引入 yaml 解析依赖。
fn extract_frontmatter_title(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return None;
    }
    for line in &lines[1..] {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("title:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !v.is_empty() {
                return Some(v.chars().take(80).collect());
            }
        }
    }
    None
}

/// 目录同步结果（与 [`ImportDirectoryResult`] 的区别：含 added/updated/deleted 三类计数）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDirectoryResult {
    pub base_id: String,
    pub added_count: usize,
    pub updated_count: usize,
    pub deleted_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub added: Vec<KnowledgeDocument>,
    pub updated: Vec<String>,
    pub deleted: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<ImportDirectoryError>,
}

/// 一键同步更新：对比文件系统与知识库，自动新增/更新/删除文档。
///
/// 逻辑：
/// - 收集目录下所有可导入文件，记录 mtime（文件修改时间）
/// - 获取知识库现有文档列表，以 source_path 为 key 建立索引
/// - **新增**：文件在磁盘上但不在 KB 中 → `add_document` + 入队索引
/// - **更新**：文件在 KB 中且 mtime 晚于文档时间 → 删旧文档 + 加新文档 + 入队索引
/// - **删除**：文档在 KB 中但对应文件不存在磁盘 → `delete_knowledge_document`
/// - **跳过**：文件在 KB 中且 mtime 未变 → 跳过
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "同步项目知识源目录")]
#[tauri::command]
pub async fn sync_project_knowledge_sources(
    app: AppHandle,
    state: State<'_, AppState>,
    base_id: String,
    source_path: String,
    recursive: Option<bool>,
) -> Result<SyncDirectoryResult, String> {
    let dir = PathBuf::from(&source_path);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("路径不存在或不是目录: {source_path}"));
    }

    let recursive = recursive.unwrap_or(true);
    let db = state.harness.db();

    // 1) 收集文件系统上的文件
    let mut disk_files: Vec<PathBuf> = Vec::new();
    let mut skipped = Vec::new();
    collect_importable_files(&dir, recursive, &None, &mut disk_files, &mut skipped)
        .map_err(|e| format!("读取目录失败 {source_path}: {e}"))?;

    // 2) 获取 KB 现有文档 → source_path → document 索引
    let existing_docs =
        axagent_dao::repo::knowledge::list_documents(db, &base_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // 用 source_path 做唯一 key 建立索引（标准化为小写）
    let mut doc_by_path: std::collections::HashMap<String, &KnowledgeDocument> =
        std::collections::HashMap::new();
    for doc in &existing_docs {
        doc_by_path.insert(doc.source_path.to_ascii_lowercase(), doc);
    }

    // 3) 处理 on-disk 文件：新增或更新
    let mut result = SyncDirectoryResult {
        base_id: base_id.clone(),
        added_count: 0,
        updated_count: 0,
        deleted_count: 0,
        skipped_count: 0,
        error_count: 0,
        added: Vec::new(),
        updated: Vec::new(),
        deleted: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    // 记录所有被匹配到的路径（用于后续找已删除的文件）
    let mut matched_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    let kb = axagent_dao::repo::knowledge::get_knowledge_base(db, &base_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let has_embedding = kb.embedding_provider.is_some();

    // 文档 id → 源文件 mtime（add_document 写入；旧数据为 0，退化为仅 size 比对）
    let doc_mtimes =
        axagent_dao::repo::knowledge::get_document_mtime_map(db, &base_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    for path in &disk_files {
        let abs = path.to_string_lossy().to_string();
        let key = abs.to_ascii_lowercase();
        let mime = axagent_document_parser::mime_from_extension(path).to_string();
        let title =
            path.strip_prefix(&dir).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(
                |_| path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            );

        matched_keys.insert(key.clone());

        if let Some(existing) = doc_by_path.get(&key) {
            // 增量比对：size + mtime 均未变化 → 跳过，避免每次同步对全部已存在
            // 文件删旧重加（大知识库会触发全量重索引）。
            // 旧数据 updated_at 为 0（未记录 mtime），此时退化为仅 size 比对。
            let fmeta = std::fs::metadata(path).ok();
            let fsize = fmeta.as_ref().map(|m| m.len() as i64).unwrap_or(-1);
            let fmtime = fmeta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let size_unchanged = existing.size_bytes == fsize;
            let doc_mtime = doc_mtimes.get(&existing.id).copied().unwrap_or(0);
            let mtime_unchanged = doc_mtime > 0 && fmtime > 0 && fmtime <= doc_mtime;
            if size_unchanged && mtime_unchanged {
                result.skipped_count += 1;
                result.skipped.push(abs);
                continue;
            }

            // 文件已存在且发生变化 → 删旧 + 加新（保证磁盘内容与 KB 一致）
            let doc_id = existing.id.clone();
            let collection_id = format!("kb_{}", base_id);
            let _ = state.vector_store.delete_document_embeddings(&collection_id, &doc_id).await;
            if let Err(e) = axagent_dao::repo::knowledge::delete_document(db, &doc_id).await {
                result.error_count += 1;
                result.errors.push(ImportDirectoryError {
                    path: abs,
                    error: format!("删除旧文档失败: {e}"),
                });
                continue;
            }
            match axagent_dao::repo::knowledge::add_document(
                db, &base_id, &title, &abs, &mime, None,
            )
            .await
            {
                Ok(new_doc) => {
                    if has_embedding {
                        let _ = axagent_dao::repo::knowledge::update_document_status(
                            db,
                            &new_doc.id,
                            "pending",
                        )
                        .await;
                        if let Err(e) = crate::index_queue::enqueue_job_sync(
                            &state,
                            &app,
                            jobs::JOB_TYPE_INDEX_DOCUMENT,
                            "kb",
                            &base_id,
                            &new_doc.id,
                            None,
                            None,
                        ) {
                            tracing::warn!("[sync_project] 入队索引失败 {}: {}", new_doc.id, e);
                        }
                    }
                    result.updated_count += 1;
                    result.updated.push(abs);
                },
                Err(e) => {
                    result.error_count += 1;
                    result.errors.push(ImportDirectoryError {
                        path: abs,
                        error: format!("重加文档失败: {e}"),
                    });
                },
            }
        } else {
            // 文件不存在于 KB → 新增
            match axagent_dao::repo::knowledge::add_document(
                db, &base_id, &title, &abs, &mime, None,
            )
            .await
            {
                Ok(new_doc) => {
                    if has_embedding {
                        let _ = axagent_dao::repo::knowledge::update_document_status(
                            db,
                            &new_doc.id,
                            "pending",
                        )
                        .await;
                        if let Err(e) = crate::index_queue::enqueue_job_sync(
                            &state,
                            &app,
                            jobs::JOB_TYPE_INDEX_DOCUMENT,
                            "kb",
                            &base_id,
                            &new_doc.id,
                            None,
                            None,
                        ) {
                            tracing::warn!("[sync_project] 入队索引失败 {}: {}", new_doc.id, e);
                        }
                    }
                    result.added_count += 1;
                    result.added.push(new_doc);
                },
                Err(e) => {
                    result.error_count += 1;
                    result.errors.push(ImportDirectoryError {
                        path: abs,
                        error: format!("添加文档失败: {e}"),
                    });
                },
            }
        }
    }

    // 4) 处理 KB 中存在但磁盘上已删除的文档
    for (key, doc) in &doc_by_path {
        if !matched_keys.contains(key) {
            let collection_id = format!("kb_{}", base_id);
            let _ = state.vector_store.delete_document_embeddings(&collection_id, &doc.id).await;
            if let Err(e) = axagent_dao::repo::knowledge::delete_document(db, &doc.id).await {
                result.error_count += 1;
                result.errors.push(ImportDirectoryError {
                    path: doc.source_path.clone(),
                    error: format!("删除已移除文档失败: {e}"),
                });
            } else {
                result.deleted_count += 1;
                result.deleted.push(doc.source_path.clone());
            }
        }
    }

    result.skipped_count += skipped.len();
    result.skipped.extend(skipped);

    tracing::info!(
        "[sync_project] 同步完成: +{} 新增, ~{} 更新, -{} 删除, ={} 跳过, !{} 错误 (kb={})",
        result.added_count,
        result.updated_count,
        result.deleted_count,
        result.skipped_count,
        result.error_count,
        base_id,
    );

    Ok(result)
}

/// 修复 Wiki 中所有笔记的 wikilink 关联。
///
/// 遍历 Wiki 下所有笔记，重新解析内容中的 `[[wikilink]]` 并同步到
/// `note_links` / `note_backlinks` 表。用于修复历史导入过程中可能
/// 遗漏的双向链接记录，确保图谱节点正确关联。
///
/// 实现委托 `resync_vault_note_links`（一次性构建全 vault 映射 + 批量写入）。
/// 旧实现逐篇调用 `sync_note_links_from_content`（每篇全量加载 vault，O(N²)），
/// 2 万篇笔记场景下不可用；且逐篇同步无法修复批量导入时被丢弃的前向引用。
///
/// 返回值：处理的笔记数量。
async fn repair_wiki_note_links(db: &sea_orm::DatabaseConnection, wiki_id: &str) -> usize {
    match axagent_dao::repo::note::resync_vault_note_links(db, wiki_id).await {
        Ok((notes, links)) => {
            tracing::info!(
                "[repair_links] Wiki {} 链接重建完成: {} 篇笔记, {} 条链接",
                wiki_id,
                notes,
                links
            );
            notes
        },
        Err(e) => {
            tracing::warn!("[repair_links] Wiki {} 链接重建失败: {e}", wiki_id);
            0
        },
    }
}

/// 将知识图谱的实体/关系桥接到 Wiki vault 中：为每个实体创建一篇笔记，
/// 在内容中嵌入 `[[关联实体]]` wikilinks，使 Wiki 图谱视图展示关联关系。
async fn bridge_graph_to_wiki(
    db: &sea_orm::DatabaseConnection,
    kb_id: &str,
    wiki_id: &str,
) -> (usize, usize) {
    // 1) 读取图谱侧全部实体 + 关系
    let entities = match knowledge_entities::Entity::find()
        .filter(knowledge_entities::Column::KnowledgeBaseId.eq(kb_id))
        .all(db)
        .await
    {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("[graph_to_wiki] 读取 entity 失败: {e}");
            return (0, 0);
        },
    };
    let relations = match knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::KnowledgeBaseId.eq(kb_id))
        .all(db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[graph_to_wiki] 读取 relation 失败: {e}");
            return (0, 0);
        },
    };

    // 2) 建立 entity_id → name 索引
    let name_by_id: HashMap<String, String> =
        entities.iter().map(|e| (e.id.clone(), e.name.clone())).collect();

    // 3) 建立 entity_id → [(target_id, relation_type)]
    let mut rel_map: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for r in &relations {
        rel_map
            .entry(r.source_entity_id.clone())
            .or_default()
            .push((r.target_entity_id.clone(), r.relation_type.clone()));
        rel_map
            .entry(r.target_entity_id.clone())
            .or_default()
            .push((r.source_entity_id.clone(), format!("inverse_{}", r.relation_type)));
    }

    // 4) 读取 Wiki 已有笔记标题，跳过已存在的
    let existing = axagent_dao::repo::note::list_notes(db, wiki_id).await.unwrap_or_default();
    let existing_titles: HashSet<String> = existing.iter().map(|n| n.title.clone()).collect();

    let mut created = 0usize;
    let mut skipped = 0usize;

    for entity in &entities {
        if existing_titles.contains(&entity.name) {
            skipped += 1;
            continue;
        }

        // 构建笔记内容：关联节点用 [[wikilinks]] 嵌入
        let mut content = format!(
            "# {}\n\n> ℹ️ 从知识图谱自动导入的实体节点\n\n**类型**: {} \n\n",
            entity.name, entity.entity_type
        );
        if let Some(related) = rel_map.get(&entity.id) {
            let mut links: Vec<String> = Vec::new();
            for (tid, rtype) in related {
                if let Some(tname) = name_by_id.get(tid) {
                    if tname != &entity.name {
                        links.push(format!("- [[{}]]  — *{}*", tname, rtype));
                    }
                }
            }
            if !links.is_empty() {
                content.push_str("## 关联节点\n\n");
                content.push_str(&links.join("\n"));
            }
        }

        let input = axagent_harness::note_dtos::CreateNoteInput {
            vault_id: wiki_id.to_string(),
            title: entity.name.clone(),
            file_path: format!("graph-entities/{}.md", entity.name),
            content,
            author: "graph-import".to_string(),
            page_type: None,
            source_refs: None,
        };

        let created_note = axagent_dao::repo::note::create_note(db, input).await;
        match created_note {
            Ok(_) => {
                // 链接同步不再逐篇执行：create_note 内部的同步只含「已导入」笔记映射，
                // 前向引用会被丢弃；且此处逐篇调用是 O(N²)。统一由导入流程末尾的
                // resync_vault_note_links（全量映射 + 批量写入）完成。
                created += 1;
            },
            Err(e) => {
                tracing::warn!("[graph_to_wiki] 创建笔记失败 {}: {e}", entity.name);
                skipped += 1;
            },
        }
    }

    tracing::info!(
        "[graph_to_wiki] 桥接完成: 创建 {} 篇, 跳过 {} 篇 (kb={}, wiki={})",
        created,
        skipped,
        kb_id,
        wiki_id,
    );

    (created, skipped)
}

// ── 项目知识源一键导入 ───────────────────────────────────

/// 项目知识源导入结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectKnowledgeImportResult {
    /// Wiki 知识库 ID（存放所有 markdown 文件）
    pub wiki_id: String,
    pub wiki_name: String,
    pub wiki_imported: usize,
    pub wiki_failed: usize,
    pub wiki_skipped: usize,
    /// RAG 知识库 ID（存放 lemonhu 知识图谱实体 + 关系）
    pub kb_id: String,
    pub kb_name: String,
    pub entity_count: usize,
    pub relation_count: usize,
    /// 本次图谱→Wiki 桥接创建的笔记数（含 [[wikilinks]]）
    pub bridged_notes: usize,
    pub bridged_skipped: usize,
    pub embedding_provider: Option<String>,
    /// 本次操作是否变更了 embedding_provider（前端据此提示用户重建索引）
    pub embedding_changed: bool,
}

/// 一键导入项目知识源：创建 Wiki 知识库 + 导入知识图谱。
///
/// 参数：
/// - `source_path`：要导入的目录绝对路径（如 `/path/to/knowledge-sources`）
/// - `source_name`：知识源名称（默认 `项目知识源`）。
///   - Wiki vault 名 = `source_name`
///   - RAG KB 名 = `{source_name}图谱`
/// - `mode`：模式（默认 `create`）
///   - `create`：清理同名无 embedding 残次 KB → 创建/复用 Wiki + KB → 全量导入
///   - `update`：找到/创建同名 Wiki + KB → 软删除 Wiki 现有 notes → 重新导入笔记和 wiki_pages
///     （图谱实体/关系按 id 幂等，已存在则跳过）
/// - `embedding_provider`：可选向量模型，格式 `providerId::modelId`。
///   - 创建模式：新 Wiki/KB 直接写入该字段；复用已有 Wiki/KB 时若与现有不同则更新。
///   - 更新模式：传入时与现有不同则更新；不传则保持现状。
///   - 返回 `embedding_changed=true` 时前端应提示用户重建索引。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "导入项目知识源")]
#[tauri::command]
pub async fn import_project_knowledge_sources(
    app: AppHandle,
    state: State<'_, AppState>,
    source_path: String,
    source_name: Option<String>,
    mode: Option<String>,
    embedding_provider: Option<String>,
) -> Result<ProjectKnowledgeImportResult, String> {
    let dir = PathBuf::from(&source_path);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("路径不存在或不是目录: {source_path}"));
    }

    let source_name = source_name.unwrap_or_else(|| "项目知识源".to_string());
    let mode = mode.unwrap_or_else(|| "create".to_string());
    let is_update = match mode.as_str() {
        "create" => false,
        "update" => true,
        other => return Err(format!("不支持的模式: {other}（仅支持 create / update）")),
    };

    // 校验 embedding_provider 格式：必须为 `providerId::modelId` 或 None
    let embedding_provider = match embedding_provider.as_deref() {
        None => None,
        Some("") => None,
        Some(ep) => {
            let parts: Vec<&str> = ep.splitn(2, "::").collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return Err(format!(
                    "embedding_provider 格式非法：'{ep}'（应为 providerId::modelId）"
                ));
            }
            Some(ep.to_string())
        },
    };

    let wiki_name = source_name.clone();
    let kb_name = format!("{source_name}图谱");

    // ── 所有 DB 操作一次性完成（释放 state 的借用）──
    let (
        wiki_id,
        wiki_name,
        kb_id,
        kb_name,
        (entity_count, relation_count, _doc_count),
        final_embedding_provider,
        embedding_changed,
    ) = {
        let db = state.harness.db();

        // 1) create 模式：清理同名无 embedding 的残次 KB（update 模式不动）
        if !is_update {
            let existing_bases =
                axagent_dao::repo::knowledge::list_knowledge_bases(db).await.map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
            for kb in &existing_bases {
                if kb.name == kb_name && kb.embedding_provider.is_none() {
                    tracing::info!("[import_project] 清理旧的残次 KB: {} ({})", kb.name, kb.id);
                    let collection_id = format!("kb_{}", kb.id);
                    let _ = state.vector_store.delete_collection(&collection_id).await;
                    let _ = axagent_dao::repo::knowledge::delete_knowledge_base(db, &kb.id).await;
                }
            }
        }

        // 2) 创建/复用 Wiki vault
        //    - 新建：直接写入 embedding_provider
        //    - 复用：若传入的 embedding_provider 与现有不同，调用 update_wiki 同步字段
        let (wiki_id, wiki_embedding_changed) = {
            let existing_wikis = axagent_dao::repo::wiki::list_wikis(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            if let Some(w) = existing_wikis.into_iter().find(|w| w.name == wiki_name) {
                tracing::info!("[import_project] 复用已有 Wiki: {} ({})", wiki_name, w.id);
                let changed = match &embedding_provider {
                    Some(ep) if w.embedding_provider.as_deref() != Some(ep.as_str()) => {
                        tracing::info!(
                            "[import_project] Wiki {} embedding_provider 变更：{:?} => {:?}",
                            w.id,
                            w.embedding_provider,
                            embedding_provider
                        );
                        let _ = axagent_dao::repo::wiki::update_wiki(
                            db,
                            &w.id,
                            None,
                            None,
                            embedding_provider.clone(),
                            None,
                        )
                        .await
                        .map_err(|e| {
                            String::from(crate::commands::error::ErrorResponse::from_error(
                                e,
                                crate::commands::error::ErrorCategory::Unrecoverable,
                            ))
                        })?;
                        true
                    },
                    _ => false,
                };
                (w.id, changed)
            } else {
                tracing::info!("[import_project] 创建新 Wiki: {}", wiki_name);
                let wiki = axagent_dao::repo::wiki::create_wiki(
                    db,
                    axagent_dao::repo::wiki::CreateWikiInput {
                        name: wiki_name.clone(),
                        description: Some(format!("从 {} 自动导入的项目知识源", source_path)),
                        root_path: source_path.clone(),
                        embedding_provider: embedding_provider.clone(),
                        knowledge_base_id: None,
                    },
                )
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                // 新建时若指定了 embedding_provider，视为「变更」（前端提示需要建索引）
                (wiki.id, embedding_provider.is_some())
            }
        };

        // 2.5) update 模式：软删除 Wiki 下现有 notes，让 wiki_import_obsidian_vault 重新导入磁盘最新内容
        if is_update {
            let existing_notes =
                axagent_dao::repo::note::list_notes(db, &wiki_id).await.map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
            let note_count = existing_notes.len();
            for note in &existing_notes {
                let _ = axagent_dao::repo::note::delete_note(db, &note.id).await;
            }
            if note_count > 0 {
                tracing::info!(
                    "[import_project] update 模式：软删除 Wiki {} 下 {} 条现有 notes",
                    wiki_id,
                    note_count
                );
            }
        }

        // 3) 创建/复用 KB + 同步 embedding_provider
        let (kb_id, kb_embedding_provider, kb_embedding_changed) = {
            let existing_bases =
                axagent_dao::repo::knowledge::list_knowledge_bases(db).await.map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
            if let Some(kb) = existing_bases.into_iter().find(|b| b.name == kb_name) {
                let changed = match &embedding_provider {
                    Some(ep) if kb.embedding_provider.as_deref() != Some(ep.as_str()) => {
                        tracing::info!(
                            "[import_project] KB {} embedding_provider 变更：{:?} => {:?}",
                            kb.id,
                            kb.embedding_provider,
                            embedding_provider
                        );
                        let _ = axagent_dao::repo::knowledge::update_knowledge_base(
                            db,
                            &kb.id,
                            axagent_harness::types::UpdateKnowledgeBaseInput {
                                name: None,
                                description: None,
                                embedding_provider: embedding_provider.clone(),
                                enabled: None,
                                icon_type: None,
                                icon_value: None,
                                update_icon: false,
                                embedding_dimensions: None,
                                update_embedding_dimensions: false,
                                retrieval_threshold: None,
                                update_retrieval_threshold: false,
                                retrieval_top_k: None,
                                update_retrieval_top_k: false,
                                chunk_size: None,
                                update_chunk_size: false,
                                chunk_overlap: None,
                                update_chunk_overlap: false,
                                separator: None,
                                update_separator: false,
                            },
                        )
                        .await
                        .map_err(|e| {
                            String::from(crate::commands::error::ErrorResponse::from_error(
                                e,
                                crate::commands::error::ErrorCategory::Unrecoverable,
                            ))
                        })?;
                        true
                    },
                    _ => false,
                };
                let updated = if changed {
                    axagent_dao::repo::knowledge::get_knowledge_base(db, &kb.id).await.map_err(
                        |e| {
                            String::from(crate::commands::error::ErrorResponse::from_error(
                                e,
                                crate::commands::error::ErrorCategory::Unrecoverable,
                            ))
                        },
                    )?
                } else {
                    kb
                };
                (updated.id, updated.embedding_provider, changed)
            } else {
                let new_kb = axagent_dao::repo::knowledge::create_knowledge_base(
                    db,
                    axagent_harness::types::CreateKnowledgeBaseInput {
                        name: kb_name.clone(),
                        description: Some("lemonhu A 股知识图谱（实体 + 关系）".into()),
                        embedding_provider: embedding_provider.clone(),
                        enabled: Some(true),
                        kind: Default::default(),
                        vault_path: None,
                    },
                )
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                (new_kb.id, new_kb.embedding_provider, embedding_provider.is_some())
            }
        };

        // 3.5) 将 KB ID 关联到 Wiki，建立 Wiki 与 KB 的 1:1 关联
        // 这是修复 Wiki 图谱关联断裂的关键步骤
        if let Err(e) = axagent_dao::repo::wiki::update_wiki(
            db,
            &wiki_id,
            None,
            None,
            None,
            Some(Some(kb_id.clone())),
        )
        .await
        {
            tracing::warn!("[import_project] 关联 Wiki {} 与 KB {} 失败: {}", wiki_id, kb_id, e);
        } else {
            tracing::info!("[import_project] 成功关联 Wiki {} 与 KB {}", wiki_id, kb_id);
        }

        // 4) 导入 lemonhu 图谱（update 模式下强制重新导入 wiki_pages；实体/关系按 id 幂等）
        let lemonhu_dir = dir.join("lemonhu");
        let graph_result = if lemonhu_dir.exists() {
            import_lemonhu_graph(db, &kb_id, &lemonhu_dir, is_update).await
        } else {
            (0, 0, 0)
        };

        // Wiki 与 KB 任一发生变更，则整体视为 embedding_changed
        let embedding_changed = wiki_embedding_changed || kb_embedding_changed;
        (wiki_id, wiki_name, kb_id, kb_name, graph_result, kb_embedding_provider, embedding_changed)
    }; // ← db 引用在此释放，state 恢复可移动状态

    // 5) 桥接图谱→Wiki：为图谱中的实体创建 Wiki 笔记，内含 [[wikilinks]] 关联
    let (bridged_notes, bridged_skipped) =
        bridge_graph_to_wiki(state.harness.db(), &kb_id, &wiki_id).await;

    // 5.5) 链接修复已移至 Wiki 导入（步骤 6）之后执行：
    // wiki_import_obsidian_vault 才是笔记量最大的导入源，在其之前修复只会
    // 处理到部分笔记，且此时构建的映射不完整（前向引用仍会丢失）。

    // 5.6) 失效 Wiki 图谱缓存，确保下次加载时获取最新的图谱数据
    let _ =
        axagent_dao::repo::wiki_graph_cache::invalidate_cache(state.harness.db(), &wiki_id).await;

    // 6) 入队 KB 文档索引任务
    // import_lemonhu_graph 创建文档时仅写入 DB（indexing_status="pending"），未入队 index_queue。
    // 此处统一补齐：查询 KB 下所有 pending 文档，循环入队 JOB_TYPE_INDEX_DOCUMENT，
    // 否则 RAG 检索永远查不到这些文档（vector_store 中无对应 embedding）。
    let kb_has_embedding = final_embedding_provider.is_some();
    if kb_has_embedding {
        let pending_docs: Vec<String> = {
            let db = state.harness.db();
            knowledge_documents::Entity::find()
                .filter(knowledge_documents::Column::KnowledgeBaseId.eq(&kb_id))
                .filter(knowledge_documents::Column::IndexingStatus.eq("pending"))
                .all(db)
                .await
                .map(|docs| docs.into_iter().map(|d| d.id).collect())
                .unwrap_or_default()
        };
        if !pending_docs.is_empty() {
            let count = pending_docs.len();
            for doc_id in &pending_docs {
                let _ = crate::index_queue::enqueue_job_sync(
                    &state,
                    &app,
                    jobs::JOB_TYPE_INDEX_DOCUMENT,
                    "kb",
                    &kb_id,
                    doc_id,
                    None,
                    None,
                );
            }
            tracing::info!("[import_project] 入队 {count} 个 KB 文档索引任务 (kb={kb_id})");
        }
    } else {
        tracing::info!(
            "[import_project] KB 未配置 embedding_provider，跳过文档索引入队 (kb={kb_id})"
        );
    }

    // ── Wiki markdown 导入（state 被移入但不需再使用）──
    // 先克隆 db 句柄（DatabaseConnection 是 Arc 包装，克隆廉价），
    // state 移入导入函数后仍需用它做链接重建与缓存失效。
    let db_after_import = state.harness.db().clone();
    let wiki_result = crate::commands::wiki::wiki_import_obsidian_vault(
        app.clone(),
        state,
        wiki_id.clone(),
        source_path.clone(),
    )
    .await
    .map_err(|e| format!("导入 Wiki 笔记失败: {e}"))?;

    // 7) 全量重建链接表：必须在所有笔记导入完成之后执行。
    // 逐篇导入时的链接同步只含「已导入」笔记的映射，指向后续导入笔记的
    // 前向引用被静默丢弃 —— 这是图谱 0 边、无聚类的根因。
    let repaired_links = repair_wiki_note_links(&db_after_import, &wiki_id).await;
    if repaired_links > 0 {
        tracing::info!(
            "[import_project] 重建 Wiki {} 链接表完成（{} 篇笔记）",
            wiki_id,
            repaired_links
        );
    }

    // 8) 失效图谱缓存（链接表已全量变化）
    let _ = axagent_dao::repo::wiki_graph_cache::invalidate_cache(&db_after_import, &wiki_id).await;

    let result = ProjectKnowledgeImportResult {
        wiki_id,
        wiki_name,
        wiki_imported: wiki_result.imported,
        wiki_failed: wiki_result.failed,
        wiki_skipped: wiki_result.skipped,
        kb_id,
        kb_name,
        entity_count,
        relation_count,
        bridged_notes,
        bridged_skipped,
        embedding_provider: final_embedding_provider,
        embedding_changed,
    };

    tracing::info!(
        "[import_project] {} 完成: Wiki +{}/-{}/={} notes, Graph {} 实体 + {} 关系, Bridge {} notes",
        if is_update { "更新" } else { "导入" },
        result.wiki_imported,
        result.wiki_failed,
        result.wiki_skipped,
        result.entity_count,
        result.relation_count,
        result.bridged_notes,
    );

    Ok(result)
}
