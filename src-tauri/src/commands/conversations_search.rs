use crate::AppState;
use sea_orm::ConnectionTrait;
use tauri::State;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSearchResult {
    pub conversation_id: String,
    pub conversation_title: String,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}

#[tauri::command]
pub async fn session_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SessionSearchResult>, String> {
    let max = limit.unwrap_or(10);

    let rows = state
        .harness
        .db()
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT \
                m.conversation_id, \
                m.role, \
                snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, \
                bm25(messages_fts) as rank \
            FROM messages_fts \
            JOIN messages m ON m.rowid = messages_fts.rowid \
            WHERE messages_fts MATCH ? \
            ORDER BY rank \
            LIMIT ?",
            vec![query.into(), (max as i64).into()],
        ))
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let conversation_id: String = row
            .try_get("", "conversation_id")
            .map_err(|e| e.to_string())?;
        let role: String = row.try_get("", "role").map_err(|e| e.to_string())?;
        let snippet: String = row.try_get("", "snippet").map_err(|e| e.to_string())?;
        let rank: f64 = row.try_get("", "rank").map_err(|e| e.to_string())?;

        let title = axagent_core::repo::conversation::get_conversation(
            state.harness.db(),
            &conversation_id,
        )
        .await
        .ok()
        .map(|c| c.title)
        .unwrap_or_else(|| "Unknown".to_string());

        results.push(SessionSearchResult {
            conversation_id,
            conversation_title: title,
            role,
            snippet,
            rank,
        });
    }

    Ok(results)
}
