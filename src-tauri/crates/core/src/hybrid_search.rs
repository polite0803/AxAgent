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

        let table_name = format!("vec_{}_meta", collection_id.replace('-', "_"));

        let sql = format!(
            "SELECT id, document_id, chunk_index, content, bm25(matchinfo({table_name}), 'norm=1') as bm25_score \
             FROM {table_name} \
             WHERE {table_name} MATCH ?1 \
             ORDER BY bm25_score DESC \
             LIMIT ?2"
        );

        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                &sql,
                vec![sanitized.into(), (top_k as i64).into()],
            ))
            .await
            .map_err(|e| AxAgentError::Provider(format!("BM25 search failed: {}", e)))?
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

        Ok(rows)
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
            .fold(1f32, f32::max);

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

    let mut sanitized = String::with_capacity(trimmed.len() * 2);
    let mut in_phrase = false;

    for c in trimmed.chars() {
        match c {
            'a'..='z'
            | 'A'..='Z'
            | '0'..='9'
            | ' '
            | '\t'
            | '\n'
            | '-'
            | '_'
            | '.'
            | '@'
            | '#'
            | '*' => {
                sanitized.push(c);
            }
            '"' => {
                in_phrase = !in_phrase;
                sanitized.push(c);
            }
            '(' | ')' => {
                sanitized.push(c);
            }
            _ => {
                // 保留 Unicode 字母、表意文字（中日韩等），确保非 ASCII 搜索可用
                if c.is_alphabetic() || c.is_alphanumeric() {
                    sanitized.push(c);
                }
            }
        }
    }

    sanitized
}
