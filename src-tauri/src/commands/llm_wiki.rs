// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_agent::{
    ingest_pipeline, ingest_queue, lint_checker, purpose_manager, query_engine, schema_manager,
    wiki_compiler,
};
use axagent_agent_macro::agent_command;
use axagent_dao::repo::note_backlink_repository::DaoNoteBacklinkRepository;
use axagent_dao::repo::note_repository::DaoNoteRepository;
use axagent_dao::repo::wiki;
use axagent_dao::repo::wiki_repository::DaoWikiRepository;
use axagent_dao::repo::wiki_source_repository::DaoWikiSourceRepository;
use axagent_entities::wiki_sync_queue;
use axagent_harness::kit_bridge::KitMarkdownParser;
use axagent_harness::repositories;
use axagent_harness::types::ProviderType;
use axagent_harness::wiki_dtos::{
    NoteBacklinkRepository, NoteRepository, WikiRepository, WikiSourceRepository,
};
use axagent_harness::{ProviderAdapter, ProviderRequestContext, resolve_base_url_for_type};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct WikiOutput {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub schema_version: String,
    pub description: Option<String>,
    pub note_count: i32,
    pub source_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<axagent_entities::wikis::Model> for WikiOutput {
    fn from(m: axagent_entities::wikis::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            root_path: m.root_path,
            schema_version: m.schema_version,
            description: m.description,
            note_count: m.note_count,
            source_count: m.source_count,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WikiOperationOutput {
    pub id: i64,
    pub wiki_id: String,
    pub operation_type: String,
    pub target_type: String,
    pub target_id: String,
    pub status: String,
    pub details_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

impl From<axagent_entities::wiki_operations::Model> for WikiOperationOutput {
    fn from(m: axagent_entities::wiki_operations::Model) -> Self {
        Self {
            id: m.id,
            wiki_id: m.wiki_id,
            operation_type: m.operation_type,
            target_type: m.target_type,
            target_id: m.target_id,
            status: m.status,
            details_json: m.details_json,
            error_message: m.error_message,
            created_at: m.created_at,
            completed_at: m.completed_at,
        }
    }
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateOnly, description = "列出所有知识库")]
#[tauri::command]
pub async fn llm_wiki_list(state: State<'_, AppState>) -> Result<Vec<WikiOutput>, String> {
    let wikis = axagent_entities::wikis::Entity::find()
        .order_by(axagent_entities::wikis::Column::CreatedAt, sea_orm::Order::Desc)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(wikis.into_iter().map(WikiOutput::from).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWikiInput {
    pub name: String,
    pub root_path: String,
    pub description: Option<String>,
    pub embedding_provider: Option<String>,
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "创建新 Wiki")]
#[tauri::command]
pub async fn llm_wiki_create(
    state: State<'_, AppState>,
    input: CreateWikiInput,
) -> Result<WikiOutput, String> {
    let wiki_input = wiki::CreateWikiInput {
        name: input.name,
        description: input.description,
        root_path: input.root_path,
        embedding_provider: input.embedding_provider,
        knowledge_base_id: None,
    };

    let model = wiki::create_wiki(state.harness.db(), wiki_input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(WikiOutput {
        id: model.id,
        name: model.name,
        root_path: model.root_path,
        schema_version: model.schema_version,
        description: model.description,
        note_count: model.note_count,
        source_count: model.source_count,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

#[agent_command(domain = wiki, safety = Dangerous, call_mode = StateInput, description = "删除 Wiki")]
#[tauri::command]
pub async fn llm_wiki_delete(state: State<'_, AppState>, wiki_id: String) -> Result<(), String> {
    let collection_id = format!("wiki_{}", wiki_id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    wiki::delete_wiki(state.harness.db(), &wiki_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 删除 wiki source 记录，并清理 notes.source_refs 中的失效引用。
///
/// 设计说明：
/// - source 删除后，notes.source_refs 中残留的 source_id 会导致引用失效，
///   因此需要扫描该 wiki 下所有 notes，从 source_refs 中移除该 source_id。
/// - note 本身保留（内容仍可查看），向量也保留（仍可检索）。
/// - 若 note 的 source_refs 仅包含该 source_id，则 note 仍保留（避免误删用户内容）。
/// - wiki_pages.source_ids 是编译产物，下次重新编译会自动更新，此处不清理。
#[agent_command(domain = wiki, safety = Dangerous, call_mode = StateInput, description = "删除 Wiki 数据源")]
#[tauri::command]
pub async fn llm_wiki_delete_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    let db = state.harness.db();

    // 1. 查询 source 获取 wiki_id（用于扫描 notes）
    let source = axagent_entities::wiki_sources::Entity::find_by_id(&source_id)
        .one(db)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                axagent_harness::core_error::AxAgentError::NotFound(format!(
                    "WikiSource {}",
                    source_id
                )),
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let wiki_id = source.wiki_id.clone();

    // 2. 删除 source 表记录
    let deleted = axagent_entities::wiki_sources::Entity::delete_by_id(&source_id)
        .exec(db)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    if deleted.rows_affected == 0 {
        return Ok(());
    }

    // 3. 扫描该 wiki 下所有 notes，从 source_refs 中移除该 source_id
    let notes = axagent_dao::repo::note::list_notes(db, &wiki_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut cleaned_count = 0usize;
    for note in notes {
        if let Some(ref refs) = note.source_refs {
            if refs.contains(&source_id) {
                let new_refs: Vec<String> =
                    refs.iter().filter(|r| *r != &source_id).cloned().collect();
                let new_refs_json: serde_json::Value = if new_refs.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::to_value(&new_refs).unwrap_or(serde_json::Value::Null)
                };

                // 局部更新：仅修改 source_refs 和 updated_at，避免全字段回写
                axagent_entities::notes::Entity::update_many()
                    .col_expr(
                        axagent_entities::notes::Column::SourceRefs,
                        sea_orm::sea_query::Expr::value(new_refs_json),
                    )
                    .col_expr(
                        axagent_entities::notes::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::value(chrono::Utc::now().timestamp()),
                    )
                    .filter(axagent_entities::notes::Column::Id.eq(&note.id))
                    .exec(db)
                    .await
                    .map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })?;
                cleaned_count += 1;
            }
        }
    }

    if cleaned_count > 0 {
        tracing::info!(
            "Deleted wiki source {} and cleaned {} notes' source_refs",
            source_id,
            cleaned_count
        );
    } else {
        tracing::info!("Deleted wiki source {} (no notes referenced it)", source_id);
    }

    Ok(())
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "获取 Wiki 操作历史列表")]
#[tauri::command]
pub async fn llm_wiki_operations_list(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<Vec<WikiOperationOutput>, String> {
    let operations = axagent_entities::wiki_operations::Entity::find()
        .filter(axagent_entities::wiki_operations::Column::WikiId.eq(&wiki_id))
        .order_by(axagent_entities::wiki_operations::Column::CreatedAt, sea_orm::Order::Desc)
        .limit(100)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(operations.into_iter().map(WikiOperationOutput::from).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestSourceInput {
    pub wiki_id: String,
    pub source_type: String,
    pub path: String,
    pub url: Option<String>,
    pub title: Option<String>,
    /// 内联文本内容，提供后将直接作为源内容使用，不从文件系统读取
    pub content: Option<String>,
}

// 输出结构遵循全站 camelCase 标准（AGENTS.md 规范 #13）。此前注释声称前端
// 依赖 snake_case 字段名与事实不符：IngestPanel 消费 item.rawPath /
// result.sourceId（camelCase），真机下曾因缺 rename_all 得到 undefined。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResultOutput {
    pub source_id: String,
    pub raw_path: String,
    pub title: String,
    /// 后台批量索引的笔记总数，供前端结合 wiki-note-indexed 事件计算真实进度
    pub generated_note_count: usize,
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "摄取 Wiki 文件内容")]
#[tauri::command]
pub async fn llm_wiki_ingest(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: IngestSourceInput,
) -> Result<IngestResultOutput, String> {
    let db = Arc::new(state.harness.db().clone());
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(db.clone()));
    let wiki_source_repo: Arc<dyn WikiSourceRepository> =
        Arc::new(DaoWikiSourceRepository::new(db.clone()));
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(db));
    let pipeline = ingest_pipeline::IngestPipeline::new(wiki_repo, wiki_source_repo, note_repo);

    let source = ingest_pipeline::IngestSource {
        source_type: match input.source_type.as_str() {
            "web" => ingest_pipeline::IngestSourceType::WebArticle,
            "paper" => ingest_pipeline::IngestSourceType::Paper,
            "book" => ingest_pipeline::IngestSourceType::Book,
            "pdf" => ingest_pipeline::IngestSourceType::Pdf,
            "docx" => ingest_pipeline::IngestSourceType::Docx,
            "xlsx" => ingest_pipeline::IngestSourceType::Xlsx,
            "pptx" => ingest_pipeline::IngestSourceType::Pptx,
            _ => ingest_pipeline::IngestSourceType::RawMarkdown,
        },
        path: input.path,
        url: input.url,
        title: input.title,
        folder_context: None,
        content: input.content,
    };

    let result = pipeline.ingest(&input.wiki_id, source).await?;

    // 后台批量索引生成的笔记（R2：所有 LLM 生成页统一入 RAG）
    crate::indexing::spawn_wiki_note_batch_indexing(crate::indexing::WikiBatchIndexingTask {
        app,
        db: state.harness.db().clone(),
        master_key: state.harness.master_key_owned(),
        vector_store: state.vector_store.clone(),
        wiki_id: input.wiki_id.clone(),
        note_ids: result.generated_note_ids.clone(),
        log_label: "llm_wiki.ingest",
        completion_event: None,
    });

    // 操作历史：ingest 成功落一条审计记录（失败仅告警，不打断主流程）
    if let Err(e) = wiki::log_wiki_operation(
        state.harness.db(),
        wiki::WikiOperationEntry {
            wiki_id: input.wiki_id.clone(),
            operation_type: "ingest".to_string(),
            target_type: "source".to_string(),
            target_id: result.source_id.clone(),
            status: "completed".to_string(),
            details: Some(serde_json::json!({
                "generatedNotes": result.generated_note_ids.len()
            })),
            error_message: None,
        },
    )
    .await
    {
        tracing::warn!("[llm_wiki] 记录 ingest 操作历史失败: {e}");
    }

    Ok(IngestResultOutput {
        source_id: result.source_id,
        raw_path: result.raw_path,
        title: result.title,
        generated_note_count: result.generated_note_ids.len(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileInput {
    pub wiki_id: String,
    pub source_ids: Vec<String>,
}

// 输出结构刻意保持 snake_case（无 rename_all），前端类型依赖此契约，勿改。
#[derive(Debug, Serialize)]
pub struct CompileResultOutput {
    pub new_pages: Vec<CompiledPageOutput>,
    pub updated_pages: Vec<CompiledPageOutput>,
    pub errors: Vec<String>,
}

// 输出结构刻意保持 snake_case（无 rename_all），前端类型依赖此契约，勿改。
#[derive(Debug, Serialize)]
pub struct CompiledPageOutput {
    pub title: String,
    pub content: String,
    pub page_type: String,
    pub source_ids: Vec<String>,
}

fn resolve_provider_adapter(
    provider_type: &ProviderType,
) -> Result<Arc<dyn ProviderAdapter>, String> {
    match provider_type {
        ProviderType::OpenAI => Ok(Arc::new(axagent_providers::openai::OpenAIAdapter::new())),
        ProviderType::OpenAIResponses => {
            Ok(Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new()))
        },
        ProviderType::Anthropic => {
            Ok(Arc::new(axagent_providers::anthropic::AnthropicAdapter::new()))
        },
        ProviderType::Gemini => Ok(Arc::new(axagent_providers::gemini::GeminiAdapter::new())),
        ProviderType::OpenClaw => Ok(Arc::new(axagent_providers::openclaw::OpenClawAdapter::new())),
        ProviderType::Hermes => Ok(Arc::new(axagent_providers::hermes::HermesAdapter::new())),
        ProviderType::Ollama => Ok(Arc::new(axagent_providers::ollama::OllamaAdapter::new())),
        ProviderType::LlamaCpp => {
            Ok(Arc::new(axagent_providers::llama_cpp::LlamaCppAdapter::new()))
        },
    }
}

fn parse_embedding_provider(ep: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = ep.splitn(2, "::").collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            format!("Invalid embedding_provider format '{}'. Expected 'providerId::modelId'", ep),
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

async fn build_llm_adapter(
    db: &sea_orm::DatabaseConnection,
    master_key: &[u8; 32],
    embedding_provider: &str,
) -> Result<(Arc<dyn ProviderAdapter>, ProviderRequestContext, String), String> {
    // 兼容历史脏数据（纯 provider_id 格式），先解析再补全
    let (resolved, _was_legacy) =
        crate::indexing::resolve_embedding_provider(db, embedding_provider).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let (provider_id, model_id) = parse_embedding_provider(&resolved)?;

    let provider =
        axagent_dao::repo::provider::get_provider(db, &provider_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let key = axagent_dao::repo::provider::get_active_key(db, &provider_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, master_key).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let settings = axagent_dao::repo::settings::get_settings(db).await.unwrap_or_default();

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path,
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
            &provider.proxy_config,
            &settings,
        ),
        custom_headers: provider.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = resolve_provider_adapter(&provider.provider_type)?;

    Ok((adapter, ctx, model_id))
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "编译 Wiki 索引")]
#[tauri::command]
pub async fn llm_wiki_compile(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: CompileInput,
) -> Result<CompileResultOutput, String> {
    let wiki_model = axagent_dao::repo::wiki::get_wiki_model(state.harness.db(), &input.wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let embedding_provider = wiki_model.embedding_provider.clone().ok_or_else(|| {
        "Wiki has no embedding_provider configured. Set one in wiki settings.".to_string()
    })?;

    let (adapter, ctx, model) =
        build_llm_adapter(state.harness.db(), state.harness.master_key(), &embedding_provider)
            .await?;

    let compiler = wiki_compiler::WikiCompiler::new(
        repositories::note_repository(),
        repositories::wiki_repository(),
        repositories::wiki_page_repository(),
        repositories::wiki_source_repository(),
        repositories::wiki_operation_repository(),
        adapter,
        ctx,
        model,
    );

    let result = compiler.compile(&input.wiki_id, input.source_ids).await?;

    let pages_to_index: Vec<(String, String)> = result
        .new_pages
        .iter()
        .chain(result.updated_pages.iter())
        .map(|p| (p.title.clone(), p.content.clone()))
        .collect();

    if !pages_to_index.is_empty() {
        let wiki = axagent_dao::repo::wiki::get_wiki(state.harness.db(), &input.wiki_id)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

        if wiki.embedding_provider.is_some() {
            let container = axagent_search::rag::KnowledgeContainer::from_wiki(&wiki);
            let db = state.harness.db().clone();
            let master_key = state.harness.master_key_owned();
            let vector_store = state.vector_store.clone();
            let wiki_id = input.wiki_id.clone();
            let app_for_emit = app.clone();

            tokio::spawn(catch_unwind_logged("llm_wiki.compile", async move {
                for (title, content) in &pages_to_index {
                    let note_result = axagent_entities::notes::Entity::find()
                        .filter(axagent_entities::notes::Column::VaultId.eq(&wiki_id))
                        .filter(axagent_entities::notes::Column::Title.eq(title))
                        .filter(axagent_entities::notes::Column::IsDeleted.eq(0))
                        .one(&db)
                        .await;

                    if let Ok(Some(note_model)) = note_result {
                        let collection_id = format!("wiki_{}", wiki_id);
                        let _ = vector_store
                            .delete_document_embeddings(&collection_id, &note_model.id)
                            .await;

                        let result = crate::indexing::index_source(
                            &db,
                            &master_key,
                            &vector_store,
                            &container,
                            &note_model.id,
                            content,
                            None,
                            None,
                        )
                        .await;

                        if let Err(e) = &result {
                            tracing::error!(
                                "Wiki compile indexing failed for {}: {}",
                                note_model.id,
                                e
                            );
                            let _ = app_for_emit.emit(
                                "wiki-note-indexed",
                                serde_json::json!({
                                    "noteId": note_model.id,
                                    "success": false,
                                    "error": e.to_string(),
                                }),
                            );
                        } else {
                            let _ = app_for_emit.emit(
                                "wiki-note-indexed",
                                serde_json::json!({
                                    "noteId": note_model.id,
                                    "success": true,
                                }),
                            );
                        }
                    }
                }
            }));
        }
    }

    Ok(CompileResultOutput {
        new_pages: result
            .new_pages
            .into_iter()
            .map(|p| CompiledPageOutput {
                title: p.title,
                content: p.content,
                page_type: p.page_type,
                source_ids: p.source_ids,
            })
            .collect(),
        updated_pages: result
            .updated_pages
            .into_iter()
            .map(|p| CompiledPageOutput {
                title: p.title,
                content: p.content,
                page_type: p.page_type,
                source_ids: p.source_ids,
            })
            .collect(),
        errors: result.errors,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryInput {
    pub wiki_id: String,
    pub query: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// 输出结构刻意保持 snake_case（无 rename_all），前端类型依赖此契约，勿改。
#[derive(Debug, Serialize)]
pub struct QueryResultOutput {
    pub pages: Vec<PageResultOutput>,
    pub total: usize,
}

// 输出结构刻意保持 snake_case（无 rename_all），前端类型依赖此契约，勿改。
#[derive(Debug, Serialize)]
pub struct PageResultOutput {
    pub note_id: String,
    pub title: String,
    pub content_snippet: String,
    pub relevance_score: f64,
    pub link_paths: Vec<String>,
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "查询 Wiki RAG 知识库")]
#[tauri::command]
pub async fn llm_wiki_query(
    state: State<'_, AppState>,
    input: QueryInput,
) -> Result<QueryResultOutput, String> {
    let wiki = axagent_dao::repo::wiki::get_wiki(state.harness.db(), &input.wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let ctx = query_engine::QueryContext {
        query: input.query.clone(),
        wiki_id: input.wiki_id,
        limit: input.limit.unwrap_or(10),
        offset: input.offset.unwrap_or(0),
    };

    let db = state.harness.db().clone();
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(Arc::new(db.clone())));
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(Arc::new(db.clone())));
    let backlink_repo: Arc<dyn NoteBacklinkRepository> =
        Arc::new(DaoNoteBacklinkRepository::new(Arc::new(db)));
    let engine = query_engine::QueryEngine::new(note_repo, wiki_repo, backlink_repo);

    let result = if let Some(ref ep) = wiki.embedding_provider {
        match generate_query_embedding(&state, ep, &input.query, wiki.embedding_dimensions).await {
            Ok(embedding) => {
                let vs =
                    Arc::new(WikiVectorSearchAdapter { vector_store: state.vector_store.clone() });
                let engine = engine.with_vector_store(vs);
                match engine.query_with_embedding(&ctx, &embedding).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(
                            "query_with_embedding failed, falling back to keyword: {}",
                            e
                        );
                        engine.query(&ctx).await?
                    },
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to generate query embedding, falling back to keyword: {}",
                    e
                );
                engine.query(&ctx).await?
            },
        }
    } else {
        engine.query(&ctx).await?
    };

    Ok(QueryResultOutput {
        pages: result
            .pages
            .into_iter()
            .map(|p| PageResultOutput {
                note_id: p.note_id,
                title: p.title,
                content_snippet: p.content_snippet,
                relevance_score: p.relevance_score,
                link_paths: p.link_paths,
            })
            .collect(),
        total: result.total,
    })
}

async fn generate_query_embedding(
    state: &AppState,
    embedding_provider: &str,
    query: &str,
    dimensions: Option<i32>,
) -> Result<Vec<f32>, String> {
    let embed_fn = crate::indexing::ProviderEmbedFn;
    let dims = dimensions.map(|d| d as usize);
    let embed_response = axagent_search::rag::AsyncEmbedFn::generate(
        &embed_fn,
        state.harness.db(),
        state.harness.master_key(),
        embedding_provider,
        vec![query.to_string()],
        dims,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    embed_response.embeddings.into_iter().next().ok_or_else(|| {
        crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::vector::EMBEDDING_FAILED,
            "No query embedding returned",
        )
    })
}

struct WikiVectorSearchAdapter {
    vector_store: Arc<axagent_search::vector_store::VectorStore>,
}

#[async_trait::async_trait]
impl query_engine::VectorSearch for WikiVectorSearchAdapter {
    async fn search(
        &self,
        wiki_id: &str,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(String, f64)>, String> {
        let collection_id = format!("wiki_{}", wiki_id);
        let results = self
            .vector_store
            .search(&collection_id, query_embedding.to_vec(), top_k)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

        Ok(results.into_iter().map(|r| (r.document_id, r.score as f64)).collect())
    }
}

/// 记录单笔记 lint 操作历史（best-effort：查 note 拿 vault_id 作为 wiki_id）。
async fn log_lint_history(
    db: &sea_orm::DatabaseConnection,
    result: &lint_checker::LintResult,
) -> Result<(), String> {
    let note = axagent_dao::repo::note::get_note(db, &result.note_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    wiki::log_wiki_operation(
        db,
        wiki::WikiOperationEntry {
            wiki_id: note.vault_id,
            operation_type: "lint".to_string(),
            target_type: "note".to_string(),
            target_id: result.note_id.clone(),
            status: "completed".to_string(),
            details: Some(serde_json::json!({
                "issueCount": result.issues.len(),
                "score": result.score,
            })),
            error_message: None,
        },
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "对 Wiki 笔记执行 lint 检查")]
#[tauri::command]
pub async fn llm_wiki_lint(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<lint_checker::LintResult, String> {
    let parser: Box<dyn KitMarkdownParser> =
        Box::new(axagent_kit::markdown_parser::MarkdownParser::new());
    let checker = lint_checker::LintChecker::new(
        repositories::note_repository(),
        repositories::wiki_repository(),
        repositories::wiki_page_repository(),
        repositories::note_backlink_repository(),
        parser,
    );
    let result = checker.lint_note(&note_id).await?;

    // 操作历史：lint 结果落一条审计记录（失败仅告警，不打断主流程）
    if let Err(e) = log_lint_history(state.harness.db(), &result).await {
        tracing::warn!("[llm_wiki] 记录 lint 操作历史失败: {e}");
    }

    Ok(result)
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "更新 Wiki 笔记质量评分")]
#[tauri::command]
pub async fn llm_wiki_lint_update_score(
    _state: State<'_, AppState>,
    note_id: String,
) -> Result<f64, String> {
    let parser: Box<dyn KitMarkdownParser> =
        Box::new(axagent_kit::markdown_parser::MarkdownParser::new());
    let checker = lint_checker::LintChecker::new(
        repositories::note_repository(),
        repositories::wiki_repository(),
        repositories::wiki_page_repository(),
        repositories::note_backlink_repository(),
        parser,
    );
    checker.update_quality_score(&note_id).await
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "获取 Wiki Schema 定义")]
#[tauri::command]
pub async fn llm_wiki_get_schema(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<String, String> {
    let db = state.harness.db().clone();
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(Arc::new(db.clone())));
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(Arc::new(db)));
    let manager = schema_manager::SchemaManager::new(note_repo, wiki_repo);
    manager.get_current_schema(&wiki_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateFrontmatterInput {
    pub wiki_id: String,
    pub frontmatter: serde_json::Map<String, serde_json::Value>,
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "验证 Wiki 笔记 frontmatter")]
#[tauri::command]
pub async fn llm_wiki_validate_frontmatter(
    state: State<'_, AppState>,
    input: ValidateFrontmatterInput,
) -> Result<Vec<String>, String> {
    let db = state.harness.db().clone();
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(Arc::new(db.clone())));
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(Arc::new(db)));
    let manager = schema_manager::SchemaManager::new(note_repo, wiki_repo);
    manager.validate_frontmatter(&input.wiki_id, &input.frontmatter).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "创建 Wiki Schema 新版本")]
#[tauri::command]
pub async fn llm_wiki_create_schema_version(
    state: State<'_, AppState>,
    wiki_id: String,
    version: String,
    description: Option<String>,
) -> Result<schema_manager::SchemaVersion, String> {
    let db = state.harness.db().clone();
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(Arc::new(db.clone())));
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(Arc::new(db)));
    let manager = schema_manager::SchemaManager::new(note_repo, wiki_repo);
    manager.create_schema_version(&wiki_id, &version, description).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSchemaInput {
    pub wiki_id: String,
    pub content: String,
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "更新 Wiki Schema")]
#[tauri::command]
pub async fn llm_wiki_update_schema(
    state: State<'_, AppState>,
    input: UpdateSchemaInput,
) -> Result<(), String> {
    let wiki = axagent_dao::repo::wiki::get_wiki_model(state.harness.db(), &input.wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let schema_path = std::path::PathBuf::from(&wiki.root_path).join("SCHEMA.md");
    if let Some(parent) = schema_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }
    tokio::fs::write(&schema_path, &input.content).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut am = wiki.into_active_model();
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(())
}

#[agent_command(domain = wiki, safety = Dangerous, call_mode = StateInput, description = "删除 Wiki Schema 版本")]
#[tauri::command]
pub async fn llm_wiki_delete_schema(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<(), String> {
    let wiki = axagent_dao::repo::wiki::get_wiki_model(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let schema_path = std::path::PathBuf::from(&wiki.root_path).join("SCHEMA.md");
    if schema_path.exists() {
        tokio::fs::remove_file(&schema_path).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    Ok(())
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "对 Wiki 库执行 lint 检查")]
#[tauri::command]
pub async fn llm_wiki_lint_vault(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<Vec<lint_checker::LintResult>, String> {
    let parser: Box<dyn KitMarkdownParser> =
        Box::new(axagent_kit::markdown_parser::MarkdownParser::new());
    let checker = lint_checker::LintChecker::new(
        repositories::note_repository(),
        repositories::wiki_repository(),
        repositories::wiki_page_repository(),
        repositories::note_backlink_repository(),
        parser,
    );
    let results = checker.lint_vault(&wiki_id).await?;

    // 操作历史：聚合一条 vault 级 lint 记录（失败仅告警，不打断主流程）
    let note_count = results.len();
    let avg_score = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|r| r.score).sum::<f64>() / note_count as f64
    };
    if let Err(e) = wiki::log_wiki_operation(
        state.harness.db(),
        wiki::WikiOperationEntry {
            wiki_id: wiki_id.clone(),
            operation_type: "lint_vault".to_string(),
            target_type: "wiki".to_string(),
            target_id: wiki_id.clone(),
            status: "completed".to_string(),
            details: Some(serde_json::json!({
                "noteCount": note_count,
                "avgScore": avg_score
            })),
            error_message: None,
        },
    )
    .await
    {
        tracing::warn!("[llm_wiki] 记录 lint_vault 操作历史失败: {e}");
    }

    Ok(results)
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "自动修复 Wiki lint 问题")]
#[tauri::command]
pub async fn llm_wiki_auto_fix(
    state: State<'_, AppState>,
    wiki_id: String,
    note_id: Option<String>,
) -> Result<Vec<String>, String> {
    let parser: Box<dyn KitMarkdownParser> =
        Box::new(axagent_kit::markdown_parser::MarkdownParser::new());
    let checker = lint_checker::LintChecker::new(
        repositories::note_repository(),
        repositories::wiki_repository(),
        repositories::wiki_page_repository(),
        repositories::note_backlink_repository(),
        parser,
    );
    let fixed = checker.auto_fix(&wiki_id, note_id.as_deref()).await?;

    // 操作历史：auto_fix 结果落一条审计记录（失败仅告警，不打断主流程）
    if let Err(e) = wiki::log_wiki_operation(
        state.harness.db(),
        wiki::WikiOperationEntry {
            wiki_id: wiki_id.clone(),
            operation_type: "auto_fix".to_string(),
            target_type: "wiki".to_string(),
            target_id: note_id.clone().unwrap_or_else(|| wiki_id.clone()),
            status: "completed".to_string(),
            details: Some(serde_json::json!({ "fixedCount": fixed.len() })),
            error_message: None,
        },
    )
    .await
    {
        tracing::warn!("[llm_wiki] 记录 auto_fix 操作历史失败: {e}");
    }

    Ok(fixed)
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "向 Wiki 提问并获取回答")]
#[tauri::command]
pub async fn llm_wiki_ask(
    state: State<'_, AppState>,
    wiki_id: String,
    question: String,
) -> Result<String, String> {
    let wiki_model = axagent_dao::repo::wiki::get_wiki_model(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let embedding_provider = wiki_model.embedding_provider.clone().ok_or_else(|| {
        crate::commands::error::ErrorResponse::err(
            crate::commands::error_code::knowledge::NO_EMBEDDING_PROVIDER,
        )
    })?;

    let (adapter, ctx, model) =
        build_llm_adapter(state.harness.db(), state.harness.master_key(), &embedding_provider)
            .await?;

    let db = state.harness.db().clone();
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(Arc::new(db.clone())));
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(Arc::new(db.clone())));
    let backlink_repo: Arc<dyn NoteBacklinkRepository> =
        Arc::new(DaoNoteBacklinkRepository::new(Arc::new(db)));
    let engine = query_engine::QueryEngine::new(note_repo, wiki_repo, backlink_repo)
        .with_llm(adapter, ctx, model);

    engine.ask(&wiki_id, &question).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteBase64Input {
    pub wiki_id: String,
    pub file_name: String,
    pub base64_content: String,
    pub source_type: String,
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "将 Base64 内容写入 Wiki 文件")]
#[tauri::command]
pub async fn write_base64_to_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: WriteBase64Input,
) -> Result<String, String> {
    let wiki = axagent_dao::repo::wiki::get_wiki_model(state.harness.db(), &input.wiki_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let bytes = base64_decode(&input.base64_content)?;

    let raw_dir = std::path::PathBuf::from(&wiki.root_path).join("raw");
    tokio::fs::create_dir_all(&raw_dir).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if input.file_name.contains("..")
        || input.file_name.contains('/')
        || input.file_name.contains('\\')
    {
        return Err(crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::security::PATH_TRAVERSAL,
            format!("Invalid file name: {}", input.file_name),
        ));
    }

    let file_path = raw_dir.join(&input.file_name);
    tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let _source_content =
        String::from_utf8(bytes).unwrap_or_else(|_| "[Binary content]".to_string());

    let db = Arc::new(state.harness.db().clone());
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(db.clone()));
    let wiki_source_repo: Arc<dyn WikiSourceRepository> =
        Arc::new(DaoWikiSourceRepository::new(db.clone()));
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(db));
    let pipeline = ingest_pipeline::IngestPipeline::new(wiki_repo, wiki_source_repo, note_repo);
    let source = ingest_pipeline::IngestSource {
        source_type: match input.source_type.as_str() {
            "web" => ingest_pipeline::IngestSourceType::WebArticle,
            "paper" => ingest_pipeline::IngestSourceType::Paper,
            "pdf" => ingest_pipeline::IngestSourceType::Pdf,
            "docx" => ingest_pipeline::IngestSourceType::Docx,
            _ => ingest_pipeline::IngestSourceType::RawMarkdown,
        },
        path: file_path.to_string_lossy().to_string(),
        url: None,
        title: Some(input.file_name.clone()),
        folder_context: None,
        content: None,
    };

    let result = pipeline.ingest(&input.wiki_id, source).await?;

    // R2 修复：生成页补入 RAG 索引（此前 write_base64_to_file 只落库不入索引）
    crate::indexing::spawn_wiki_note_batch_indexing(crate::indexing::WikiBatchIndexingTask {
        app,
        db: state.harness.db().clone(),
        master_key: state.harness.master_key_owned(),
        vector_store: state.vector_store.clone(),
        wiki_id: input.wiki_id.clone(),
        note_ids: result.generated_note_ids.clone(),
        log_label: "llm_wiki.write_base64",
        completion_event: None,
    });

    Ok(result.source_id)
}

fn base64_decode(encoded: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|e| {
        crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::common::INVALID_INPUT,
            format!("Base64 decode failed: {e}"),
        )
    })
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "处理 Wiki 待同步队列项")]
#[tauri::command]
pub async fn wiki_sync_process_pending(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<usize, String> {
    let pending = wiki_sync_queue::Entity::find()
        .filter(wiki_sync_queue::Column::WikiId.eq(&wiki_id))
        .filter(wiki_sync_queue::Column::Status.eq("pending"))
        .all(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let mut processed = 0;
    for item in pending {
        if item.retry_count >= 3 {
            // 重试次数超限：标记为 "failed" 避免无限循环查询
            // 先保存 error_message，因为 into_active_model 会消费 item
            let last_error = item.error_message.clone().unwrap_or_default();
            let mut am = item.into_active_model();
            am.status = Set("failed".to_string());
            am.error_message =
                Set(Some(format!("exceeded max retry count (3), last error: {}", last_error)));
            let _ = am.update(state.harness.db()).await;
            continue;
        }

        let item_clone = item.clone();
        let mut am = item.into_active_model();
        am.status = Set("processing".to_string());
        am.update(state.harness.db()).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

        match process_sync_event(
            state.harness.db(),
            state.harness.master_key(),
            state.vector_store.as_ref(),
            &item_clone,
        )
        .await
        {
            Ok(_) => {
                let mut am = item_clone.clone().into_active_model();
                am.status = Set("completed".to_string());
                am.processed_at = Set(Some(chrono::Utc::now().timestamp()));
                // 处理已成功，状态更新失败时仅记录日志，不返回错误，
                // 避免状态永久卡在 "processing" 导致下次查询无法重试。
                if let Err(e) = am.update(state.harness.db()).await {
                    tracing::error!(
                        "[wiki-sync] 队列项 {} 处理成功但状态更新为 completed 失败: {}",
                        item_clone.id,
                        e
                    );
                }
                processed += 1;
            },
            Err(e) => {
                let mut am = item_clone.clone().into_active_model();
                am.status = Set("failed".to_string());
                am.error_message = Set(Some(e.to_string()));
                am.retry_count = Set(item_clone.retry_count + 1);
                if let Err(update_err) = am.update(state.harness.db()).await {
                    tracing::error!(
                        "[wiki-sync] 队列项 {} 处理失败且状态更新为 failed 也失败: {}",
                        item_clone.id,
                        update_err
                    );
                }
            },
        }
    }

    Ok(processed)
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "创建 Wiki 同步队列项")]
#[tauri::command]
pub async fn wiki_sync_enqueue(
    state: State<'_, AppState>,
    wiki_id: String,
    event_type: String,
    target_type: String,
    target_id: String,
    payload: Option<String>,
) -> Result<i64, String> {
    let payload_json = payload.and_then(|p| serde_json::from_str(&p).ok());

    let model = wiki_sync_queue::ActiveModel {
        wiki_id: Set(wiki_id),
        event_type: Set(event_type),
        target_type: Set(target_type),
        target_id: Set(target_id),
        payload: Set(payload_json),
        status: Set("pending".to_string()),
        retry_count: Set(0),
        error_message: Set(None),
        created_at: Set(chrono::Utc::now().timestamp()),
        processed_at: Set(None),
        ..Default::default()
    };

    let result =
        wiki_sync_queue::Entity::insert(model).exec(state.harness.db()).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(result.last_insert_id)
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "获取 Wiki 同步队列状态")]
#[tauri::command]
pub async fn wiki_sync_get_queue(
    state: State<'_, AppState>,
    wiki_id: String,
    status: Option<String>,
) -> Result<Vec<wiki_sync_queue::Model>, String> {
    let mut query = wiki_sync_queue::Entity::find();
    query = query.filter(wiki_sync_queue::Column::WikiId.eq(wiki_id));

    if let Some(s) = status {
        query = query.filter(wiki_sync_queue::Column::Status.eq(s));
    }

    query
        .order_by(wiki_sync_queue::Column::CreatedAt, sea_orm::Order::Desc)
        .limit(100)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "处理单个 Wiki 同步队列项")]
