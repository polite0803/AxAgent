//! Serializable cache snapshot used for persistence between application sessions.
//!
//! When the application shuts down, cached data from L1 memory caches
//! (EmbeddingCache, TextHashCache, VectorSearchCache) is serialized into
//! a `CacheSnapshot` and written atomically to disk. On next startup,
//! the snapshot is deserialized and loaded back into memory, avoiding
//! expensive cold-start cache rebuilds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::vector_cache::CacheEntrySnapshot;

/// A snapshot of in-memory cache state, serializable for disk persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSnapshot {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// ISO 8601 timestamp of when the snapshot was created.
    pub created_at: String,
    /// Entries from the embedding cache (text hash → embedding vector).
    pub embedding_cache: HashMap<String, Vec<f32>>,
    /// Entries from the text hash cache (document ID → hash string).
    pub text_hash_cache: HashMap<String, String>,
    /// Entries from the vector search cache (query key → serialized results).
    pub vector_search_cache: Vec<CacheEntrySnapshot>,
}

impl Default for CacheSnapshot {
    fn default() -> Self {
        Self {
            version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            embedding_cache: HashMap::new(),
            text_hash_cache: HashMap::new(),
            vector_search_cache: Vec::new(),
        }
    }
}
