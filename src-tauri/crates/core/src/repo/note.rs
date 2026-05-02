use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::entity::{note_backlinks, note_links, notes};
use crate::error::{AxAgentError, Result};
use crate::utils::gen_id;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLink {
    pub id: i64,
    pub vault_id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub link_text: String,
    pub link_type: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSearchResult {
    pub note: Note,
    pub snippet: String,
    pub score: f64,
}

pub fn model_to_note(m: notes::Model) -> Note {
    Note {
        id: m.id,
        vault_id: m.vault_id,
        title: m.title,
        file_path: m.file_path,
        content: m.content,
        content_hash: m.content_hash,
        author: m.author,
        page_type: m.page_type,
        source_refs: m
            .source_refs
            .map(|j| serde_json::from_value(j).unwrap_or_default()),
        related_pages: m
            .related_pages
            .map(|j| serde_json::from_value(j).unwrap_or_default()),
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

fn model_to_link(m: note_links::Model) -> NoteLink {
    NoteLink {
        id: m.id,
        vault_id: m.vault_id,
        source_note_id: m.source_note_id,
        target_note_id: m.target_note_id,
        link_text: m.link_text,
        link_type: m.link_type,
        created_at: m.created_at,
    }
}

pub async fn list_notes(db: &DatabaseConnection, vault_id: &str) -> Result<Vec<Note>> {
    let models = notes::Entity::find()
        .filter(notes::Column::VaultId.eq(vault_id))
        .filter(notes::Column::IsDeleted.eq(0))
        .order_by_asc(notes::Column::Title)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_note).collect())
}

pub async fn get_note(db: &DatabaseConnection, id: &str) -> Result<Note> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    Ok(model_to_note(model))
}

pub async fn get_note_by_path(
    db: &DatabaseConnection,
    vault_id: &str,
    file_path: &str,
) -> Result<Note> {
    let model = notes::Entity::find()
        .filter(notes::Column::VaultId.eq(vault_id))
        .filter(notes::Column::FilePath.eq(file_path))
        .filter(notes::Column::IsDeleted.eq(0))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note at path {}", file_path)))?;

    Ok(model_to_note(model))
}

pub async fn create_note(db: &DatabaseConnection, input: CreateNoteInput) -> Result<Note> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();
    let content_hash = calculate_content_hash(&input.content);

    let am = notes::ActiveModel {
        id: Set(id.clone()),
        vault_id: Set(input.vault_id.clone()),
        title: Set(input.title.clone()),
        file_path: Set(input.file_path.clone()),
        content: Set(input.content.clone()),
        content_hash: Set(content_hash),
        author: Set(input.author.clone()),
        page_type: Set(input.page_type.clone()),
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

    am.insert(db).await?;

    get_note(db, &id).await
}

pub async fn update_note(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateNoteInput,
) -> Result<Note> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    let mut am = model.into_active_model();

    if let Some(title) = input.title {
        am.title = Set(title);
    }

    if let Some(content) = input.content {
        am.content = Set(content.clone());
        am.content_hash = Set(calculate_content_hash(&content));
        am.user_edited = Set(1);
        am.user_edited_at = Set(Some(chrono::Utc::now().timestamp()));
    }

    if let Some(page_type) = input.page_type {
        am.page_type = Set(Some(page_type));
    }

    if let Some(related_pages) = input.related_pages {
        am.related_pages = Set(Some(
            serde_json::to_value(related_pages).unwrap_or_default(),
        ));
    }

    am.updated_at = Set(chrono::Utc::now().timestamp());

    am.update(db).await?;

    get_note(db, id).await
}

pub async fn delete_note(db: &DatabaseConnection, id: &str) -> Result<()> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    let mut am = model.into_active_model();
    am.is_deleted = Set(1);
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    Ok(())
}

pub async fn get_note_links(db: &DatabaseConnection, note_id: &str) -> Result<Vec<NoteLink>> {
    let models = note_links::Entity::find()
        .filter(note_links::Column::SourceNoteId.eq(note_id))
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_link).collect())
}

