// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchResult {
    pub conversation_id: String,
    pub conversation_title: String,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}

/// 搜索结果（含谱系信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultWithLineage {
    pub result: SessionSearchResult,
    /// 父会话 ID（如果存在）
    pub parent_conversation_id: Option<String>,
    /// 完整会话谱系（从根到当前）
    pub lineage: Vec<LineageNode>,
}

/// 谱系节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub conversation_id: String,
    pub title: String,
    pub is_root: bool,
}

/// LLM 摘要结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSummaryResult {
    pub query: String,
    pub total_matches: usize,
    pub summary: String,
    pub key_findings: Vec<String>,
    pub search_results: Vec<SearchResultWithLineage>,
}

/// CJK token 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CjkToken {
    pub original: String,
    pub trigram: String,
    pub language: CjkLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CjkLanguage {
    Chinese,
    Japanese,
    Korean,
    None,
}

/// 全文搜索会话消息
///
/// 支持：regex / case_sensitive / session_filter / date_from / date_to / offset / limit
#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "搜索对话会话消息")]
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

// ========================================================================
// P0-3 增强功能：CJK 支持、会话谱系搜索、LLM 摘要召回
// ========================================================================

/// 分析文本是否包含 CJK 字符
pub fn detect_cjk(text: &str) -> Vec<CjkToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let lang = classify_cjk_char(ch);

        if lang != CjkLanguage::None {
            // 提取连续 CJK 字符序列
            let mut seq = String::new();
            while i < chars.len() && classify_cjk_char(chars[i]) == lang {
                seq.push(chars[i]);
                i += 1;
            }

            // 对序列生成 trigram
            if seq.len() >= 3 {
                for j in 0..=(seq.len() - 3) {
                    let trigram: String = seq.chars().skip(j).take(3).collect();
                    tokens.push(CjkToken {
                        original: seq.clone(),
                        trigram,
                        language: lang.clone(),
                    });
                }
            } else if seq.len() >= 2 {
                tokens.push(CjkToken {
                    original: seq.clone(),
                    trigram: seq.clone(),
                    language: lang,
                });
            }
        } else {
            i += 1;
        }
    }

    tokens
}

/// 分类 CJK 字符
fn classify_cjk_char(ch: char) -> CjkLanguage {
    let cp = ch as u32;
    // CJK Unified Ideographs Extension A
    if (0x3400..=0x4DBF).contains(&cp) || (0x20000..=0x2A6DF).contains(&cp) {
        return CjkLanguage::Chinese;
    }
    // CJK Unified Ideographs
    if (0x4E00..=0x9FFF).contains(&cp) {
        return CjkLanguage::Chinese;
    }
    // Hiragana + Katakana
    if (0x3040..=0x309F).contains(&cp) || (0x30A0..=0x30FF).contains(&cp) {
        return CjkLanguage::Japanese;
    }
    // Hangul Syllables
    if (0xAC00..=0xD7AF).contains(&cp) || (0x1100..=0x11FF).contains(&cp) {
        return CjkLanguage::Korean;
    }
    CjkLanguage::None
}

