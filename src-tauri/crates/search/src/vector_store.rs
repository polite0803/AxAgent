// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::disallowed_types)]

use std::collections::HashMap;
// SAFETY: parking_lot::Mutex (别名 StdMutex) 用于保护 HashMap 的查找操作，
// 仅用于快速获取集合级别的 tokio::sync::Mutex 引用，不跨越 await 点。
// 实际的临界区由 tokio::sync::Mutex 保护，cross-await 安全。
use parking_lot::Mutex as StdMutex;
#[allow(clippy::disallowed_types)]
use std::sync::Arc;

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, QueryResult,
    Statement, TransactionTrait, Value,
};

use axagent_harness::core_error::{AxAgentError, Result};

fn validate_collection_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(AxAgentError::Validation("Collection name cannot be empty".to_string()));
    }
    // 允许连字符：实际落库表名由 `validated_collection_name` 统一清洗为下划线，
    // 此处仅做字符集校验，需与下游清洗逻辑保持一致（否则 wiki/kb/mem 的 UUID 含连字符会误拒）。
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AxAgentError::Validation(format!(
            "Invalid collection name '{}': only alphanumeric characters, hyphens and underscores are allowed",
            name
        )));
    }
    if name.len() > 64 {
        return Err(AxAgentError::Validation(format!(
            "Collection name '{}' is too long (max 64 characters)",
            name
        )));
    }
    Ok(())
}

/// Register the sqlite-vec extension globally.
///
/// Must be called **once** before any SQLite connection is opened.
///
/// NOTE: Disabled on Android by default due to ARM64 ABI incompatibility
/// between sqlite-vec's compiled C code and the device's SQLite library.
/// Vector search will be unavailable on Android.
/// Set `AXAGENT_FORCE_VEC=1` in environment to override for debugging.
pub fn register_sqlite_vec_extension() {
    #[cfg(target_os = "android")]
    {
        if std::env::var("AXAGENT_FORCE_VEC").as_deref() == Ok("1") {
            tracing::info!(
                "AXAGENT_FORCE_VEC=1 set — attempting sqlite-vec registration on Android"
            );
            // SAFETY:
            // - sqlite3_auto_extension is called before any database connections are opened (during registration).
            // - transmute converts sqlite3_vec_init function pointer to the expected sqlite3 extension init signature.
            // - This is the standard pattern for loading SQLite extensions via auto_extension.
            // - Only executed when AXAGENT_FORCE_VEC=1 is set (opt-in debugging).
            unsafe {
                libsqlite3_sys::sqlite3_auto_extension(Some(std::mem::transmute(
                    sqlite_vec::sqlite3_vec_init as *const (),
                )));
            }
        } else {
            tracing::warn!(
                "sqlite-vec disabled on Android — vector search unavailable. \
                 Set AXAGENT_FORCE_VEC=1 to override."
            );
        }
    }
    #[cfg(not(target_os = "android"))]
    // SAFETY:
    // - Same transmute pattern but with explicit type annotation for the target signature.
    // - sqlite3_vec_init is the canonical entry point provided by the sqlite-vec crate.
    // - The transmute is safe because sqlite3_vec_init matches the expected sqlite3 extension init function signature.
    // - sqlite3_auto_extension is thread-safe per SQLite documentation and must be called before any DB connections.
    unsafe {
        libsqlite3_sys::sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut libsqlite3_sys::sqlite3,
                *mut *mut ::std::os::raw::c_char,
                *const libsqlite3_sys::sqlite3_api_routines,
            ) -> i32,
        >(
            sqlite_vec::sqlite3_vec_init as *const ()
        )));
    }
}

/// A single embedding record for storage in the vector database.
#[derive(Debug, Clone)]
pub struct EmbeddingRecord {
    /// Unique chunk identifier
    pub id: String,
    /// Parent document identifier
    pub document_id: String,
    /// Position of this chunk within the document
    pub chunk_index: i32,
    /// Text content of the chunk
    pub content: String,
    /// Embedding vector
    pub embedding: Vec<f32>,
}

/// A result returned from vector similarity search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    /// Distance score (lower is more similar for L2 distance)
    pub score: f32,
    /// Whether this chunk has an embedding in vec0
    pub has_embedding: bool,
}

/// Configuration for HNSW (Hierarchical Navigable Small World) index.
/// HNSW provides faster approximate nearest neighbor search for large collections.
///
/// Default values are suitable for most use cases:
/// - Small collections (< 10k vectors): Use default k-NN (exact search)
/// - Medium collections (10k-100k): ef_search=50, m=12, ef_construction=100
/// - Large collections (> 100k): ef_search=100, m=16, ef_construction=200
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Construction time search width (higher = slower build, better graph quality)
    /// Default: 100
    pub ef_construction: usize,
    /// Max connections per node (higher = better recall, more memory)
    /// Default: 16
    pub m: usize,
    /// Search width (higher = slower search, better recall)
    /// Default: 50
    pub ef_search: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self { ef_construction: 100, m: 16, ef_search: 50 }
    }
}

/// Vector store for knowledge base embeddings.
///
/// Backend-agnostic facade over two implementations selected at runtime by the
/// underlying `DatabaseConnection` backend:
///
/// - **SQLite** (`DbBackend::Sqlite`): uses the `sqlite-vec` `vec0` virtual table
///   for embeddings + an FTS5 trigram table for keyword search.
/// - **PostgreSQL** (`DbBackend::Postgres`): uses a `VECTOR(n)` column (pgvector)
///   for embeddings + a generated `tsvector` column (GIN-indexed) for keyword search.
///
/// The `vec_collections` registry table and all chunk-metadata (`*_meta`) tables
/// are identical across backends; only the embedding storage and search operators
/// differ, isolated in the per-method backend branches below.
#[derive(Debug, Clone)]
// SAFETY: VectorStore 中的 StdMutex 仅用于保护 HashMap 的查找操作，
// 不跨越 await 点。实际的临界区由 tokio::sync::Mutex 保护，cross-await 安全。
#[allow(clippy::disallowed_types)]
pub struct VectorStore {
    db: DatabaseConnection,
    /// Per-collection serialization locks for upsert operations.
    ///
    /// Prevents primary key conflicts on PostgreSQL (where `rowid` is not auto-increment)
    /// when concurrent index jobs try to `MAX(rowid)+1` on the same collection table.
    /// The outer `StdMutex` guards the HashMap lookup; each collection has its own
    /// `tokio::sync::Mutex<()>` for the actual critical section (cross-await safe).
    upsert_locks: Arc<StdMutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl VectorStore {
    /// Create a VectorStore that uses an existing sea-orm connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db, upsert_locks: Arc::new(StdMutex::new(HashMap::new())) }
    }

    /// True when the underlying connection is PostgreSQL (pgvector path).
    fn is_pg(&self) -> bool {
        self.db.get_database_backend() == DbBackend::Postgres
    }

    /// The actual backend of the underlying connection (used for placeholder style).
    fn be(&self) -> DbBackend {
        self.db.get_database_backend()
    }