#[tauri::command]
pub async fn wiki_sync_process(state: State<'_, AppState>, queue_id: i64) -> Result<(), String> {
    let model = wiki_sync_queue::Entity::find_by_id(queue_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            crate::commands::error::ErrorResponse::err_with_detail(
                crate::commands::error_code::wiki::NOT_FOUND,
                format!("Queue item {queue_id} not found"),
            )
        })?;

    let model_clone = model.clone();
    let mut am = model.into_active_model();
    am.status = Set("processing".to_string());
    am.update(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let result = process_sync_event(
        state.harness.db(),
        state.harness.master_key(),
        state.vector_store.as_ref(),
        &model_clone,
    )
    .await;

    match result {
        Ok(_) => {
            let mut am = model_clone.clone().into_active_model();
            am.status = Set("completed".to_string());
            am.processed_at = Set(Some(chrono::Utc::now().timestamp()));
            // 处理已成功，状态更新失败时仅记录日志，不返回错误，
            // 避免状态永久卡在 "processing" 导致下次查询无法重试。
            if let Err(e) = am.update(state.harness.db()).await {
                tracing::error!(
                    "[wiki-sync] 队列项 {} 处理成功但状态更新为 completed 失败: {}",
                    model_clone.id,
                    e
                );
            }

            // 操作历史：sync 事件处理结果落审计记录（失败仅告警，不打断主流程）
            if let Err(e) = wiki::log_wiki_operation(
                state.harness.db(),
                wiki::WikiOperationEntry {
                    wiki_id: model_clone.wiki_id.clone(),
                    operation_type: "sync".to_string(),
                    target_type: model_clone.target_type.clone(),
                    target_id: model_clone.target_id.clone(),
                    status: "completed".to_string(),
                    details: Some(serde_json::json!({ "eventType": model_clone.event_type })),
                    error_message: None,
                },
            )
            .await
            {
                tracing::warn!("[wiki-sync] 记录 sync 操作历史失败: {e}");
            }
            Ok(())
        },
        Err(e) => {
            let err_detail = e.to_string();
            let mut am = model_clone.clone().into_active_model();
            am.status = Set("failed".to_string());
            am.error_message = Set(Some(err_detail.clone()));
            am.retry_count = Set(model_clone.retry_count + 1);
            if let Err(update_err) = am.update(state.harness.db()).await {
                tracing::error!(
                    "[wiki-sync] 队列项 {} 处理失败且状态更新为 failed 也失败: {}",
                    model_clone.id,
                    update_err
                );
            }

            // 操作历史：sync 失败也落审计记录（失败仅告警）
            if let Err(log_err) = wiki::log_wiki_operation(
                state.harness.db(),
                wiki::WikiOperationEntry {
                    wiki_id: model_clone.wiki_id.clone(),
                    operation_type: "sync".to_string(),
                    target_type: model_clone.target_type.clone(),
                    target_id: model_clone.target_id.clone(),
                    status: "failed".to_string(),
                    details: Some(serde_json::json!({ "eventType": model_clone.event_type })),
                    error_message: Some(err_detail),
                },
            )
            .await
            {
                tracing::warn!("[wiki-sync] 记录 sync 失败操作历史失败: {log_err}");
            }

            // C-3: 迁移到 ErrorResponse，保留原始错误信息用于调试
            Err(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            )
            .to_string())
        },
    }
}

