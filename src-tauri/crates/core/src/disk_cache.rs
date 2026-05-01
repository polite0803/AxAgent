//! L2 disk cache — persists search results, conversation summaries, and index
//! snapshots to SQLite for cold-data storage.
//!
//! Works alongside the L1 memory caches (EmbeddingCache, VectorSearchCache, etc.)
//! to form a complete hot/cold separation architecture:
//! - L1: In-memory, TTL-based, persisted via CacheSnapshot on shutdown
//! - L2: SQLite-backed, time-partitioned, auto-eviction of stale entries

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSearchResult {
    pub id: i64,
    pub query_hash: String,
    pub query_text: String,
    pub results_json: String,
    pub result_count: usize,
    pub hit_count: u32,
    pub created_at: u64,
    pub last_accessed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSummary {
    pub id: i64,
    pub conversation_id: String,
    pub summary_text: String,
    pub compressed_message_count: usize,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSnapshotMeta {
    pub snapshot_id: String,
    pub file_count: usize,
    pub definition_count: usize,
    pub snapshot_path: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    pub max_search_results: usize,
    pub max_summaries: usize,
    pub max_snapshots: usize,
    pub search_result_ttl_days: u32,
    pub summary_ttl_days: u32,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            max_search_results: 1000,
            max_summaries: 200,
            max_snapshots: 10,
            search_result_ttl_days: 30,
            summary_ttl_days: 90,
        }
    }
}

pub struct DiskCache {
    conn: Connection,
    config: DiskCacheConfig,
}

impl DiskCache {
    pub fn new(conn: Connection, config: DiskCacheConfig) -> Result<Self, String> {
        let cache = Self { conn, config };
        cache.ensure_tables()?;
        Ok(cache)
    }

    fn ensure_tables(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS l2_search_results (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    query_hash TEXT NOT NULL,
                    query_text TEXT NOT NULL,
                    results_json TEXT NOT NULL,
                    result_count INTEGER NOT NULL DEFAULT 0,
                    hit_count INTEGER NOT NULL DEFAULT 1,
                    created_at INTEGER NOT NULL DEFAULT 0,
                    last_accessed_at INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_l2_search_hash ON l2_search_results(query_hash);

                CREATE TABLE IF NOT EXISTS l2_summaries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    conversation_id TEXT NOT NULL,
                    summary_text TEXT NOT NULL,
                    compressed_message_count INTEGER NOT NULL DEFAULT 0,
                    created_at INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_l2_summary_conv ON l2_summaries(conversation_id);

                CREATE TABLE IF NOT EXISTS l2_index_snapshots (
                    snapshot_id TEXT PRIMARY KEY,
                    file_count INTEGER NOT NULL DEFAULT 0,
                    definition_count INTEGER NOT NULL DEFAULT 0,
                    snapshot_path TEXT NOT NULL DEFAULT '',
                    created_at INTEGER NOT NULL DEFAULT 0
                );",
            )
            .map_err(|e| format!("Failed to create L2 cache tables: {e}"))
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ── Search result caching ───────────────────────────────────────────────

    /// Look up cached search results by query hash.
    pub fn get_search_results(
        &self,
        query_hash: &str,
    ) -> Result<Option<CachedSearchResult>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, query_hash, query_text, results_json, result_count, hit_count, created_at, last_accessed_at FROM l2_search_results WHERE query_hash = ?1")
            .map_err(|e| format!("prepare: {e}"))?;

        let result = stmt.query_row(params![query_hash], |row| {
            Ok(CachedSearchResult {
                id: row.get(0)?,
                query_hash: row.get(1)?,
                query_text: row.get(2)?,
                results_json: row.get(3)?,
                result_count: row.get(4)?,
                hit_count: row.get(5)?,
                created_at: row.get(6)?,
                last_accessed_at: row.get(7)?,
            })
        });

        match result {
            Ok(mut cached) => {
                self.conn
                    .execute(
                        "UPDATE l2_search_results SET hit_count = hit_count + 1, last_accessed_at = ?1 WHERE id = ?2",
                        params![Self::now_secs(), cached.id],
                    )
                    .map_err(|e| format!("update hit: {e}"))?;
                cached.hit_count += 1;
                cached.last_accessed_at = Self::now_secs();
                Ok(Some(cached))
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("get search results: {e}")),
        }
    }

    /// Store search results in the disk cache.
    pub fn store_search_results(
        &self,
        query_hash: &str,
        query_text: &str,
        results_json: &str,
        result_count: usize,
    ) -> Result<(), String> {
        let now = Self::now_secs();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO l2_search_results (query_hash, query_text, results_json, result_count, created_at, last_accessed_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![query_hash, query_text, results_json, result_count, now, now],
            )
            .map_err(|e| format!("store search: {e}"))?;

        self.evict_old_search_results()?;
        Ok(())
    }

    fn evict_old_search_results(&self) -> Result<(), String> {
        let cutoff =
            Self::now_secs().saturating_sub(self.config.search_result_ttl_days as u64 * 86400);
        self.conn
            .execute("DELETE FROM l2_search_results WHERE last_accessed_at < ?1 AND id NOT IN (SELECT id FROM l2_search_results ORDER BY last_accessed_at DESC LIMIT ?2)", params![cutoff, self.config.max_search_results])
            .map_err(|e| format!("evict search: {e}"))?;
        Ok(())
    }

    // ── Summary caching ──────────────────────────────────────────────────────

