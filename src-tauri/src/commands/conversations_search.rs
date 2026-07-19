// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use tauri::State;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSearchResult {
    pub conversation_id: String,
    pub conversation_title: String,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}

/// 全文搜索会话消息
///
/// 支持：regex / case_sensitive / session_filter / date_from / date_to / offset / limit
#[tauri::command]
pub async fn session_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
    regex: Option<bool>,
    case_sensitive: Option<bool>,
    session_filter: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    offset: Option<u32>,
) -> Result<Vec<SessionSearchResult>, String> {
    let is_regex = regex.unwrap_or(false);
    let is_case_sensitive = case_sensitive.unwrap_or(false);
    let max = limit.unwrap_or(10);
    let off = offset.unwrap_or(0);

    let db = state.harness.db();

    if db.get_database_backend() == DbBackend::Postgres {
        search_postgres(
            db,
            &query,
            is_regex,
            is_case_sensitive,
            session_filter,
            date_from,
            date_to,
            max,
            off,
        )
        .await
    } else {
        search_sqlite(
            db,
            &query,
            is_regex,
            is_case_sensitive,
            session_filter,
            date_from,
            date_to,
            max,
            off,
        )
        .await
    }
}

// ── PostgreSQL ──────────────────────────────────────────────

async fn search_postgres(
    db: &sea_orm::DatabaseConnection,
    query: &str,
    is_regex: bool,
    is_case_sensitive: bool,
    session_filter: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Vec<SessionSearchResult>, String> {
    let mut wheres: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    let mut param_idx = 1u32;

    if is_regex {
        // Regex 模式：使用 content ~ 'pattern' 而非 tsquery
        wheres.push(format!("m.content ~ ${}", param_idx));
        values.push(query.to_string().into());
        param_idx += 1;
    } else if is_case_sensitive {
        wheres.push(format!("m.content_tsv @@ plainto_tsquery('english', ${})", param_idx));
        values.push(query.to_string().into());
        param_idx += 1;
    } else {
        wheres.push(format!("m.content_tsv @@ plainto_tsquery('simple', ${})", param_idx));
        values.push(query.to_string().into());
        param_idx += 1;
    }

    if let Some(filter) = session_filter {
        if !filter.is_empty() {
            // ANY(array) — PostgreSQL 数组绑定
            let placeholders: Vec<String> = filter
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let idx = param_idx + i as u32;
                    format!("${}", idx)
                })
                .collect();
            wheres.push(format!("m.conversation_id = ANY(ARRAY[{}])", placeholders.join(",")));
            for id in &filter {
                values.push(id.into());
            }
            param_idx += filter.len() as u32;
        }
    }

    if let Some(from) = date_from {
        if let Ok(ts) = parse_iso_timestamp(&from) {
            wheres.push(format!("m.created_at >= ${}", param_idx));
            values.push(ts.into());
            param_idx += 1;
        }
    }

    if let Some(to) = date_to {
        if let Ok(ts) = parse_iso_timestamp(&to) {
            wheres.push(format!("m.created_at <= ${}", param_idx));
            values.push(ts.into());
            param_idx += 1;
        }
    }

    let where_clause = if wheres.is_empty() {
        "TRUE".to_string()
    } else {
        wheres.join(" AND ")
    };

    // 根据查询模式选择 SELECT 列
    let snippet_col = if is_regex {
        // Regex 没有 ts_headline，直接用 content 截取
        "SUBSTRING(m.content, 1, 200) as snippet".to_string()
    } else if is_case_sensitive {
        format!(
            "ts_headline('english', m.content, plainto_tsquery('english', ${query_idx}), 'MaxWords=24, MinWords=5') as snippet",
            query_idx = 1
        )
    } else {
        format!(
            "ts_headline('simple', m.content, plainto_tsquery('simple', ${query_idx}), 'MaxWords=24, MinWords=5') as snippet",
            query_idx = 1
        )
    };

    let rank_col = if is_regex {
        "0.0 as rank".to_string()
    } else if is_case_sensitive {
        format!("ts_rank(m.content_tsv, plainto_tsquery('english', ${})) as rank", 1)
    } else {
        format!("ts_rank(m.content_tsv, plainto_tsquery('simple', ${})) as rank", 1)
    };

    let limit_idx = param_idx;
    values.push((limit as i64).into());
    param_idx += 1;

    let offset_idx = param_idx;
    values.push((offset as i64).into());

    let sql = format!(
        "SELECT \
            m.conversation_id, \
            c.title as conversation_title, \
            m.role, \
            {snippet_col}, \
            {rank_col} \
        FROM messages m \
        JOIN conversations c ON c.id = m.conversation_id \
        WHERE {where_clause} \
        ORDER BY rank DESC \
        LIMIT ${limit_idx} OFFSET ${offset_idx}"
    );

    execute_raw_query(db, DbBackend::Postgres, &sql, values).await
}

// ── SQLite ──────────────────────────────────────────────────