/// 构建 FTS5 查询，支持 CJK trigram 匹配
fn build_fts5_query_cjk(query: &str) -> String {
    let tokens = detect_cjk(query);
    if tokens.is_empty() {
        // 非 CJK 文本，正常分词
        return query
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|tok| format!("\"{}\"", tok.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
    }

    // CJK 文本：使用 trigram 匹配
    let trigrams: Vec<String> =
        tokens.iter().map(|t| format!("\"{}\"", t.trigram.replace('"', ""))).collect();

    if trigrams.is_empty() {
        return String::new();
    }

    trigrams.join(" OR ")
}

/// 使用 CJK 增强的 FTS5 搜索
async fn search_sqlite_cjk(
    db: &sea_orm::DatabaseConnection,
    query: &str,
    session_filter: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Vec<SessionSearchResult>, String> {
    let fts_query = build_fts5_query_cjk(query);
    if fts_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut wheres: Vec<String> = vec!["messages_fts MATCH ?".to_string()];
    let mut values: Vec<Value> = Vec::new();
    values.push(fts_query.into());

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

/// 获取会话谱系
async fn get_conversation_lineage(
    db: &sea_orm::DatabaseConnection,
    conversation_id: &str,
) -> Result<(Option<String>, Vec<LineageNode>), String> {
    let mut lineage = Vec::new();
    let mut current_id = conversation_id.to_string();
    let mut parent_id = None;

    // 向上遍历最多 10 层
    for _ in 0..10 {
        let sql = "SELECT id, title, parent_conversation_id FROM conversations WHERE id = ?";
        let result = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                sql,
                vec![current_id.clone().into()],
            ))
            .await;

        match result {
            Ok(Some(row)) => {
                let id: String = row.try_get("", "id").map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                let title: String = row.try_get("", "title").map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                let parent: Option<String> =
                    row.try_get("", "parent_conversation_id").ok().flatten();

                let is_root = parent.is_none();
                lineage.push(LineageNode { conversation_id: id, title, is_root });

                match parent {
                    Some(pid) => {
                        parent_id = Some(pid.clone());
                        current_id = pid;
                    },
                    None => break,
                }
            },
            _ => break,
        }
    }

    Ok((parent_id, lineage))
}

/// 带谱系的会话搜索
#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "搜索会话并返回谱系信息")]
#[tauri::command]
pub async fn session_search_with_lineage(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
    include_lineage: Option<bool>,
) -> Result<Vec<SearchResultWithLineage>, String> {
    let max = limit.unwrap_or(10);
    let off = offset.unwrap_or(0);
    let with_lineage = include_lineage.unwrap_or(true);

    let db = state.harness.db();
    let results = search_sqlite_cjk(db, &query, None, None, None, max, off).await?;

    let mut results_with_lineage = Vec::with_capacity(results.len());
    for result in results {
        let (parent_id, lineage) = if with_lineage {
            get_conversation_lineage(db, &result.conversation_id).await.unwrap_or_default()
        } else {
            (None, Vec::new())
        };

        results_with_lineage.push(SearchResultWithLineage {
            result,
            parent_conversation_id: parent_id,
            lineage,
        });
    }

    Ok(results_with_lineage)
}

/// 带结构化摘要的会话搜索
///
/// 搜索后生成结构化摘要和关键发现（含会话谱系）。
/// 注：当前为命令层模板摘要；后续可接 auxiliary_client 升级为 LLM 摘要。
#[agent_command(domain = conversations, safety = Caution, call_mode = StateInput, description = "搜索会话并生成结构化摘要")]
#[tauri::command]
pub async fn session_search_with_summary(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<SearchSummaryResult, String> {
    let max = limit.unwrap_or(10);

    let db = state.harness.db();
    let results = search_sqlite_cjk(db, &query, None, None, None, max, 0).await?;

    let total_matches = results.len();

    // 获取谱系信息
    let mut results_with_lineage = Vec::with_capacity(results.len());
    for result in &results {
        let (parent_id, lineage) =
            get_conversation_lineage(db, &result.conversation_id).await.unwrap_or_default();

        results_with_lineage.push(SearchResultWithLineage {
            result: result.clone(),
            parent_conversation_id: parent_id,
            lineage,
        });
    }

    // 生成搜索结果摘要（在命令层生成结构化摘要，无需 LLM）
    let summary = generate_search_summary(&query, &results_with_lineage);
    let key_findings = extract_key_findings(&results_with_lineage);

    Ok(SearchSummaryResult {
        query,
        total_matches,
        summary,
        key_findings,
        search_results: results_with_lineage,
    })
}

/// 生成搜索摘要
fn generate_search_summary(query: &str, results: &[SearchResultWithLineage]) -> String {
    if results.is_empty() {
        return format!("未找到与 \"{}\" 相关的会话。", query);
    }

    let conversation_ids: Vec<String> = results
        .iter()
        .map(|r| r.result.conversation_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let titles: Vec<String> = results
        .iter()
        .map(|r| r.result.conversation_title.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let roles: Vec<String> = results
        .iter()
        .map(|r| r.result.role.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let lineage_count = results.iter().filter(|r| !r.lineage.is_empty()).count();

    format!(
        "搜索 \"{}\" 共找到 {} 条匹配，分布在 {} 个会话中。\n\n\
         涉及的会话：{}\n\n\
         匹配角色：{}\n\n\
         包含谱系信息的会话：{} 个\n\n\
         相关度最高的匹配片段：\n{}",
        query,
        results.len(),
        conversation_ids.len(),
        titles.iter().take(3).cloned().collect::<Vec<_>>().join("、"),
        roles.join("、"),
        lineage_count,
        results
            .iter()
            .take(3)
            .enumerate()
            .map(|(i, r)| format!(
                "  {}. [{}] ...{}... (相关度: {:.2})",
                i + 1,
                r.result.conversation_title,
                r.result.snippet,
                r.result.rank
            ))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// 提取关键发现
fn extract_key_findings(results: &[SearchResultWithLineage]) -> Vec<String> {
    let mut findings = Vec::new();

    if results.is_empty() {
        return findings;
    }

    // 1. 最相关的会话
    if let Some(top) = results.first() {
        findings.push(format!(
            "最相关会话：\"{}\"，相关度 {:.2}",
            top.result.conversation_title, top.result.rank
        ));
    }

    // 2. 涉及的用户/助手消息比例
    let user_count = results.iter().filter(|r| r.result.role == "user").count();
    let assistant_count = results.iter().filter(|r| r.result.role == "assistant").count();
    findings.push(format!("角色分布：用户消息 {} 条，助手消息 {} 条", user_count, assistant_count));

    // 3. 谱系信息
    let with_lineage = results.iter().filter(|r| !r.lineage.is_empty()).count();
    if with_lineage > 0 {
        findings.push(format!("{} 个会话包含谱系信息，可追溯上下文历史", with_lineage));
    }

    findings
}
