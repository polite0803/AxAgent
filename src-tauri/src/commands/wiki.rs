// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_dao::repo::index_jobs as jobs;
use axagent_dao::repo::louvain::{self, LouvainResult};
use axagent_dao::repo::note::{CreateNoteInput, GraphData, Note, NoteLink, UpdateNoteInput};
use axagent_dao::repo::note_graph::LinkGraph;
use axagent_dao::repo::wiki::{self, CreateWikiTemplateInput, NoteVersion, WikiTemplate};
use axagent_harness::types::NoteSearchResult;
use axagent_search::hybrid_search::{FusionAlgorithm, HybridSearchOptions, HybridSearcher};
use axagent_search::rag::{RAGSource, WikiVaultRAG, collection_id};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

/// 同步 IO 包装：把 std::fs 调用扔到 spawn_blocking 线程池，避免阻塞 tokio runtime。
/// 多个小文件操作适合 inline `spawn_blocking`。
async fn write_file_blocking(path: PathBuf, content: Vec<u8>) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || std::fs::write(&path, &content))
        .await
        .map_err(std::io::Error::other)?
}

async fn read_to_string_blocking(path: PathBuf) -> std::io::Result<String> {
    tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(std::io::Error::other)?
}

async fn create_dir_all_blocking(path: PathBuf) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || std::fs::create_dir_all(&path))
        .await
        .map_err(std::io::Error::other)?
}

