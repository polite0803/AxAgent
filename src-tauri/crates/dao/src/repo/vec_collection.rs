// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::vec_collections;
pub use axagent_entities::vec_collections::{Column, Entity, INDEX_TYPE_FLAT, INDEX_TYPE_HNSW};
use axagent_harness::core_error::{AxAgentError, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VecCollection {
    pub collection_id: String,
    pub dimensions: i32,
    pub embedding_model: Option<String>,
    pub index_type: String,
    pub hnsw_ef_construction: Option<i32>,
    pub hnsw_m: Option<i32>,
    pub hnsw_ef_search: Option<i32>,
    pub vector_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_indexed_at: Option<i64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateVecCollectionInput {
    pub collection_id: String,
    pub dimensions: i32,
    pub embedding_model: Option<String>,
    pub index_type: Option<String>,
    pub hnsw_ef_construction: Option<i32>,
    pub hnsw_m: Option<i32>,
    pub hnsw_ef_search: Option<i32>,
    pub metadata: Option<String>,
}

fn model_to_collection(m: vec_collections::Model) -> VecCollection {
    VecCollection {
        collection_id: m.collection_id,
        dimensions: m.dimensions,
        embedding_model: m.embedding_model,
        index_type: m.index_type,
        hnsw_ef_construction: m.hnsw_ef_construction,
        hnsw_m: m.hnsw_m,
        hnsw_ef_search: m.hnsw_ef_search,
        vector_count: m.vector_count,
        created_at: m.created_at,
        updated_at: m.updated_at,
        last_indexed_at: m.last_indexed_at,
        metadata: m.metadata,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn create_collection(
    db: &DatabaseConnection,
    input: CreateVecCollectionInput,
) -> Result<VecCollection> {
    let now = now_ms();
    let am = vec_collections::ActiveModel {
        collection_id: Set(input.collection_id.clone()),
        dimensions: Set(input.dimensions),
        embedding_model: Set(input.embedding_model),
        index_type: Set(input
            .index_type
            .unwrap_or_else(|| INDEX_TYPE_FLAT.to_string())),
        hnsw_ef_construction: Set(input.hnsw_ef_construction),
        hnsw_m: Set(input.hnsw_m),
        hnsw_ef_search: Set(input.hnsw_ef_search),
        vector_count: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        last_indexed_at: Set(None),
        metadata: Set(input.metadata),
    };

    am.insert(db).await?;
    get_collection(db, &input.collection_id).await
}

pub async fn get_collection(db: &DatabaseConnection, collection_id: &str) -> Result<VecCollection> {
    let model = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("VecCollection {}", collection_id)))?;
    Ok(model_to_collection(model))
}

pub async fn find_collection(
    db: &DatabaseConnection,
    collection_id: &str,
) -> Result<Option<VecCollection>> {
    let model = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?;
    Ok(model.map(model_to_collection))
}

pub async fn get_collection_dimensions(
    db: &DatabaseConnection,
    collection_id: &str,
) -> Result<Option<i32>> {
    let model = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?;
    Ok(model.map(|m| m.dimensions))
}

pub async fn list_collections(db: &DatabaseConnection) -> Result<Vec<VecCollection>> {
    let models = vec_collections::Entity::find()
        .order_by_desc(vec_collections::Column::UpdatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_collection).collect())
}

pub async fn update_collection_embedding_model(
    db: &DatabaseConnection,
    collection_id: &str,
    model_name: Option<&str>,
) -> Result<()> {
    let existing = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("VecCollection {}", collection_id)))?;

    let mut am: vec_collections::ActiveModel = existing.into();
    am.embedding_model = Set(model_name.map(|s| s.to_string()));
    am.updated_at = Set(now_ms());
    am.update(db).await?;
    Ok(())
}

pub async fn increment_vector_count(
    db: &DatabaseConnection,
    collection_id: &str,
    delta: i64,
) -> Result<()> {
    let existing = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("VecCollection {}", collection_id)))?;

    let new_count = std::cmp::max(existing.vector_count + delta, 0);
    let mut am: vec_collections::ActiveModel = existing.into();
    am.vector_count = Set(new_count);
    am.updated_at = Set(now_ms());
    am.update(db).await?;
    Ok(())
}

pub async fn set_vector_count(
    db: &DatabaseConnection,
    collection_id: &str,
    count: i64,
) -> Result<()> {
    let existing = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("VecCollection {}", collection_id)))?;

    let mut am: vec_collections::ActiveModel = existing.into();
    am.vector_count = Set(std::cmp::max(count, 0));
    am.updated_at = Set(now_ms());
    am.update(db).await?;
    Ok(())
}

pub async fn mark_indexed(db: &DatabaseConnection, collection_id: &str) -> Result<()> {
    let existing = vec_collections::Entity::find_by_id(collection_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("VecCollection {}", collection_id)))?;

    let mut am: vec_collections::ActiveModel = existing.into();
    am.last_indexed_at = Set(Some(now_ms()));
    am.updated_at = Set(now_ms());
    am.update(db).await?;
    Ok(())
}

pub async fn delete_collection(db: &DatabaseConnection, collection_id: &str) -> Result<()> {
    let result = vec_collections::Entity::delete_by_id(collection_id)
        .exec(db)
        .await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("VecCollection {}", collection_id)));
    }
    Ok(())
}

pub async fn collection_exists(db: &DatabaseConnection, collection_id: &str) -> Result<bool> {
    let count = vec_collections::Entity::find_by_id(collection_id)
        .count(db)
        .await?;
    Ok(count > 0)
}

pub async fn upsert_collection(
    db: &DatabaseConnection,
    input: CreateVecCollectionInput,
) -> Result<VecCollection> {
    if collection_exists(db, &input.collection_id).await? {
        let existing = get_collection(db, &input.collection_id).await?;
        if existing.dimensions != input.dimensions {
            return Err(AxAgentError::Validation(format!(
                "Dimension mismatch for collection {}: existing={}, requested={}",
                input.collection_id, existing.dimensions, input.dimensions
            )));
        }

        let model = vec_collections::Entity::find_by_id(&input.collection_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                AxAgentError::NotFound(format!("VecCollection {}", input.collection_id))
            })?;

        let mut am: vec_collections::ActiveModel = model.into();
        if let Some(m) = input.embedding_model {
            am.embedding_model = Set(Some(m));
        }
        if let Some(idx_type) = input.index_type {
            am.index_type = Set(idx_type);
        }
        if input.hnsw_ef_construction.is_some() {
            am.hnsw_ef_construction = Set(input.hnsw_ef_construction);
        }
        if input.hnsw_m.is_some() {
            am.hnsw_m = Set(input.hnsw_m);
        }
        if input.hnsw_ef_search.is_some() {
            am.hnsw_ef_search = Set(input.hnsw_ef_search);
        }
        am.updated_at = Set(now_ms());
        am.update(db).await?;
        get_collection(db, &input.collection_id).await
    } else {
        create_collection(db, input).await
    }
}
