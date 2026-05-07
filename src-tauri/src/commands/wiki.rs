use crate::AppState;
use axagent_core::repo::note::{CreateNoteInput, GraphData, Note, NoteLink, UpdateNoteInput};
use axagent_core::types::NoteSearchResult;
use tauri::{AppHandle, Emitter, State};

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
pub async fn wiki_notes_get(state: State<'_, AppState>, id: String) -> Result<Note, String> {
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
pub async fn wiki_notes_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
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
    axagent_core::repo::note::sync_note_links(&state.sea_db, &vault_id, &source_note_id, links)
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
            if content_lower.contains(&query_lower)
                || note.title.to_lowercase().contains(&query_lower)
            {
                let snippet_start = content_lower.find(&query_lower).unwrap_or(0);
                let snippet = note
                    .content
                    .chars()
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

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
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

#[tauri::command]
pub async fn sync_note_to_knowledge_base(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    knowledge_base_id: String,
) -> Result<(), String> {
    let note = axagent_core::repo::note::get_note(&state.sea_db, &note_id)
        .await
        .map_err(|e| e.to_string())?;

    let file_name = format!("{}.md", note.title.replace('/', "_"));
    let data_dir = state.app_data_dir.join("wiki_sync").join(&note.vault_id);
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

    let full_path = data_dir.join(&file_name);
    std::fs::write(&full_path, &note.content).map_err(|e| e.to_string())?;

    let source_path = full_path.to_string_lossy().to_string();

    let doc = axagent_core::repo::knowledge::add_document(
        &state.sea_db,
        &knowledge_base_id,
        &note.title,
        &source_path,
        "text/markdown",
        Some("wiki-sync"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let kb = axagent_core::repo::knowledge::get_knowledge_base(&state.sea_db, &knowledge_base_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref embedding_provider) = kb.embedding_provider {
        let db = state.sea_db.clone();
        let master_key = state.master_key;
        let vector_store = state.vector_store.clone();
        let doc_id = doc.id.clone();
        let src_path = source_path.clone();
        let mime = "text/markdown".to_string();
        let ep = embedding_provider.clone();
        let chunk_sz = kb.chunk_size;
        let chunk_ov = kb.chunk_overlap;
        let kb_id = knowledge_base_id.clone();
        let semaphore = state.indexing_semaphore.clone();
        let separator = kb.separator.clone();

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await;
            let result = crate::indexing::index_knowledge_document(
                &db,
                &master_key,
                &vector_store,
                &kb_id,
                &doc_id,
                &src_path,
                &mime,
                &ep,
                chunk_sz,
                chunk_ov,
                separator,
            )
            .await;

            if let Err(e) = &result {
                let err_msg = e.to_string();
                tracing::error!("Indexing failed for synced doc {}: {}", doc_id, err_msg);
                let _ = axagent_core::repo::knowledge::update_document_status_with_error(
                    &db,
                    &doc_id,
                    "failed",
                    Some(&err_msg),
                )
                .await;
            }

            let _ = app.emit(
                "knowledge-document-indexed",
                serde_json::json!({
                    "documentId": doc_id,
                    "success": result.is_ok(),
                    "error": result.err().map(|e| e.to_string()),
                }),
            );
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn sync_knowledge_document_to_wiki(
    state: State<'_, AppState>,
    document_id: String,
    vault_id: String,
) -> Result<(), String> {
    let doc = axagent_core::repo::knowledge::get_document(&state.sea_db, &document_id)
        .await
        .map_err(|e| e.to_string())?;

    let content = {
        let path = std::path::Path::new(&doc.source_path);
        if path.exists() {
            axagent_core::document_parser::extract_text(path, &doc.mime_type)
                .map_err(|e| format!("Failed to extract text: {}", e))?
        } else {
            let collection_name = format!("kb_{}", &doc.knowledge_base_id);
            match state
                .vector_store
                .list_document_chunks(&collection_name, &doc.id)
                .await
            {
                Ok(chunks) if !chunks.is_empty() => chunks
                    .into_iter()
                    .map(|c| c.content)
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                _ => {
                    return Err(format!(
                        "Document file not found at '{}' and no indexed chunks available. \
                         The document may have been deleted or the source is a remote URL.",
                        doc.source_path
                    ))
                },
            }
        }
    };

    let input = CreateNoteInput {
        vault_id: vault_id.clone(),
        title: doc.title.clone(),
        file_path: format!("synced/{}.md", doc.title.replace('/', "_")),
        content,
        author: "knowledge-sync".to_string(),
        page_type: Some("synced".to_string()),
        source_refs: Some(vec![doc.id.clone()]),
    };

    axagent_core::repo::note::create_note(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
