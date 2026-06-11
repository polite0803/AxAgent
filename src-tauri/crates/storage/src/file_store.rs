#![allow(clippy::result_large_err)]
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::cloud_storage::SyncEngine;
use crate::storage_paths::validate_relative_path;
use axagent_harness::core_error::Result;

pub struct FileStore {
    base_dir: PathBuf,
    sync_engine: Option<Arc<SyncEngine>>,
}

pub struct SavedFile {
    pub hash: String,
    pub storage_path: String,
    pub size_bytes: i64,
}

impl FileStore {
    pub fn new() -> Self {
        Self {
            base_dir: crate::storage_paths::documents_root(),
            sync_engine: None,
        }
    }

    pub fn with_sync_engine(root: PathBuf, engine: Option<Arc<SyncEngine>>) -> Self {
        Self {
            base_dir: root,
            sync_engine: engine,
        }
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self {
            base_dir: root,
            sync_engine: None,
        }
    }
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStore {
    fn validate_path(&self, storage_path: &str) -> Result<()> {
        validate_relative_path(storage_path).map_err(|msg| {
            axagent_harness::core_error::AxAgentError::Validation(format!(
                "Invalid storage path: {}",
                msg
            ))
        })
    }

    pub fn save_file(
        &self,
        data: &[u8],
        original_name: &str,
        mime_type: &str,
    ) -> Result<SavedFile> {
        let hash = Self::compute_hash(data);
        let relative_path =
            crate::storage_paths::build_relative_path(original_name, mime_type, &hash);
        let abs_path = self.base_dir.join(&relative_path);

        if let Some(parent) = abs_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if !abs_path.exists() {
            std::fs::write(&abs_path, data)?;
        }

        if let Some(ref engine) = self.sync_engine {
            let key = relative_path.clone();
            let data_vec = data.to_vec();
            let mime = mime_type.to_string();
            let engine = engine.clone();
            tokio::spawn(async move {
                let _ = engine.backend.put(&key, &data_vec, &mime).await;
            });
        }

        Ok(SavedFile {
            hash,
            storage_path: relative_path,
            size_bytes: data.len() as i64,
        })
    }

    pub fn read_file(&self, storage_path: &str) -> Result<Vec<u8>> {
        self.validate_path(storage_path)?;
        let path = self.resolve_path(storage_path);

        if path.exists() {
            return Ok(std::fs::read(&path)?);
        }

        #[cfg(mobile)]
        if let Some(ref engine) = self.sync_engine {
            let rt = tokio::runtime::Handle::current();
            let fetch_result = rt.block_on(engine.fetch_file(storage_path, &path));
            if fetch_result.is_ok() {
                return Ok(std::fs::read(&path)?);
            }
        }

        Err(axagent_harness::core_error::AxAgentError::NotFound(format!(
            "File not found: {}",
            storage_path
        )))
    }

    pub fn delete_file(&self, storage_path: &str) -> Result<()> {
        self.validate_path(storage_path)?;
        let path = self.resolve_path(storage_path);

        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        if let Some(ref engine) = self.sync_engine {
            let key = storage_path.to_string();
            let engine = engine.clone();
            tokio::spawn(async move {
                let _ = engine.backend.delete(&key).await;
            });
        }

        Ok(())
    }

    fn resolve_path(&self, storage_path: &str) -> PathBuf {
        let resolved = self.base_dir.join(storage_path);

        resolved.components().collect::<PathBuf>()
    }

    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
