// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::repo::note::calculate_content_hash;
use axagent_entities::{wiki_page_versions, wiki_templates, wikis};
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
    let models = wikis::Entity::find()
        .order_by(wikis::Column::UpdatedAt, Order::Desc)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_wiki).collect())
}

pub async fn update_wiki(
    db: &DatabaseConnection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    embedding_provider: Option<String>,
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
    am.updated_at = Set(chrono::Utc::now().timestamp());

    am.update(db).await?;

    get_wiki(db, id).await
}

pub async fn delete_wiki(db: &DatabaseConnection, id: &str) -> Result<()> {
    let model = wikis::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", id)))?;

    let mut am = model.into_active_model();
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    Ok(())
}

pub async fn increment_note_count(db: &DatabaseConnection, wiki_id: &str) -> Result<()> {
    let model = wikis::Entity::find_by_id(wiki_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", wiki_id)))?;

    let mut am = model.clone().into_active_model();
    am.note_count = Set(model.note_count + 1);
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    Ok(())
}

pub async fn increment_source_count(db: &DatabaseConnection, wiki_id: &str) -> Result<()> {
    let model = wikis::Entity::find_by_id(wiki_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", wiki_id)))?;

    let mut am = model.clone().into_active_model();
    am.source_count = Set(model.source_count + 1);
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: i64,
    pub wiki_id: String,
    pub note_id: String,
    pub title: String,
    pub content: String,
    pub content_hash: String,
    pub author: String,
    pub created_at: i64,
}

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
        wiki_page_versions::Entity::delete_by_id(id)
            .exec(db)
            .await?;
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

pub async fn delete_wiki_template(db: &DatabaseConnection, id: &str) -> Result<()> {
    wiki_templates::Entity::delete_by_id(id).exec(db).await?;

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
