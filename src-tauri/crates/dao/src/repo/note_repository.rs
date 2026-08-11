// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of NoteRepository using SeaORM.

use std::sync::Arc;

use axagent_entities::notes;
use axagent_harness::note_dtos::{CreateNoteInput, Note, UpdateNoteInput};
use axagent_harness::wiki_dtos::NoteRepository;
use sea_orm::*;

/// DAO implementation of NoteRepository.
pub struct DaoNoteRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoNoteRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn model_to_note(m: notes::Model) -> Note {
        // 解析 tags JSON 数组
        let tags: Vec<String> =
            m.tags.and_then(|json| serde_json::from_value(json).ok()).unwrap_or_default();

        Note {
            id: m.id,
            vault_id: m.vault_id,
            title: m.title,
            file_path: m.file_path,
            content: m.content,
            content_hash: m.content_hash,
            author: m.author,
            page_type: m.page_type,
            tags,
            source_refs: m.source_refs.map(|j| serde_json::from_value(j).unwrap_or_default()),
            related_pages: m.related_pages.map(|j| serde_json::from_value(j).unwrap_or_default()),
            quality_score: m.quality_score,
            last_linted_at: m.last_linted_at,
            last_compiled_at: m.last_compiled_at,
            compiled_source_hash: m.compiled_source_hash,
            user_edited: m.user_edited != 0,
            user_edited_at: m.user_edited_at,
            created_at: m.created_at,
            updated_at: m.updated_at,
            is_deleted: m.is_deleted != 0,
        }
    }
}

#[async_trait::async_trait]
impl NoteRepository for DaoNoteRepository {
    async fn find_by_id(&self, note_id: &str) -> Result<Option<Note>, String> {
        let model = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(model.map(Self::model_to_note))
    }

    async fn find_by_vault_and_title(
        &self,
        vault_id: &str,
        title: &str,
        include_deleted: bool,
    ) -> Result<Vec<Note>, String> {
        let mut query = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(vault_id.to_string()))
            .filter(notes::Column::Title.eq(title.to_string()));
        if !include_deleted {
            query = query.filter(notes::Column::IsDeleted.eq(0));
        }
        let models = query.all(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(models.into_iter().map(Self::model_to_note).collect())
    }

    async fn find_by_vault(
        &self,
        vault_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Note>, String> {
        let mut query =
            notes::Entity::find().filter(notes::Column::VaultId.eq(vault_id.to_string()));
        if !include_deleted {
            query = query.filter(notes::Column::IsDeleted.eq(0));
        }
        let models = query.all(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(models.into_iter().map(Self::model_to_note).collect())
    }

    async fn update_note(&self, note_id: &str, input: UpdateNoteInput) -> Result<Note, String> {
        let model = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Note {} not found", note_id))?;

        let mut am = model.into_active_model();

        if let Some(title) = input.title {
            am.title = Set(title);
        }
        if let Some(content) = input.content {
            let new_hash = axagent_harness::note_dtos::calculate_content_hash(&content);
            am.content = Set(content);
            am.content_hash = Set(new_hash);
        }
        if let Some(page_type) = input.page_type {
            am.page_type = Set(Some(page_type));
        }
        if let Some(related_pages) = input.related_pages {
            am.related_pages = Set(Some(serde_json::to_value(related_pages).unwrap_or_default()));
        }
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.user_edited = Set(1);
        am.user_edited_at = Set(Some(chrono::Utc::now().timestamp()));

        let updated = am.update(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(Self::model_to_note(updated))
    }

    async fn create_note(&self, input: CreateNoteInput) -> Result<Note, String> {
        let now = chrono::Utc::now().timestamp();
        let id = axagent_harness::util_fns::gen_id();
        let content_hash = axagent_harness::note_dtos::calculate_content_hash(&input.content);

        // 从内容中提取 tags
        let tags = super::note::extract_tags_from_content(&input.content);
        let tags_json = serde_json::to_value(tags).unwrap_or_default();

        let am = notes::ActiveModel {
            id: Set(id.clone()),
            vault_id: Set(input.vault_id),
            title: Set(input.title),
            file_path: Set(input.file_path),
            content: Set(input.content),
            content_hash: Set(content_hash),
            author: Set(input.author),
            page_type: Set(input.page_type),
            tags: Set(Some(tags_json)),
            source_refs: Set(input
                .source_refs
                .map(|v| serde_json::to_value(v).unwrap_or_default())),
            related_pages: Set(None),
            quality_score: Set(None),
            last_linted_at: Set(None),
            last_compiled_at: Set(None),
            compiled_source_hash: Set(None),
            user_edited: Set(0),
            user_edited_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            is_deleted: Set(0),
        };

        let model = am.insert(self.db.as_ref()).await.map_err(|e| format!("DB error: {}", e))?;

        Ok(Self::model_to_note(model))
    }

    async fn find_link_target_ids(&self, note_id: &str) -> Result<Vec<String>, String> {
        use axagent_entities::note_links;

        let links = note_links::Entity::find()
            .filter(note_links::Column::SourceNoteId.eq(note_id.to_string()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;
        Ok(links.into_iter().map(|l| l.target_note_id).collect())
    }
}