    fn is_valid_collection_id(collection_id: &str) -> bool {
        !collection_id.is_empty()
            && collection_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    fn sanitize_collection_id(collection_id: &str) -> String {
        collection_id.chars().map(|c| if c == '-' { '_' } else { c }).collect()
    }

    fn validated_collection_name(collection_id: &str) -> Result<String> {
        if !Self::is_valid_collection_id(collection_id) {
            return Err(AxAgentError::Validation("Invalid collection_id: must contain only alphanumeric characters, hyphens, and underscores".to_string()));
        }
        Ok(format!("vec_{}", Self::sanitize_collection_id(collection_id)))
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Get or create a per-collection serialization mutex for upsert operations.
    ///
    /// All upsert threads for the same `collection_id` will share one mutex,
    /// ensuring `MAX(rowid)+1` is safe. Different collections remain fully concurrent.
    fn collection_upsert_mutex(&self, collection_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut map = self.upsert_locks.lock();
        map.entry(collection_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    // ── DDL builders (backend-specific) ──────────────────────────────────

    /// CREATE TABLE for the metadata table. On PostgreSQL an extra generated
    /// `tsvector` column backs keyword search.
    fn meta_ddl(&self, name: &str) -> String {
        if self.is_pg() {
            format!(
                "CREATE TABLE IF NOT EXISTS {name}_meta (\n  \
                 rowid BIGINT PRIMARY KEY,\n  \
                 id TEXT NOT NULL UNIQUE,\n  \
                 document_id TEXT NOT NULL,\n  \
                 chunk_index INTEGER NOT NULL,\n  \
                 content TEXT NOT NULL,\n  \
                 content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple', content)) STORED\n)"
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {name}_meta (\n  \
                 rowid INTEGER PRIMARY KEY AUTOINCREMENT,\n  \
                 id TEXT NOT NULL UNIQUE,\n  \
                 document_id TEXT NOT NULL,\n  \
                 chunk_index INTEGER NOT NULL,\n  \
                 content TEXT NOT NULL\n)"
            )
        }
    }

    /// CREATE for the embedding table (vec0 virtual table vs pgvector column).
    fn vec_table_ddl(&self, name: &str, dimensions: usize, hnsw: Option<&HnswConfig>) -> String {
        if self.is_pg() {
            format!(
                "CREATE TABLE IF NOT EXISTS {name} (rowid BIGINT PRIMARY KEY, embedding VECTOR({dimensions}))"
            )
        } else {
            match hnsw {
                Some(h) => format!(
                    "CREATE VIRTUAL TABLE IF NOT EXISTS {name} USING vec0(embedding float[{dimensions}], hnsw(ef_construction={}, m={}, ef_search={}))",
                    h.ef_construction, h.m, h.ef_search
                ),
                None => format!(
                    "CREATE VIRTUAL TABLE IF NOT EXISTS {name} USING vec0(embedding float[{dimensions}])"
                ),
            }
        }
    }

    /// Ensure the pgvector ANN index exists (PostgreSQL only). Tries HNSW, then
    /// ivfflat, then silently proceeds without an index (exact scan) — index is
    /// a performance optimization, never a correctness requirement.
    async fn ensure_vector_index(&self, name: &str, _dimensions: usize, hnsw: Option<&HnswConfig>) {
        if !self.is_pg() {
            return;
        }
        let m = hnsw.map(|h| h.m).unwrap_or(16);
        let hnsw_sql = format!(
            "CREATE INDEX IF NOT EXISTS idx_{name}_vec ON {name} USING hnsw (embedding vector_cosine_ops) WITH (m = {m})"
        );
        if let Err(e) = self.exec(&hnsw_sql).await {
            tracing::warn!("PG HNSW index creation failed for {name}, trying ivfflat: {e}");
            let ivf = format!(
                "CREATE INDEX IF NOT EXISTS idx_{name}_vec ON {name} USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100)"
            );
            if let Err(e2) = self.exec(&ivf).await {
                tracing::warn!(
                    "PG ivfflat index creation also failed for {name} (non-critical): {e2}"
                );
            }
        }
    }

    /// INSERT statement for a single embedding row (cast to vector on PostgreSQL).
    fn embedding_insert_sql(&self, name: &str) -> String {
        if self.is_pg() {
            format!("INSERT INTO {name} (rowid, embedding) VALUES ($1, $2::vector)")
        } else {
            format!("INSERT INTO {name} (rowid, embedding) VALUES ($1, $2)")
        }
    }

    /// Vector similarity search SELECT (vec0 MATCH+k vs pgvector `<=>`).
    fn search_sql(&self, name: &str) -> String {
        if self.is_pg() {
            format!(
                "SELECT m.id, m.document_id, m.chunk_index, m.content, v.embedding <=> $1::vector AS distance \
                 FROM {name} v \
                 JOIN {name}_meta m ON m.rowid = v.rowid \
                 ORDER BY distance LIMIT $2"
            )
        } else {
            format!(
                "SELECT m.id, m.document_id, m.chunk_index, m.content, v.distance \
                 FROM {name} v \
                 JOIN {name}_meta m ON m.rowid = v.rowid \
                 WHERE v.embedding MATCH $1 AND k = $2 \
                 ORDER BY v.distance"
            )
        }
    }

    async fn registry_upsert_collection(
        &self,
        collection_id: &str,
        dimensions: usize,
        index_type: &str,
        hnsw_config: Option<&HnswConfig>,
    ) {
        let dim_i32 = dimensions as i32;
        let now = Self::now_ms();
        let (ef_c, m, ef_s) = match hnsw_config {
            Some(c) => (Some(c.ef_construction as i32), Some(c.m as i32), Some(c.ef_search as i32)),
            None => (None, None, None),
        };

        // sanitize_collection_id 已确保只含字母数字和下划线，表名拼接安全
        let sanitized = Self::sanitize_collection_id(collection_id);
        let meta_table = format!("vec_{sanitized}_meta");

        // 使用 ON CONFLICT (collection_id) DO UPDATE 语法（PG 和 SQLite 3.24+ 都支持），
        // 替代原来的 `INSERT OR IGNORE`（SQLite 专有语法，PG 上会语法错误）。
        // 修复关键 bug：原实现在 PG 上 INSERT 失败导致 registry 永远没有记录，
        // 后续每次 upsert_embeddings 都会走 `None` 分支重新执行 ensure_collection 的全部 DDL，
        // 产生大量 PG notice 日志噪声，且每次索引都重复执行无用的 DDL。
        let sql = format!(
            "INSERT INTO vec_collections \
             (collection_id, dimensions, index_type, hnsw_ef_construction, hnsw_m, hnsw_ef_search, \
              vector_count, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, \
                     COALESCE((SELECT COUNT(*) FROM {meta_table}), 0), $7, $7) \
             ON CONFLICT (collection_id) DO UPDATE SET \
              updated_at = EXCLUDED.updated_at, \
              index_type = EXCLUDED.index_type, \
              vector_count = COALESCE((SELECT COUNT(*) FROM {meta_table}), 0)",
            meta_table = meta_table,
        );

        let params: Vec<Value> = vec![
            collection_id.into(),
            dim_i32.into(),
            index_type.into(),
            ef_c.into(),
            m.into(),
            ef_s.into(),
            now.into(),
        ];

        if let Err(e) = self.exec_with_params(&sql, params).await {
            tracing::warn!(
                "Failed to upsert vec_collections registry for {}: {} (non-critical, table may not exist yet)",
                collection_id,
                e
            );
        }
    }

    async fn registry_get_collection(&self, collection_id: &str) -> Option<(i32, String)> {
        // 参数化查询，避免 SQL 注入
        let stmt = Statement::from_sql_and_values(
            self.be(),
            "SELECT dimensions, index_type FROM vec_collections WHERE collection_id = $1",
            [collection_id.into()],
        );
        let row = self.db.query_one_raw(stmt).await.ok().flatten();

        row.and_then(|r| {
            let dim: i32 = r.try_get("", "dimensions").ok()?;
            let idx_type: String = r.try_get("", "index_type").ok()?;
            Some((dim, idx_type))
        })
    }

    async fn registry_get_dimensions(&self, collection_id: &str) -> Option<usize> {
        self.registry_get_collection(collection_id).await.map(|(dim, _)| dim as usize)
    }

    async fn registry_delete_collection(&self, collection_id: &str) {
        // 参数化查询
        let _ = self
            .exec_with_params(
                "DELETE FROM vec_collections WHERE collection_id = $1",
                [collection_id.into()],
            )
            .await;
    }

    /// P2-2: 将 collection 在注册表中标记为 disabled，避免坏掉的 collection 被反复使用
    async fn registry_mark_disabled(&self, collection_id: &str) {
        let now = Self::now_ms();
        // 参数化查询：collection_id 用占位符，now 为 i64 安全拼接
        let _ = self
            .exec_with_params(
                &format!(
                    "UPDATE vec_collections SET index_type = 'disabled', updated_at = {now} \
                     WHERE collection_id = $1"
                ),
                [collection_id.into()],
            )
            .await;
    }

    async fn registry_update_vector_count(&self, collection_id: &str) {
        // sanitized 仅允许字母数字和下划线（来自 sanitize_collection_id），表名拼接安全
        let sanitized = Self::sanitize_collection_id(collection_id);
        let now = Self::now_ms();
        let _ = self
            .exec_with_params(
                &format!(
                    "UPDATE vec_collections SET vector_count = (SELECT COUNT(*) FROM vec_{sanitized}_meta), \
                     updated_at = {now}, last_indexed_at = {now} \
                     WHERE collection_id = $1"
                ),
                [collection_id.into()],
            )
            .await;
    }

    async fn registry_get_dimensions_pragma(&self, collection_id: &str) -> Result<Option<usize>> {
        if self.is_pg() {
            // PostgreSQL 维度以 vec_collections 注册表为准，无需解析列类型
            return Ok(None);
        }
        let name = Self::validated_collection_name(collection_id)?;
        let table_exists = self.table_exists(&name).await?;
        if !table_exists {
            return Ok(None);
        }
        let row = self
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                format!(
                    "SELECT dimensions FROM pragma_table_info('{name}') WHERE name = 'embedding'"
                ),
            ))
            .await
            .ok()
            .flatten();
        Ok(row.and_then(|r| r.try_get::<String>("", "type").ok()).and_then(|t| {
            t.trim_start_matches("float[").trim_end_matches(']').parse::<usize>().ok()
        }))
    }

    /// Ensure both the metadata and embedding tables exist for a collection.
    /// Validates that existing vector dimensions match the requested dimensions.
    /// Also registers collection metadata in vec_collections registry table.
    /// For existing collections that pre-date the registry (upgrades), this
    /// will backfill the registry entry automatically.
    pub async fn ensure_collection(&self, collection_id: &str, dimensions: usize) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;

        if self.is_pg() {
            // pgvector 扩展在 PostgreSQL 上按需启用（幂等）
            let _ = self.exec("CREATE EXTENSION IF NOT EXISTS vector").await;
        }

        self.exec(&self.meta_ddl(&name)).await?;

        self.exec(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{name}_doc ON {name}_meta(document_id)"
        ))
        .await?;

        let registry_dim = self.registry_get_dimensions(collection_id).await;
        let pragma_dim = self.registry_get_dimensions_pragma(collection_id).await?;

        if let Some(existing) = registry_dim.or(pragma_dim) {
            if existing != dimensions {
                return Err(AxAgentError::Validation(format!(
                    "Dimension mismatch for collection {collection_id}: existing={existing}, requested={dimensions}. \
                     Rebuild the index or clear the collection before changing embedding dimensions."
                )));
            }
            if registry_dim.is_none() {
                self.registry_upsert_collection(collection_id, dimensions, "flat", None).await;
            }
        } else {
            self.exec(&self.vec_table_ddl(&name, dimensions, None)).await?;
            self.ensure_vector_index(&name, dimensions, None).await;
            self.registry_upsert_collection(collection_id, dimensions, "flat", None).await;
        }

        if self.is_pg() {
            // PostgreSQL 关键词检索由 meta 表的 content_tsv 生成列承担，无需 FTS5
        } else {
            let _ = self.ensure_fts5_index(collection_id).await;
        }

        Ok(())
    }

