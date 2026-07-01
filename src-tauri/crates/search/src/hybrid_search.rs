// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};

use crate::vector_store::{VectorSearchResult, VectorStore};
use axagent_harness::core_error::{AxAgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub vector_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub combined_score: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum FusionAlgorithm {
    /// Weighted linear combination of normalized scores.
    Weighted,
    /// Reciprocal Rank Fusion — robust to score scale differences, default k=60.
    #[default]
    Rrf,
}

#[derive(Debug, Clone)]
pub struct HybridSearchOptions {
    pub vector_weight: f32,
    pub bm25_weight: f32,
    pub top_k: usize,
    pub min_score: Option<f32>,
    pub fusion: FusionAlgorithm,
    pub rrf_k: f32,
}

impl Default for HybridSearchOptions {
    fn default() -> Self {
        Self {
            vector_weight: 0.7,
            bm25_weight: 0.3,
            top_k: 10,
            min_score: None,
            fusion: FusionAlgorithm::Rrf,
            rrf_k: 60.0,
        }
    }
}

pub struct HybridSearcher {
    db: DatabaseConnection,
    vector_store: VectorStore,
}

impl HybridSearcher {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            vector_store: VectorStore::new(db.clone()),
            db,
        }
    }

    pub fn vector_store(&self) -> &VectorStore {
        &self.vector_store
    }

    pub async fn ensure_fts5_index(&self, collection_id: &str) -> Result<()> {
        let safe_name = sanitize_name_for_table(collection_id);
        let meta_table = format!("vec_{safe_name}_meta");
        let fts_table = format!("{meta_table}_fts");

        let table_exists: bool = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                vec![fts_table.clone().into()],
            ))
            .await
            .map(|r| r.is_some())
            .unwrap_or(false);

        if table_exists {
            let rebuild_sql = format!("INSERT INTO {fts_table}({fts_table}) VALUES('rebuild')");
            let _ = self.db.execute_unprepared(&rebuild_sql).await;
            return Ok(());
        }

        let create_sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {fts_table} USING fts5(
                id UNINDEXED,
                document_id UNINDEXED,
                chunk_index UNINDEXED,
                content,
                content={meta_table},
                content_rowid=rowid,
                tokenize='trigram'
            )"
        );

        self.db.execute_unprepared(&create_sql).await.map_err(|e| {
            AxAgentError::Provider(format!("FTS5 trigram index creation failed: {}", e))
        })?;

        let populated: Option<i64> = self
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                format!("SELECT COUNT(*) as cnt FROM {fts_table}"),
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get::<i64>("", "cnt").ok());

        if populated.unwrap_or(0) == 0 {
            let populate_sql = format!(
                "INSERT INTO {fts_table}(rowid, id, document_id, chunk_index, content) \
                 SELECT rowid, id, document_id, chunk_index, content FROM {meta_table}"
            );
            if let Err(e) = self.db.execute_unprepared(&populate_sql).await {
                tracing::debug!("FTS5 initial population failed (non-critical): {}", e);
            }
        }

        Ok(())
    }

    pub async fn hybrid_search(
        &self,
        collection_id: &str,
        query: &str,
        query_embedding: Vec<f32>,
        options: HybridSearchOptions,
    ) -> Result<Vec<HybridSearchResult>> {
        let vector_results = self
            .vector_store
            .search(collection_id, query_embedding.clone(), options.top_k * 3)
            .await?;
        let bm25_results = self
            .bm25_search(collection_id, query, options.top_k * 3)
            .await?;

        let combined = match options.fusion {
            FusionAlgorithm::Weighted => self.merge_results_weighted(
                vector_results,
                bm25_results,
                options.vector_weight,
                options.bm25_weight,
            ),
            FusionAlgorithm::Rrf => {
                self.merge_results_rrf(vector_results, bm25_results, options.rrf_k)
            },
        };

        let mut filtered: Vec<HybridSearchResult> = combined
            .into_iter()
            .filter(|r| {
                if let Some(min) = options.min_score {
                    r.combined_score >= min
                } else {
                    true
                }
            })
            .take(options.top_k)
            .collect();

        filtered.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(filtered)
    }

    async fn bm25_search(
        &self,
        collection_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Bm25Result>> {
        let sanitized = sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let safe_name = sanitize_name_for_table(collection_id);
        let meta_table = format!("vec_{safe_name}_meta");
        let fts_table = format!("{meta_table}_fts");

        let fts_sql = format!(
            "SELECT m.id, m.document_id, m.chunk_index, m.content, bm25({fts_table}) as bm25_score \
             FROM {fts_table} f \
             JOIN {meta_table} m ON m.rowid = f.rowid \
             WHERE {fts_table} MATCH ?1 \
             ORDER BY bm25_score \
             LIMIT ?2"
        );

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                &fts_sql,
                vec![sanitized.clone().into(), (top_k as i64).into()],
            ))
            .await;

        match rows {
            Ok(rows) if !rows.is_empty() => {
                let results: Vec<Bm25Result> = rows
                    .into_iter()
                    .filter_map(|row| {
                        let id: String = row.try_get("", "id").ok()?;
                        let document_id: String = row.try_get("", "document_id").ok()?;
                        let chunk_index: i32 = row.try_get("", "chunk_index").ok()?;
                        let content: String = row.try_get("", "content").ok()?;
                        let bm25_raw: f64 = row.try_get("", "bm25_score").ok().unwrap_or(0.0);
                        let bm25_score = (-bm25_raw as f32).max(0.0);

                        Some(Bm25Result {
                            id,
                            document_id,
                            chunk_index,
                            content,
                            bm25_score,
                        })
                    })
                    .collect();

                if !results.is_empty() {
                    return Ok(results);
                }

                self.bm25_search_fallback(&meta_table, &sanitized, top_k)
                    .await
            },
            _ => {
                self.bm25_search_fallback(&meta_table, &sanitized, top_k)
                    .await
            },
        }
    }

    async fn bm25_search_fallback(
        &self,
        meta_table: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Bm25Result>> {
        let words: Vec<&str> = query.split_whitespace().take(8).collect();
        if words.is_empty() {
            return Ok(vec![]);
        }

        let conditions: Vec<String> = words
            .iter()
            .map(|w| format!("content LIKE '%{}%'", w.replace('\'', "''")))
            .collect();
        let where_clause = conditions.join(" OR ");

        let sql = format!(
            "SELECT id, document_id, chunk_index, content, \
             (CASE WHEN content LIKE '%{}%' THEN 1.0 ELSE 0.3 END) as bm25_score \
             FROM {meta_table} \
             WHERE {where_clause} \
             LIMIT ?1",
            words.first().unwrap_or(&"").replace('\'', "''")
        );

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                &sql,
                vec![(top_k as i64).into()],
            ))
            .await
            .map_err(|e| AxAgentError::Provider(format!("BM25 fallback search failed: {}", e)))?;

        let results: Vec<Bm25Result> = rows
            .into_iter()
            .filter_map(|row| {
                let id: String = row.try_get("", "id").ok()?;
                let document_id: String = row.try_get("", "document_id").ok()?;
                let chunk_index: i32 = row.try_get("", "chunk_index").ok()?;
                let content: String = row.try_get("", "content").ok()?;
                let bm25_score: f32 = row.try_get("", "bm25_score").ok()?;

                Some(Bm25Result {
                    id,
                    document_id,
                    chunk_index,
                    content,
                    bm25_score,
                })
            })
            .collect();

        Ok(results)
    }

    fn merge_results_rrf(
        &self,
        vector_results: Vec<VectorSearchResult>,
        bm25_results: Vec<Bm25Result>,
        k: f32,
    ) -> Vec<HybridSearchResult> {
        let mut score_map: std::collections::HashMap<String, HybridSearchResult> =
            std::collections::HashMap::new();

        for (rank, vr) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank as f32) + 1.0);
            score_map.insert(
                vr.id.clone(),
                HybridSearchResult {
                    id: vr.id.clone(),
                    document_id: vr.document_id.clone(),
                    chunk_index: vr.chunk_index,
                    content: vr.content.clone(),
                    vector_score: Some(1.0 - vr.score),
                    bm25_score: None,
                    combined_score: rrf_score,
                },
            );
        }

        for (rank, br) in bm25_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank as f32) + 1.0);
            if let Some(existing) = score_map.get_mut(&br.id) {
                existing.bm25_score = Some(br.bm25_score);
                existing.combined_score += rrf_score;
            } else {
                score_map.insert(
                    br.id.clone(),
                    HybridSearchResult {
                        id: br.id.clone(),
                        document_id: br.document_id.clone(),
                        chunk_index: br.chunk_index,
                        content: br.content.clone(),
                        vector_score: None,
                        bm25_score: Some(br.bm25_score),
                        combined_score: rrf_score,
                    },
                );
            }
        }

        score_map.into_values().collect()
    }

    fn merge_results_weighted(
        &self,
        vector_results: Vec<VectorSearchResult>,
        bm25_results: Vec<Bm25Result>,
        vector_weight: f32,
        bm25_weight: f32,
    ) -> Vec<HybridSearchResult> {
        let mut score_map: std::collections::HashMap<String, HybridSearchResult> =
            std::collections::HashMap::new();

        let max_vector_distance = vector_results
            .iter()
            .map(|r| r.score)
            .fold(0f32, f32::max)
            .max(f32::EPSILON);
        let max_bm25_score = bm25_results
            .iter()
            .map(|r| r.bm25_score)
            .fold(0f32, f32::max)
            .max(f32::EPSILON);

        for vr in vector_results {
            let normalized_vector = 1.0 - (vr.score / max_vector_distance).clamp(0.0, 1.0);

            let (bm25_part, bm25_raw) = bm25_results
                .iter()
                .find(|b| b.id == vr.id)
                .map(|b| {
                    let norm = b.bm25_score / max_bm25_score;
                    (Some(norm), Some(b.bm25_score))
                })
                .unwrap_or((None, None));

            let combined =
                normalized_vector * vector_weight + bm25_part.unwrap_or(0.0) * bm25_weight;

            score_map.insert(
                vr.id.clone(),
                HybridSearchResult {
                    id: vr.id,
                    document_id: vr.document_id,
                    chunk_index: vr.chunk_index,
                    content: vr.content,
                    vector_score: Some(normalized_vector),
                    bm25_score: bm25_raw,
                    combined_score: combined,
                },
            );
        }

        for br in bm25_results {
            if score_map.contains_key(&br.id) {
                continue;
            }
            let normalized_bm25 = br.bm25_score / max_bm25_score;
            let combined = if vector_weight > 0.0 {
                normalized_bm25 * bm25_weight
            } else {
                normalized_bm25
            };

            score_map.insert(
                br.id.clone(),
                HybridSearchResult {
                    id: br.id,
                    document_id: br.document_id,
                    chunk_index: br.chunk_index,
                    content: br.content,
                    vector_score: None,
                    bm25_score: Some(br.bm25_score),
                    combined_score: combined,
                },
            );
        }

        score_map.into_values().collect()
    }
}

#[derive(Debug, Clone)]
struct Bm25Result {
    id: String,
    document_id: String,
    chunk_index: i32,
    content: String,
    bm25_score: f32,
}

fn sanitize_name_for_table(collection_id: &str) -> String {
    collection_id
        .chars()
        .map(|c| if c == '-' { '_' } else { c })
        .collect()
}

fn sanitize_fts5_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();

    for c in trimmed.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' || (c > '\u{4e00}' && c < '\u{9fff}') {
            current.push(c);
        } else if !current.is_empty() {
            if current.len() >= 3 {
                tokens.push(current.replace('\'', "''"));
            }
            current = String::new();
        }
    }
    if !current.is_empty() && current.len() >= 3 {
        tokens.push(current.replace('\'', "''"));
    }

    if tokens.is_empty() {
        return String::new();
    }

    tokens.join(" OR ")
}