pub async fn get_note_backlinks(db: &DatabaseConnection, note_id: &str) -> Result<Vec<NoteLink>> {
    let models = note_backlinks::Entity::find()
        .filter(note_backlinks::Column::TargetNoteId.eq(note_id))
        .all(db)
        .await?;

    Ok(models
        .into_iter()
        .map(|m| NoteLink {
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

pub async fn create_note_link(
    db: &DatabaseConnection,
    vault_id: &str,
    source_note_id: &str,
    target_note_id: &str,
    link_text: &str,
    link_type: &str,
) -> Result<NoteLink> {
    let id = note_links::Entity::insert(note_links::ActiveModel {
        vault_id: Set(vault_id.to_string()),
        source_note_id: Set(source_note_id.to_string()),
        target_note_id: Set(target_note_id.to_string()),
        link_text: Set(link_text.to_string()),
        link_type: Set(link_type.to_string()),
        created_at: Set(chrono::Utc::now().timestamp()),
        ..Default::default()
    })
    .exec_with_returning(db)
    .await?;

    Ok(model_to_link(id))
}

pub async fn sync_note_links(
    db: &DatabaseConnection,
    vault_id: &str,
    source_note_id: &str,
    links: Vec<(String, String, String)>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    note_links::Entity::delete_many()
        .filter(note_links::Column::SourceNoteId.eq(source_note_id))
        .exec(db)
        .await?;

    for (target_note_id, link_text, link_type) in links {
        note_links::Entity::insert(note_links::ActiveModel {
            vault_id: Set(vault_id.to_string()),
            source_note_id: Set(source_note_id.to_string()),
            target_note_id: Set(target_note_id),
            link_text: Set(link_text),
            link_type: Set(link_type),
            created_at: Set(now),
            ..Default::default()
        })
        .exec(db)
        .await?;
    }

    Ok(())
}

pub fn calculate_content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub tags: Vec<String>,
    pub link_count: i32,
    pub backlink_count: i32,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub async fn get_vault_graph(db: &DatabaseConnection, vault_id: &str) -> Result<GraphData> {
    let notes = list_notes(db, vault_id).await?;
    let links = note_links::Entity::find()
        .filter(note_links::Column::VaultId.eq(vault_id))
        .all(db)
        .await?;
    let backlinks = note_backlinks::Entity::find()
        .filter(note_backlinks::Column::VaultId.eq(vault_id))
        .all(db)
        .await?;

    let note_ids: std::collections::HashSet<_> = notes.iter().map(|n| n.id.clone()).collect();

    let mut link_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut backlink_counts: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

    for link in &links {
        if note_ids.contains(&link.target_note_id) {
            *link_counts.entry(link.source_note_id.clone()).or_insert(0) += 1;
            *backlink_counts
                .entry(link.target_note_id.clone())
                .or_insert(0) += 1;
        }
    }

    for backlink in &backlinks {
        if note_ids.contains(&backlink.source_note_id) {
            *link_counts
                .entry(backlink.source_note_id.clone())
                .or_insert(0) += 1;
            *backlink_counts
                .entry(backlink.target_note_id.clone())
                .or_insert(0) += 1;
        }
    }

    let mut nodes: Vec<GraphNode> = Vec::new();
    for note in &notes {
        if note.is_deleted {
            continue;
        }
        let tags = extract_tags_from_content(&note.content);
        nodes.push(GraphNode {
            id: note.id.clone(),
            title: note.title.clone(),
            node_type: note.page_type.clone().unwrap_or_else(|| "note".to_string()),
            tags,
            link_count: *link_counts.get(&note.id).unwrap_or(&0),
            backlink_count: *backlink_counts.get(&note.id).unwrap_or(&0),
            path: note.file_path.clone(),
        });
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    for link in &links {
        if note_ids.contains(&link.target_note_id) {
            edges.push(GraphEdge {
                source: link.source_note_id.clone(),
                target: link.target_note_id.clone(),
                edge_type: "link".to_string(),
            });
        }
    }

    for backlink in &backlinks {
        if note_ids.contains(&backlink.source_note_id) {
            edges.push(GraphEdge {
                source: backlink.source_note_id.clone(),
                target: backlink.target_note_id.clone(),
                edge_type: "backlink".to_string(),
            });
        }
    }

    Ok(GraphData { nodes, edges })
}

fn extract_tags_from_content(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            let tag = line.trim_start_matches('#').trim();
            if !tag.is_empty() {
                tags.push(tag.to_string());
            }
        }
    }
    tags
}
