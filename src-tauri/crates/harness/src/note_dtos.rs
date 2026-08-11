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
    pub tags: Vec<String>,
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
