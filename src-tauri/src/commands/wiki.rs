use crate::AppState;
use axagent_core::hybrid_search::{HybridSearchOptions, HybridSearcher};
use axagent_core::rag::{collection_id, RAGSource, WikiVaultRAG};
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

    let wiki = axagent_core::repo::wiki::get_wiki(&state.sea_db, &vault_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref _ep) = wiki.embedding_provider {
        match wiki_notes_search_hybrid(&state, &vault_id, &query, top_k).await {
            Ok(results) => return Ok(results),
            Err(e) => {
                tracing::warn!(
                    "Hybrid search failed for wiki {}, falling back to keyword: {}",
                    vault_id,
                    e
                );
            },
        }
    }

    wiki_notes_search_keyword(&state, &vault_id, &query, top_k).await
}

async fn wiki_notes_search_hybrid(
    state: &AppState,
    vault_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<NoteSearchResult>, String> {
    let wiki = axagent_core::repo::wiki::get_wiki(&state.sea_db, vault_id)
        .await
        .map_err(|e| e.to_string())?;

    let ep = wiki
        .embedding_provider
        .as_ref()
        .ok_or("No embedding provider")?;
    let dimensions = wiki.embedding_dimensions.map(|d| d as usize);

    let embed_fn = crate::indexing::ProviderEmbedFn;
    let embed_response = axagent_core::rag::AsyncEmbedFn::generate(
        &embed_fn,
        &state.sea_db,
        &state.master_key,
        ep,
        vec![query.to_string()],
        dimensions,
    )
    .await
    .map_err(|e| e.to_string())?;

    let query_embedding = embed_response
        .embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "No query embedding returned".to_string())?;

    let collection_id = collection_id(WikiVaultRAG.collection_prefix(), vault_id);
    let searcher = HybridSearcher::new(state.sea_db.clone());

    let options = HybridSearchOptions {
        vector_weight: 0.7,
        bm25_weight: 0.3,
        top_k,
        min_score: None,
    };

    let hybrid_results = searcher
        .hybrid_search(&collection_id, query, query_embedding, options)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for hybrid_result in &hybrid_results {
        let note =
            match axagent_core::repo::note::get_note(&state.sea_db, &hybrid_result.document_id)
                .await
            {
                Ok(n) => n,
                Err(_) => continue,
            };

        let snippet = extract_highlight_snippet(&note.content, query, 50, 150);
        let score = hybrid_result.combined_score as f64;

        results.push(NoteSearchResult {
            note,
            snippet,
            score,
        });
    }

    Ok(results)
}

async fn wiki_notes_search_keyword(
    state: &AppState,
    vault_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<NoteSearchResult>, String> {
    let notes = axagent_core::repo::note::list_notes(&state.sea_db, vault_id)
        .await
        .map_err(|e| e.to_string())?;

    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    let num_docs = notes.len() as f64;
    let avg_dl = if !notes.is_empty() {
        notes.iter().map(|n| n.content.len() as f64).sum::<f64>() / num_docs
    } else {
        1.0
    };

    let mut df: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for word in &query_words {
        let count = notes
            .iter()
            .filter(|n| {
                n.content.to_lowercase().contains(word) || n.title.to_lowercase().contains(word)
            })
            .count() as f64;
        df.insert(word, count);
    }

    let mut results: Vec<NoteSearchResult> = Vec::new();

    for note in notes {
        let score =
            compute_note_bm25_score(&note, &query_lower, &query_words, &df, num_docs, avg_dl);
        if score <= 0.0 {
            continue;
        }

        let snippet = extract_highlight_snippet(&note.content, query, 50, 150);

        results.push(NoteSearchResult {
            note,
            snippet,
            score,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(top_k);

    Ok(results)
}

const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

fn compute_note_bm25_score(
    note: &Note,
    query_lower: &str,
    query_words: &[&str],
    df: &std::collections::HashMap<&str, f64>,
    num_docs: f64,
    avg_dl: f64,
) -> f64 {
    let content_lower = note.content.to_lowercase();
    let title_lower = note.title.to_lowercase();
    let dl = note.content.len() as f64;

    let mut score = 0.0_f64;

    if title_lower.contains(query_lower) {
        score += 2.0;
    } else {
        for word in query_words {
            if title_lower.contains(word) {
                score += 0.8;
            }
        }
    }

    for word in query_words {
        let tf = content_lower.matches(word).count() as f64;
        if tf == 0.0 {
            continue;
        }
        let df_val = df.get(word).copied().unwrap_or(0.0);
        let idf = ((num_docs - df_val + 0.5) / (df_val + 0.5) + 1.0).ln();
        let tf_norm =
            (tf * (BM25_K1 + 1.0)) / (tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (dl / avg_dl)));
        score += idf * tf_norm;
    }

    if let Some(qs) = note.quality_score {
        score += qs * 0.3;
    }

    score
}

fn extract_highlight_snippet(
    content: &str,
    query: &str,
    context_chars: usize,
    max_snippet_len: usize,
) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    let best_pos = if !query_lower.is_empty() {
        content_lower.find(&query_lower)
    } else {
        None
    };

    let best_pos = best_pos.or_else(|| {
        query_words
            .iter()
            .filter_map(|w| content_lower.find(w))
            .min()
    });

    let start = match best_pos {
        Some(pos) => pos.saturating_sub(context_chars),
        None => 0,
    };

    let chars: Vec<char> = content.chars().collect();
    let total_len = chars.len();

    let start_char = start.min(total_len);
    let end_char = (start_char + max_snippet_len).min(total_len);

    let mut snippet: String = chars[start_char..end_char].iter().collect();

    if end_char < total_len {
        snippet.push_str("...");
    }
    if start_char > 0 {
        snippet = format!("...{}", snippet);
    }

    snippet
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