fn enqueue_wiki_note_indexing(
    state: &State<'_, AppState>,
    app: &AppHandle,
    wiki_id: &str,
    note_id: &str,
) {
    let _ = crate::index_queue::enqueue_job_sync(
        state,
        app,
        jobs::JOB_TYPE_INDEX_WIKI_NOTE,
        "wiki",
        wiki_id,
        note_id,
        None,
        None,
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkInfo {
    pub note_id: String,
    pub title: String,
    pub snippets: Vec<String>,
}

#[tauri::command]
pub async fn wiki_notes_list(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<Note>, String> {
    axagent_dao::repo::note::list_notes(state.harness.db(), &vault_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get(state: State<'_, AppState>, id: String) -> Result<Note, String> {
    axagent_dao::repo::note::get_note(state.harness.db(), &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get_by_path(
    state: State<'_, AppState>,
    vault_id: String,
    file_path: String,
) -> Result<Note, String> {
    axagent_dao::repo::note::get_note_by_path(state.harness.db(), &vault_id, &file_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateNoteInput,
) -> Result<Note, String> {
    let note = axagent_dao::repo::note::create_note(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())?;

    enqueue_wiki_note_indexing(&state, &app, &note.vault_id, &note.id);

    Ok(note)
}

#[tauri::command]
pub async fn wiki_notes_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: UpdateNoteInput,
) -> Result<Note, String> {
    if input.content.is_some() || input.title.is_some() {
        if let Ok(existing) = axagent_dao::repo::note::get_note(state.harness.db(), &id).await {
            let _ = wiki::create_version(
                state.harness.db(),
                &existing.vault_id,
                &existing.id,
                &existing.title,
                &existing.content,
                &existing.author,
            )
            .await;
        }
    }

    let updated = axagent_dao::repo::note::update_note(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())?;

    let _ = wiki::delete_old_versions(state.harness.db(), &id, 20).await;

    enqueue_wiki_note_indexing(&state, &app, &updated.vault_id, &updated.id);

    Ok(updated)
}

#[tauri::command]
pub async fn wiki_notes_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if let Ok(existing) = axagent_dao::repo::note::get_note(state.harness.db(), &id).await {
        let collection_id = format!("wiki_{}", existing.vault_id);
        let _ = state.vector_store.delete_document_embeddings(&collection_id, &id).await;

        let _ = wiki::create_version(
            state.harness.db(),
            &existing.vault_id,
            &existing.id,
            &existing.title,
            &existing.content,
            &existing.author,
        )
        .await;
    }

    axagent_dao::repo::note::delete_note(state.harness.db(), &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebuild_wiki_index(
    app: AppHandle,
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<(), String> {
    let wiki = axagent_dao::repo::wiki::get_wiki(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;

    let _embedding_provider =
        wiki.embedding_provider.as_ref().ok_or("No embedding provider configured for this wiki")?;

    let container = axagent_search::rag::KnowledgeContainer::from_wiki(&wiki);

    let collection_id = format!("wiki_{}", wiki_id);
    let _ = state.vector_store.delete_collection(&collection_id).await;

    let notes = axagent_dao::repo::note::list_notes(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let vector_store = state.vector_store.clone();
    let wid = wiki_id.clone();

    tokio::spawn(catch_unwind_logged("wiki.rebuild_index", async move {
        for note in &notes {
            let result = crate::indexing::index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                &note.id,
                &note.content,
                None,
                None,
            )
            .await;

            if let Err(e) = &result {
                tracing::error!("Wiki re-indexing failed for note {}: {}", note.id, e);
            }

            let _ = app.emit(
                "wiki-note-indexed",
                serde_json::json!({
                    "noteId": note.id,
                    "success": result.is_ok(),
                    "error": result.as_ref().err().map(|e| e.to_string()),
                    "isRebuild": true,
                }),
            );
        }

        let _ = app.emit("wiki-rebuild-complete", serde_json::json!({ "wikiId": wid }));
    }));

    Ok(())
}

#[tauri::command]
pub async fn wiki_notes_get_links(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<NoteLink>, String> {
    axagent_dao::repo::note::get_note_links(state.harness.db(), &note_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_notes_get_backlinks(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<BacklinkInfo>, String> {
    let links = axagent_dao::repo::note::get_note_backlinks(state.harness.db(), &note_id)
        .await
        .map_err(|e| e.to_string())?;

    let target_note = axagent_dao::repo::note::get_note(state.harness.db(), &note_id).await.ok();
    let target_title = target_note.as_ref().map(|n| n.title.as_str()).unwrap_or("");

    let mut map: std::collections::HashMap<String, BacklinkInfo> = std::collections::HashMap::new();

    for link in &links {
        let source_note =
            match axagent_dao::repo::note::get_note(state.harness.db(), &link.source_note_id).await
            {
                Ok(n) => n,
                Err(_) => continue,
            };

        let snippets = extract_link_context_snippets(&source_note.content, target_title, 80);

        let entry = map.entry(link.source_note_id.clone()).or_insert_with(|| BacklinkInfo {
            note_id: link.source_note_id.clone(),
            title: source_note.title.clone(),
            snippets: Vec::new(),
        });
        entry.snippets.extend(snippets);
    }

    Ok(map.into_values().collect())
}

fn extract_link_context_snippets(
    content: &str,
    target_title: &str,
    context_chars: usize,
) -> Vec<String> {
    if target_title.is_empty() {
        return Vec::new();
    }

    let link_pattern = format!("[[{}]]", target_title);
    let chars: Vec<char> = content.chars().collect();
    let total_len = chars.len();
    let pattern_chars: Vec<char> = link_pattern.chars().collect();
    let pattern_len = pattern_chars.len();

    let mut snippets = Vec::new();
    let mut i = 0;

    while i + pattern_len <= total_len {
        let window: Vec<char> = chars[i..i + pattern_len].to_vec();
        if window == pattern_chars {
            let start = i.saturating_sub(context_chars);
            let end = (i + pattern_len + context_chars).min(total_len);

            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&chars[start..end].iter().collect::<String>());
            if end < total_len {
                snippet.push_str("...");
            }

            snippets.push(snippet);
            i += pattern_len;
        } else {
            i += 1;
        }
    }

    snippets
}

#[tauri::command]
pub async fn wiki_notes_sync_links(
    state: State<'_, AppState>,
    vault_id: String,
    source_note_id: String,
    links: Vec<(String, String, String)>,
) -> Result<(), String> {
    axagent_dao::repo::note::sync_note_links(state.harness.db(), &vault_id, &source_note_id, links)
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

    let wiki = axagent_dao::repo::wiki::get_wiki(state.harness.db(), &vault_id)
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
    let wiki = axagent_dao::repo::wiki::get_wiki(state.harness.db(), vault_id)
        .await
        .map_err(|e| e.to_string())?;

    let ep = wiki.embedding_provider.as_ref().ok_or("No embedding provider")?;
    let dimensions = wiki.embedding_dimensions.map(|d| d as usize);

    let embed_fn = crate::indexing::ProviderEmbedFn;
    let embed_response = axagent_search::rag::AsyncEmbedFn::generate(
        &embed_fn,
        state.harness.db(),
        state.harness.master_key(),
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
    let searcher = HybridSearcher::new(state.harness.db().clone());

    let options = HybridSearchOptions {
        vector_weight: 0.7,
        bm25_weight: 0.3,
        top_k,
        min_score: None,
        fusion: FusionAlgorithm::Rrf,
        rrf_k: 60.0,
    };

    let hybrid_results = searcher
        .hybrid_search(&collection_id, query, query_embedding, options)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for hybrid_result in &hybrid_results {
        let note =
            match axagent_dao::repo::note::get_note(state.harness.db(), &hybrid_result.document_id)
                .await
            {
                Ok(n) => n,
                Err(_) => continue,
            };

        let snippet = extract_highlight_snippet(&note.content, query, 50, 150);
        let score = hybrid_result.combined_score as f64;

        results.push(NoteSearchResult { note: note.into(), snippet, score });
    }

    Ok(results)
}

async fn wiki_notes_search_keyword(
    state: &AppState,
    vault_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<NoteSearchResult>, String> {
    let notes = axagent_dao::repo::note::list_notes(state.harness.db(), vault_id)
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

        results.push(NoteSearchResult { note: note.into(), snippet, score });
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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

    let best_pos =
        best_pos.or_else(|| query_words.iter().filter_map(|w| content_lower.find(w)).min());

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
    axagent_dao::repo::note::get_vault_graph(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_graph_communities(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<LouvainResult, String> {
    let graph_data = axagent_dao::repo::note::get_vault_graph(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;

    let link_graph = LinkGraph::from_graph_data(graph_data);
    let result = louvain::detect_communities(link_graph);
    Ok(result)
}

#[tauri::command]
pub async fn sync_note_to_knowledge_base(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    knowledge_base_id: String,
) -> Result<(), String> {
    let note = axagent_dao::repo::note::get_note(state.harness.db(), &note_id)
        .await
        .map_err(|e| e.to_string())?;

    let file_name = format!("{}.md", note.title.replace('/', "_"));
    let data_dir = state.app_data_dir.join("wiki_sync").join(&note.vault_id);
    create_dir_all_blocking(data_dir.clone()).await.map_err(|e| e.to_string())?;

    let full_path = data_dir.join(&file_name);
    write_file_blocking(full_path.clone(), note.content.into_bytes())
        .await
        .map_err(|e| e.to_string())?;

    let source_path = full_path.to_string_lossy().to_string();

    let doc = axagent_dao::repo::knowledge::add_document(
        state.harness.db(),
        &knowledge_base_id,
        &note.title,
        &source_path,
        "text/markdown",
        Some("wiki-sync"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let kb =
        axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &knowledge_base_id)
            .await
            .map_err(|e| e.to_string())?;

    if kb.embedding_provider.is_some() {
        let _ = axagent_dao::repo::knowledge::update_document_status(
            state.harness.db(),
            &doc.id,
            "pending",
        )
        .await;
        let _ = crate::index_queue::enqueue_job_sync(
            &state,
            &app,
            jobs::JOB_TYPE_INDEX_DOCUMENT,
            "kb",
            &knowledge_base_id,
            &doc.id,
            None,
            None,
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn sync_knowledge_document_to_wiki(
    app: AppHandle,
    state: State<'_, AppState>,
    document_id: String,
    vault_id: String,
) -> Result<(), String> {
    let doc = axagent_dao::repo::knowledge::get_document(state.harness.db(), &document_id)
        .await
        .map_err(|e| e.to_string())?;

    let content = {
        let path = std::path::Path::new(&doc.source_path);
        if path.exists() {
            axagent_document_parser::extract_text(path, &doc.mime_type)
                .map_err(|e| format!("Failed to extract text: {}", e))?
        } else {
            let collection_name = format!("kb_{}", &doc.knowledge_base_id);
            match state.vector_store.list_document_chunks(&collection_name, &doc.id).await {
                Ok(chunks) if !chunks.is_empty() => {
                    chunks.into_iter().map(|c| c.content).collect::<Vec<_>>().join("\n\n")
                },
                _ => {
                    return Err(format!(
                        "Document file not found at '{}' and no indexed chunks available. \
                         The document may have been deleted or the source is a remote URL.",
                        doc.source_path
                    ));
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

    let note = axagent_dao::repo::note::create_note(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())?;

    enqueue_wiki_note_indexing(&state, &app, &vault_id, &note.id);

    Ok(())
}

#[tauri::command]
pub async fn wiki_note_versions(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<NoteVersion>, String> {
    wiki::list_versions(state.harness.db(), &note_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_note_get_version(
    state: State<'_, AppState>,
    version_id: i64,
) -> Result<NoteVersion, String> {
    wiki::get_version(state.harness.db(), version_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_note_restore_version(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    version_id: i64,
) -> Result<Note, String> {
    let version =
        wiki::get_version(state.harness.db(), version_id).await.map_err(|e| e.to_string())?;

    let note = axagent_dao::repo::note::get_note(state.harness.db(), &note_id)
        .await
        .map_err(|e| e.to_string())?;

    wiki::create_version(
        state.harness.db(),
        &note.vault_id,
        &note.id,
        &note.title,
        &note.content,
        &note.author,
    )
    .await
    .map_err(|e| e.to_string())?;

    let input = UpdateNoteInput {
        title: Some(version.title.clone()),
        content: Some(version.content.clone()),
        page_type: None,
        related_pages: None,
    };

    let updated = axagent_dao::repo::note::update_note(state.harness.db(), &note_id, input)
        .await
        .map_err(|e| e.to_string())?;

    let _ = wiki::delete_old_versions(state.harness.db(), &note_id, 20).await;

    enqueue_wiki_note_indexing(&state, &app, &updated.vault_id, &updated.id);

    Ok(updated)
}

#[tauri::command]
pub async fn wiki_template_list(
    state: State<'_, AppState>,
    wiki_id: String,
) -> Result<Vec<WikiTemplate>, String> {
    wiki::list_wiki_templates(state.harness.db(), &wiki_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_template_create(
    state: State<'_, AppState>,
    input: CreateWikiTemplateInput,
) -> Result<WikiTemplate, String> {
    wiki::create_wiki_template(state.harness.db(), input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_template_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    wiki::delete_wiki_template(state.harness.db(), &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_note_create_from_template(
    app: AppHandle,
    state: State<'_, AppState>,
    vault_id: String,
    template_id: String,
    title: Option<String>,
) -> Result<Note, String> {
    let template = wiki::get_wiki_template(state.harness.db(), &template_id)
        .await
        .map_err(|e| e.to_string())?;

    let wiki_obj =
        wiki::get_wiki(state.harness.db(), &vault_id).await.map_err(|e| e.to_string())?;

    let content = wiki::apply_template_variables(&template.content, &wiki_obj.name);

    let note_title = title.unwrap_or_else(|| template.name.clone());
    let now = chrono::Utc::now().timestamp();
    let file_path =
        format!("templates/{}-{}.md", template.name.replace(' ', "_").to_lowercase(), now);

    let input = CreateNoteInput {
        vault_id: vault_id.clone(),
        title: note_title,
        file_path,
        content,
        author: "template".to_string(),
        page_type: template.page_type,
        source_refs: None,
    };

    let note = axagent_dao::repo::note::create_note(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())?;

    enqueue_wiki_note_indexing(&state, &app, &vault_id, &note.id);

    Ok(note)
}

#[tauri::command]
pub async fn wiki_create_daily_note(
    app: AppHandle,
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Note, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let file_path = format!("daily/{}.md", today);

    match axagent_dao::repo::note::get_note_by_path(state.harness.db(), &vault_id, &file_path).await
    {
        Ok(note) => Ok(note),
        Err(_) => {
            let content = format!("# {}\n\n## Tasks\n\n## Notes\n\n## Ideas\n", today);

            let input = CreateNoteInput {
                vault_id: vault_id.clone(),
                title: today.clone(),
                file_path,
                content,
                author: "user".to_string(),
                page_type: Some("daily".to_string()),
                source_refs: None,
            };

            let note = axagent_dao::repo::note::create_note(state.harness.db(), input)
                .await
                .map_err(|e| e.to_string())?;

            enqueue_wiki_note_indexing(&state, &app, &vault_id, &note.id);

            Ok(note)
        },
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStats {
    pub imported: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportStats {
    pub exported: usize,
    pub failed: usize,
}

#[tauri::command]
pub async fn wiki_import_obsidian_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    wiki_id: String,
    vault_path: String,
) -> Result<ImportStats, String> {
    let root = std::path::Path::new(&vault_path);
    if !root.is_dir() {
        return Err(format!("Path is not a directory: {}", vault_path));
    }

    let existing = axagent_dao::repo::note::list_notes(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;
    let existing_titles: std::collections::HashSet<String> =
        existing.iter().map(|n| n.title.clone()).collect();

    let mut md_files: Vec<std::path::PathBuf> = Vec::new();
    collect_md_files(root, &mut md_files);

    let mut imported = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for file_path in &md_files {
        let raw = match read_to_string_blocking(file_path.clone()).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", file_path.display(), e);
                failed += 1;
                continue;
            },
        };

        let (frontmatter, content) = parse_frontmatter(&raw);

        let title = frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string())
            });

        if existing_titles.contains(&title) {
            skipped += 1;
            continue;
        }

        let tags: Vec<String> = frontmatter
            .get("tags")
            .and_then(|v| {
                if v.is_sequence() {
                    v.as_sequence().map(|seq| {
                        seq.iter().filter_map(|item| item.as_str().map(String::from)).collect()
                    })
                } else if v.is_string() {
                    v.as_str().map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let relative =
            file_path.strip_prefix(root).unwrap_or(file_path).to_string_lossy().to_string();

        let content_with_tags = if tags.is_empty() {
            content.clone()
        } else {
            let tag_lines: Vec<String> = tags.iter().map(|t| format!("#{}", t)).collect();
            format!("{}\n\n{}", tag_lines.join("\n"), content)
        };

        let input = CreateNoteInput {
            vault_id: wiki_id.clone(),
            title,
            file_path: relative,
            content: content_with_tags,
            author: "obsidian-import".to_string(),
            page_type: None,
            source_refs: None,
        };

        match axagent_dao::repo::note::create_note(state.harness.db(), input).await {
            Ok(note) => {
                enqueue_wiki_note_indexing(&state, &app, &wiki_id, &note.id);
                imported += 1;
            },
            Err(e) => {
                tracing::warn!("Failed to create note from {}: {}", file_path.display(), e);
                failed += 1;
            },
        }
    }

    Ok(ImportStats { imported, failed, skipped })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMdImportStats {
    pub imported: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total: usize,
}

/// 将 KNOWLEDGE.md（精炼知识源）导入为 Wiki 笔记。
/// 按 `## ` 标题分割章节，每个章节创建一条笔记。
/// 自动触发向量索引，使知识可通过 RAG 管道检索。
#[tauri::command]
pub async fn wiki_import_knowledge_md(
    app: AppHandle,
    state: State<'_, AppState>,
    wiki_id: String,
    file_path: Option<String>,
) -> Result<KnowledgeMdImportStats, String> {
    let default_path = std::path::Path::new(".workbuddy/memory/KNOWLEDGE.md");
    let path = file_path.as_deref().unwrap_or_default();
    let knowledge_path = if path.is_empty() {
        default_path
    } else {
        std::path::Path::new(path)
    };

    // 检查文件是否存在
    if !knowledge_path.exists() {
        return Err(format!("KNOWLEDGE.md not found at: {}", knowledge_path.display()));
    }

    let raw = read_to_string_blocking(knowledge_path.to_path_buf())
        .await
        .map_err(|e| format!("Failed to read KNOWLEDGE.md: {}", e))?;

    // 解析章节：按 `## ` 分割，跳过第一个（标题/引言）
    let sections: Vec<&str> = raw.split("\n## ").collect();
    if sections.is_empty() {
        return Ok(KnowledgeMdImportStats { imported: 0, failed: 0, skipped: 0, total: 0 });
    }

    // 获取已有笔记标题，跳过重复
    let existing = axagent_dao::repo::note::list_notes(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;
    let existing_titles: std::collections::HashSet<String> =
        existing.iter().map(|n| n.title.clone()).collect();

    let mut imported = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for section in &sections {
        // 提取标题和内容
        let (title, content) = if let Some(pos) = section.find('\n') {
            let title_raw = section[..pos].trim().to_string();
            let body = section[pos + 1..].trim().to_string();
            // 跳过 INTRODUCTION 和元信息
            if title_raw.is_empty()
                || title_raw.to_lowercase().contains("introduction")
                || title_raw.starts_with("---")
            {
                continue;
            }
            (title_raw, body)
        } else {
            // 无换行的短片段，跳过
            continue;
        };

        if title.is_empty() {
            continue;
        }

        if existing_titles.contains(&title) {
            skipped += 1;
            continue;
        }

        let input = CreateNoteInput {
            vault_id: wiki_id.clone(),
            title: title.clone(),
            file_path: format!("knowledge/{}.md", title),
            content: format!("## {}\n\n{}", title, content),
            author: "knowledge-md-import".to_string(),
            page_type: Some("knowledge".to_string()),
            source_refs: Some(vec![knowledge_path.to_string_lossy().to_string()]),
        };

        match axagent_dao::repo::note::create_note(state.harness.db(), input).await {
            Ok(note) => {
                enqueue_wiki_note_indexing(&state, &app, &wiki_id, &note.id);
                imported += 1;
            },
            Err(e) => {
                tracing::warn!("Failed to create note from section '{}': {}", title, e);
                failed += 1;
            },
        }
    }

    Ok(KnowledgeMdImportStats { imported, failed, skipped, total: sections.len() })
}

fn collect_md_files(current: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name =
                    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if dir_name.starts_with('.') {
                    continue;
                }
                collect_md_files(&path, files);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path);
            }
        }
    }
}

fn parse_frontmatter(raw: &str) -> (serde_yaml::Value, String) {
    if !raw.starts_with("---") {
        return (serde_yaml::Value::Null, raw.to_string());
    }

    let rest = &raw[3..];
    let end = match rest.find("---") {
        Some(pos) => pos,
        None => return (serde_yaml::Value::Null, raw.to_string()),
    };

    let yaml_str = &rest[..end];
    let body = rest[end + 3..].trim_start_matches('\n').trim_start_matches('\r');

    let frontmatter = match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
        Ok(v) => v,
        Err(_) => serde_yaml::Value::Null,
    };

    (frontmatter, body.to_string())
}

#[tauri::command]
pub async fn wiki_export_markdown(
    state: State<'_, AppState>,
    wiki_id: String,
    output_path: String,
) -> Result<ExportStats, String> {
    let notes = axagent_dao::repo::note::list_notes(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;

    let output_dir = std::path::Path::new(&output_path);
    create_dir_all_blocking(output_dir.to_path_buf()).await.map_err(|e| e.to_string())?;

    let mut exported = 0usize;
    let mut failed = 0usize;

    for note in &notes {
        let sub_dir = if let Some(ref pt) = note.page_type {
            if pt.is_empty() {
                output_dir.to_path_buf()
            } else {
                let d = output_dir.join(sanitize_filename(pt));
                create_dir_all_blocking(d.clone()).await.map_err(|e| e.to_string())?;
                d
            }
        } else {
            output_dir.to_path_buf()
        };

        let file_name = format!("{}.md", sanitize_filename(&note.title));
        let full_path = sub_dir.join(&file_name);

        let created_str = format_timestamp(note.created_at);
        let updated_str = format_timestamp(note.updated_at);

        let tags = extract_tags_from_note_content(&note.content);
        let tags_yaml = if tags.is_empty() {
            "[]".to_string()
        } else {
            let items: Vec<String> = tags.iter().map(|t| format!("  - {}", t)).collect();
            format!("\n{}", items.join("\n"))
        };

        let frontmatter = format!(
            "---\ntitle: {}\ntags:{}\ncreated_at: {}\nupdated_at: {}\n---\n",
            escape_yaml_string(&note.title),
            tags_yaml,
            created_str,
            updated_str,
        );

        let file_content = format!("{}{}", frontmatter, note.content);

        match write_file_blocking(full_path.clone(), file_content.into_bytes()).await {
            Ok(_) => exported += 1,
            Err(e) => {
                tracing::warn!("Failed to write {}: {}", full_path.display(), e);
                failed += 1;
            },
        }
    }

    Ok(ExportStats { exported, failed })
}

#[tauri::command]
pub async fn wiki_export_html(
    state: State<'_, AppState>,
    wiki_id: String,
    output_path: String,
) -> Result<ExportStats, String> {
    let notes = axagent_dao::repo::note::list_notes(state.harness.db(), &wiki_id)
        .await
        .map_err(|e| e.to_string())?;

    let output_dir = std::path::Path::new(&output_path);
    create_dir_all_blocking(output_dir.to_path_buf()).await.map_err(|e| e.to_string())?;

    let mut exported = 0usize;
    let mut failed = 0usize;

    let note_titles: std::collections::HashMap<String, String> =
        notes.iter().map(|n| (n.title.clone(), sanitize_filename(&n.title))).collect();

    for note in &notes {
        let html_file_name = format!("{}.html", sanitize_filename(&note.title));
        let full_path = output_dir.join(&html_file_name);

        let html_body = markdown_to_simple_html(&note.content, &note_titles);

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; max-width: 980px; margin: 0 auto; padding: 45px; color: #24292e; line-height: 1.6; }}
h1 {{ border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }}
h2 {{ border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }}
a {{ color: #0366d6; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
code {{ background: #f6f8fa; padding: 0.2em 0.4em; border-radius: 3px; font-size: 85%; }}
pre {{ background: #f6f8fa; padding: 16px; border-radius: 6px; overflow: auto; }}
blockquote {{ border-left: 4px solid #dfe2e5; padding: 0 1em; color: #6a737d; margin: 0 0 16px 0; }}
ul, ol {{ padding-left: 2em; }}
.wikilink {{ color: #0366d6; background: #f1f8ff; padding: 1px 4px; border-radius: 3px; }}
</style>
</head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
            escape_html(&note.title),
            escape_html(&note.title),
            html_body,
        );

        match write_file_blocking(full_path.clone(), html.into_bytes()).await {
            Ok(_) => exported += 1,
            Err(e) => {
                tracing::warn!("Failed to write {}: {}", full_path.display(), e);
                failed += 1;
            },
        }
    }

    let index_path = output_dir.join("index.html");
    let mut index_items = String::new();
    for note in &notes {
        let href = format!("{}.html", sanitize_filename(&note.title));
        index_items.push_str(&format!(
            r#"<li><a href="{}">{}</a></li>"#,
            href,
            escape_html(&note.title),
        ));
    }

    let index_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Wiki Index</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; max-width: 980px; margin: 0 auto; padding: 45px; color: #24292e; line-height: 1.6; }}
h1 {{ border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }}
a {{ color: #0366d6; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
ul {{ list-style: none; padding-left: 0; }}
li {{ padding: 4px 0; }}
</style>
</head>
<body>
<h1>Wiki Index</h1>
<ul>
{}
</ul>
</body>
</html>"#,
        index_items,
    );

    write_file_blocking(index_path.clone(), index_html.into_bytes())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExportStats { exported, failed })
}

#[tauri::command]
pub async fn wiki_note_export_pdf(
    state: State<'_, AppState>,
    note_id: String,
    output_path: String,
) -> Result<String, String> {
    let note = axagent_dao::repo::note::get_note(state.harness.db(), &note_id)
        .await
        .map_err(|e| e.to_string())?;

    let output = std::path::Path::new(&output_path);
    if let Some(parent) = output.parent() {
        create_dir_all_blocking(parent.to_path_buf()).await.map_err(|e| e.to_string())?;
    }

    let html_body = markdown_to_simple_html(&note.content, &std::collections::HashMap::new());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; max-width: 980px; margin: 0 auto; padding: 45px; color: #24292e; line-height: 1.6; }}
h1 {{ border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }}
h2 {{ border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }}
a {{ color: #0366d6; text-decoration: none; }}
code {{ background: #f6f8fa; padding: 0.2em 0.4em; border-radius: 3px; font-size: 85%; }}
pre {{ background: #f6f8fa; padding: 16px; border-radius: 6px; overflow: auto; }}
blockquote {{ border-left: 4px solid #dfe2e5; padding: 0 1em; color: #6a737d; margin: 0 0 16px 0; }}
ul, ol {{ padding-left: 2em; }}
.wikilink {{ color: #0366d6; background: #f1f8ff; padding: 1px 4px; border-radius: 3px; }}
@media print {{ body {{ padding: 0; max-width: none; }} }}
</style>
</head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
        escape_html(&note.title),
        escape_html(&note.title),
        html_body,
    );

    let html_output = if output.extension().map(|e| e == "pdf").unwrap_or(false) {
        output.with_extension("html")
    } else {
        output.to_path_buf()
    };

    write_file_blocking(html_output.clone(), html.into_bytes()).await.map_err(|e| e.to_string())?;

    let html_path = html_output.to_string_lossy().to_string();
    let _ = open::that(&html_output);

    Ok(html_path)
}

fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c == '/'
                || c == '\\'
                || c == ':'
                || c == '*'
                || c == '?'
                || c == '"'
                || c == '<'
                || c == '>'
                || c == '|'
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn escape_yaml_string(s: &str) -> String {
    if s.contains(':') || s.contains('#') || s.contains('"') || s.contains('\'') || s.contains('\n')
    {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn extract_tags_from_note_content(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') && !line.starts_with("##") {
            let tag = line.trim_start_matches('#').trim().to_string();
            if !tag.is_empty() {
                tags.push(tag);
            }
        }
    }
    tags
}

fn markdown_to_simple_html(
    md: &str,
    note_titles: &std::collections::HashMap<String, String>,
) -> String {
    let mut html = String::new();
    let mut in_list = false;
    let mut in_code_block = false;
    let mut code_content = String::new();

    for line in md.lines() {
        if line.trim().starts_with("```") {
            if in_code_block {
                html.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(&code_content)));
                code_content.clear();
                in_code_block = false;
            } else {
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("### ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(stripped)));
        } else if let Some(stripped) = trimmed.strip_prefix("## ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", escape_html(stripped)));
        } else if let Some(stripped) = trimmed.strip_prefix("# ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(stripped)));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            let item_text = inline_markdown_to_html(&trimmed[2..], note_titles);
            html.push_str(&format!("<li>{}</li>\n", item_text));
        } else if let Some(stripped) = trimmed.strip_prefix("> ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                inline_markdown_to_html(stripped, note_titles)
            ));
        } else {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<p>{}</p>\n", inline_markdown_to_html(trimmed, note_titles)));
        }
    }

    if in_list {
        html.push_str("</ul>\n");
    }
    if in_code_block {
        html.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(&code_content)));
    }

    html
}

fn inline_markdown_to_html(
    text: &str,
    note_titles: &std::collections::HashMap<String, String>,
) -> String {
    let result = escape_html(text);
    let result = replace_wikilinks(&result, note_titles);
    let result = replace_inline_pairs(&result, "**", "<strong>", "</strong>");
    let result = replace_inline_pairs(&result, "*", "<em>", "</em>");
    let result = replace_inline_backticks(&result);
    replace_inline_links(&result)
}

fn replace_wikilinks(
    text: &str,
    note_titles: &std::collections::HashMap<String, String>,
) -> String {
    let mut result = String::new();
    let text_len = text.len();

    let bytes: &[u8] = text.as_bytes();
    let mut pos = 0usize;

    while pos < text_len {
        if bytes[pos] == b'[' && pos + 1 < text_len && bytes[pos + 1] == b'[' {
            if let Some(end) = find_closing_brackets(bytes, pos + 2) {
                let link_text = &text[pos + 2..end];
                let file_name = note_titles
                    .get(link_text)
                    .cloned()
                    .unwrap_or_else(|| sanitize_filename(link_text));
                result.push_str(&format!(
                    "<a href=\"{}.html\" class=\"wikilink\">{}</a>",
                    file_name, link_text
                ));
                pos = end + 2;
                continue;
            }
        }
        result.push(bytes[pos] as char);
        pos += 1;
    }

    result
}

fn find_closing_brackets(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 1 < bytes.len() {
        if bytes[pos] == b']' && bytes[pos + 1] == b']' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn replace_inline_pairs(text: &str, marker: &str, open_tag: &str, close_tag: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;
    let marker_len = marker.len();

    while let Some(start) = remaining.find(marker) {
        result.push_str(&remaining[..start]);
        let after_first = &remaining[start + marker_len..];
        if let Some(end) = after_first.find(marker) {
            let inner = &after_first[..end];
            result.push_str(open_tag);
            result.push_str(inner);
            result.push_str(close_tag);
            remaining = &after_first[end + marker_len..];
        } else {
            result.push_str(marker);
            remaining = after_first;
        }
    }
    result.push_str(remaining);
    result
}

fn replace_inline_backticks(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find('`') {
        result.push_str(&remaining[..start]);
        let after_first = &remaining[start + 1..];
        if let Some(end) = after_first.find('`') {
            let inner = &after_first[..end];
            result.push_str("<code>");
            result.push_str(inner);
            result.push_str("</code>");
            remaining = &after_first[end + 1..];
        } else {
            result.push('`');
            remaining = after_first;
        }
    }
    result.push_str(remaining);
    result
}

fn replace_inline_links(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find('[') {
        result.push_str(&remaining[..start]);
        let after_bracket = &remaining[start + 1..];

        if let Some(close_bracket) = after_bracket.find(']') {
            let link_text = &after_bracket[..close_bracket];
            let after_close = &after_bracket[close_bracket + 1..];

            if let Some(after_open) = after_close.strip_prefix('(') {
                if let Some(close_paren) = after_open.find(')') {
                    let url = &after_open[..close_paren];
                    result.push_str(&format!("<a href=\"{}\">{}</a>", url, link_text));
                    remaining = &after_close[close_paren + 2..];
                    continue;
                }
            }
            result.push('[');
            remaining = after_bracket;
        } else {
            result.push('[');
            remaining = after_bracket;
        }
    }
    result.push_str(remaining);
    result
}