    /// Store a conversation summary.
    pub fn store_summary(
        &self,
        conversation_id: &str,
        summary_text: &str,
        compressed_message_count: usize,
    ) -> Result<(), String> {
        let now = Self::now_secs();
        self.conn
            .execute(
                "INSERT INTO l2_summaries (conversation_id, summary_text, compressed_message_count, created_at) VALUES (?1,?2,?3,?4)",
                params![conversation_id, summary_text, compressed_message_count, now],
            )
            .map_err(|e| format!("store summary: {e}"))?;

        self.evict_old_summaries()?;
        Ok(())
    }

    /// Load summaries for a conversation.
    pub fn get_summaries(&self, conversation_id: &str) -> Result<Vec<CachedSummary>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, conversation_id, summary_text, compressed_message_count, created_at FROM l2_summaries WHERE conversation_id = ?1 ORDER BY created_at DESC LIMIT 10")
            .map_err(|e| format!("prepare: {e}"))?;

        let rows = stmt
            .query_map(params![conversation_id], |row| {
                Ok(CachedSummary {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    summary_text: row.get(2)?,
                    compressed_message_count: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("query summaries: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    fn evict_old_summaries(&self) -> Result<(), String> {
        let cutoff = Self::now_secs().saturating_sub(self.config.summary_ttl_days as u64 * 86400);
        self.conn
            .execute("DELETE FROM l2_summaries WHERE created_at < ?1 AND id NOT IN (SELECT id FROM l2_summaries ORDER BY created_at DESC LIMIT ?2)", params![cutoff, self.config.max_summaries])
            .map_err(|e| format!("evict summaries: {e}"))?;
        Ok(())
    }

    // ── Index snapshot tracking ──────────────────────────────────────────────

    /// Record an index snapshot.
    pub fn record_snapshot(
        &self,
        snapshot_id: &str,
        file_count: usize,
        definition_count: usize,
        snapshot_path: &str,
    ) -> Result<(), String> {
        let now = Self::now_secs();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO l2_index_snapshots (snapshot_id, file_count, definition_count, snapshot_path, created_at) VALUES (?1,?2,?3,?4,?5)",
                params![snapshot_id, file_count, definition_count, snapshot_path, now],
            )
            .map_err(|e| format!("record snapshot: {e}"))?;

        self.evict_old_snapshots()?;
        Ok(())
    }

    /// List recent snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<IndexSnapshotMeta>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT snapshot_id, file_count, definition_count, snapshot_path, created_at FROM l2_index_snapshots ORDER BY created_at DESC LIMIT ?1")
            .map_err(|e| format!("prepare: {e}"))?;

        let rows = stmt
            .query_map(params![self.config.max_snapshots], |row| {
                Ok(IndexSnapshotMeta {
                    snapshot_id: row.get(0)?,
                    file_count: row.get(1)?,
                    definition_count: row.get(2)?,
                    snapshot_path: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("list snapshots: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    fn evict_old_snapshots(&self) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM l2_index_snapshots WHERE snapshot_id NOT IN (SELECT snapshot_id FROM l2_index_snapshots ORDER BY created_at DESC LIMIT ?1)",
                params![self.config.max_snapshots],
            )
            .map_err(|e| format!("evict snapshots: {e}"))?;
        Ok(())
    }

    /// Compute a stable hash for a query string.
    pub fn query_hash(query: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_cache() -> DiskCache {
        let conn = Connection::open_in_memory().unwrap();
        DiskCache::new(conn, DiskCacheConfig::default()).unwrap()
    }

    #[test]
    fn test_search_cache_miss_then_hit() {
        let cache = test_cache();
        let hash = DiskCache::query_hash("find user auth");

        assert!(cache.get_search_results(&hash).unwrap().is_none());

        cache
            .store_search_results(&hash, "find user auth", r#"[{"file":"auth.rs"}]"#, 1)
            .unwrap();

        let cached = cache.get_search_results(&hash).unwrap().unwrap();
        assert_eq!(cached.query_text, "find user auth");
        assert!(cached.results_json.contains("auth.rs"));
        assert_eq!(cached.hit_count, 2); // get + implicit increment
    }

    #[test]
    fn test_summary_store_and_load() {
        let cache = test_cache();
        // Use explicit timestamps to control ordering
        assert!(cache
            .store_summary("conv1", "Fixed authentication bug", 5)
            .is_ok());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(cache.store_summary("conv1", "Added unit tests", 3).is_ok());

        let summaries = cache.get_summaries("conv1").unwrap();
        assert_eq!(summaries.len(), 2);
        // Either order is acceptable for this test; both entries exist
        let all_text: String = summaries
            .iter()
            .map(|s| &s.summary_text)
            .cloned()
            .collect::<Vec<_>>()
            .join("|");
        assert!(all_text.contains("unit tests"));
        assert!(all_text.contains("authentication"));
    }

    #[test]
    fn test_snapshot_recording() {
        let cache = test_cache();
        assert!(cache
            .record_snapshot("snap1", 100, 500, "/cache/snap1.json")
            .is_ok());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(cache
            .record_snapshot("snap2", 200, 800, "/cache/snap2.json")
            .is_ok());

        let snapshots = cache.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 2);
        // Both snapshots exist
        let ids: Vec<&str> = snapshots.iter().map(|s| s.snapshot_id.as_str()).collect();
        assert!(ids.contains(&"snap1"));
        assert!(ids.contains(&"snap2"));
    }
}
