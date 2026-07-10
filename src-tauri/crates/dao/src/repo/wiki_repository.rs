// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use axagent_entities::{wiki_page_versions, wikis};
use axagent_harness::types::Wiki;
use axagent_harness::wiki_dtos::{NoteVersion, WikiRepository};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};

/// DAO implementation of WikiRepository using SeaORM.
pub struct DaoWikiRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoWikiRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
}

#[async_trait::async_trait]
impl WikiRepository for DaoWikiRepository {
    async fn find_by_id(&self, wiki_id: &str) -> Result<Option<Wiki>, String> {
        let model = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(model.map(|m| Self::model_to_wiki(m)))
    }

    async fn create_version(
        &self,
        wiki_id: &str,
        note_id: &str,
        title: &str,
        content: &str,
        author: &str,
    ) -> Result<NoteVersion, String> {
        let now = chrono::Utc::now().timestamp();
        let content_hash = axagent_harness::note_dtos::calculate_content_hash(content);

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

        let model = am.insert(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(NoteVersion {
            id: model.id,
            wiki_id: model.wiki_id,
            note_id: model.note_id,
            title: model.title,
            content: model.content,
            content_hash: model.content_hash,
            author: model.author,
            created_at: model.created_at,
        })
    }

    async fn increment_note_count(&self, wiki_id: &str) -> Result<(), String> {
        let model = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let new_count = model.note_count + 1;
        let mut am = model.into_active_model();
        am.note_count = Set(new_count);
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.update(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(())
    }

    async fn update_schema_version(&self, wiki_id: &str, version: &str) -> Result<(), String> {
        let model = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let mut am = model.into_active_model();
        am.schema_version = Set(version.to_string());
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.update(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;
        Ok(())
    }
}
