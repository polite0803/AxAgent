//! FTS5 Full-Text Search module for enhanced cross-session retrieval
//!
//! Features:
//! - SQLite FTS5 virtual tables for trajectories, memories, skills
//! - BM25 ranking with configurable parameters
//! - Phrase matching and proximity search
//! - Snippet generation with highlight markers

use crate::trajectory::{Trajectory, TrajectoryOutcome};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTS5Config {
    pub bm25_k1: f64,
    pub bm25_b: f64,
    pub snippet_size: usize,
    pub highlight_open: String,
    pub highlight_close: String,
}

impl Default for FTS5Config {
    fn default() -> Self {
        Self {
            bm25_k1: 1.5,
            bm25_b: 0.75,
            snippet_size: 300,
            highlight_open: "【".to_string(),
            highlight_close: "】".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTS5Query {
    pub query: String,
    pub limit: usize,
    pub offset: usize,
    pub filter_type: Option<String>,
    pub filter_session_id: Option<String>,
    pub min_relevance: Option<f64>,
}

impl Default for FTS5Query {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 10,
            offset: 0,
            filter_type: None,
            filter_session_id: None,
            min_relevance: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTS5Result {
    pub id: String,
    pub doc_type: String,
    pub content: String,
    pub snippet: String,
    pub rank: f64,
    pub bm25_score: f64,
    pub session_id: Option<String>,
    pub timestamp: i64,
    pub metadata: Option<String>,
}

pub struct FTS5Search {
    conn: Arc<Mutex<Connection>>,
    config: FTS5Config,
}

impl FTS5Search {
    pub fn new(conn: Arc<Mutex<Connection>>, config: FTS5Config) -> Self {
        Self { conn, config }
    }

    pub async fn create_fts_tables(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute_batch(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS trajectories_fts USING fts5(
                    id UNINDEXED,
                    session_id UNINDEXED,
                    topic,
                    summary,
                    content,
                    outcome UNINDEXED,
                    quality_score UNINDEXED,
                    created_at UNINDEXED,
                    tokenize='porter unicode61'
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_memories_fts USING fts5(
                    id UNINDEXED,
                    memory_type UNINDEXED,
                    content,
                    entities,
                    created_at UNINDEXED,
                    tokenize='porter unicode61'
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_skills_fts USING fts5(
                    id UNINDEXED,
                    name,
                    description,
                    content,
                    category UNINDEXED,
                    tags,
                    created_at UNINDEXED,
                    tokenize='porter unicode61'
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_messages_fts USING fts5(
                    id UNINDEXED,
                    session_id UNINDEXED,
                    role UNINDEXED,
                    content,
                    created_at UNINDEXED,
                    tokenize='porter unicode61'
                );

                "#,
            )
            .context("Failed to create FTS5 tables")?;

            info!("FTS5 tables created successfully");
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn index_trajectory(&self, trajectory: &Trajectory, session_id: &str) -> Result<()> {
        let conn = self.conn.clone();
        let trajectory = trajectory.clone();
        let session_id = session_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();

            let content = trajectory
                .steps
                .iter()
                .map(|s| s.content.clone())
                .collect::<Vec<_>>()
                .join("\n");

            let outcome_str = match trajectory.outcome {
                TrajectoryOutcome::Success => "success",
                TrajectoryOutcome::Partial => "partial",
                TrajectoryOutcome::Failure => "failure",
                TrajectoryOutcome::Abandoned => "abandoned",
            };

            conn.execute(
                r#"INSERT INTO trajectories_fts (id, session_id, topic, summary, content, outcome, quality_score, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                params![
                    trajectory.id,
                    session_id,
                    trajectory.topic,
                    trajectory.summary,
                    content,
                    outcome_str,
                    trajectory.quality.overall,
                    trajectory.created_at.timestamp()
                ],
            )?;

            debug!("Indexed trajectory {} for FTS5", trajectory.id);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn index_memory(
        &self,
        id: &str,
        memory_type: &str,
        content: &str,
        entities: &[String],
    ) -> Result<()> {
        let conn = self.conn.clone();
        let id = id.to_string();
        let memory_type = memory_type.to_string();
        let content = content.to_string();
        let entities = entities.to_vec();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();

            conn.execute(
                r#"INSERT INTO trajectory_memories_fts (id, memory_type, content, entities, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5)"#,
                params![
                    id,
                    memory_type,
                    content,
                    entities.join(" "),
                    chrono::Utc::now().timestamp()
                ],
            )?;

            debug!("Indexed memory {} for FTS5", id);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn index_skill(
        &self,
        id: &str,
        name: &str,
        description: &str,
        content: &str,
        category: &str,
        tags: &[String],
    ) -> Result<()> {
        let conn = self.conn.clone();
        let id = id.to_string();
        let name = name.to_string();
        let description = description.to_string();
        let content = content.to_string();
        let category = category.to_string();
        let tags = tags.to_vec();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();

            conn.execute(
                r#"INSERT INTO trajectory_skills_fts (id, name, description, content, category, tags, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                params![
                    id,
                    name,
                    description,
                    content,
                    category,
                    tags.join(" "),
                    chrono::Utc::now().timestamp()
                ],
            )?;

            debug!("Indexed skill {} for FTS5", id);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn index_message(&self, msg: &crate::storage::Message) -> Result<()> {
        let conn = self.conn.clone();
        let msg = msg.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute(
                r#"INSERT INTO trajectory_messages_fts (id, session_id, role, content, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5)"#,
                params![
                    msg.id,
                    msg.session_id,
                    msg.role,
                    msg.content,
                    msg.created_at.timestamp()
                ],
            )?;
            debug!("Indexed message {} for FTS5", msg.id);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn delete_from_fts(&self, table: &str, id: &str) -> Result<()> {
        let conn = self.conn.clone();
        let table = table.to_string();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let sql = format!("INSERT INTO {}({}, id, content) VALUES('delete', ?1, ?2)", table, table);
            conn.execute(&sql, params![id, ""])?;
            debug!("Deleted {} from FTS5 table {}", id, table);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn search(&self, query: FTS5Query) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.clone();
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let mut results = Vec::new();

            let tables = if let Some(ref filter) = query.filter_type {
                vec![filter.clone()]
            } else {
                vec![
                    "trajectories_fts".to_string(),
                    "trajectory_memories_fts".to_string(),
                    "trajectory_skills_fts".to_string(),
                    "trajectory_messages_fts".to_string(),
                ]
            };

            for table in tables {
                let sql = match table.as_str() {
                    "trajectories_fts" => {
                        r#"
                        SELECT 
                            t.id,
                            'trajectory' as doc_type,
                            COALESCE(t.topic, '') || ' ' || COALESCE(t.summary, '') || ' ' || COALESCE(t.content, '') as content,
                            t.session_id,
                            t.created_at,
                            t.quality_score,
                            t.outcome,
                            bm25(trajectories_fts) as rank
                        FROM trajectories_fts t
                        WHERE trajectories_fts MATCH ?1
                        ORDER BY rank
                        LIMIT ?2 OFFSET ?3
                        "#
                    },
                    "trajectory_memories_fts" => {
                        r#"
                        SELECT 
                            m.id,
                            'memory' as doc_type,
                            m.content,
                            NULL as session_id,
                            m.created_at,
                            NULL as quality_score,
                            NULL as outcome,
                            bm25(trajectory_memories_fts) as rank
                        FROM trajectory_memories_fts m
                        WHERE trajectory_memories_fts MATCH ?1
                        ORDER BY rank
                        LIMIT ?2 OFFSET ?3
                        "#
                    },
                    "trajectory_skills_fts" => {
                        r#"
                        SELECT 
                            s.id,
                            'skill' as doc_type,
                            s.name || ' ' || s.description || ' ' || s.content as content,
                            NULL as session_id,
                            s.created_at,
                            NULL as quality_score,
                            NULL as outcome,
                            bm25(trajectory_skills_fts) as rank
                        FROM trajectory_skills_fts s
                        WHERE trajectory_skills_fts MATCH ?1
                        ORDER BY rank
                        LIMIT ?2 OFFSET ?3
                        "#
                    },
                    "trajectory_messages_fts" => {
                        r#"
                        SELECT 
                            m.id,
                            'message' as doc_type,
                            m.content,
                            m.session_id,
                            m.created_at,
                            NULL as quality_score,
                            NULL as outcome,
                            bm25(trajectory_messages_fts) as rank
                        FROM trajectory_messages_fts m
                        WHERE trajectory_messages_fts MATCH ?1
                        ORDER BY rank
                        LIMIT ?2 OFFSET ?3
                        "#
                    },
                    _ => continue,
                };

                let mut stmt = conn.prepare(sql)?;

                let rows = stmt.query_map(
                    params![query.query, query.limit as i64, query.offset as i64],
                    |row| {
                        Ok(FTS5Result {
                            id: row.get(0)?,
                            doc_type: row.get(1)?,
                            content: row.get(2)?,
                            snippet: String::new(),
                            rank: row.get(7)?,
                            bm25_score: row.get(7)?,
                            session_id: row.get(3)?,
                            timestamp: row.get(4)?,
                            metadata: None,
                        })
                    },
                )?;

                for row in rows.filter_map(|r| r.ok()) {
                    let mut result = row;
                    result.snippet = Self::generate_snippet(&result.content, &query.query, &config);
                    results.push(result);
                }
            }

            results.sort_by(|a, b| {
                b.rank
                    .partial_cmp(&a.rank)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            if let Some(min_rel) = query.min_relevance {
                results.retain(|r| -r.rank >= min_rel);
            }

            results.truncate(query.limit);

            Ok(results)
        })
        .await??
    }

    pub async fn search_phrase(&self, phrase: &str, in_field: &str) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.clone();
        let config = self.config.clone();
        let phrase = phrase.to_string();
        let in_field = in_field.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();

            let mut stmt = conn.prepare(&format!(
                r#"
                SELECT 
                    id,
                    '{}' as doc_type,
                    {field},
                    session_id,
                    created_at,
                    quality_score,
                    outcome,
                    bm25({table}) as rank
                FROM {table}
                WHERE {table} MATCH ?1
                ORDER BY rank
                LIMIT 10
                "#,
                in_field,
                field = in_field,
                table = "trajectories_fts"
            ))?;

            let query_str = format!("\"{}\"", phrase.replace("\"", "\"\""));
            let rows = stmt.query_map(params![query_str], |row| {
                Ok(FTS5Result {
                    id: row.get(0)?,
                    doc_type: row.get(1)?,
                    content: row.get(2)?,
                    snippet: String::new(),
                    rank: row.get(7)?,
                    bm25_score: row.get(7)?,
                    session_id: row.get(3)?,
                    timestamp: row.get(4)?,
                    metadata: None,
                })
            })?;

            let mut results: Vec<FTS5Result> = rows.filter_map(|r| r.ok()).collect();
            for result in &mut results {
                result.snippet = Self::generate_snippet(&result.content, &phrase, &config);
            }

            Ok(results)
        })
        .await??
    }

    pub async fn search_proximity(
        &self,
        term1: &str,
        term2: &str,
        distance: i32,
    ) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.clone();
        let config = self.config.clone();
        let term1 = term1.to_string();
        let term2 = term2.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();

            let query = format!("\"{}\" NEAR/{} \"{}\"", term1, distance, term2);

            let mut stmt = conn.prepare(
                r#"
                SELECT 
                    t.id,
                    'trajectory' as doc_type,
                    t.topic || ' ' || t.summary || ' ' || t.content as content,
                    t.session_id,
                    t.created_at,
                    t.quality_score,
                    t.outcome,
                    bm25(trajectories_fts) as rank
                FROM trajectories_fts t
                WHERE trajectories_fts MATCH ?1
                ORDER BY rank
                LIMIT 10
                "#,
            )?;

            let rows = stmt.query_map(params![query], |row| {
                Ok(FTS5Result {
                    id: row.get(0)?,
                    doc_type: row.get(1)?,
                    content: row.get(2)?,
                    snippet: String::new(),
                    rank: row.get(7)?,
                    bm25_score: row.get(7)?,
                    session_id: row.get(3)?,
                    timestamp: row.get(4)?,
                    metadata: None,
                })
            })?;

            let mut results: Vec<FTS5Result> = rows.filter_map(|r| r.ok()).collect();
            for result in &mut results {
                result.snippet = Self::generate_snippet(
                    &result.content,
                    &format!("{} NEAR/{} {}", term1, distance, term2),
                    &config,
                );
            }

            Ok(results)
        })
        .await??
    }

    fn generate_snippet(content: &str, query: &str, config: &FTS5Config) -> String {
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        let mut best_pos = 0;
        let mut best_matches = 0;

        for (i, _) in content.char_indices() {
            let window = &content_lower[i..std::cmp::min(i + 200, content_lower.len())];
            let matches = query_terms
                .iter()
                .filter(|t| window.contains(&t.to_lowercase()))
                .count();
            if matches > best_matches {
                best_matches = matches;
                best_pos = i;
            }
        }

        let start = best_pos.saturating_sub(50);
        let end = std::cmp::min(start + config.snippet_size, content.len());

        let mut snippet = content[start..end].to_string();

        for term in query_terms {
            let pattern = format!("(?i){}", regex::escape(term));
            if let Ok(re) = regex::Regex::new(&pattern) {
                snippet = re
                    .replace_all(
                        &snippet,
                        format!(
                            "{}{}{}",
                            config.highlight_open, term, config.highlight_close
                        ),
                    )
                    .to_string();
            }
        }

        if start > 0 {
            snippet = format!("...{}", snippet);
        }
        if end < content.len() {
            snippet = format!("{}...", snippet);
        }

        snippet
    }

    pub async fn optimize(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute_batch(
                r#"
                INSERT INTO trajectories_fts(trajectories_fts) VALUES('optimize');
                INSERT INTO trajectory_memories_fts(trajectory_memories_fts) VALUES('optimize');
                INSERT INTO trajectory_skills_fts(trajectory_skills_fts) VALUES('optimize');
                INSERT INTO trajectory_messages_fts(trajectory_messages_fts) VALUES('optimize');
                "#,
            )?;
            info!("FTS5 indexes optimized");
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn vacuum(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute_batch(
                r#"
                INSERT INTO trajectories_fts(trajectories_fts) VALUES('vacuum');
                INSERT INTO trajectory_memories_fts(trajectory_memories_fts) VALUES('vacuum');
                INSERT INTO trajectory_skills_fts(trajectory_skills_fts) VALUES('vacuum');
                INSERT INTO trajectory_messages_fts(trajectory_messages_fts) VALUES('vacuum');
                "#,
            )?;
            info!("FTS5 indexes vacuumed");
            Ok(())
        })
        .await??;
        Ok(())
    }
}
