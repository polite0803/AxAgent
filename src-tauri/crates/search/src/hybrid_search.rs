// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
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
    /// 多引擎 RAG：sparse neural 检索分数（SPLADE/BGE-M3 sparse 等）。
    /// 当前实现暂未接入 sparse encoder，该字段始终为 None，留作扩展位。
    #[serde(default)]
    pub sparse_score: Option<f32>,
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
    /// 多引擎 RAG：sparse neural 路径权重（默认 0，表示不启用 sparse 路径）。
    /// 当 sparse_weight > 0 且 sparse encoder 可用时，会走三路融合。
    pub sparse_weight: f32,
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
            sparse_weight: 0.0,
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
        Self { vector_store: VectorStore::new(db.clone()), db }
    }

    pub fn vector_store(&self) -> &VectorStore {
        &self.vector_store
    }

    pub async fn ensure_fts5_index(&self, collection_id: &str) -> Result<()> {
        if self.db.get_database_backend() == DbBackend::Postgres {
            // PostgreSQL 关键词检索由 VectorStore 的 content_tsv 生成列 + GIN 索引承担。
            // 直接复用 VectorStore 同名方法创建/校验 GIN 索引（幂等）。
            return self.vector_store.ensure_fts5_index(collection_id).await;
        }

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
        self.hybrid_search_with_filter(collection_id, query, query_embedding, options, None).await
    }

    /// Hybrid search with optional `document_id` list filter
    /// (multi-document collaboration).
    ///
    /// When `doc_ids` is `Some` and non-empty, both the vector path
    /// (`vector_store::search_with_filter`) and the BM25 path apply the same
    /// `document_id IN (...)` predicate so the fused result set is scoped to
    /// the requested subset of documents.
    pub async fn hybrid_search_with_filter(
        &self,
        collection_id: &str,
        query: &str,
        query_embedding: Vec<f32>,
        options: HybridSearchOptions,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<HybridSearchResult>> {
        let vector_results = self
            .vector_store
            .search_with_filter(collection_id, query_embedding.clone(), options.top_k * 3, doc_ids)
            .await?;
        let bm25_results =
            self.bm25_search_with_filter(collection_id, query, options.top_k * 3, doc_ids).await?;

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
            b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(filtered)
    }

    /// 纯 FTS（BM25）检索：embedding 未配置时的降级路径（R9 遗留）。
    /// 不生成 query embedding、不做向量召回与融合，直接返回 BM25 命中，
    /// 收尾语义（min_score 过滤 / 降序 / top_k 截断）与 `hybrid_search_with_filter` 保持一致，
    /// 调用方无需区分结果来源。
    pub async fn fts_only_search_with_filter(
        &self,
        collection_id: &str,
        query: &str,
        options: HybridSearchOptions,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<HybridSearchResult>> {
        let bm25_results =
            self.bm25_search_with_filter(collection_id, query, options.top_k, doc_ids).await?;

        let mut filtered: Vec<HybridSearchResult> = bm25_results
            .into_iter()
            .map(|br| HybridSearchResult {
                id: br.id,
                document_id: br.document_id,
                chunk_index: br.chunk_index,
                content: br.content,
                vector_score: None,
                bm25_score: Some(br.bm25_score),
                sparse_score: None,
                combined_score: br.bm25_score,
            })
            .filter(|r| options.min_score.is_some_and(|min| r.combined_score >= min))
            .collect();

        filtered.sort_by(|a, b| {
            b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        filtered.truncate(options.top_k);

        Ok(filtered)
    }

    /// BM25 keyword search with optional `document_id` list filter.
    /// Filters apply identically to the FTS5 (SQLite) and tsvector (PG) paths.
    async fn bm25_search_with_filter(
        &self,
        collection_id: &str,
        query: &str,
        top_k: usize,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<Bm25Result>> {
        // 集合表不存在（如记忆命名空间尚未写入任何条目）时优雅返回空结果，
        // 避免对缺失的 _meta 表执行 SQL 导致查询报错。
        let safe_name_early = sanitize_name_for_table(collection_id);
        let meta_exists = self
            .vector_store
            .table_exists(&format!("vec_{safe_name_early}_meta"))
            .await
            .unwrap_or(false);
        if !meta_exists {
            return Ok(vec![]);
        }

        if self.db.get_database_backend() == DbBackend::Postgres {
            return self.bm25_search_pg_with_filter(collection_id, query, top_k, doc_ids).await;
        }

        let sanitized = sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let safe_name = sanitize_name_for_table(collection_id);
        let meta_table = format!("vec_{safe_name}_meta");
        let fts_table = format!("{meta_table}_fts");

        let (fts_sql, mut params) = match doc_ids {
            Some(ids) if !ids.is_empty() => {
                // SQLite 占位符全部用显式编号避免歧义：
                //   ?1 = query, ?2..?(N+1) = doc_ids, ?(N+2) = top_k
                let placeholders = ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 2))
                    .collect::<Vec<_>>()
                    .join(", ");
                let limit_ph = format!("?{}", ids.len() + 2);
                let in_clause = format!(" AND m.document_id IN ({placeholders})");
                let sql = format!(
                    "SELECT m.id, m.document_id, m.chunk_index, m.content, bm25({fts_table}) as bm25_score \
                     FROM {fts_table} f \
                     JOIN {meta_table} m ON m.rowid = f.rowid \
                     WHERE {fts_table} MATCH ?1{in_clause} \
                     ORDER BY bm25_score \
                     LIMIT {limit_ph}"
                );
                (sql, ids.iter().cloned().map(Value::from).collect::<Vec<_>>())
            },
            _ => {
                let sql = format!(
                    "SELECT m.id, m.document_id, m.chunk_index, m.content, bm25({fts_table}) as bm25_score \
                     FROM {fts_table} f \
                     JOIN {meta_table} m ON m.rowid = f.rowid \
                     WHERE {fts_table} MATCH ?1 \
                     ORDER BY bm25_score \
                     LIMIT ?2"
                );
                (sql, Vec::new())
            },
        };

        // SQLite 参数顺序: [query, doc_ids..., top_k]
        let mut values = Vec::with_capacity(params.len() + 2);
        values.push(sanitized.clone().into());
        values.append(&mut params);
        values.push((top_k as i64).into());

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(DbBackend::Sqlite, &fts_sql, values))
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

                        Some(Bm25Result { id, document_id, chunk_index, content, bm25_score })
                    })
                    .collect();

                if !results.is_empty() {
                    return Ok(results);
                }

                self.bm25_search_fallback_with_filter(&meta_table, &sanitized, top_k, doc_ids).await
            },
            _ => {
                self.bm25_search_fallback_with_filter(&meta_table, &sanitized, top_k, doc_ids).await
            },
        }
    }

    /// PostgreSQL keyword search with optional `document_id` list filter.
    async fn bm25_search_pg_with_filter(
        &self,
        collection_id: &str,
        query: &str,
        top_k: usize,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<Bm25Result>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let safe_name = sanitize_name_for_table(collection_id);
        let meta_table = format!("vec_{safe_name}_meta");

        // Build optional IN clause; PG placeholders are positional ($n).
        let (in_clause, mut params) = match doc_ids {
            Some(ids) if !ids.is_empty() => {
                let placeholders = ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 3))
                    .collect::<Vec<_>>()
                    .join(", ");
                let clause = format!(" AND m.document_id IN ({placeholders})");
                (clause, ids.iter().cloned().map(Value::from).collect::<Vec<_>>())
            },
            _ => (String::new(), Vec::new()),
        };

        let sql = format!(
            "SELECT m.id, m.document_id, m.chunk_index, m.content, \
             ts_rank(m.content_tsv, query) AS bm25_score \
             FROM {meta_table} m, plainto_tsquery('simple', $1) query \
             WHERE m.content_tsv @@ query{in_clause} \
             ORDER BY bm25_score DESC \
             LIMIT $2"
        );

        let mut values = Vec::with_capacity(params.len() + 2);
        values.push(trimmed.to_string().into());
        values.push((top_k as i64).into());
        values.append(&mut params);

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres, &sql, values))
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
                        let rank: f64 = row.try_get("", "bm25_score").ok().unwrap_or(0.0);
                        let bm25_score = rank as f32;
                        Some(Bm25Result { id, document_id, chunk_index, content, bm25_score })
                    })
                    .collect();

                if !results.is_empty() {
                    return Ok(results);
                }
                self.bm25_search_fallback_pg_with_filter(&meta_table, trimmed, top_k, doc_ids).await
            },
            _ => {
                self.bm25_search_fallback_pg_with_filter(&meta_table, trimmed, top_k, doc_ids).await
            },
        }
    }

    async fn bm25_search_fallback_with_filter(
        &self,
        meta_table: &str,
        query: &str,
        top_k: usize,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<Bm25Result>> {
        if self.db.get_database_backend() == DbBackend::Postgres {
            return self
                .bm25_search_fallback_pg_with_filter(meta_table, query, top_k, doc_ids)
                .await;
        }

        let words: Vec<&str> = query.split_whitespace().take(8).collect();
        if words.is_empty() {
            return Ok(vec![]);
        }

        let conditions: Vec<String> =
            words.iter().map(|w| format!("content LIKE '%{}%'", w.replace('\'', "''"))).collect();
        let where_clause = conditions.join(" OR ");

        // SQLite 路径：占位符全部用显式编号避免歧义。
        //   无 doc_ids: VALUES=[top_k], LIMIT ?1
        //   有 doc_ids: VALUES=[doc_ids..., top_k], IN(?1..?N), LIMIT ?(N+1)
        let (where_with_filter, mut params, limit_placeholder) = match doc_ids {
            Some(ids) if !ids.is_empty() => {
                let placeholders = ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                let limit_ph = format!("?{}", ids.len() + 1);
                let where_with_in = format!("({where_clause}) AND document_id IN ({placeholders})");
                (where_with_in, ids.iter().cloned().map(Value::from).collect::<Vec<_>>(), limit_ph)
            },
            _ => (where_clause, Vec::new(), "?1".to_string()),
        };

        let sql = format!(
            "SELECT id, document_id, chunk_index, content, \
             (CASE WHEN content LIKE '%{}%' THEN 1.0 ELSE 0.3 END) as bm25_score \
             FROM {meta_table} \
             WHERE {where_with_filter} \
             LIMIT {limit_placeholder}",
            words.first().unwrap_or(&"").replace('\'', "''")
        );

        let mut values: Vec<Value> = Vec::with_capacity(params.len() + 1);
        values.append(&mut params);
        values.push((top_k as i64).into());

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(DbBackend::Sqlite, &sql, values))
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

                Some(Bm25Result { id, document_id, chunk_index, content, bm25_score })
            })
            .collect();

        Ok(results)
    }

    /// PostgreSQL keyword fallback: substring (`ILIKE`) match when the tsvector
    /// path yields nothing. Mirrors the SQLite `LIKE` fallback semantics.
    async fn bm25_search_fallback_pg_with_filter(
        &self,
        meta_table: &str,
        query: &str,
        top_k: usize,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<Bm25Result>> {
        let words: Vec<&str> = query.split_whitespace().take(8).collect();
        if words.is_empty() {
            return Ok(vec![]);
        }

        let conditions: Vec<String> =
            words.iter().map(|w| format!("content ILIKE '%{}%'", w.replace('\'', "''"))).collect();
        let mut where_clause = conditions.join(" OR ");

        // PostgreSQL 占位符必须从 $1 连续编号且不可跳号：
        //   values 顺序 = [top_k, doc_ids...] → $1=top_k(LIMIT), $2..$(N+1)=doc_ids
        // 修复前 bug：无 doc_ids 时 LIMIT $2（缺 $1）、有 doc_ids 时 LIMIT $N+2（跳号），
        // 导致 sqlx 报 "绑定消息提供了1个参数,但是已准备好语句要求2个参数"。
        let (in_clause, mut params, limit_ph) = match doc_ids {
            Some(ids) if !ids.is_empty() => {
                let placeholders = ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 2))
                    .collect::<Vec<_>>()
                    .join(", ");
                let clause = format!(" AND document_id IN ({placeholders})");
                (clause, ids.iter().cloned().map(Value::from).collect::<Vec<_>>(), "$1".to_string())
            },
            _ => (String::new(), Vec::new(), "$1".to_string()),
        };

        where_clause = format!("({where_clause}){in_clause}");

        let sql = format!(
            "SELECT id, document_id, chunk_index, content, \
             (CASE WHEN content ILIKE '%{}%' THEN 1.0 ELSE 0.3 END) as bm25_score \
             FROM {meta_table} \
             WHERE {where_clause} \
             LIMIT {limit_ph}",
            words.first().unwrap_or(&"").replace('\'', "''")
        );

        let mut values: Vec<Value> = Vec::with_capacity(params.len() + 1);
        values.push((top_k as i64).into());
        values.append(&mut params);

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres, &sql, values))
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

                Some(Bm25Result { id, document_id, chunk_index, content, bm25_score })
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
                    sparse_score: None,
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
                        sparse_score: None,
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

        let max_vector_distance =
            vector_results.iter().map(|r| r.score).fold(0f32, f32::max).max(f32::EPSILON);
        let max_bm25_score =
            bm25_results.iter().map(|r| r.bm25_score).fold(0f32, f32::max).max(f32::EPSILON);

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
                    sparse_score: None,
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
                    sparse_score: None,
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
    collection_id.chars().map(|c| if c == '-' { '_' } else { c }).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_search_options_default_fusion_rrf() {
        let opts = HybridSearchOptions::default();
        assert_eq!(opts.fusion, FusionAlgorithm::Rrf);
        assert!((opts.vector_weight - 0.7).abs() < f32::EPSILON);
        assert!((opts.bm25_weight - 0.3).abs() < f32::EPSILON);
        assert_eq!(opts.top_k, 10);
        assert_eq!(opts.rrf_k, 60.0);
    }

    #[test]
    fn test_hybrid_search_options_weighted_fusion() {
        let opts = HybridSearchOptions {
            fusion: FusionAlgorithm::Weighted,
            vector_weight: 0.5,
            bm25_weight: 0.5,
            ..Default::default()
        };
        assert_eq!(opts.fusion, FusionAlgorithm::Weighted);
        assert!((opts.vector_weight - 0.5).abs() < f32::EPSILON);
        assert!((opts.bm25_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hybrid_search_result_serialization() {
        let result = HybridSearchResult {
            id: "test-id".to_string(),
            document_id: "doc-id".to_string(),
            chunk_index: 3,
            content: "test content".to_string(),
            vector_score: Some(0.85),
            bm25_score: Some(0.42),
            sparse_score: None,
            combined_score: 0.65,
        };
        let json = serde_json::to_value(&result).expect("测试：to_value 应成功");
        assert_eq!(json["id"], "test-id");
        assert_eq!(json["chunk_index"], 3);
        assert!((json["combined_score"].as_f64().expect("测试应成功") - 0.65).abs() < 0.001);
    }

    #[test]
    fn test_fusion_algorithm_serde() {
        assert_eq!(serde_json::to_value(FusionAlgorithm::Rrf).expect("测试应成功"), "Rrf");
        assert_eq!(
            serde_json::to_value(FusionAlgorithm::Weighted).expect("测试应成功"),
            "Weighted"
        );
    }

    #[test]
    fn test_fusion_algorithm_default_is_rrf() {
        assert_eq!(FusionAlgorithm::default(), FusionAlgorithm::Rrf);
    }

    #[test]
    fn test_sanitize_name_for_table() {
        assert_eq!(sanitize_name_for_table("my-collection"), "my_collection");
        assert_eq!(sanitize_name_for_table("simple"), "simple");
        assert_eq!(sanitize_name_for_table("a-b-c"), "a_b_c");
    }

    #[test]
    fn test_hybrid_search_result_default_combined() {
        let result = HybridSearchResult {
            id: "id".into(),
            document_id: "doc".into(),
            chunk_index: 0,
            content: "".into(),
            vector_score: None,
            bm25_score: None,
            sparse_score: None,
            combined_score: 0.0,
        };
        assert_eq!(result.combined_score, 0.0);
        assert!(result.vector_score.is_none());
    }
}
