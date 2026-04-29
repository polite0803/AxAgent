use axagent_agent::{ingest_pipeline, lint_checker, query_engine, schema_manager, wiki_compiler};
use axagent_core::entity::wiki_sync_queue;
use axagent_core::repo::wiki;
use crate::AppState;
use sea_orm::{EntityTrait, ColumnTrait, QueryOrder, QuerySelect, ActiveModelTrait, IntoActiveModel, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

impl From<axagent_core::entity::wikis::Model> for WikiOutput {
    fn from(m: axagent_core::entity::wikis::Model) -> Self {
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

impl From<axagent_core::entity::wiki_operations::Model> for WikiOperationOutput {
    fn from(m: axagent_core::entity::wiki_operations::Model) -> Self {
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

#[tauri::command]
pub async fn llm_wiki_list(
    state: State<'_, AppState>,
) -> Result<Vec<WikiOutput>, String> {
    let wikis = axagent_core::entity::wikis::Entity::find()
        .order_by(axagent_core::entity::wikis::Column::CreatedAt, sea_orm::Order::Desc)
        .all(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(wikis.into_iter().map(WikiOutput::from).collect())
}

#[derive(Debug, Deserialize)]
pub struct CreateWikiInput {
    pub name: String,
    pub root_path: String,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn llm_wiki_create(
    state: State<'_, AppState>,
    input: CreateWikiInput,
) -> Result<WikiOutput, String> {
    let wiki_input = wiki::CreateWikiInput {
        name: input.name,
        description: input.description,
        root_path: input.root_path,
        embedding_provider: None,
    };

    let model = wiki::create_wiki(&state.sea_db, wiki_input)
        .await
        .map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn llm_wiki_delete(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<(), String> {
    wiki::delete_wiki(&state.sea_db, &wiki_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_wiki_operations_list(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<Vec<WikiOperationOutput>, String> {
    let operations = axagent_core::entity::wiki_operations::Entity::find()
        .filter(axagent_core::entity::wiki_operations::Column::WikiId.eq(&wiki_id))
        .order_by(axagent_core::entity::wiki_operations::Column::CreatedAt, sea_orm::Order::Desc)
        .limit(100)
        .all(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(operations.into_iter().map(WikiOperationOutput::from).collect())
}

#[derive(Debug, Deserialize)]
pub struct IngestSourceInput {
    pub wiki_id: String,
    pub source_type: String,
    pub path: String,
    pub url: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IngestResultOutput {
    pub source_id: String,
    pub raw_path: String,
    pub title: String,
}

#[tauri::command]
pub async fn llm_wiki_ingest(
    state: State<'_, AppState>,
    input: IngestSourceInput,
) -> Result<IngestResultOutput, String> {
    let pipeline = ingest_pipeline::IngestPipeline::new(Arc::new(state.sea_db.clone()));

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
    };

    let result = pipeline.ingest(&input.wiki_id, source).await?;

    Ok(IngestResultOutput {
        source_id: result.source_id,
        raw_path: result.raw_path,
        title: result.title,
    })
}

#[derive(Debug, Deserialize)]
pub struct CompileInput {
    pub wiki_id: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CompileResultOutput {
    pub new_pages: Vec<CompiledPageOutput>,
    pub updated_pages: Vec<CompiledPageOutput>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CompiledPageOutput {
    pub title: String,
    pub content: String,
    pub page_type: String,
    pub source_ids: Vec<String>,
}

#[tauri::command]
pub async fn llm_wiki_compile(
    state: State<'_, AppState>,
    input: CompileInput,
) -> Result<CompileResultOutput, String> {
    let compiler = wiki_compiler::WikiCompiler::new(Arc::new(state.sea_db.clone()));
    let result = compiler.compile(&input.wiki_id, input.source_ids).await?;

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
pub struct QueryInput {
    pub wiki_id: String,
    pub query: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct QueryResultOutput {
    pub pages: Vec<PageResultOutput>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct PageResultOutput {
    pub note_id: String,
    pub title: String,
    pub content_snippet: String,
    pub relevance_score: f64,
    pub link_paths: Vec<String>,
}

#[tauri::command]
pub async fn llm_wiki_query(
    state: State<'_, AppState>,
    input: QueryInput,
) -> Result<QueryResultOutput, String> {
    let engine = query_engine::QueryEngine::new(Arc::new(state.sea_db.clone()));
    let ctx = query_engine::QueryContext {
        query: input.query,
        wiki_id: input.wiki_id,
        limit: input.limit.unwrap_or(10),
        offset: input.offset.unwrap_or(0),
    };

    let result = engine.query(&ctx).await?;

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

#[tauri::command]
pub async fn llm_wiki_lint(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<lint_checker::LintResult, String> {
    let checker = lint_checker::LintChecker::new(Arc::new(state.sea_db.clone()));
    checker.lint_note(&note_id).await
}

#[tauri::command]
pub async fn llm_wiki_lint_update_score(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<f64, String> {
    let checker = lint_checker::LintChecker::new(Arc::new(state.sea_db.clone()));
    checker.update_quality_score(&note_id).await
}

#[tauri::command]
pub async fn llm_wiki_get_schema(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<String, String> {
    let manager = schema_manager::SchemaManager::new(Arc::new(state.sea_db.clone()));
    manager.get_current_schema(&wiki_id).await
}

#[derive(Debug, Deserialize)]
pub struct ValidateFrontmatterInput {
    pub wiki_id: String,
    pub frontmatter: serde_json::Map<String, serde_json::Value>,
}

#[tauri::command]
pub async fn llm_wiki_validate_frontmatter(
    state: State<'_, AppState>,
    input: ValidateFrontmatterInput,
) -> Result<Vec<String>, String> {
    let manager = schema_manager::SchemaManager::new(Arc::new(state.sea_db.clone()));
    manager
        .validate_frontmatter(&input.wiki_id, &input.frontmatter)
        .await
}

#[tauri::command]
pub async fn llm_wiki_create_schema_version(
    state: State<'_, AppState>,
    wiki_id: String,
    version: String,
    description: Option<String>,
) -> Result<schema_manager::SchemaVersion, String> {
    let manager = schema_manager::SchemaManager::new(Arc::new(state.sea_db.clone()));
    manager.create_schema_version(&wiki_id, &version, description).await
}

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

    let result = wiki_sync_queue::Entity::insert(model)
        .exec(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.last_insert_id)
}

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
        .all(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_sync_process(
    state: State<'_, AppState>,
    queue_id: i64,
) -> Result<(), String> {
    let model = wiki_sync_queue::Entity::find_by_id(queue_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Queue item not found".to_string())?;

    let model_clone = model.clone();
    let mut am = model.into_active_model();
    am.status = Set("processing".to_string());
    am.update(&state.sea_db).await.map_err(|e| e.to_string())?;

    let result = process_sync_event(&state.sea_db, &model_clone).await;

    match result {
        Ok(_) => {
            let mut am = model_clone.clone().into_active_model();
            am.status = Set("completed".to_string());
            am.processed_at = Set(Some(chrono::Utc::now().timestamp()));
            am.update(&state.sea_db).await.map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            let mut am = model_clone.clone().into_active_model();
            am.status = Set("failed".to_string());
            am.error_message = Set(Some(e.to_string()));
            am.retry_count = Set(model_clone.retry_count + 1);
            am.update(&state.sea_db).await.map_err(|e| e.to_string())?;
            Err(e.to_string())
        }
    }
}

async fn process_sync_event(
    _db: &sea_orm::DatabaseConnection,
    model: &wiki_sync_queue::Model,
) -> Result<(), axagent_core::error::AxAgentError> {
    match model.event_type.as_str() {
        "note_created" | "note_updated" => {
            // Index the note to vector store
            Ok(())
        }
        "note_deleted" => {
            // Remove from vector store
            Ok(())
        }
        "source_ingested" => {
            // Process source ingestion
            Ok(())
        }
        "schema_updated" => {
            // Update schema tracking
            Ok(())
        }
        "wiki_created" | "wiki_deleted" => {
            // Handle wiki-level events
            Ok(())
        }
        _ => Ok(()),
    }
}

#[tauri::command]
pub async fn wiki_check_capacity(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<axagent_core::rag::CapacityCheckResult, String> {
    axagent_core::rag::check_vault_rag_capacity(&state.sea_db, &wiki_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_get_capacity_info(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<axagent_core::rag::VaultCapacityInfo, String> {
    axagent_core::rag::get_vault_capacity_info(&state.sea_db, &wiki_id)
        .await
        .map_err(|e| e.to_string())
}