/// 处理单条 `wiki_sync_queue` 事件。
///
/// R3/R4 收敛：`wiki_sync_queue` 定位为**观测/审计通道**（供前端同步中心展示与重试），
/// 向量索引的唯一执行链是 `index_jobs` 队列（enqueue_index → run_indexing → index_source）。
/// 此前 note_created/note_updated 在此重复执行 index_source，与 index_jobs 双跑一次 embedding；
/// note_deleted/wiki_deleted 无任何生产者（wiki_notes_delete / delete_wiki 已直接清向量）。
/// 因此这里不再执行副作用，仅记录事件日志，由 wiki_sync_process_pending 标记状态。
async fn process_sync_event(
    _db: &sea_orm::DatabaseConnection,
    _master_key: &[u8; 32],
    _vector_store: &axagent_search::vector_store::VectorStore,
    model: &wiki_sync_queue::Model,
) -> Result<(), axagent_harness::core_error::AxAgentError> {
    match model.event_type.as_str() {
        "note_created" | "note_updated" => {
            tracing::debug!(
                "Sync: note {} {}（向量索引由 index_jobs 链负责，此处仅审计）",
                model.target_id,
                model.event_type
            );
            Ok(())
        },
        "note_deleted" | "source_ingested" | "schema_updated" | "wiki_created" | "wiki_deleted" => {
            tracing::debug!("Sync: 事件 {}（无副作用，仅审计记录）", model.event_type);
            Ok(())
        },
        _ => {
            tracing::warn!("Sync: unknown event type '{}'", model.event_type);
            Ok(())
        },
    }
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "检查 Wiki RAG 容量")]
#[tauri::command]
pub async fn wiki_check_capacity(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<axagent_search::rag::CapacityCheckResult, String> {
    axagent_search::rag::check_vault_rag_capacity(state.harness.db(), &wiki_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "获取 Wiki 容量信息")]
#[tauri::command]
pub async fn wiki_get_capacity_info(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<axagent_search::rag::VaultCapacityInfo, String> {
    axagent_search::rag::get_vault_capacity_info(state.harness.db(), &wiki_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "获取 Wiki 目的描述")]
#[tauri::command]
pub async fn llm_wiki_get_purpose(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<String, String> {
    let wiki_repo: Arc<dyn WikiRepository> =
        Arc::new(DaoWikiRepository::new(Arc::new(state.harness.db().clone())));
    purpose_manager::PurposeManager::load(&*wiki_repo, &wiki_id).await
}

#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "更新 Wiki 目的描述")]
#[tauri::command]
pub async fn llm_wiki_update_purpose(
    state: State<'_, AppState>,
    wiki_id: String,
    content: String,
) -> Result<(), String> {
    let wiki_repo: Arc<dyn WikiRepository> =
        Arc::new(DaoWikiRepository::new(Arc::new(state.harness.db().clone())));
    purpose_manager::PurposeManager::save(&*wiki_repo, &wiki_id, &content).await
}

// ── 文件夹递归导入 ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportInput {
    pub wiki_id: String,
    pub folder_path: String,
}

#[derive(Debug, Serialize)]
pub struct FolderImportPreviewOutput {
    pub file_name: String,
    pub file_path: String,
    pub folder_context: String,
    pub file_type: String,
    pub estimated_size: u64,
}

impl From<ingest_queue::FolderImportPreviewItem> for FolderImportPreviewOutput {
    fn from(item: ingest_queue::FolderImportPreviewItem) -> Self {
        Self {
            file_name: item.file_name,
            file_path: item.file_path,
            folder_context: item.folder_context,
            file_type: format!("{:?}", item.file_type),
            estimated_size: item.estimated_size,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FolderImportResultOutput {
    pub task_ids: Vec<String>,
    pub imported_count: usize,
    pub failed_files: Vec<String>,
}

/// 预览文件夹内容（不执行实际导入）
#[agent_command(domain = wiki, safety = Safe, call_mode = StateInput, description = "预览文件夹内容（不执行实际导入）")]
#[tauri::command]
pub async fn llm_wiki_import_folder_preview(
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<Vec<FolderImportPreviewOutput>, String> {
    let db = Arc::new(state.harness.db().clone());
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(db.clone()));
    let wiki_source_repo: Arc<dyn WikiSourceRepository> =
        Arc::new(DaoWikiSourceRepository::new(db.clone()));
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(db));
    let pipeline =
        Arc::new(ingest_pipeline::IngestPipeline::new(wiki_repo, wiki_source_repo, note_repo));
    let queue = ingest_queue::IngestQueue::new(pipeline, String::new());

    let items = queue.get_folder_import_preview(&folder_path).await?;
    Ok(items.into_iter().map(FolderImportPreviewOutput::from).collect())
}

/// 递归导入文件夹中所有文件到 Wiki
#[agent_command(domain = wiki, safety = Caution, call_mode = StateInput, description = "递归导入文件夹中所有文件到 Wiki")]
#[tauri::command]
pub async fn llm_wiki_import_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: FolderImportInput,
) -> Result<FolderImportResultOutput, String> {
    let db = Arc::new(state.harness.db().clone());
    let wiki_repo: Arc<dyn WikiRepository> = Arc::new(DaoWikiRepository::new(db.clone()));
    let wiki_source_repo: Arc<dyn WikiSourceRepository> =
        Arc::new(DaoWikiSourceRepository::new(db.clone()));
    let note_repo: Arc<dyn NoteRepository> = Arc::new(DaoNoteRepository::new(db.clone()));
    let pipeline =
        Arc::new(ingest_pipeline::IngestPipeline::new(wiki_repo, wiki_source_repo, note_repo));

    let app_data_dir = crate::paths::axagent_home();
    let queue_dir = format!("{}/wiki_{}/import_queue", app_data_dir.display(), input.wiki_id);
    let queue = ingest_queue::IngestQueue::new(pipeline, queue_dir);

    let task_ids = queue.import_folder(&input.wiki_id, &input.folder_path).await?;
    let total = task_ids.len();

    let mut imported_count = 0usize;
    let mut failed_files = Vec::new();
    let mut all_note_ids: Vec<String> = Vec::new();

    // 逐个处理导入任务
    for task_id in &task_ids {
        let result = queue.process_next().await;
        match result {
            Some(ingest_result) => {
                all_note_ids.extend(ingest_result.generated_note_ids);
                imported_count += 1;
            },
            None => {
                if let Some(task) = queue.get_task(task_id).await {
                    let err_msg = task.error_message.unwrap_or_else(|| "Unknown error".to_string());
                    failed_files.push(format!("{}: {}", task.source.path, err_msg));
                } else {
                    failed_files.push(task_id.clone());
                }
            },
        }
    }

    // 为已导入的文件触发向量索引（R2：统一走公共批量索引 helper）
    crate::indexing::spawn_wiki_note_batch_indexing(crate::indexing::WikiBatchIndexingTask {
        app,
        db: state.harness.db().clone(),
        master_key: state.harness.master_key_owned(),
        vector_store: state.vector_store.clone(),
        wiki_id: input.wiki_id.clone(),
        note_ids: all_note_ids,
        log_label: "llm_wiki.import_folder_indexing",
        completion_event: Some("wiki-folder-import-complete"),
    });

    let _ = total; // 避免未使用警告
    Ok(FolderImportResultOutput { task_ids, imported_count, failed_files })
}