    /// Query the current embedding dimensions of a collection.
    /// First checks the vec_collections registry, then falls back to pragma parsing.
    /// Returns None if the table does not exist.
    pub async fn get_collection_dimensions(&self, collection_id: &str) -> Result<Option<usize>> {
        validate_collection_name(collection_id)?;

        if let Some(dim) = self.registry_get_dimensions(collection_id).await {
            return Ok(Some(dim));
        }

        self.registry_get_dimensions_pragma(collection_id).await
    }

    /// Prepare collection for (re)indexing: if dimensions match, does nothing;
    /// if dimensions differ (embedding model changed), resets the collection.
    ///
    /// Unlike `ensure_collection`, which errors on dimension mismatch, this method
    /// automatically resets the collection, which is the desired behavior when
    /// starting a fresh indexing job after the embedding provider/model has changed.
    pub async fn prepare_collection_for_indexing(
        &self,
        collection_id: &str,
        dimensions: usize,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");
        let fts_table = format!("{meta_table}_fts");

        let existing_dims = self.get_collection_dimensions(collection_id).await?;

        match existing_dims {
            Some(existing) if existing == dimensions => {
                if !self.is_pg() {
                    let _ = self.ensure_fts5_index(collection_id).await;
                }
                Ok(())
            },
            Some(_mismatched) => {
                tracing::info!(
                    collection_id = %collection_id,
                    old_dim = existing_dims,
                    new_dim = dimensions,
                    "Embedding dimensions changed, resetting collection for re-indexing"
                );
                if self.is_pg() {
                    // PostgreSQL：直接 DROP + 重建向量表（content_tsv 由生成列维护）
                    let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}")).await;
                    self.exec(&self.vec_table_ddl(&name, dimensions, None)).await?;
                    self.ensure_vector_index(&name, dimensions, None).await;
                    self.registry_delete_collection(collection_id).await;
                    Ok(())
                } else {
                    // SQLite：重命名旧表为 _bak_{ts} 再重建，失败时回滚
                    let ts = chrono::Utc::now().timestamp_millis();
                    let backup_name = format!("{name}_bak_{}", ts);
                    let backup_meta = format!("{meta_table}_bak_{}", ts);
                    let backup_fts = format!("{fts_table}_bak_{}", ts);
                    let rename_res =
                        self.exec(&format!("ALTER TABLE {name} RENAME TO {backup_name}")).await;
                    if rename_res.is_err() {
                        // 旧表可能不存在（已删除），忽略
                    }
                    let _ = self
                        .exec(&format!("ALTER TABLE {meta_table} RENAME TO {backup_meta}"))
                        .await;
                    let _ =
                        self.exec(&format!("ALTER TABLE {fts_table} RENAME TO {backup_fts}")).await;
                    match self.ensure_collection(collection_id, dimensions).await {
                        Ok(()) => {
                            let _ = self.exec(&format!("DROP TABLE IF EXISTS {backup_name}")).await;
                            let _ = self.exec(&format!("DROP TABLE IF EXISTS {backup_meta}")).await;
                            let _ = self.exec(&format!("DROP TABLE IF EXISTS {backup_fts}")).await;
                            self.registry_delete_collection(collection_id).await;
                            Ok(())
                        },
                        Err(e) => {
                            let _ = self
                                .exec(&format!("ALTER TABLE {backup_name} RENAME TO {name}"))
                                .await;
                            let _ = self
                                .exec(&format!("ALTER TABLE {backup_meta} RENAME TO {meta_table}"))
                                .await;
                            let _ = self
                                .exec(&format!("ALTER TABLE {backup_fts} RENAME TO {fts_table}"))
                                .await;
                            Err(e)
                        },
                    }
                }
            },
            None => self.ensure_collection(collection_id, dimensions).await,
        }
    }

    /// Ensure a collection exists with HNSW indexing for faster approximate nearest neighbor search.
    pub async fn ensure_collection_hnsw(
        &self,
        collection_id: &str,
        dimensions: usize,
        hnsw_config: HnswConfig,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;

        if self.is_pg() {
            let _ = self.exec("CREATE EXTENSION IF NOT EXISTS vector").await;
        }

        self.exec(&self.meta_ddl(&name)).await?;

        self.exec(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{name}_doc ON {name}_meta(document_id)"
        ))
        .await?;

        let registry_dim = self.registry_get_dimensions(collection_id).await;
        let pragma_dim = self.registry_get_dimensions_pragma(collection_id).await?;

        if let Some(existing) = registry_dim.or(pragma_dim) {
            if existing != dimensions {
                return Err(AxAgentError::Validation(format!(
                    "Dimension mismatch for collection {collection_id}: existing={existing}, requested={dimensions}. \
                     Rebuild the index or clear the collection before changing embedding dimensions."
                )));
            }
            self.registry_upsert_collection(collection_id, dimensions, "hnsw", Some(&hnsw_config))
                .await;
        } else {
            self.exec(&self.vec_table_ddl(&name, dimensions, Some(&hnsw_config))).await?;
            self.ensure_vector_index(&name, dimensions, Some(&hnsw_config)).await;
            self.registry_upsert_collection(collection_id, dimensions, "hnsw", Some(&hnsw_config))
                .await;
        }

        if self.is_pg() {
            // PostgreSQL 关键词检索由 content_tsv 生成列承担
        } else {
            let _ = self.ensure_fts5_index(collection_id).await;
        }

        Ok(())
    }

    /// Upsert embedding records for a single document.
    pub async fn upsert_embeddings(
        &self,
        collection_id: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        if records.is_empty() {
            return Ok(());
        }

        let dimensions = records[0].embedding.len();

        for (i, record) in records.iter().enumerate() {
            if record.embedding.len() != dimensions {
                return Err(AxAgentError::Provider(format!(
                    "Embedding dimension mismatch at record {}: got {} but expected {}",
                    i,
                    record.embedding.len(),
                    dimensions
                )));
            }
        }

        // Serialize upserts per collection to prevent pk conflicts on PostgreSQL
        let upsert_mutex = self.collection_upsert_mutex(collection_id);
        let _upsert_guard = upsert_mutex.lock().await;

        self.prepare_collection_for_indexing(collection_id, dimensions).await?;

        let name = Self::validated_collection_name(collection_id)?;
        let doc_id = &records[0].document_id;

        let txn = self.db.begin().await.map_err(Self::wrap)?;

        let result = async {
            self.delete_rows_by_document_inner(&txn, &name, doc_id).await?;

            let meta_max = self
                .txn_query_one(
                    &txn,
                    &format!("SELECT COALESCE(MAX(rowid), 0) AS max_rid FROM {name}_meta"),
                )
                .await?
                .and_then(|r| r.try_get::<i64>("", "max_rid").ok())
                .unwrap_or(0);

            let vec_max = self
                .txn_query_one(
                    &txn,
                    &format!("SELECT COALESCE(MAX(rowid), 0) AS max_rid FROM {name}"),
                )
                .await
                .ok()
                .flatten()
                .and_then(|r| r.try_get::<i64>("", "max_rid").ok())
                .unwrap_or(0);

            let start_rowid: i64 = meta_max.max(vec_max) + 1;

            // 内容级去重（v17.1 修复）：同 collection 中 content 已存在的 chunk 跳过。
            // document 级去重（delete_rows_by_document_inner）只防同 document_id 重复，
            // 无法防「不同 document_id + 相同内容」的重复导入（如知识图谱重复灌入）。
            // 用 PG 内置 md5() 做字节级比较（text 等值比较会因历史乱码 chunk 触发 UTF8 校验失败）。
            // 批次内同样维护 seen 集合，避免同批两条相同内容（不同 document_id）都插入。
            let mut seen_in_batch: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            for (rid, record) in (start_rowid..).zip(records.iter()) {
                if !seen_in_batch.insert(record.content.clone()) {
                    tracing::info!(
                        "[vector_store] 跳过同批重复内容 chunk: doc={} idx={}",
                        record.document_id,
                        record.chunk_index
                    );
                    continue;
                }
                let dup = txn
                    .query_one_raw(Statement::from_sql_and_values(
                        self.be(),
                        format!(
                            "SELECT 1 AS x FROM {name}_meta WHERE md5(content) = md5($1) LIMIT 1"
                        ),
                        vec![record.content.clone().into()],
                    ))
                    .await
                    .map_err(Self::wrap)?
                    .is_some();
                if dup {
                    tracing::info!(
                        "[vector_store] 跳过已存在内容 chunk: doc={} idx={}",
                        record.document_id,
                        record.chunk_index
                    );
                    continue;
                }

                let vec_json = Self::embedding_to_json(&record.embedding);

                self.txn_exec_params(
                    &txn,
                    &self.embedding_insert_sql(&name),
                    vec![rid.into(), vec_json.into()],
                )
                .await?;

                self.txn_exec_params(
                    &txn,
                    &format!(
                        "INSERT INTO {name}_meta (rowid, id, document_id, chunk_index, content) \
                         VALUES ($1, $2, $3, $4, $5)"
                    ),
                    vec![
                        rid.into(),
                        record.id.clone().into(),
                        record.document_id.clone().into(),
                        record.chunk_index.into(),
                        record.content.clone().into(),
                    ],
                )
                .await?;
            }

            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                txn.commit().await.map_err(Self::wrap)?;
                self.registry_update_vector_count(collection_id).await;
                let cid = collection_id.to_string();
                let db = self.clone();
                // Fire-and-forget: FTS 索引重建失败不应阻塞主写入路径，但需记录错误。
                tokio::spawn(async move {
                    db.rebuild_fts_index(&cid).await;
                });
                Ok(())
            },
            Err(e) => {
                let _ = txn.rollback().await;
                Err(e)
            },
        }
    }

    /// Add a single chunk to an existing collection.
    /// Returns the generated chunk ID.
    pub async fn add_single_chunk(
        &self,
        collection_id: &str,
        document_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<String> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Err(AxAgentError::NotFound("Collection not found".into()));
        }

        // Serialize upserts per collection to prevent pk conflicts on PostgreSQL
        let upsert_mutex = self.collection_upsert_mutex(collection_id);
        let _upsert_guard = upsert_mutex.lock().await;

        let max_index = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                format!("SELECT COALESCE(MAX(chunk_index), -1) AS max_idx FROM {meta_table} WHERE document_id = $1"),
                vec![document_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?
            .and_then(|r| r.try_get::<i32>("", "max_idx").ok())
            .unwrap_or(-1);

        let chunk_index = max_index + 1;
        let chunk_id = format!("{}_{}", document_id, chunk_index);

        let meta_max = self
            .db
            .query_one_raw(Statement::from_string(
                self.be(),
                format!("SELECT COALESCE(MAX(rowid), 0) AS max_rid FROM {meta_table}"),
            ))
            .await
            .map_err(Self::wrap)?
            .and_then(|r| r.try_get::<i64>("", "max_rid").ok())
            .unwrap_or(0);

        let vec_max = self
            .db
            .query_one_raw(Statement::from_string(
                self.be(),
                format!("SELECT COALESCE(MAX(rowid), 0) AS max_rid FROM {name}"),
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get::<i64>("", "max_rid").ok())
            .unwrap_or(0);

        let rid: i64 = meta_max.max(vec_max) + 1;
        let vec_json = Self::embedding_to_json(embedding);

        let txn = self.db.begin().await.map_err(Self::wrap)?;

        if let Err(e) = self
            .txn_exec_params(
                &txn,
                &self.embedding_insert_sql(&name),
                vec![rid.into(), vec_json.into()],
            )
            .await
        {
            let _ = txn.rollback().await;
            return Err(e);
        }

        if let Err(e) = self
            .txn_exec_params(
                &txn,
                &format!(
                    "INSERT INTO {meta_table} (rowid, id, document_id, chunk_index, content) \
                         VALUES ($1, $2, $3, $4, $5)"
                ),
                vec![
                    rid.into(),
                    chunk_id.clone().into(),
                    document_id.to_string().into(),
                    chunk_index.into(),
                    content.to_string().into(),
                ],
            )
            .await
        {
            let _ = txn.rollback().await;
            return Err(e);
        }
        txn.commit().await.map_err(Self::wrap)?;
        self.registry_update_vector_count(collection_id).await;
        let cid = collection_id.to_string();
        let db = self.clone();
        tokio::spawn(async move {
            db.rebuild_fts_index(&cid).await;
        });
        Ok(chunk_id)
    }

    /// Search for the most similar vectors in a knowledge base.
    pub async fn search(
        &self,
        knowledge_base_id: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        self.search_with_filter(knowledge_base_id, query_embedding, top_k, None).await
    }

    /// Vector similarity search with optional `document_id` list filter
    /// (multi-document collaboration: restrict retrieval to a subset of docs).
    ///
    /// When `doc_ids` is `Some` and non-empty, an extra `m.document_id IN (...)`
    /// predicate is added to the SQL; `None` or empty means no filtering.
    pub async fn search_with_filter(
        &self,
        knowledge_base_id: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        doc_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>> {
        validate_collection_name(knowledge_base_id)?;
        let name = Self::validated_collection_name(knowledge_base_id)?;

        if !self.table_exists(&format!("{name}_meta")).await? {
            // 集合尚无任何索引数据（如新建且未写入条目的命名空间），属正常业务场景，
            // 降级为 debug 避免每次会话搜索刷屏
            tracing::debug!("Vector store: table {name}_meta does not exist, returning empty");
            return Ok(vec![]);
        }

        let vec_json = Self::embedding_to_json(&query_embedding);

        // Build the optional document_id IN clause.
        // Empty filter → no constraint; non-empty → parameterised placeholders.
        let (sql, mut params) = match doc_ids {
            Some(ids) if !ids.is_empty() => {
                let base_sql = if self.is_pg() {
                    // PG: $1=embedding, $2=top_k, $3..=doc_ids
                    let placeholders = ids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("${}", i + 3))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let in_clause = format!(" AND m.document_id IN ({placeholders})");
                    format!(
                        "SELECT m.id, m.document_id, m.chunk_index, m.content, v.embedding <=> $1::vector AS distance \
                         FROM {name} v \
                         JOIN {name}_meta m ON m.rowid = v.rowid \
                         WHERE TRUE{in_clause} \
                         ORDER BY distance LIMIT $2"
                    )
                } else {
                    // SQLite: $1=embedding, $2=top_k, $3..=doc_ids
                    let placeholders = ids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("${}", i + 3))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let in_clause = format!(" AND m.document_id IN ({placeholders})");
                    format!(
                        "SELECT m.id, m.document_id, m.chunk_index, m.content, v.distance \
                         FROM {name} v \
                         JOIN {name}_meta m ON m.rowid = v.rowid \
                         WHERE v.embedding MATCH $1 AND k = $2{in_clause} \
                         ORDER BY v.distance"
                    )
                };
                (base_sql, ids.iter().cloned().map(Value::from).collect::<Vec<_>>())
            },
            _ => (self.search_sql(&name), Vec::new()),
        };

        // Compose final parameter list in correct order:
        //   SQLite: [embedding, k, doc_ids...]
        //   PG:     [embedding, k, doc_ids...]
        let mut values = Vec::with_capacity(params.len() + 2);
        values.push(vec_json.into());
        values.push((top_k as i64).into());
        values.append(&mut params);

        let rows = match self
            .db
            .query_all_raw(Statement::from_sql_and_values(self.be(), &sql, values))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Vector store: search query failed for {name}: {e}");
                return Err(AxAgentError::Provider(format!("Vector search failed: {e}")));
            },
        };

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(VectorSearchResult {
                id: row.try_get("", "id").map_err(Self::wrap)?,
                document_id: row.try_get("", "document_id").map_err(Self::wrap)?,
                chunk_index: row.try_get("", "chunk_index").map_err(Self::wrap)?,
                content: row.try_get("", "content").map_err(Self::wrap)?,
                score: row.try_get::<f64>("", "distance").map(|v| v as f32).map_err(Self::wrap)?,
                has_embedding: true,
            });
        }

        Ok(results)
    }

    /// Delete all embeddings belonging to a specific document.
    pub async fn delete_document_embeddings(
        &self,
        knowledge_base_id: &str,
        document_id: &str,
    ) -> Result<()> {
        validate_collection_name(knowledge_base_id)?;
        let name = Self::validated_collection_name(knowledge_base_id)?;

        if !self.table_exists(&format!("{name}_meta")).await? {
            return Ok(());
        }

        self.delete_rows_by_document(&name, document_id).await?;
        self.registry_update_vector_count(knowledge_base_id).await;
        let cid = knowledge_base_id.to_string();
        let db = self.clone();
        // Fire-and-forget: FTS 索引重建失败不应阻塞主删除路径
        tokio::spawn(async move {
            db.rebuild_fts_index(&cid).await;
        });
        Ok(())
    }

    /// 插入仅包含元数据的行（不写入向量，用于存储自定义元信息）
    ///
    /// 使用 INSERT 并忽略唯一约束冲突——如果 id 已存在则直接跳过。
    /// 这在能力索引器多次注册同一 capability 时很重要（启动时 register_all_capabilities
    /// 会先 restore_metadata_from_store 再 index_passport，导致同一个 id 被写入两次）。
    pub async fn insert_metadata_only_chunk(
        &self,
        collection_id: &str,
        document_id: &str,
        chunk_id: &str,
        content: &str,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Err(AxAgentError::NotFound("Collection not found".into()));
        }

        // 幂等 upsert：先按 id 删除旧行，再分配新 rowid 插入。
        // 同一 capability_id 在每次启动重新索引时会重复调用本方法，
        // 若直接裸 INSERT 会违反 meta 表 id 列的唯一约束
        // （vec_capabilities_meta_id_key / vec_*_meta_id_key）。
        self.db
            .execute_raw(Statement::from_sql_and_values(
                self.be(),
                format!("DELETE FROM {meta_table} WHERE id = $1"),
                vec![chunk_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?;

        let max_rid = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                format!("SELECT COALESCE(MAX(rowid), 0) AS max_rid FROM {meta_table}"),
                vec![],
            ))
            .await
            .map_err(Self::wrap)?
            .and_then(|r| r.try_get::<i64>("", "max_rid").ok())
            .unwrap_or(0);

        let new_rid = max_rid + 1;

        self.db
            .execute_raw(Statement::from_sql_and_values(
                self.be(),
                format!(
                    "INSERT INTO {meta_table} (rowid, id, document_id, chunk_index, content) \
                     VALUES ($1, $2, $3, 0, $4)"
                ),
                vec![
                    new_rid.into(),
                    chunk_id.to_string().into(),
                    document_id.to_string().into(),
                    content.to_string().into(),
                ],
            ))
            .await
            .map_err(Self::wrap)?;

        // meta 直写不经 upsert_embeddings，FTS 不会自动感知，需显式触发重建，
        // 否则该 chunk 在关键词检索中永远不可见。
        let cid = collection_id.to_string();
        let db = self.clone();
        tokio::spawn(async move {
            db.rebuild_fts_index(&cid).await;
        });

        Ok(())
    }

    /// 列出所有元数据行（包含 document_id），用于元数据恢复
    pub async fn list_all_metadata_rows(
        &self,
        collection_id: &str,
    ) -> Result<Vec<(i64, String, String, String)>> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Ok(vec![]);
        }

        let rows = self
            .db
            .query_all_raw(Statement::from_string(
                self.be(),
                format!("SELECT rowid, id, document_id, content FROM {meta_table} ORDER BY rowid"),
            ))
            .await
            .map_err(Self::wrap)?;

        let mut result = Vec::new();
        for row in rows {
            let rowid = row.try_get::<i64>("", "rowid").unwrap_or(0);
            let id = row.try_get::<String>("", "id").unwrap_or_default();
            let doc_id = row.try_get::<String>("", "document_id").unwrap_or_default();
            let content = row.try_get::<String>("", "content").unwrap_or_default();
            result.push((rowid, id, doc_id, content));
        }

        Ok(result)
    }

    /// Drop both tables for a knowledge base.
    pub async fn delete_collection(&self, knowledge_base_id: &str) -> Result<()> {
        validate_collection_name(knowledge_base_id)?;
        let name = Self::validated_collection_name(knowledge_base_id)?;
        if self.is_pg() {
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}")).await;
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}_meta")).await;
        } else {
            let fts_table = format!("{name}_meta_fts");
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}")).await;
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}_meta")).await;
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {fts_table}")).await;
        }
        self.registry_delete_collection(knowledge_base_id).await;
        Ok(())
    }

    /// Clear only the embedding vectors (vec0), keeping chunk metadata (_meta) intact.
    pub async fn clear_embeddings(&self, collection_id: &str) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;

        let dim = self.get_collection_dimensions(collection_id).await?;

        if self.is_pg() {
            // PostgreSQL：直接重建向量表（生成列 content_tsv 不受影响）
            if let Some(d) = dim {
                let _ = self.exec(&format!("DROP TABLE IF EXISTS {name}")).await;
                self.exec(&self.vec_table_ddl(&name, d, None)).await?;
                self.ensure_vector_index(&name, d, None).await;
            } else {
                self.registry_mark_disabled(collection_id).await;
            }
            self.registry_update_vector_count(collection_id).await;
            return Ok(());
        }

        // ── SQLite 路径：用临时名重命名旧 vec0 表，失败则回滚 ──
        let ts = chrono::Utc::now().timestamp_millis();
        let tmp_name = format!("{name}_old_{}", ts);

        let rename_res = self.exec(&format!("ALTER TABLE {name} RENAME TO {tmp_name}")).await;
        let renamed = rename_res.is_ok();

        if let Some(d) = dim {
            let create_res = self.exec(&self.vec_table_ddl(&name, d, None)).await;
            if let Err(e) = create_res {
                if renamed {
                    let _ = self.exec(&format!("ALTER TABLE {tmp_name} RENAME TO {name}")).await;
                } else {
                    self.registry_mark_disabled(collection_id).await;
                }
                return Err(AxAgentError::Provider(format!(
                    "重建 vec0 表失败，collection={}, err={}",
                    collection_id, e
                )));
            }
        } else {
            self.registry_mark_disabled(collection_id).await;
        }

        if renamed {
            let _ = self.exec(&format!("DROP TABLE IF EXISTS {tmp_name}")).await;
        }

        self.registry_update_vector_count(collection_id).await;
        Ok(())
    }

    /// List all chunk metadata with rowids for re-embedding.
    pub async fn list_all_chunks(&self, collection_id: &str) -> Result<Vec<(i64, String, String)>> {
        validate_collection_name(collection_id)?;
        self.list_chunks_raw(collection_id, None).await
    }

    /// List chunks (rowid, id, content) for a specific document.
    pub async fn list_document_chunks_raw(
        &self,
        collection_id: &str,
        document_id: &str,
    ) -> Result<Vec<(i64, String, String)>> {
        validate_collection_name(collection_id)?;
        self.list_chunks_raw(collection_id, Some(document_id)).await
    }

    /// Internal helper: list chunks with optional document_id filter.
    async fn list_chunks_raw(
        &self,
        collection_id: &str,
        document_id: Option<&str>,
    ) -> Result<Vec<(i64, String, String)>> {
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Ok(vec![]);
        }

        let rows = if let Some(doc_id) = document_id {
            self.db
                .query_all_raw(Statement::from_sql_and_values(
                    self.be(),
                    format!("SELECT rowid, id, content FROM \"{meta_table}\" WHERE document_id = $1 ORDER BY rowid"),
                    vec![doc_id.to_string().into()],
                ))
                .await
                .map_err(Self::wrap)?
        } else {
            self.db
                .query_all_raw(Statement::from_string(
                    self.be(),
                    format!("SELECT rowid, id, content FROM {meta_table} ORDER BY rowid"),
                ))
                .await
                .map_err(Self::wrap)?
        };

        let mut result = Vec::new();
        for row in &rows {
            let rid: i64 = row.try_get("", "rowid").map_err(Self::wrap)?;
            let id: String = row.try_get("", "id").map_err(Self::wrap)?;
            let content: String = row.try_get("", "content").map_err(Self::wrap)?;
            result.push((rid, id, content));
        }

        Ok(result)
    }

    /// Re-insert embeddings for existing chunks (used after clear_embeddings).
    pub async fn reinsert_embeddings(
        &self,
        collection_id: &str,
        entries: Vec<(i64, Vec<f32>)>, // (rowid, embedding)
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        self.upsert_document_embeddings(collection_id, entries).await
    }

    /// Insert or replace embeddings for specific rowids.
    pub async fn upsert_document_embeddings(
        &self,
        collection_id: &str,
        entries: Vec<(i64, Vec<f32>)>,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        if entries.is_empty() {
            return Ok(());
        }

        let dimensions = entries[0].1.len();
        let name = Self::validated_collection_name(collection_id)?;

        if self.is_pg() {
            let _ = self.exec("CREATE EXTENSION IF NOT EXISTS vector").await;
        }

        // Serialize upserts per collection to prevent pk conflicts on PostgreSQL
        let upsert_mutex = self.collection_upsert_mutex(collection_id);
        let _upsert_guard = upsert_mutex.lock().await;

        self.db
            .execute_raw(Statement::from_string(
                self.be(),
                self.vec_table_ddl(&name, dimensions, None),
            ))
            .await
            .map_err(Self::wrap)?;

        let txn = self.db.begin().await.map_err(Self::wrap)?;

        for (rid, embedding) in &entries {
            if let Err(e) = self
                .txn_exec_params(
                    &txn,
                    &format!("DELETE FROM {name} WHERE rowid = $1"),
                    vec![(*rid).into()],
                )
                .await
            {
                tracing::warn!(
                    "Failed to DELETE existing embedding rowid={} from {}: {} (continuing)",
                    rid,
                    name,
                    e
                );
            }

            let vec_json = Self::embedding_to_json(embedding);
            if let Err(e) = self
                .txn_exec_params(
                    &txn,
                    &self.embedding_insert_sql(&name),
                    vec![(*rid).into(), vec_json.into()],
                )
                .await
            {
                let _ = txn.rollback().await;
                return Err(e);
            };
        }

        txn.commit().await.map_err(Self::wrap)?;
        self.registry_update_vector_count(collection_id).await;
        let cid = collection_id.to_string();
        let db = self.clone();
        tokio::spawn(async move {
            db.rebuild_fts_index(&cid).await;
        });
        Ok(())
    }

    /// Set the embedding model name for a collection in the registry.
    pub async fn set_collection_embedding_model(
        &self,
        collection_id: &str,
        model: Option<&str>,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let cid = collection_id.replace('\'', "''");
        let model_val = match model {
            Some(m) => format!("'{}'", m.replace('\'', "''")),
            None => "NULL".to_string(),
        };
        let now = Self::now_ms();
        if let Err(e) = self
            .exec(&format!(
                "UPDATE vec_collections SET embedding_model={model_val}, updated_at={now} \
                 WHERE collection_id='{cid}'"
            ))
            .await
        {
            tracing::debug!(
                "Failed to update embedding model for {}: {} (non-critical)",
                collection_id,
                e
            );
        }
        Ok(())
    }

    pub async fn ensure_fts5_index(&self, collection_id: &str) -> Result<()> {
        if self.is_pg() {
            // PostgreSQL 关键词检索由 meta 表的 content_tsv 生成列 + GIN 索引承担，
            // 该索引在 meta_ddl 中已建；此处保证索引存在（幂等）。
            let safe_name = Self::sanitize_collection_id(collection_id);
            let meta_table = format!("vec_{safe_name}_meta");
            if self.table_exists(&meta_table).await? {
                let _ = self
                    .exec(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{meta_table}_tsv ON {meta_table} USING GIN (content_tsv)"
                    ))
                    .await;
            }
            return Ok(());
        }

        validate_collection_name(collection_id)?;
        let safe_name = Self::sanitize_collection_id(collection_id);
        let meta_table = format!("vec_{safe_name}_meta");
        let fts_table = format!("{meta_table}_fts");

        let table_exists: bool = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                "SELECT name FROM sqlite_master WHERE type='table' AND name=$1",
                vec![fts_table.clone().into()],
            ))
            .await
            .map(|r| r.is_some())
            .unwrap_or(false);

        // FTS 表已存在时直接返回。内容变更后的索引维护由各写入路径的
        // `rebuild_fts_index`（fire-and-forget）负责，这里不再每次全量 'rebuild'——
        // 该方法在 ensure_collection / 每次检索前都会被调用，全量重建是纯浪费。
        if table_exists {
            return Ok(());
        }

        if !self.table_exists(&meta_table).await? {
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

        if let Err(e) = self.exec(&create_sql).await {
            tracing::debug!("FTS5 trigram index creation failed (non-critical): {}", e);
            return Ok(());
        }

        let populated: Option<i64> = self
            .db
            .query_one_raw(Statement::from_string(
                self.be(),
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
            let _ = self.exec(&populate_sql).await;
        }

        Ok(())
    }

    pub async fn rebuild_fts_index(&self, collection_id: &str) {
        if self.is_pg() {
            // 生成列在内容变更时自动维护，无需手动 rebuild
            return;
        }
        if validate_collection_name(collection_id).is_err() {
            return;
        }
        let safe_name = Self::sanitize_collection_id(collection_id);
        let fts_table = format!("vec_{safe_name}_meta_fts");

        let exists: bool = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                "SELECT name FROM sqlite_master WHERE type='table' AND name=$1",
                vec![fts_table.clone().into()],
            ))
            .await
            .map(|r| r.is_some())
            .unwrap_or(false);

        if exists {
            let rebuild_sql = format!("INSERT INTO {fts_table}({fts_table}) VALUES('rebuild')");
            let _ = self.exec(&rebuild_sql).await;
        }
    }

    /// 基于 FTS5（SQLite）或 content_tsv（PostgreSQL）的关键词检索
    ///
    /// 返回 `VectorSearchResult`，其中 `score` 语义为"越小越匹配"（与向量检索的 distance 一致）：
    /// - SQLite FTS5：`bm25()` 返回负数，越负越匹配，直接使用
    /// - PostgreSQL：`ts_rank()` 返回正数（越大越匹配），取负数统一为"越小越匹配"
    ///
    /// 当 FTS 索引不存在或后端不支持时返回空 Vec（调用方可降级为 keyword_score=0.0）。
    pub async fn fts_search(
        &self,
        collection_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Ok(vec![]);
        }

        let query = query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }

        if self.is_pg() {
            // PostgreSQL：用 content_tsv 生成列 + ts_rank 排序
            let sql = format!(
                "SELECT m.id, m.document_id, m.chunk_index, m.content, \
                 -ts_rank(m.content_tsv, plainto_tsquery('simple', $1)) AS distance \
                 FROM {meta_table} m \
                 WHERE m.content_tsv @@ plainto_tsquery('simple', $1) \
                 ORDER BY distance LIMIT $2"
            );
            let rows = self
                .db
                .query_all_raw(Statement::from_sql_and_values(
                    self.be(),
                    sql,
                    vec![query.to_string().into(), (top_k as i64).into()],
                ))
                .await
                .map_err(Self::wrap)?;

            let mut results = Vec::with_capacity(rows.len());
            for row in &rows {
                results.push(VectorSearchResult {
                    id: row.try_get("", "id").map_err(Self::wrap)?,
                    document_id: row.try_get("", "document_id").map_err(Self::wrap)?,
                    chunk_index: row.try_get("", "chunk_index").map_err(Self::wrap)?,
                    content: row.try_get("", "content").map_err(Self::wrap)?,
                    score: row
                        .try_get::<f64>("", "distance")
                        .map(|v| v as f32)
                        .map_err(Self::wrap)?,
                    has_embedding: true,
                });
            }
            Ok(results)
        } else {
            // SQLite：用 FTS5 虚拟表 + bm25() 排序
            let fts_table = format!("{meta_table}_fts");
            if !self.table_exists(&fts_table).await? {
                // FTS 索引不存在，尝试创建一次
                let _ = self.ensure_fts5_index(collection_id).await;
                if !self.table_exists(&fts_table).await? {
                    return Ok(vec![]);
                }
            }

            let sql = format!(
                "SELECT m.id, m.document_id, m.chunk_index, m.content, \
                 bm25({fts_table}) AS distance \
                 FROM {fts_table} \
                 JOIN {meta_table} m ON m.rowid = {fts_table}.rowid \
                 WHERE {fts_table} MATCH $1 \
                 ORDER BY distance LIMIT $2"
            );
            let rows = self
                .db
                .query_all_raw(Statement::from_sql_and_values(
                    self.be(),
                    sql,
                    vec![query.to_string().into(), (top_k as i64).into()],
                ))
                .await
                .map_err(Self::wrap)?;

            let mut results = Vec::with_capacity(rows.len());
            for row in &rows {
                results.push(VectorSearchResult {
                    id: row.try_get("", "id").map_err(Self::wrap)?,
                    document_id: row.try_get("", "document_id").map_err(Self::wrap)?,
                    chunk_index: row.try_get("", "chunk_index").map_err(Self::wrap)?,
                    content: row.try_get("", "content").map_err(Self::wrap)?,
                    score: row
                        .try_get::<f64>("", "distance")
                        .map(|v| v as f32)
                        .map_err(Self::wrap)?,
                    has_embedding: true,
                });
            }
            Ok(results)
        }
    }

    /// Delete a single chunk by its id from both embedding and metadata tables.
    pub async fn delete_chunk(&self, collection_id: &str, chunk_id: &str) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Ok(());
        }

        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                format!("SELECT rowid FROM {meta_table} WHERE id = $1"),
                vec![chunk_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?;

        if let Some(row) = row {
            let rid: i64 = row.try_get("", "rowid").map_err(Self::wrap)?;

            let txn = self.db.begin().await.map_err(Self::wrap)?;

            if let Err(e) = self
                .txn_exec_params(
                    &txn,
                    &format!("DELETE FROM {name} WHERE rowid = $1"),
                    vec![rid.into()],
                )
                .await
            {
                let _ = txn.rollback().await;
                return Err(e);
            }
            if let Err(e) = self
                .txn_exec_params(
                    &txn,
                    &format!("DELETE FROM {meta_table} WHERE id = $1"),
                    vec![chunk_id.to_string().into()],
                )
                .await
            {
                let _ = txn.rollback().await;
                return Err(e);
            }

            txn.commit().await.map_err(Self::wrap)?;
        }

        self.registry_update_vector_count(collection_id).await;
        let cid = collection_id.to_string();
        let db = self.clone();
        tokio::spawn(async move {
            db.rebuild_fts_index(&cid).await;
        });
        Ok(())
    }

    /// Update the text content of a single chunk in the metadata table.
    pub async fn update_chunk_content(
        &self,
        collection_id: &str,
        chunk_id: &str,
        new_content: &str,
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Err(AxAgentError::NotFound("Collection not found".into()));
        }

        self.db
            .execute_raw(Statement::from_sql_and_values(
                self.be(),
                format!("UPDATE {meta_table} SET content = $1 WHERE id = $2"),
                vec![new_content.to_string().into(), chunk_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?;

        Ok(())
    }

    /// Update the embedding vector for a single chunk identified by its chunk id.
    pub async fn update_chunk_embedding(
        &self,
        collection_id: &str,
        chunk_id: &str,
        embedding: &[f32],
    ) -> Result<()> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Err(AxAgentError::NotFound("Collection not found".into()));
        }

        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                self.be(),
                format!("SELECT rowid FROM {meta_table} WHERE id = $1"),
                vec![chunk_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?
            .ok_or_else(|| AxAgentError::NotFound(format!("Chunk {} not found", chunk_id)))?;

        let rid: i64 = row.try_get("", "rowid").map_err(Self::wrap)?;
        let vec_json = Self::embedding_to_json(embedding);

        self.db
            .execute_raw(Statement::from_sql_and_values(
                self.be(),
                format!("UPDATE {name} SET embedding = $1::vector WHERE rowid = $2"),
                vec![vec_json.into(), rid.into()],
            ))
            .await
            .map_err(Self::wrap)?;

        Ok(())
    }

    // ── private helpers ─────────────────────────────────────────────────

    /// Delete rows from both embedding and metadata tables by `document_id`.
    /// 非事务路径：直接落在连接池连接上（单条删除自动提交，无需事务包裹）。
    async fn delete_rows_by_document(&self, table_name: &str, document_id: &str) -> Result<()> {
        self.exec_with_params(
            &format!(
                "DELETE FROM {table_name} WHERE rowid IN (SELECT rowid FROM {table_name}_meta WHERE document_id = $1)"
            ),
            vec![document_id.to_string().into()],
        )
        .await?;

        self.exec_with_params(
            &format!("DELETE FROM {table_name}_meta WHERE document_id = $1"),
            vec![document_id.to_string().into()],
        )
        .await?;

        Ok(())
    }

    /// Internal implementation of delete_rows_by_document (usable inside a transaction).
    /// 必须在调用方通过 `self.db.begin()` 拿到的 `DatabaseTransaction` 上执行，
    /// 以保证删除与随后的写入落在同一连接、同一事务内。
    async fn delete_rows_by_document_inner(
        &self,
        txn: &DatabaseTransaction,
        table_name: &str,
        document_id: &str,
    ) -> Result<()> {
        self.txn_exec_params(
            txn,
            &format!(
                "DELETE FROM {table_name} WHERE rowid IN (SELECT rowid FROM {table_name}_meta WHERE document_id = $1)"
            ),
            vec![document_id.to_string().into()],
        ).await?;

        self.txn_exec_params(
            txn,
            &format!("DELETE FROM {table_name}_meta WHERE document_id = $1"),
            vec![document_id.to_string().into()],
        )
        .await?;

        Ok(())
    }

    /// Convert an embedding vector to a JSON array string (used for both backends;
    /// on PostgreSQL it is cast to `vector` via `$2::vector` at the SQL level).
    fn embedding_to_json(embedding: &[f32]) -> String {
        let mut buf = String::from("[");
        for (i, v) in embedding.iter().enumerate() {
            if i > 0 {
                buf.push(',');
            }
            use std::fmt::Write;
            let _ = write!(buf, "{v:.16}");
        }
        buf.push(']');
        buf
    }

    /// Check whether a regular table exists in the database.
    pub(crate) async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let row = if self.is_pg() {
            self.db
                .query_one_raw(Statement::from_sql_and_values(
                    self.be(),
                    "SELECT 1 FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = $1",
                    vec![table_name.to_string().into()],
                ))
                .await
                .map_err(Self::wrap)?
        } else {
            self.db
                .query_one_raw(Statement::from_sql_and_values(
                    self.be(),
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=$1",
                    vec![table_name.to_string().into()],
                ))
                .await
                .map_err(Self::wrap)?
        };
        Ok(row.is_some())
    }

    /// List all chunks stored for a specific document within a collection.
    pub async fn list_document_chunks(
        &self,
        collection_id: &str,
        document_id: &str,
    ) -> Result<Vec<VectorSearchResult>> {
        validate_collection_name(collection_id)?;
        let name = Self::validated_collection_name(collection_id)?;
        let meta_table = format!("{name}_meta");

        if !self.table_exists(&meta_table).await? {
            return Ok(vec![]);
        }

        let vec_exists = self.table_exists(&name).await?;

        let sql = if vec_exists {
            format!(
                "SELECT m.id, m.document_id, m.chunk_index, m.content, \
                 CASE WHEN v.rowid IS NOT NULL THEN 1 ELSE 0 END AS has_embedding \
                 FROM \"{meta_table}\" m \
                 LEFT JOIN \"{name}\" v ON m.rowid = v.rowid \
                 WHERE m.document_id = $1 ORDER BY m.chunk_index"
            )
        } else {
            format!(
                "SELECT id, document_id, chunk_index, content, 0 AS has_embedding \
                 FROM \"{meta_table}\" WHERE document_id = $1 ORDER BY chunk_index"
            )
        };

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                self.be(),
                &sql,
                vec![document_id.to_string().into()],
            ))
            .await
            .map_err(Self::wrap)?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let has_emb: i32 = row.try_get("", "has_embedding").unwrap_or(0);
            results.push(VectorSearchResult {
                id: row.try_get("", "id").map_err(Self::wrap)?,
                document_id: row.try_get("", "document_id").map_err(Self::wrap)?,
                chunk_index: row.try_get("", "chunk_index").map_err(Self::wrap)?,
                content: row.try_get("", "content").map_err(Self::wrap)?,
                score: 0.0,
                has_embedding: has_emb != 0,
            });
        }

        Ok(results)
    }

    /// Shorthand for executing a statement with no parameters.
    async fn exec(&self, sql: &str) -> Result<()> {
        self.db.execute_raw(Statement::from_string(self.be(), sql)).await.map_err(Self::wrap)?;
        Ok(())
    }

    /// 执行带参数的 SQL 语句（参数化查询，防止 SQL 注入）。
    async fn exec_with_params(
        &self,
        sql: &str,
        params: impl IntoIterator<Item = sea_orm::Value>,
    ) -> Result<()> {
        let stmt = Statement::from_sql_and_values(self.be(), sql, params);
        self.db.execute_raw(stmt).await.map_err(Self::wrap)?;
        Ok(())
    }

    // ── 事务句柄辅助（绑定单一连接，杜绝连接池下 BEGIN/COMMIT 落到不同连接）──

    async fn txn_exec_params(
        &self,
        txn: &DatabaseTransaction,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<()> {
        txn.execute_raw(Statement::from_sql_and_values(self.be(), sql, params))
            .await
            .map_err(Self::wrap)?;
        Ok(())
    }

    async fn txn_query_one(
        &self,
        txn: &DatabaseTransaction,
        sql: &str,
    ) -> Result<Option<QueryResult>> {
        txn.query_one_raw(Statement::from_string(self.be(), sql.to_string()))
            .await
            .map_err(Self::wrap)
    }

    fn wrap(e: DbErr) -> AxAgentError {
        AxAgentError::Provider(format!("Vector store error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_to_json_uses_dot_decimal() {
        let embedding = vec![0.5_f32, -1.25_f32, 3.14160_f32];
        let json = VectorStore::embedding_to_json(&embedding);
        assert!(!json.contains(",5"), "should not use comma as decimal: {json}");
        assert!(!json.contains(",25"), "should not use comma as decimal: {json}");
        assert!(json.contains("0.5"), "should use dot: {json}");
        assert!(json.contains("-1.25"), "should use dot: {json}");
    }

    #[test]
    fn test_embedding_to_json_format() {
        let embedding = vec![1.0_f32, 2.0_f32];
        let json = VectorStore::embedding_to_json(&embedding);
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains(','));
    }

    #[test]
    fn test_collection_id_validation() {
        assert!(VectorStore::is_valid_collection_id("kb_test-123"));
        assert!(VectorStore::is_valid_collection_id("mem_namespace_1"));
        assert!(!VectorStore::is_valid_collection_id("kb'; DROP TABLE--"));
        assert!(!VectorStore::is_valid_collection_id(""));
    }
}
