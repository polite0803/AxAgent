// SPDX-License-Identifier: AGPL-3.0-only

//! Semantic Cache — reduces duplicate LLM calls for semantically similar prompts.
//!
//! Uses the application's existing sea-orm database connection to store cached
//! LLM responses. Hash-based matching (SHA-256 of normalized prompt) for O(1)
//! lookup. Future: embedding-based cosine similarity search.
//!
//! ## Cache TTL strategy
//!
//! - fact/trivial queries: 7 days
//! - reasoning: 1 hour
//! - code: 24 hours
//! - complex: 1 hour

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use sha2::{Digest, Sha256};

// ─── Config ───

pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { max_entries: 10_000, default_ttl_secs: 3600 }
    }
}

// ─── Cache entry ───

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub id: String,
    pub response: String,
    pub model_id: Option<String>,
    pub token_count: i64,
    pub hit_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
    pub total_hits: usize,
}

// ─── Backend-aware helpers ───

/// 获取当前数据库后端。
fn backend_tag(db: &DatabaseConnection) -> DbBackend {
    db.get_database_backend()
}

// ─── SemanticCache ───

pub struct SemanticCache {
    db: DatabaseConnection,
    config: CacheConfig,
}

impl SemanticCache {
    /// Create a new semantic cache backed by the application database.
    pub async fn new(db: DatabaseConnection, config: CacheConfig) -> Result<Self, String> {
        let be = backend_tag(&db);
        Self::create_table(&db, be).await?;
        tracing::info!(
            "Semantic cache initialized (max_entries={}, default_ttl={}s, backend={:?})",
            config.max_entries,
            config.default_ttl_secs,
            be,
        );
        Ok(Self { db, config })
    }

