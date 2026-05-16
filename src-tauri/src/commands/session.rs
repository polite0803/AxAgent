use crate::commands::conversations_search::{
    session_search as inner_session_search, SessionSearchResult,
};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn session_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SessionSearchResult>, String> {
    tracing::warn!("session_search is a stub delegating to conversations_search::session_search");
    inner_session_search(state, query, limit).await
}
