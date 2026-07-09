// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of WikiPageRepository using SeaORM.

use std::sync::Arc;

use axagent_entities::wiki_pages;
use axagent_harness::wiki_dtos::{WikiPage, WikiPageRepository};
use sea_orm::*;

/// DAO implementation of WikiPageRepository.
pub struct DaoWikiPageRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoWikiPageRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn model_to_wiki_page(m: wiki_pages::Model) -> WikiPage {
        WikiPage {
            id: m.id,
            wiki_id: m.wiki_id,
            note_id: m.note_id,
            page_type: m.page_type,
            title: m.title,
            source_ids: m.source_ids,
            quality_score: m.quality_score,
            last_linted_at: m.last_linted_at,
            last_compiled_at: m.last_compiled_at,
            compiled_source_hash: m.compiled_source_hash,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[async_trait::async_trait]
impl WikiPageRepository for DaoWikiPageRepository {
    async fn find_by_note_id(&self, note_id: &str) -> Result<Option<WikiPage>, String> {
        let model = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(note_id.to_string()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(model.map(|m| Self::model_to_wiki_page(m)))
    }

    async fn upsert(&self, page: WikiPage) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();

        let existing = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(&page.note_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        if let Some(model) = existing {
            let mut am = model.into_active_model();
            am.wiki_id = Set(page.wiki_id);
            am.page_type = Set(page.page_type);
            am.title = Set(page.title);
            am.source_ids = Set(page.source_ids);
            am.quality_score = Set(page.quality_score);
            am.last_linted_at = Set(page.last_linted_at);
            am.last_compiled_at = Set(page.last_compiled_at);
            am.compiled_source_hash = Set(page.compiled_source_hash);
            am.updated_at = Set(now);
            am.update(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;
        } else {
            let am = wiki_pages::ActiveModel {
                id: Set(page.id),
                wiki_id: Set(page.wiki_id),
                note_id: Set(page.note_id),
                page_type: Set(page.page_type),
                title: Set(page.title),
                source_ids: Set(page.source_ids),
                quality_score: Set(page.quality_score),
                last_linted_at: Set(page.last_linted_at),
                last_compiled_at: Set(page.last_compiled_at),
                compiled_source_hash: Set(page.compiled_source_hash),
                created_at: Set(now),
                updated_at: Set(now),
            };
            am.insert(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;
        }

        Ok(())
    }

    async fn update_lint_result(
        &self,
        note_id: &str,
        quality_score: Option<f64>,
        last_linted_at: Option<i64>,
    ) -> Result<(), String> {
        let model = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(note_id.to_string()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        if let Some(m) = model {
            let mut am = m.into_active_model();
            am.quality_score = Set(quality_score);
            am.last_linted_at = Set(last_linted_at);
            am.update(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;
        }

        Ok(())
    }

    async fn find_by_wiki_id(&self, wiki_id: &str) -> Result<Vec<WikiPage>, String> {
        let models = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::WikiId.eq(wiki_id.to_string()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(models.into_iter().map(|m| Self::model_to_wiki_page(m)).collect())
    }
}
