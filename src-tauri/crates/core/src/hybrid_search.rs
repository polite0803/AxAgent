use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};

use crate::error::{AxAgentError, Result};
use crate::vector_store::{VectorSearchResult, VectorStore};

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

#[derive(Debug, Clone)]
pub struct HybridSearchOptions {
    pub vector_weight: f32,
    pub bm25_weight: f32,
    pub top_k: usize,
    pub min_score: Option<f32>,
}

impl Default for HybridSearchOptions {
    fn default() -> Self {
        Self {
            vector_weight: 0.7,
            bm25_weight: 0.3,
            top_k: 10,
            min_score: None,
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

    pub async fn ensure_fts5_index(&self, collection_id: &str) -> Result<()> {
        let safe_name = collection_id.replace('-', "_");
        let table_name = format!("vec_{}_meta", safe_name);
        let fts_table = format!("{}_fts", table_name);

        let create_sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {fts_table} USING fts5(id, document_id, content, content='vec_{safe_name}_meta', content_rowid=rowid)"
        );

        self.db
            .execute_unprepared(&create_sql)
            .await
            .map_err(|e| AxAgentError::Provider(format!("FTS5 index creation failed: {}", e)))?;

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
            .search(collection_id, query_embedding.clone(), options.top_k * 2)
            .await?;
        let bm25_results = self
            .bm25_search(collection_id, query, options.top_k * 2)
            .await?;

        let combined = self.merge_results(
            vector_results,
            bm25_results,
            options.vector_weight,
            options.bm25_weight,
        );

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

        filtered.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

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

        let safe_name = collection_id.replace('-', "_");
        let table_name = format!("vec_{}_meta", safe_name);
        let fts_table = format!("{}_fts", table_name);

        let fts_sql = format!(
            "SELECT f.id, f.document_id, f.chunk_index, f.content, bm25({fts_table}) as bm25_score \
             FROM {fts_table} f \
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

                self.bm25_search_fallback(&table_name, &sanitized, top_k)
                    .await
            },
            _ => {
                self.bm25_search_fallback(&table_name, &sanitized, top_k)
                    .await
            },
        }
    }

    async fn bm25_search_fallback(
        &self,
        table_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Bm25Result>> {
        let words: Vec<&str> = query.split_whitespace().take(5).collect();
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
             (CASE WHEN content LIKE '%{}%' THEN 0.5 ELSE 0.1 END) as bm25_score \
             FROM {table_name} \
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

    fn merge_results(
        &self,
        vector_results: Vec<VectorSearchResult>,
        bm25_results: Vec<Bm25Result>,
        vector_weight: f32,
        bm25_weight: f32,
    ) -> Vec<HybridSearchResult> {
        let mut score_map: std::collections::HashMap<String, HybridSearchResult> =
            std::collections::HashMap::new();

        let max_vector_score = vector_results
            .iter()
            .map(|r| r.score)
            .fold(1f32, f32::min)
            .max(1f32);
        let max_bm25_score = bm25_results
            .iter()
            .map(|r| r.bm25_score)
            .fold(1f32, f32::max)
            .max(1f32);

        for vr in vector_results {
            let normalized_vector_score = 1.0 - (vr.score / max_vector_score);
            let content = vr.content.clone();
            let id = vr.id.clone();

            let combined = if bm25_weight > 0.0 {
                let bm25_score = bm25_results
                    .iter()
                    .find(|b| b.id == vr.id)
                    .map(|b| b.bm25_score)
                    .unwrap_or(0.0);
                let normalized_bm25 = if max_bm25_score > 0.0 {
                    bm25_score / max_bm25_score
                } else {
                    0.0
                };
                normalized_vector_score * vector_weight + normalized_bm25 * bm25_weight
            } else {
                normalized_vector_score
            };

            score_map.insert(
                id.clone(),
                HybridSearchResult {
                    id,
                    document_id: vr.document_id,
                    chunk_index: vr.chunk_index,
                    content,
                    vector_score: Some(normalized_vector_score),
                    bm25_score: None,
                    combined_score: combined,
                },
            );
        }

        for br in bm25_results {
            let normalized_bm25 = if max_bm25_score > 0.0 {
                br.bm25_score / max_bm25_score
            } else {
                0.0
            };
            let combined = if vector_weight > 0.0 {
                let vector_score = score_map
                    .get(&br.id)
                    .and_then(|r| r.vector_score)
                    .unwrap_or(0.0);
                vector_score * vector_weight + normalized_bm25 * bm25_weight
            } else {
                normalized_bm25
            };

            if let Some(existing) = score_map.get_mut(&br.id) {
                existing.bm25_score = Some(normalized_bm25);
                existing.combined_score = combined;
            } else {
                score_map.insert(
                    br.id.clone(),
                    HybridSearchResult {
                        id: br.id,
                        document_id: br.document_id,
                        chunk_index: br.chunk_index,
                        content: br.content,
                        vector_score: None,
                        bm25_score: Some(normalized_bm25),
                        combined_score: combined,
                    },
                );
            }
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

fn sanitize_fts5_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let words: Vec<String> = trimmed
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .map(|w| {
            let mut clean = String::new();
            for c in w.chars() {
                match c {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => clean.push(c),
                    _ => {
                        if c.is_alphabetic() || c.is_alphanumeric() {
                            clean.push(c);
                        }
                    },
                }
            }
            clean
        })
        .filter(|w| !w.is_empty())
        .take(10)
        .collect();

    if words.is_empty() {
        return String::new();
    }

    words.join(" OR ")
}
