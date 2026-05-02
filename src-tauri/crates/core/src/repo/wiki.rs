use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::entity::wikis;
use crate::error::{AxAgentError, Result};
use crate::utils::gen_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wiki {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: String,
    pub schema_version: String,
    pub note_count: i32,
    pub source_count: i32,
    pub embedding_provider: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

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
