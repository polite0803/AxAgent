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
use std::sync::{Arc, RwLock};
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
    conn: Arc<RwLock<Connection>>,
    config: FTS5Config,
}

impl FTS5Search {
    pub fn new(conn: Arc<RwLock<Connection>>, config: FTS5Config) -> Self {
        Self { conn, config }
    }

    pub fn create_fts_tables(&self) -> Result<()> {
        let conn = self.conn.read().unwrap();

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

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                id UNINDEXED,
                memory_type UNINDEXED,
                content,
                entities,
                created_at UNINDEXED,
                tokenize='porter unicode61'
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
                id UNINDEXED,
                name,
                description,
                content,
                category UNINDEXED,
                tags,
                created_at UNINDEXED,
                tokenize='porter unicode61'
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                id UNINDEXED,
                session_id UNINDEXED,
                role UNINDEXED,
                content,
                created_at UNINDEXED,
                tokenize='porter unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS trajectories_ai AFTER INSERT ON trajectories_fts BEGIN
                INSERT INTO trajectories_fts(rowid, id, session_id, topic, summary, content, outcome, quality_score, created_at)
                VALUES (NEW.rowid, NEW.id, NEW.session_id, NEW.topic, NEW.summary, NEW.content, NEW.outcome, NEW.quality_score, NEW.created_at);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories_fts BEGIN
                INSERT INTO memories_fts(rowid, id, memory_type, content, entities, created_at)
                VALUES (NEW.rowid, NEW.id, NEW.memory_type, NEW.content, NEW.entities, NEW.created_at);
            END;

            CREATE TRIGGER IF NOT EXISTS skills_ai AFTER INSERT ON skills_fts BEGIN
                INSERT INTO skills_fts(rowid, id, name, description, content, category, tags, created_at)
                VALUES (NEW.rowid, NEW.id, NEW.name, NEW.description, NEW.content, NEW.category, NEW.tags, NEW.created_at);
            END;
            "#,
        )
        .context("Failed to create FTS5 tables")?;

        info!("FTS5 tables created successfully");
        Ok(())
    }

    pub fn index_trajectory(&self, trajectory: &Trajectory, session_id: &str) -> Result<()> {
        let conn = self.conn.read().unwrap();

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
    }

    pub fn index_memory(
        &self,
        id: &str,
        memory_type: &str,
        content: &str,
        entities: &[String],
    ) -> Result<()> {
        let conn = self.conn.read().unwrap();

        conn.execute(
            r#"INSERT INTO memories_fts (id, memory_type, content, entities, created_at)
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
    }

    pub fn index_skill(
        &self,
        id: &str,
        name: &str,
        description: &str,
        content: &str,
        category: &str,
        tags: &[String],
    ) -> Result<()> {
        let conn = self.conn.read().unwrap();

        conn.execute(
            r#"INSERT INTO skills_fts (id, name, description, content, category, tags, created_at)
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
    }

    pub fn search(&self, query: FTS5Query) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.read().unwrap();
        let mut results = Vec::new();

        let tables = if let Some(ref filter) = query.filter_type {
            vec![filter.clone()]
        } else {
            vec![
                "trajectories_fts".to_string(),
                "memories_fts".to_string(),
                "skills_fts".to_string(),
                "messages_fts".to_string(),
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
                }
                "memories_fts" => {
                    r#"
                    SELECT 
                        m.id,
                        'memory' as doc_type,
                        m.content,
                        NULL as session_id,
                        m.created_at,
                        NULL as quality_score,
                        NULL as outcome,
                        bm25(memories_fts) as rank
                    FROM memories_fts m
                    WHERE memories_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2 OFFSET ?3
                    "#
                }
                "skills_fts" => {
                    r#"
                    SELECT 
                        s.id,
                        'skill' as doc_type,
                        s.name || ' ' || s.description || ' ' || s.content as content,
                        NULL as session_id,
                        s.created_at,
                        NULL as quality_score,
                        NULL as outcome,
                        bm25(skills_fts) as rank
                    FROM skills_fts s
                    WHERE skills_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2 OFFSET ?3
                    "#
                }
                "messages_fts" => {
                    r#"
                    SELECT 
                        m.id,
                        'message' as doc_type,
                        m.content,
                        m.session_id,
                        m.created_at,
                        NULL as quality_score,
                        NULL as outcome,
                        bm25(messages_fts) as rank
                    FROM messages_fts m
                    WHERE messages_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2 OFFSET ?3
                    "#
                }
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
                result.snippet = self.generate_snippet(&result.content, &query.query);
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
    }

    pub fn search_phrase(&self, phrase: &str, in_field: &str) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.read().unwrap();

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

        let query = format!("\"{}\"", phrase.replace("\"", "\"\""));
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
            result.snippet = self.generate_snippet(&result.content, phrase);
        }

        Ok(results)
    }

    pub fn search_proximity(
        &self,
        term1: &str,
        term2: &str,
        distance: i32,
    ) -> Result<Vec<FTS5Result>> {
        let conn = self.conn.read().unwrap();

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
            result.snippet = self.generate_snippet(
                &result.content,
                &format!("{} NEAR/{} {}", term1, distance, term2),
            );
        }

        Ok(results)
    }

    fn generate_snippet(&self, content: &str, query: &str) -> String {
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
        let end = std::cmp::min(start + self.config.snippet_size, content.len());

        let mut snippet = content[start..end].to_string();

        for term in query_terms {
            let pattern = format!("(?i){}", regex::escape(term));
            if let Ok(re) = regex::Regex::new(&pattern) {
                snippet = re
                    .replace_all(
                        &snippet,
                        format!(
                            "{}{}{}",
                            self.config.highlight_open, term, self.config.highlight_close
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

    pub fn optimize(&self) -> Result<()> {
        let conn = self.conn.read().unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO trajectories_fts(trajectories_fts) VALUES('optimize');
            INSERT INTO memories_fts(memories_fts) VALUES('optimize');
            INSERT INTO skills_fts(skills_fts) VALUES('optimize');
            INSERT INTO messages_fts(messages_fts) VALUES('optimize');
            "#,
        )?;
        info!("FTS5 indexes optimized");
        Ok(())
    }

    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.read().unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO trajectories_fts(trajectories_fts) VALUES('vacuum');
            INSERT INTO memories_fts(memories_fts) VALUES('vacuum');
            INSERT INTO skills_fts(skills_fts) VALUES('vacuum');
            INSERT INTO messages_fts(messages_fts) VALUES('vacuum');
            "#,
        )?;
        info!("FTS5 indexes vacuumed");
        Ok(())
    }
}
