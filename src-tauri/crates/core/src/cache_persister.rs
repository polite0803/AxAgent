//! Cache persistence manager — saves L1 memory caches to disk on shutdown
//! and restores them on startup.
//!
//! Uses atomic write (tmp file → rename) to prevent snapshot corruption
//! if the application crashes mid-write.
//!
//! Snapshots are stored in `{documents_root}/cache/cache_snapshot.json`.

use crate::cache_snapshot::CacheSnapshot;
use std::path::PathBuf;

pub struct CachePersister {
    snapshot_dir: PathBuf,
}

impl CachePersister {
    /// Create a persister that stores snapshots under the application's
    /// documents root (typically `~/Documents/axagent/cache/`).
    pub fn new() -> Self {
        Self {
            snapshot_dir: crate::storage_paths::documents_root().join("cache"),
        }
    }

    /// Create a persister with an explicit snapshot directory (useful for testing).
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { snapshot_dir: dir }
    }

    fn snapshot_path(&self) -> PathBuf {
        self.snapshot_dir.join("cache_snapshot.json")
    }

    fn tmp_path(&self) -> PathBuf {
        self.snapshot_dir.join("cache_snapshot.json.tmp")
    }

    /// Load a previously persisted cache snapshot from disk.
    ///
    /// Returns `None` if the snapshot file does not exist or cannot be parsed.
    /// Callers should handle the `None` case gracefully — a missing snapshot
    /// is not an error, it just means a cold start.
    pub fn load(&self) -> Option<CacheSnapshot> {
        let path = self.snapshot_path();
        if !path.exists() {
            tracing::debug!("No cache snapshot found at {}", path.display());
            return None;
        }
        let data = match std::fs::read_to_string(&path) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to read cache snapshot: {e}");
                return None;
            }
        };
        match serde_json::from_str::<CacheSnapshot>(&data) {
            Ok(snapshot) => {
                tracing::info!(
                    "Loaded cache snapshot v{}: {} embedding entries, {} text hash entries, {} vector search entries",
                    snapshot.version,
                    snapshot.embedding_cache.len(),
                    snapshot.text_hash_cache.len(),
                    snapshot.vector_search_cache.len(),
                );
                Some(snapshot)
            }
            Err(e) => {
                tracing::warn!("Failed to parse cache snapshot: {e}");
                None
            }
        }
    }

    /// Persist a cache snapshot to disk with atomic write semantics.
    ///
    /// Data is first written to a temporary file, then atomically renamed
    /// to the final destination. This prevents snapshot corruption if the
    /// application crashes mid-write.
    pub fn save(&self, snapshot: &CacheSnapshot) -> Result<(), String> {
        std::fs::create_dir_all(&self.snapshot_dir)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;

        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize cache snapshot: {e}"))?;

        // Write to temp file first
        std::fs::write(self.tmp_path(), &json)
            .map_err(|e| format!("Failed to write temporary snapshot: {e}"))?;

        // Atomic rename
        std::fs::rename(self.tmp_path(), self.snapshot_path())
            .map_err(|e| format!("Failed to finalize snapshot: {e}"))?;

        tracing::info!("Cache snapshot persisted ({} bytes)", json.len());
        Ok(())
    }

    /// Delete the persisted snapshot file.
    pub fn clear(&self) {
        let path = self.snapshot_path();
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("Failed to clear cache snapshot: {e}");
            } else {
                tracing::info!("Cache snapshot cleared");
            }
        }
    }

    /// Returns whether a persisted snapshot exists on disk.
    #[must_use]
    pub fn snapshot_exists(&self) -> bool {
        self.snapshot_path().exists()
    }
}

impl Default for CachePersister {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache_snapshot::CacheSnapshot;

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("axagent_cache_test");
        let _ = std::fs::remove_dir_all(&dir);

        let persister = CachePersister::with_dir(dir.clone());
        assert!(!persister.snapshot_exists());

        let mut snapshot = CacheSnapshot::default();
        snapshot
            .embedding_cache
            .insert("hash1".to_string(), vec![0.1, 0.2, 0.3]);
        snapshot
            .text_hash_cache
            .insert("doc1".to_string(), "abc123".to_string());

        persister.save(&snapshot).unwrap();
        assert!(persister.snapshot_exists());

        let loaded = persister.load().unwrap();
        assert_eq!(loaded.embedding_cache.len(), 1);
        assert_eq!(loaded.text_hash_cache.len(), 1);
        assert_eq!(loaded.embedding_cache.get("hash1").unwrap(), &vec![0.1, 0.2, 0.3]);

        // Cleanup
        persister.clear();
        assert!(!persister.snapshot_exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_returns_none() {
        let dir = std::env::temp_dir().join("axagent_cache_missing_test");
        let _ = std::fs::remove_dir_all(&dir);
        let persister = CachePersister::with_dir(dir);
        assert!(persister.load().is_none());
    }

    #[test]
    fn test_default_does_not_panic() {
        let persister = CachePersister::default();
        // Just verify construction doesn't panic — actual path may not exist
        let _ = persister;
    }
}
