//! Cloud storage trait and DTOs
//!
//! Defines the `StorageBackend` trait and its supporting types.
//! Implementations live in `axagent-core`, consumers in `axagent-storage`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;

// ─── Storage Object Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub key: String,
    pub data: Vec<u8>,
    pub content_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObjectMeta {
    pub key: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub objects: Vec<StorageObjectMeta>,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
}

// ─── Storage Backend Trait ────────────────────────────────────────────

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<StorageObject>;
    async fn put(&self, key: &str, data: &[u8], content_type: &str) -> Result<StorageObjectMeta>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(
        &self,
        prefix: &str,
        limit: usize,
        continuation_token: Option<&str>,
    ) -> Result<ListResult>;
    async fn head(&self, key: &str) -> Result<StorageObjectMeta>;
    async fn check_connection(&self) -> Result<bool>;

    async fn delete_if_match(&self, key: &str, etag: &str) -> Result<bool> {
        let meta = self.head(key).await?;
        if meta.etag.as_deref() != Some(etag) {
            return Ok(false);
        }
        self.delete(key).await?;
        Ok(true)
    }
}
