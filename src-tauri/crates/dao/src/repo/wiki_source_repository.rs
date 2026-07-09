// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use axagent_entities::wiki_sources;
use axagent_harness::wiki_dtos::{InsertWikiSourceInput, WikiSource, WikiSourceRepository};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

pub struct DaoWikiSourceRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoWikiSourceRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn model_to_dto(m: wiki_sources::Model) -> WikiSource {
    WikiSource {
        id: m.id,
        wiki_id: m.wiki_id,
        source_type: m.source_type,
        source_path: m.source_path,
        title: m.title,
        mime_type: m.mime_type,
        size_bytes: m.size_bytes,
        content_hash: m.content_hash,
        metadata_json: m.metadata_json,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

#[async_trait::async_trait]
impl WikiSourceRepository for DaoWikiSourceRepository {
    async fn insert(&self, input: InsertWikiSourceInput) -> Result<WikiSource, String> {
        let now = chrono::Utc::now().timestamp();

        let am = wiki_sources::ActiveModel {
            id: Set(input.id),
            wiki_id: Set(input.wiki_id),
            source_type: Set(input.source_type),
            source_path: Set(input.source_path),
            title: Set(input.title),
            mime_type: Set(input.mime_type),
            size_bytes: Set(input.size_bytes),
            content_hash: Set(input.content_hash),
            metadata_json: Set(input.metadata_json),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        Ok(model_to_dto(model))
    }

    async fn find_by_wiki_id(&self, wiki_id: &str) -> Result<Vec<WikiSource>, String> {
        let models = wiki_sources::Entity::find()
            .filter(wiki_sources::Column::WikiId.eq(wiki_id))
            .order_by_asc(wiki_sources::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(models.into_iter().map(model_to_dto).collect())
    }

    async fn count_by_wiki_id(&self, wiki_id: &str) -> Result<usize, String> {
        let count = wiki_sources::Entity::find()
            .filter(wiki_sources::Column::WikiId.eq(wiki_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(count as usize)
    }

    async fn find_by_id(&self, source_id: &str) -> Result<Option<WikiSource>, String> {
        let model = wiki_sources::Entity::find_by_id(source_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(model.map(model_to_dto))
    }
}
