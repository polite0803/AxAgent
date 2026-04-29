use crate::AppState;
use axagent_core::repo::note::{
    CreateNoteInput, GraphData, Note, NoteLink, UpdateNoteInput,
};
use axagent_core::types::NoteSearchResult;
use tauri::State;

#[tauri::command]
pub async fn wiki_notes_list(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<Note>, String> {
    axagent_core::repo::note::list_notes(&state.sea_db, &vault_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<Note, String> {
    axagent_core::repo::note::get_note(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get_by_path(
    state: State<'_, AppState>,
    vault_id: String,
    file_path: String,
) -> Result<Note, String> {
    axagent_core::repo::note::get_note_by_path(&state.sea_db, &vault_id, &file_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_create(
    state: State<'_, AppState>,
    input: CreateNoteInput,
) -> Result<Note, String> {
    axagent_core::repo::note::create_note(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_update(
    state: State<'_, AppState>,
    id: String,
    input: UpdateNoteInput,
) -> Result<Note, String> {
    axagent_core::repo::note::update_note(&state.sea_db, &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_core::repo::note::delete_note(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get_links(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<NoteLink>, String> {
    axagent_core::repo::note::get_note_links(&state.sea_db, &note_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get_backlinks(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<NoteLink>, String> {
    axagent_core::repo::note::get_note_backlinks(&state.sea_db, &note_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_sync_links(
    state: State<'_, AppState>,
    vault_id: String,
    source_note_id: String,
    links: Vec<(String, String, String)>,
) -> Result<(), String> {
    axagent_core::repo::note::sync_note_links(
        &state.sea_db,
        &vault_id,
        &source_note_id,
        links,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_search(
    state: State<'_, AppState>,
    vault_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<NoteSearchResult>, String> {
    let top_k = top_k.unwrap_or(10);
    let notes = axagent_core::repo::note::list_notes(&state.sea_db, &vault_id)
        .await
        .map_err(|e| e.to_string())?;

    let query_lower = query.to_lowercase();
    let mut results: Vec<NoteSearchResult> = notes
        .into_iter()
        .filter_map(|note| {
            let content_lower = note.content.to_lowercase();
            if content_lower.contains(&query_lower) || note.title.to_lowercase().contains(&query_lower) {
                let snippet_start = content_lower.find(&query_lower).unwrap_or(0);
                let snippet = note.content.chars()
                    .skip(snippet_start.saturating_sub(50))
                    .take(100)
                    .collect::<String>();
                Some(NoteSearchResult {
                    note,
                    snippet,
                    score: 1.0,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);

    Ok(results)
}

#[tauri::command]
pub async fn get_wiki_graph(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<GraphData, String> {
    axagent_core::repo::note::get_vault_graph(&state.sea_db, &wiki_id)
        .await
        .map_err(|e| e.to_string())
}