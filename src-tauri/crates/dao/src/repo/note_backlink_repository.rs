// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of NoteBacklinkRepository using SeaORM.

use std::collections::HashMap;
use std::sync::Arc;

use axagent_entities::note_backlinks;
use axagent_harness::wiki_dtos::NoteBacklinkRepository;
use sea_orm::*;

/// DAO implementation of NoteBacklinkRepository.
pub struct DaoNoteBacklinkRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoNoteBacklinkRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl NoteBacklinkRepository for DaoNoteBacklinkRepository {
    async fn count_by_target_note_id(&self, note_id: &str) -> Result<usize, String> {
        let count = note_backlinks::Entity::find()
            .filter(note_backlinks::Column::TargetNoteId.eq(note_id.to_string()))
            .count(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(count as usize)
    }

    async fn batch_count_by_target_note_ids(
        &self,
        note_ids: &[String],
    ) -> Result<HashMap<String, i64>, String> {
        if note_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // 批量查询所有 backlink 计数，在数据库端用 GROUP BY 聚合，避免 N+1 查询
        let results = note_backlinks::Entity::find()
            .select_only()
            .column(note_backlinks::Column::TargetNoteId)
            .column_as(note_backlinks::Column::SourceNoteId.count(), "cnt")
            .filter(note_backlinks::Column::TargetNoteId.is_in(note_ids.to_vec()))
            .group_by(note_backlinks::Column::TargetNoteId)
            .into_tuple::<(String, i64)>()
            .all(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        let mut map = HashMap::with_capacity(note_ids.len());
        for (target_id, count) in results {
            map.insert(target_id, count);
        }
        Ok(map)
    }

    async fn find_by_target_note_id(
        &self,
        note_id: &str,
    ) -> Result<Vec<axagent_harness::wiki_dtos::NoteBacklink>, String> {
        let models = note_backlinks::Entity::find()
            .filter(note_backlinks::Column::TargetNoteId.eq(note_id.to_string()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?;
        Ok(models
            .into_iter()
            .map(|m| axagent_harness::wiki_dtos::NoteBacklink {
                id: m.id,
                vault_id: m.vault_id,
                source_note_id: m.source_note_id,
                target_note_id: m.target_note_id,
                link_text: m.link_text,
                link_type: m.link_type,
                created_at: m.created_at,
            })
            .collect())
    }
}