async fn search_sqlite(
    db: &sea_orm::DatabaseConnection,
    query: &str,
    is_regex: bool,
    is_case_sensitive: bool,
    session_filter: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Vec<SessionSearchResult>, String> {
    if is_regex {
        // SQLite 的 FTS5 不支持正则。退化为 LIKE 模式。
        return search_sqlite_like(
            db,
            query,
            is_case_sensitive,
            session_filter,
            date_from,
            date_to,
            limit,
            offset,
        )
        .await;
    }

    let mut wheres: Vec<String> = vec!["messages_fts MATCH ?".to_string()];
    let mut values: Vec<Value> = Vec::new();
    values.push(query.to_string().into());

    if let Some(filter) = session_filter {
        if !filter.is_empty() {
            let placeholders: Vec<&str> = filter.iter().map(|_| "?").collect();
            wheres.push(format!("m.conversation_id IN ({})", placeholders.join(",")));
            for id in &filter {
                values.push(id.into());
            }
        }
    }

    if let Some(from) = date_from {
        if let Ok(ts) = parse_iso_timestamp(&from) {
            wheres.push("m.created_at >= ?".to_string());
            values.push(ts.into());
        }
    }

    if let Some(to) = date_to {
        if let Ok(ts) = parse_iso_timestamp(&to) {
            wheres.push("m.created_at <= ?".to_string());
            values.push(ts.into());
        }
    }

    let where_clause = wheres.join(" AND ");

    values.push((limit as i64).into());
    values.push((offset as i64).into());

    let sql = format!(
        "SELECT \
            m.conversation_id, \
            c.title as conversation_title, \
            m.role, \
            snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, \
            bm25(messages_fts) as rank \
        FROM messages_fts \
        JOIN messages m ON m.rowid = messages_fts.rowid \
        JOIN conversations c ON c.id = m.conversation_id \
        WHERE {where_clause} \
        ORDER BY rank \
        LIMIT ? OFFSET ?"
    );

    execute_raw_query(db, DbBackend::Sqlite, &sql, values).await
}

/// SQLite 回退模式：用 LIKE 替代 FTS5（用于 regex/ 降级）
async fn search_sqlite_like(
    db: &sea_orm::DatabaseConnection,
    query: &str,
    _is_case_sensitive: bool,
    session_filter: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Vec<SessionSearchResult>, String> {
    let mut wheres: Vec<String> = vec!["m.content LIKE ?".to_string()];
    let mut values: Vec<Value> = Vec::new();
    values.push(format!("%{}%", query.replace('%', "\\%").replace('_', "\\_")).into());

    if let Some(filter) = session_filter {
        if !filter.is_empty() {
            let placeholders: Vec<&str> = filter.iter().map(|_| "?").collect();
            wheres.push(format!("m.conversation_id IN ({})", placeholders.join(",")));
            for id in &filter {
                values.push(id.into());
            }
        }
    }

    if let Some(from) = date_from {
        if let Ok(ts) = parse_iso_timestamp(&from) {
            wheres.push("m.created_at >= ?".to_string());
            values.push(ts.into());
        }
    }

    if let Some(to) = date_to {
        if let Ok(ts) = parse_iso_timestamp(&to) {
            wheres.push("m.created_at <= ?".to_string());
            values.push(ts.into());
        }
    }

    let where_clause = wheres.join(" AND ");

    values.push((limit as i64).into());
    values.push((offset as i64).into());

    let sql = format!(
        "SELECT \
            m.conversation_id, \
            c.title as conversation_title, \
            m.role, \
            SUBSTR(m.content, 1, 200) as snippet, \
            0.0 as rank \
        FROM messages m \
        JOIN conversations c ON c.id = m.conversation_id \
        WHERE {where_clause} \
        ORDER BY m.created_at DESC \
        LIMIT ? OFFSET ?"
    );

    execute_raw_query(db, DbBackend::Sqlite, &sql, values).await
}

// ── Common ──────────────────────────────────────────────────

fn parse_iso_timestamp(s: &str) -> Result<i64, String> {
    // 支持 ISO 8601 格式："2026-07-14T00:00:00Z" 或 "2026-07-14"
    let dt = if s.len() <= 10 {
        // 仅日期，补充时间
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| format!("parse date failed: {e}"))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| "invalid time".to_string())?
            .and_utc()
    } else {
        chrono::DateTime::parse_from_rfc3339(s)
            .map_err(|e| format!("parse datetime failed: {e}"))?
            .with_timezone(&chrono::Utc)
    };
    Ok(dt.timestamp())
}

async fn execute_raw_query(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    sql: &str,
    values: Vec<Value>,
) -> Result<Vec<SessionSearchResult>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(backend, sql, values))
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let conversation_id: String = row.try_get("", "conversation_id").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        let conversation_title: String = row.try_get("", "conversation_title").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        let role: String = row.try_get("", "role").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        let snippet: String = row.try_get("", "snippet").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        let rank: f64 = row.try_get("", "rank").map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

        results.push(SessionSearchResult {
            conversation_id,
            conversation_title,
            role,
            snippet,
            rank,
        });
    }

    Ok(results)
}
