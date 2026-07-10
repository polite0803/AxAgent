// SPDX-License-Identifier: AGPL-3.0-only

//! Wiki entity DTOs and repository traits — pure data & contract layer.
//!
//! DTOs (Wiki / Note / NoteLink) already defined elsewhere in harness:
//!   - Wiki:     `harness::types::Wiki`
//!   - Note:     `harness::note_dtos::Note`
//!   - NoteLink: `harness::types::NoteLink`
//!
//! This module adds the remaining DTOs (WikiSource, WikiPage, WikiOperation,
//! NoteBacklink) and repository traits for all wiki-related entities.

use serde::{Deserialize, Serialize};

use crate::note_dtos::Note;
use crate::types::Wiki;

// ── WikiSource DTO ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiSource {
    pub id: String,
    pub wiki_id: String,
    pub source_type: String,
    pub source_path: String,
    pub title: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub content_hash: String,
    pub metadata_json: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input for inserting a new WikiSource.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertWikiSourceInput {
    pub id: String,
    pub wiki_id: String,
    pub source_type: String,
    pub source_path: String,
    pub title: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub content_hash: String,
    pub metadata_json: Option<serde_json::Value>,
}

// ── WikiPage DTO ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiPage {
    pub id: String,
    pub wiki_id: String,
    pub note_id: String,
    pub page_type: String,
    pub title: String,
    pub source_ids: Option<serde_json::Value>,
    pub quality_score: Option<f64>,
    pub last_linted_at: Option<i64>,
    pub last_compiled_at: i64,
    pub compiled_source_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ── WikiOperation DTO ───────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiOperation {
    pub id: i64,
    pub wiki_id: String,
    pub operation_type: String,
    pub target_type: String,
    pub target_id: String,
    pub status: String,
    pub details_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

// ── NoteBacklink DTO ────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteBacklink {
    pub id: i64,
    pub vault_id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub link_text: String,
    pub link_type: String,
    pub created_at: i64,
}

// ── WikiRepository trait ────────────────────────────────────────

/// Version record for a wiki page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[async_trait::async_trait]
pub trait WikiRepository: Send + Sync {
    /// Find a wiki by its ID.
    async fn find_by_id(&self, wiki_id: &str) -> Result<Option<Wiki>, String>;

    /// Create a version record for a wiki page.
    async fn create_version(
        &self,
        wiki_id: &str,
        note_id: &str,
        title: &str,
        content: &str,
        author: &str,
    ) -> Result<NoteVersion, String>;

    /// Increment the note count for a wiki.
    async fn increment_note_count(&self, wiki_id: &str) -> Result<(), String>;

    /// Update the schema version for a wiki.
    async fn update_schema_version(&self, wiki_id: &str, version: &str) -> Result<(), String>;
}

// ── WikiSourceRepository trait ──────────────────────────────────

#[async_trait::async_trait]
pub trait WikiSourceRepository: Send + Sync {
    /// Insert a new wiki source and return the created record.
    async fn insert(&self, input: InsertWikiSourceInput) -> Result<WikiSource, String>;

    /// Find all sources belonging to a wiki.
    async fn find_by_wiki_id(&self, wiki_id: &str) -> Result<Vec<WikiSource>, String>;

    /// Count sources for a wiki.
    async fn count_by_wiki_id(&self, wiki_id: &str) -> Result<usize, String>;

    /// Find a single source by its ID.
    async fn find_by_id(&self, source_id: &str) -> Result<Option<WikiSource>, String>;
}

// ── WikiPageRepository trait ────────────────────────────────────

#[async_trait::async_trait]
pub trait WikiPageRepository: Send + Sync {
    /// Find a wiki page by its associated note ID.
    async fn find_by_note_id(&self, note_id: &str) -> Result<Option<WikiPage>, String>;

    /// Upsert a wiki page (insert if not exists, update if exists).
    async fn upsert(&self, page: WikiPage) -> Result<(), String>;

    /// Update lint result fields (quality_score, last_linted_at) for the wiki page associated with a note_id.
    async fn update_lint_result(
        &self,
        note_id: &str,
        quality_score: Option<f64>,
        last_linted_at: Option<i64>,
    ) -> Result<(), String>;

    /// Find all wiki pages belonging to a wiki.
    async fn find_by_wiki_id(&self, wiki_id: &str) -> Result<Vec<WikiPage>, String>;
}

// ── WikiOperationRepository trait ───────────────────────────────

#[async_trait::async_trait]
pub trait WikiOperationRepository: Send + Sync {
    /// Log a new wiki operation.
    async fn log(&self, operation: WikiOperation) -> Result<(), String>;
}

// ── NoteRepository trait ────────────────────────────────────────

#[async_trait::async_trait]
pub trait NoteRepository: Send + Sync {
    /// Find a single note by ID.
    async fn find_by_id(&self, note_id: &str) -> Result<Option<Note>, String>;

    /// Find notes in a vault (wiki) by title, optionally including deleted.
    async fn find_by_vault_and_title(
        &self,
        vault_id: &str,
        title: &str,
        include_deleted: bool,
    ) -> Result<Vec<Note>, String>;

    /// Find all notes in a vault (wiki), optionally including deleted.
    async fn find_by_vault(
        &self,
        vault_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Note>, String>;

    /// Update a note by ID.
    async fn update_note(
        &self,
        note_id: &str,
        input: crate::note_dtos::UpdateNoteInput,
    ) -> Result<Note, String>;

    /// Create a new note.
    async fn create_note(&self, input: crate::note_dtos::CreateNoteInput) -> Result<Note, String>;

    /// Find forward link target IDs for a given source note.
    async fn find_link_target_ids(&self, note_id: &str) -> Result<Vec<String>, String>;
}

// ── NoteBacklinkRepository trait ─────────────────────────────

#[async_trait::async_trait]
pub trait NoteBacklinkRepository: Send + Sync {
    /// Count backlinks pointing to a note.
    async fn count_by_target_note_id(&self, note_id: &str) -> Result<usize, String>;

    /// Find backlinks by target note ID.
    async fn find_by_target_note_id(&self, note_id: &str) -> Result<Vec<NoteBacklink>, String>;
}