    /// 创建缓存表 + 索引。每条 SQL 单独执行（PG 不支持 `execute_raw` 多语句）。
    async fn create_table(db: &DatabaseConnection, be: DbBackend) -> Result<(), String> {
        let create_table = if be == DbBackend::Postgres {
            // PG 用 BIGINT 匹配 entity i64；主键用 TEXT 保持与 SQLite 一致
            "CREATE TABLE IF NOT EXISTS semantic_cache (\
             id TEXT PRIMARY KEY, \
             prompt_hash TEXT NOT NULL, \
             response TEXT NOT NULL, \
             model_id TEXT, \
             token_count BIGINT DEFAULT 0, \
             task_type TEXT DEFAULT 'moderate', \
             ttl_secs BIGINT NOT NULL, \
             created_at BIGINT NOT NULL, \
             hit_count BIGINT DEFAULT 0)"
        } else {
            "CREATE TABLE IF NOT EXISTS semantic_cache (\
             id TEXT PRIMARY KEY, \
             prompt_hash TEXT NOT NULL, \
             response TEXT NOT NULL, \
             model_id TEXT, \
             token_count INTEGER DEFAULT 0, \
             task_type TEXT DEFAULT 'moderate', \
             ttl_secs INTEGER NOT NULL, \
             created_at INTEGER NOT NULL, \
             hit_count INTEGER DEFAULT 0)"
        };

        db.execute_unprepared(create_table)
            .await
            .map_err(|e| format!("Failed to create cache table: {}", e))?;

        // 索引分开执行
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_semantic_cache_hash ON semantic_cache(prompt_hash)",
        )
        .await
        .map_err(|e| format!("Failed to create hash index: {}", e))?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_semantic_cache_created ON semantic_cache(created_at)",
        )
        .await
        .map_err(|e| format!("Failed to create created_at index: {}", e))?;

        Ok(())
    }

    /// Normalize a prompt for consistent hashing.
    fn normalize_prompt(prompt: &str) -> String {
        let lower = prompt.to_lowercase();
        let mut result = String::with_capacity(lower.len());
        let mut prev_ws = false;
        for ch in lower.chars() {
            if ch.is_whitespace() {
                if !prev_ws {
                    result.push(' ');
                    prev_ws = true;
                }
            } else {
                result.push(ch);
                prev_ws = false;
            }
        }
        result.trim().to_string()
    }

    /// Compute a SHA-256 hash of the normalized prompt.
    fn hash_prompt(normalized: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check the cache for a matching prompt and model.
    pub async fn check(
        &self,
        prompt: &str,
        model_id: Option<&str>,
    ) -> Result<Option<CacheEntry>, String> {
        let normalized = Self::normalize_prompt(prompt);
        let hash = Self::hash_prompt(&normalized);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let be = backend_tag(&self.db);
        let result = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                be,
                if be == DbBackend::Postgres {
                    "SELECT id, response, model_id, token_count, hit_count \
                     FROM semantic_cache \
                     WHERE prompt_hash = $1 AND model_id IS NOT DISTINCT FROM $2 AND (created_at + ttl_secs) > $3 \
                     LIMIT 1"
                } else {
                    "SELECT id, response, model_id, token_count, hit_count \
                     FROM semantic_cache \
                     WHERE prompt_hash = ?1 AND model_id IS NOT DISTINCT FROM ?2 AND (created_at + ttl_secs) > ?3 \
                     LIMIT 1"
                },
                vec![hash.clone().into(), model_id.map(|s| s.to_string()).into(), now.into()],
            ))
            .await
            .map_err(|e| format!("Query error: {}", e))?;

        if let Some(row) = result {
            let entry = CacheEntry {
                id: row.try_get_by_index::<String>(0).unwrap_or_default(),
                response: row.try_get_by_index::<String>(1).unwrap_or_default(),
                model_id: row.try_get_by_index::<Option<String>>(2).unwrap_or(None),
                token_count: row.try_get_by_index::<i64>(3).unwrap_or(0),
                hit_count: row.try_get_by_index::<i64>(4).unwrap_or(0),
            };

            // Increment hit counter
            let _ = self
                .db
                .execute_raw(Statement::from_sql_and_values(
                    be,
                    "UPDATE semantic_cache SET hit_count = hit_count + 1 WHERE id = $1",
                    vec![entry.id.clone().into()],
                ))
                .await;

            tracing::debug!("Semantic cache HIT for hash={}", &hash[..12]);
            Ok(Some(entry))
        } else {
            tracing::debug!("Semantic cache MISS for hash={}", &hash[..12]);
            Ok(None)
        }
    }

    /// Store a response in the cache.
    pub async fn store(
        &self,
        prompt: &str,
        response: &str,
        model_id: Option<&str>,
        token_count: i64,
        task_type: &str,
        ttl_secs: Option<u64>,
    ) -> Result<(), String> {
        let normalized = Self::normalize_prompt(prompt);
        let hash = Self::hash_prompt(&normalized);
        let ttl = ttl_secs.unwrap_or(self.config.default_ttl_secs) as i64;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let be = backend_tag(&self.db);
        if be == DbBackend::Postgres {
            self.db
                .execute_raw(Statement::from_sql_and_values(
                    be,
                    "INSERT INTO semantic_cache \
                     (id, prompt_hash, response, model_id, token_count, task_type, ttl_secs, created_at, hit_count) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0) \
                     ON CONFLICT (id) DO UPDATE SET \
                     response = EXCLUDED.response, token_count = EXCLUDED.token_count, \
                     task_type = EXCLUDED.task_type, ttl_secs = EXCLUDED.ttl_secs, \
                     created_at = EXCLUDED.created_at",
                    vec![
                        hash.clone().into(),
                        hash.clone().into(),
                        response.to_string().into(),
                        model_id.map(|s| s.to_string()).into(),
                        token_count.into(),
                        task_type.to_string().into(),
                        ttl.into(),
                        now.into(),
                    ],
                ))
                .await
                .map_err(|e| format!("Insert error: {}", e))?;
        } else {
            self.db
                .execute_raw(Statement::from_sql_and_values(
                    be,
                    "INSERT OR REPLACE INTO semantic_cache \
                     (id, prompt_hash, response, model_id, token_count, task_type, ttl_secs, created_at, hit_count) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
                    vec![
                        hash.clone().into(),
                        hash.clone().into(),
                        response.to_string().into(),
                        model_id.map(|s| s.to_string()).into(),
                        token_count.into(),
                        task_type.to_string().into(),
                        ttl.into(),
                        now.into(),
                    ],
                ))
                .await
                .map_err(|e| format!("Insert error: {}", e))?;
        }

        // Evict oldest entries if over limit
        let count: Option<i64> = self
            .db
            .query_one_raw(Statement::from_string(be, "SELECT COUNT(*) FROM semantic_cache"))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok());

        if let Some(c) = count {
            if c > self.config.max_entries as i64 {
                let excess = c - self.config.max_entries as i64;
                let _ = self
                    .db
                    .execute_raw(Statement::from_sql_and_values(
                        be,
                        "DELETE FROM semantic_cache WHERE id IN (\
                         SELECT id FROM semantic_cache ORDER BY created_at ASC LIMIT $1)",
                        vec![excess.into()],
                    ))
                    .await;
                tracing::info!("Semantic cache evicted {} entries", excess);
            }
        }

        tracing::debug!("Semantic cache STORED hash={}", &hash[..12]);
        Ok(())
    }

    /// Get TTL for a given task type (in seconds).
    pub fn ttl_for_task_type(task_type: &str) -> u64 {
        match task_type {
            "fact" | "trivial" => 7 * 24 * 3600,
            "reasoning" => 3600,
            "code" => 24 * 3600,
            "complex" => 3600,
            _ => 3600,
        }
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> Result<CacheStats, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let be = backend_tag(&self.db);

        let total: i64 = self
            .db
            .query_one_raw(Statement::from_string(be, "SELECT COUNT(*) FROM semantic_cache"))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);

        let active: i64 = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                be,
                if be == DbBackend::Postgres {
                    "SELECT COUNT(*) FROM semantic_cache WHERE (created_at + ttl_secs) > $1"
                } else {
                    "SELECT COUNT(*) FROM semantic_cache WHERE (created_at + ttl_secs) > ?1"
                },
                vec![now.into()],
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);

        let total_hits: i64 = self
            .db
            .query_one_raw(Statement::from_string(
                be,
                "SELECT COALESCE(SUM(hit_count), 0) FROM semantic_cache",
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);

        Ok(CacheStats {
            total_entries: total as usize,
            active_entries: active as usize,
            expired_entries: (total - active) as usize,
            total_hits: total_hits as usize,
        })
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_prompt() {
        let result = SemanticCache::normalize_prompt("  Hello   World\n\n  Test   ");
        assert_eq!(result, "hello world test");
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = SemanticCache::hash_prompt(&SemanticCache::normalize_prompt("Hello World"));
        let h2 = SemanticCache::hash_prompt(&SemanticCache::normalize_prompt("  hello   world  "));
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_ttl_for_task_type() {
        assert_eq!(SemanticCache::ttl_for_task_type("fact"), 7 * 24 * 3600);
        assert_eq!(SemanticCache::ttl_for_task_type("reasoning"), 3600);
        assert_eq!(SemanticCache::ttl_for_task_type("code"), 24 * 3600);
        assert_eq!(SemanticCache::ttl_for_task_type("trivial"), 7 * 24 * 3600);
    }
}
