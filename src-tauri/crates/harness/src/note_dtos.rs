// SPDX-License-Identifier: AGPL-3.0-only

//! Note DTOs — pure types migrated from dao.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub vault_id: String,
    pub title: String,
    pub file_path: String,
    pub content: String,
    pub content_hash: String,
    pub author: String,
    pub page_type: Option<String>,
    pub source_refs: Option<Vec<String>>,
    pub related_pages: Option<Vec<String>>,
    pub quality_score: Option<f64>,
    pub last_linted_at: Option<i64>,
    pub last_compiled_at: Option<i64>,
    pub compiled_source_hash: Option<String>,
    pub user_edited: bool,
    pub user_edited_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub vault_id: String,
    pub title: String,
    pub file_path: String,
    pub content: String,
    pub author: String,
    pub page_type: Option<String>,
    pub source_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub page_type: Option<String>,
    pub related_pages: Option<Vec<String>>,
}

pub fn calculate_content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// ── Bridge: note_dtos::Note → rag_config::Note ──

impl From<Note> for crate::rag_config::Note {
    fn from(n: Note) -> Self {
        Self {
            id: n.id,
            vault_id: n.vault_id,
            title: n.title,
            file_path: n.file_path,
            content: n.content,
            content_hash: n.content_hash,
            author: n.author,
            page_type: n.page_type,
            source_refs: n.source_refs,
            related_pages: n.related_pages,
            quality_score: n.quality_score,
            last_linted_at: n.last_linted_at,
            last_compiled_at: n.last_compiled_at,
            compiled_source_hash: n.compiled_source_hash,
            user_edited: n.user_edited,
            user_edited_at: n.user_edited_at,
            created_at: n.created_at,
            updated_at: n.updated_at,
            is_deleted: n.is_deleted,
        }
    }
}
