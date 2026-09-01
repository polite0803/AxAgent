// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::repo::note::calculate_content_hash;
use axagent_entities::{
    note_backlinks, note_links, notes, wiki_operations, wiki_page_versions, wiki_pages,
    wiki_sources, wiki_templates, wikis,
};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::util_fns::gen_id;

// Wiki DTO 在 harness 里定义（提升到 harness 让 search 等下游 crate 不用反向依赖 dao），
// 这里 re-export 保持向后兼容。
pub use axagent_harness::types::Wiki;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWikiInput {
    pub name: String,
    pub description: Option<String>,
    pub root_path: String,
    pub embedding_provider: Option<String>,
    /// v118: 可选关联知识库 ID，建立 Wiki 与 KB 的 1:1 关联
    pub knowledge_base_id: Option<String>,
}

fn model_to_wiki(m: wikis::Model) -> Wiki {
    Wiki {
        id: m.id,
        name: m.name,
        description: m.description,
        root_path: m.root_path,
        schema_version: m.schema_version,
        note_count: m.note_count,
        source_count: m.source_count,
        embedding_provider: m.embedding_provider,
        embedding_dimensions: m.embedding_dimensions,
        retrieval_threshold: m.retrieval_threshold,
        retrieval_top_k: m.retrieval_top_k,
        knowledge_base_id: m.knowledge_base_id,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub async fn create_wiki(db: &DatabaseConnection, input: CreateWikiInput) -> Result<Wiki> {
    let now = chrono::Utc::now().timestamp();
    let id = gen_id();

    wikis::Entity::insert(wikis::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        description: Set(input.description),
        root_path: Set(input.root_path),
        schema_version: Set("1.0".to_string()),
        note_count: Set(0),
        source_count: Set(0),
        embedding_provider: Set(input.embedding_provider),
        embedding_dimensions: Set(None),
        retrieval_threshold: Set(None),
        retrieval_top_k: Set(None),
        knowledge_base_id: Set(input.knowledge_base_id),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .exec(db)
    .await?;

    get_wiki(db, &id).await
}

pub async fn get_wiki(db: &DatabaseConnection, id: &str) -> Result<Wiki> {
    let model = get_wiki_model(db, id).await?;
    Ok(model_to_wiki(model))
}

/// Returns the raw SeaORM Model for commands that need to modify the wiki record.
pub async fn get_wiki_model(db: &DatabaseConnection, id: &str) -> Result<wikis::Model> {
    wikis::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", id)))
}

pub async fn list_wikis(db: &DatabaseConnection) -> Result<Vec<Wiki>> {
    let models =
        wikis::Entity::find().order_by(wikis::Column::UpdatedAt, Order::Desc).all(db).await?;

    Ok(models.into_iter().map(model_to_wiki).collect())
}

pub async fn update_wiki(
    db: &DatabaseConnection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    embedding_provider: Option<String>,
    knowledge_base_id: Option<Option<String>>,
) -> Result<Wiki> {
    let model = wikis::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", id)))?;

    let mut am = model.into_active_model();
    if let Some(n) = name {
        am.name = Set(n);
    }
    if let Some(d) = description {
        am.description = Set(Some(d));
    }
    if let Some(ep) = embedding_provider {
        am.embedding_provider = Set(Some(ep));
    }
    if let Some(kb_id) = knowledge_base_id {
        am.knowledge_base_id = Set(kb_id);
    }
    am.updated_at = Set(chrono::Utc::now().timestamp());

    am.update(db).await?;

    get_wiki(db, id).await
}

pub async fn delete_wiki(db: &DatabaseConnection, id: &str) -> Result<()> {
    // 物理删除该 Wiki 下的所有索引任务，容器已不存在，保留 CANCELLED job 无意义
    // （该函数只接受 &DatabaseConnection，无法进事务，作为 best-effort 前置清理）
    if let Err(e) = crate::repo::index_jobs::delete_jobs_by_container(db, "wiki", id).await {
        tracing::warn!(
            wiki_id = id,
            error = %e,
            "[dao::wiki] 删除相关索引任务失败，继续级联删除"
        );
    }

    // 级联删除子表（wikis 关联表多，SQLite 启用了 PRAGMA foreign_keys=ON，
    // 未级联清理会触发外键约束失败）。
    // 事务包裹：中途任一步失败整体回滚，避免留下半删状态（R 修复批次 2）。
    // 闭包要求 'static，id 需要拷贝为 owned String。
    let wiki_id = id.to_string();
    db.transaction::<_, _, AxAgentError>(move |txn| {
        let wiki_id = wiki_id.clone();
        Box::pin(async move {
            let id = wiki_id.as_str();
            wiki_sources::Entity::delete_many()
                .filter(wiki_sources::Column::WikiId.eq(id))
                .exec(txn)
                .await?;
            wiki_page_versions::Entity::delete_many()
                .filter(wiki_page_versions::Column::WikiId.eq(id))
                .exec(txn)
                .await?;
            wiki_pages::Entity::delete_many()
                .filter(wiki_pages::Column::WikiId.eq(id))
                .exec(txn)
                .await?;
            note_links::Entity::delete_many()
                .filter(note_links::Column::VaultId.eq(id))
                .exec(txn)
                .await?;
            note_backlinks::Entity::delete_many()
                .filter(note_backlinks::Column::VaultId.eq(id))
                .exec(txn)
                .await?;
            notes::Entity::delete_many().filter(notes::Column::VaultId.eq(id)).exec(txn).await?;

            let result = wikis::Entity::delete_by_id(id).exec(txn).await?;
            if result.rows_affected == 0 {
                return Err(AxAgentError::NotFound(format!("Wiki {} not found", id)));
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| match e {
        sea_orm::TransactionError::Connection(err) => AxAgentError::from(err),
        sea_orm::TransactionError::Transaction(err) => err,
    })
}

pub async fn increment_note_count(db: &DatabaseConnection, wiki_id: &str) -> Result<()> {
    // 原子自增：避免读-改-写在并发下丢计数
    let result = wikis::Entity::update_many()
        .col_expr(wikis::Column::NoteCount, sea_query::Expr::col(wikis::Column::NoteCount).add(1))
        .col_expr(wikis::Column::UpdatedAt, sea_query::Expr::value(chrono::Utc::now().timestamp()))
        .filter(wikis::Column::Id.eq(wiki_id))
        .exec(db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Wiki {} not found", wiki_id)));
    }
    Ok(())
}

pub async fn increment_source_count(db: &DatabaseConnection, wiki_id: &str) -> Result<()> {
    // 原子自增：避免读-改-写在并发下丢计数
    let result = wikis::Entity::update_many()
        .col_expr(
            wikis::Column::SourceCount,
            sea_query::Expr::col(wikis::Column::SourceCount).add(1),
        )
        .col_expr(wikis::Column::UpdatedAt, sea_query::Expr::value(chrono::Utc::now().timestamp()))
        .filter(wikis::Column::Id.eq(wiki_id))
        .exec(db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Wiki {} not found", wiki_id)));
    }
    Ok(())
}

pub use axagent_harness::wiki_dtos::NoteVersion;

fn model_to_version(m: wiki_page_versions::Model) -> NoteVersion {
    NoteVersion {
        id: m.id,
        wiki_id: m.wiki_id,
        note_id: m.note_id,
        title: m.title,
        content: m.content,
        content_hash: m.content_hash,
        author: m.author,
        created_at: m.created_at,
    }
}

pub async fn create_version(
    db: &DatabaseConnection,
    wiki_id: &str,
    note_id: &str,
    title: &str,
    content: &str,
    author: &str,
) -> Result<NoteVersion> {
    let now = chrono::Utc::now().timestamp();
    let content_hash = calculate_content_hash(content);

    let am = wiki_page_versions::ActiveModel {
        wiki_id: Set(wiki_id.to_string()),
        note_id: Set(note_id.to_string()),
        title: Set(title.to_string()),
        content: Set(content.to_string()),
        content_hash: Set(content_hash),
        author: Set(author.to_string()),
        created_at: Set(now),
        ..Default::default()
    };

    let model = am.insert(db).await?;

    Ok(model_to_version(model))
}

pub async fn list_versions(db: &DatabaseConnection, note_id: &str) -> Result<Vec<NoteVersion>> {
    let models = wiki_page_versions::Entity::find()
        .filter(wiki_page_versions::Column::NoteId.eq(note_id))
        .order_by(wiki_page_versions::Column::CreatedAt, Order::Desc)
        .limit(50)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_version).collect())
}

pub async fn get_version(db: &DatabaseConnection, id: i64) -> Result<NoteVersion> {
    let model = wiki_page_versions::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("NoteVersion {}", id)))?;

    Ok(model_to_version(model))
}

pub async fn delete_old_versions(
    db: &DatabaseConnection,
    note_id: &str,
    keep: usize,
) -> Result<usize> {
    let all = wiki_page_versions::Entity::find()
        .filter(wiki_page_versions::Column::NoteId.eq(note_id))
        .order_by(wiki_page_versions::Column::CreatedAt, Order::Desc)
        .all(db)
        .await?;

    if all.len() <= keep {
        return Ok(0);
    }

    let to_delete: Vec<i64> = all.into_iter().skip(keep).map(|m| m.id).collect();
    let count = to_delete.len();

    for id in to_delete {
        wiki_page_versions::Entity::delete_by_id(id).exec(db).await?;
    }

    Ok(count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiTemplate {
    pub id: String,
    pub wiki_id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub page_type: Option<String>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWikiTemplateInput {
    pub wiki_id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub page_type: Option<String>,
    pub is_builtin: bool,
}

/// 更新 Wiki 模板的输入（部分更新，None 字段不覆盖）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWikiTemplateInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub page_type: Option<String>,
}

fn model_to_template(m: wiki_templates::Model) -> WikiTemplate {
    WikiTemplate {
        id: m.id,
        wiki_id: m.wiki_id,
        name: m.name,
        description: m.description,
        content: m.content,
        page_type: m.page_type,
        is_builtin: m.is_builtin,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub async fn list_wiki_templates(
    db: &DatabaseConnection,
    wiki_id: &str,
) -> Result<Vec<WikiTemplate>> {
    let models = wiki_templates::Entity::find()
        .filter(wiki_templates::Column::WikiId.eq(wiki_id))
        .order_by(wiki_templates::Column::Name, Order::Asc)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_template).collect())
}

pub async fn create_wiki_template(
    db: &DatabaseConnection,
    input: CreateWikiTemplateInput,
) -> Result<WikiTemplate> {
    let now = chrono::Utc::now().timestamp();
    let id = gen_id();

    wiki_templates::Entity::insert(wiki_templates::ActiveModel {
        id: Set(id.clone()),
        wiki_id: Set(input.wiki_id),
        name: Set(input.name),
        description: Set(input.description),
        content: Set(input.content),
        page_type: Set(input.page_type),
        is_builtin: Set(input.is_builtin),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .exec(db)
    .await?;

    get_wiki_template(db, &id).await
}

pub async fn get_wiki_template(db: &DatabaseConnection, id: &str) -> Result<WikiTemplate> {
    let model = wiki_templates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("WikiTemplate {} not found", id)))?;

    Ok(model_to_template(model))
}

/// 更新 Wiki 模板（部分更新：只覆盖传入的字段）。
///
/// `is_builtin` 不在输入中 —— 内置标记不允许经 update 翻转；
/// 删除保护见 [`delete_wiki_template`]。
pub async fn update_wiki_template(
    db: &DatabaseConnection,
    input: UpdateWikiTemplateInput,
) -> Result<WikiTemplate> {
    let existing = wiki_templates::Entity::find_by_id(&input.id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("WikiTemplate {} not found", input.id)))?;

    let mut am: wiki_templates::ActiveModel = existing.into();
    if let Some(name) = input.name {
        am.name = Set(name);
    }
    if let Some(description) = input.description {
        am.description = Set(Some(description));
    }
    if let Some(content) = input.content {
        am.content = Set(content);
    }
    if let Some(page_type) = input.page_type {
        am.page_type = Set(Some(page_type));
    }
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    get_wiki_template(db, &input.id).await
}

pub async fn delete_wiki_template(db: &DatabaseConnection, id: &str) -> Result<()> {
    // is_builtin 保护：内置模板（每日笔记等系统依赖）不可删除
    let existing = wiki_templates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("WikiTemplate {} not found", id)))?;
    if existing.is_builtin {
        return Err(AxAgentError::Validation(format!("内置模板不可删除: {}", existing.name)));
    }

    wiki_templates::Entity::delete_by_id(id).exec(db).await?;

    Ok(())
}

/// Wiki 操作历史条目。参数打包为结构体，避免 `log_wiki_operation`
/// 参数过多触发 clippy::too_many_arguments。
pub struct WikiOperationEntry {
    pub wiki_id: String,
    pub operation_type: String,
    pub target_type: String,
    pub target_id: String,
    pub status: String,
    pub details: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

/// 记录一条 Wiki 操作历史。
///
/// 操作历史全覆盖：此前唯一写入点是 compile（target_id 用随机 id 无意义），
/// ingest / lint / sync / restore 等路径统一走本函数，target_id 传真实对象 ID
/// （note_id / source_id），操作时间线才可追溯。失败由调用方 best-effort 处理
/// （记日志不打断主流程）。
pub async fn log_wiki_operation(db: &DatabaseConnection, entry: WikiOperationEntry) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    wiki_operations::ActiveModel {
        wiki_id: Set(entry.wiki_id),
        operation_type: Set(entry.operation_type),
        target_type: Set(entry.target_type),
        target_id: Set(entry.target_id),
        status: Set(entry.status),
        details_json: Set(entry.details),
        error_message: Set(entry.error_message),
        created_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub fn apply_template_variables(content: &str, wiki_name: &str) -> String {
    let now = chrono::Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let title = "";

    content
        .replace("{{date}}", &date)
        .replace("{{title}}", title)
        .replace("{{tags}}", "")
        .replace("{{wiki_name}}", wiki_name)
}

/// v110: 检查 Wiki 中是否已存在对应知识库文档的笔记。
///
/// 通过 source_refs 字段查找是否已有引用该文档的笔记。
/// 用于 KB→Wiki 自动同步时避免重复创建。
pub async fn note_exists_for_document(
    db: &DatabaseConnection,
    _vault_id: &str,
    doc_id: &str,
) -> Result<bool> {
    let pattern = format!("%:doc:{}%", doc_id);
    let sql = "SELECT COUNT(*) FROM notes WHERE source_refs LIKE ?1 AND is_deleted = 0";
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            vec![pattern.into()],
        ))
        .await?;

    Ok(match row {
        Some(r) => r.try_get::<i64>("", "COUNT(*)").unwrap_or(0) > 0,
        None => false,
    })
}

// ── 知识源管理 DAO（docs/knowledge-source-ingest-plan.md P2）────────────────
// wiki_sources 从「来源登记表」升级为「知识源实体」，支撑 knowledge_source_* 命令。

fn model_to_source(m: wiki_sources::Model) -> axagent_harness::wiki_dtos::WikiSource {
    axagent_harness::wiki_dtos::WikiSource {
        id: m.id,
        wiki_id: m.wiki_id,
        source_type: m.source_type,
        source_path: m.source_path,
        title: m.title,
        mime_type: m.mime_type,
        size_bytes: m.size_bytes,
        content_hash: m.content_hash,
        metadata_json: m.metadata_json,
        schedule_cron: m.schedule_cron,
        last_fetched_at: m.last_fetched_at,
        status: m.status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// 列出全部知识源（跨 wiki），按创建时间倒序。
pub async fn list_all_sources(
    db: &DatabaseConnection,
) -> Result<Vec<axagent_harness::wiki_dtos::WikiSource>> {
    let models =
        wiki_sources::Entity::find().order_by_desc(wiki_sources::Column::CreatedAt).all(db).await?;
    Ok(models.into_iter().map(model_to_source).collect())
}

/// 按 ID 查单个知识源。
pub async fn get_source_by_id(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<axagent_harness::wiki_dtos::WikiSource>> {
    let model = wiki_sources::Entity::find_by_id(id).one(db).await?;
    Ok(model.map(model_to_source))
}

/// 知识源可编辑字段集合（避免 update_source_fields 参数超限）。
#[derive(Debug, Clone, Default)]
pub struct UpdateSourceFieldsInput {
    pub title: Option<String>,
    pub source_type: Option<String>,
    pub source_path: Option<String>,
    pub schedule_cron: Option<Option<String>>,
    pub status: Option<String>,
    pub metadata_json: Option<Option<serde_json::Value>>,
}

/// 更新知识源的可编辑字段（标题/类型/路径/调度/状态/配置）。
pub async fn update_source_fields(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateSourceFieldsInput,
) -> Result<()> {
    let mut am: wiki_sources::ActiveModel =
        match wiki_sources::Entity::find_by_id(id).one(db).await? {
            Some(m) => m.into(),
            None => {
                return Err(AxAgentError::NotFound(format!("知识源 {id} 不存在")));
            },
        };
    if let Some(v) = input.title {
        am.title = Set(v);
    }
    if let Some(v) = input.source_type {
        am.source_type = Set(v);
    }
    if let Some(v) = input.source_path {
        am.source_path = Set(v);
    }
    if let Some(v) = input.schedule_cron {
        am.schedule_cron = Set(v);
    }
    if let Some(v) = input.status {
        am.status = Set(v);
    }
    if let Some(v) = input.metadata_json {
        am.metadata_json = Set(v);
    }
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;
    Ok(())
}

/// 抓取完成后更新指纹与时间戳（增量更新闭环核心）。
pub async fn update_source_fetch_meta(
    db: &DatabaseConnection,
    id: &str,
    content_hash: &str,
    last_fetched_at: i64,
) -> Result<()> {
    let mut am: wiki_sources::ActiveModel =
        match wiki_sources::Entity::find_by_id(id).one(db).await? {
            Some(m) => m.into(),
            None => {
                return Err(AxAgentError::NotFound(format!("知识源 {id} 不存在")));
            },
        };
    am.content_hash = Set(content_hash.to_string());
    am.last_fetched_at = Set(Some(last_fetched_at));
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;
    Ok(())
}

/// 删除知识源，返回是否删除了行。
pub async fn delete_source_by_id(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let result = wiki_sources::Entity::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}
