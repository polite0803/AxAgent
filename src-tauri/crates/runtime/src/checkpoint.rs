//! Checkpoint System - Persistent state snapshots for recovery
//!
//! Features:
//! - Checkpoint creation and restoration
//! - Incremental snapshots
//! - Metadata tracking
//! - Auto-cleanup of old checkpoints

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    pub checkpoint_type: CheckpointType,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub metadata: CheckpointMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointType {
    Full,
    Incremental,
    StateOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub session_id: String,
    pub agent_id: Option<String>,
    pub workflow_id: Option<String>,
    pub step_index: Option<usize>,
    pub parent_checkpoint: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CheckpointManager {
    checkpoints: Arc<RwLock<HashMap<String, Checkpoint>>>,
    storage_dir: PathBuf,
    max_checkpoints: usize,
    auto_cleanup: bool,
}

impl CheckpointManager {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
            max_checkpoints: 10,
            auto_cleanup: true,
        }
    }

    pub fn with_max_checkpoints(mut self, max: usize) -> Self {
        self.max_checkpoints = max;
        self
    }

    pub fn with_auto_cleanup(mut self, enabled: bool) -> Self {
        self.auto_cleanup = enabled;
        self
    }

    pub fn create_checkpoint(
        &self,
        name: &str,
        checkpoint_type: CheckpointType,
        data: &[u8],
        metadata: CheckpointMetadata,
    ) -> Result<Checkpoint, CheckpointError> {
        let checkpoint_id = format!("cp_{}", uuid_simple());
        let filename = format!("{}.bin", checkpoint_id);
        let path = self.storage_dir.join(&filename);

        fs::create_dir_all(&self.storage_dir)
            .map_err(|e| CheckpointError::IoError(e.to_string()))?;

        fs::write(&path, data).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        let size_bytes = data.len() as u64;

        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            name: name.to_string(),
            checkpoint_type,
            path,
            size_bytes,
            created_at: Utc::now(),
            metadata,
        };

        {
            let mut checkpoints = self
                .checkpoints
                .write()
                .map_err(|_| CheckpointError::LockError)?;
            checkpoints.insert(checkpoint_id, checkpoint.clone());
        }

        if self.auto_cleanup {
            let _ = self.cleanup_old_checkpoints();
        }

        Ok(checkpoint)
    }

    pub fn get_checkpoint(&self, id: &str) -> Result<Checkpoint, CheckpointError> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| CheckpointError::LockError)?;
        checkpoints
            .get(id)
            .cloned()
            .ok_or_else(|| CheckpointError::NotFound(id.to_string()))
    }

    pub fn load_checkpoint_data(&self, id: &str) -> Result<Vec<u8>, CheckpointError> {
        let checkpoint = self.get_checkpoint(id)?;
        fs::read(&checkpoint.path).map_err(|e| CheckpointError::IoError(e.to_string()))
    }

    pub fn list_checkpoints(&self) -> Result<Vec<Checkpoint>, CheckpointError> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| CheckpointError::LockError)?;
        let mut list: Vec<Checkpoint> = checkpoints.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }

    pub fn delete_checkpoint(&self, id: &str) -> Result<(), CheckpointError> {
        let checkpoint = self.get_checkpoint(id)?;

        if checkpoint.path.exists() {
            fs::remove_file(&checkpoint.path)
                .map_err(|e| CheckpointError::IoError(e.to_string()))?;
        }

        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| CheckpointError::LockError)?;
        checkpoints.remove(id);

        Ok(())
    }

    pub fn cleanup_old_checkpoints(&self) -> Result<usize, CheckpointError> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| CheckpointError::LockError)?;

        if checkpoints.len() <= self.max_checkpoints {
            return Ok(0);
        }

        let mut list: Vec<(String, DateTime<Utc>)> = checkpoints
            .iter()
            .map(|(id, cp)| (id.clone(), cp.created_at))
            .collect();

        list.sort_by(|a, b| b.1.cmp(&a.1));

        let to_remove: Vec<String> = list
            .into_iter()
            .skip(self.max_checkpoints)
            .map(|(id, _)| id)
            .collect();

        for id in &to_remove {
            if let Some(cp) = checkpoints.get(id) {
                if cp.path.exists() {
                    let _ = fs::remove_file(&cp.path);
                }
            }
            checkpoints.remove(id);
        }

        Ok(to_remove.len())
    }

    pub fn get_checkpoints_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<Checkpoint>, CheckpointError> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| CheckpointError::LockError)?;
        let mut result: Vec<Checkpoint> = checkpoints
            .values()
            .filter(|cp| cp.metadata.session_id == session_id)
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    pub fn get_latest_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Checkpoint>, CheckpointError> {
        Ok(self
            .get_checkpoints_for_session(session_id)?
            .into_iter()
            .next())
    }

    pub fn total_size(&self) -> Result<u64, CheckpointError> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| CheckpointError::LockError)?;
        Ok(checkpoints.values().map(|cp| cp.size_bytes).sum())
    }

    pub fn restore(&self, checkpoint_id: &str) -> Result<Vec<u8>, CheckpointError> {
        let checkpoint = self.get_checkpoint(checkpoint_id)?;
        let data =
            fs::read(&checkpoint.path).map_err(|e| CheckpointError::IoError(e.to_string()))?;
        Ok(data)
    }

    pub fn restore_with_rebuild(
        &self,
        checkpoint_id: &str,
        _incremental_chain: &[String],
    ) -> Result<Vec<u8>, CheckpointError> {
        let checkpoint = self.get_checkpoint(checkpoint_id)?;

        if checkpoint.checkpoint_type == CheckpointType::Full {
            return self.restore(checkpoint_id);
        }

        let mut reconstructed: Vec<u8> = Vec::new();

        if let Some(parent_id) = &checkpoint.metadata.parent_checkpoint {
            let parent_data = self.restore(parent_id)?;
            reconstructed.extend(parent_data);
        }

        let current_data =
            fs::read(&checkpoint.path).map_err(|e| CheckpointError::IoError(e.to_string()))?;
        reconstructed.extend(current_data);

        Ok(reconstructed)
    }

    pub fn create_incremental(
        &self,
        name: &str,
        base_checkpoint_id: &str,
        delta_data: &[u8],
        metadata: CheckpointMetadata,
    ) -> Result<Checkpoint, CheckpointError> {
        let _base_checkpoint = self.get_checkpoint(base_checkpoint_id)?;

        let incremental_id = format!("cp_{}", uuid_simple());
        let filename = format!("{}.inc", incremental_id);
        let path = self.storage_dir.join(&filename);

        fs::create_dir_all(&self.storage_dir)
            .map_err(|e| CheckpointError::IoError(e.to_string()))?;

        fs::write(&path, delta_data).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        let mut new_metadata = metadata;
        new_metadata.parent_checkpoint = Some(base_checkpoint_id.to_string());

        let checkpoint = Checkpoint {
            id: incremental_id.clone(),
            name: name.to_string(),
            checkpoint_type: CheckpointType::Incremental,
            path,
            size_bytes: delta_data.len() as u64,
            created_at: Utc::now(),
            metadata: new_metadata,
        };

        {
            let mut checkpoints = self
                .checkpoints
                .write()
                .map_err(|_| CheckpointError::LockError)?;
            checkpoints.insert(incremental_id, checkpoint.clone());
        }

        Ok(checkpoint)
    }

    pub fn verify_checkpoint(&self, checkpoint_id: &str) -> Result<bool, CheckpointError> {
        let checkpoint = self.get_checkpoint(checkpoint_id)?;
        if !checkpoint.path.exists() {
            return Ok(false);
        }
        let data =
            fs::read(&checkpoint.path).map_err(|e| CheckpointError::IoError(e.to_string()))?;
        Ok(data.len() as u64 == checkpoint.size_bytes)
    }

    pub fn import_checkpoint(&mut self, path: PathBuf) -> Result<Checkpoint, CheckpointError> {
        let data = fs::read(&path).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("imported.bin")
            .to_string();

        let new_path = self.storage_dir.join(&filename);
        fs::copy(&path, &new_path).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        let checkpoint = Checkpoint {
            id: uuid_simple(),
            name: format!("imported_{}", filename),
            checkpoint_type: CheckpointType::Full,
            path: new_path,
            size_bytes: data.len() as u64,
            created_at: Utc::now(),
            metadata: CheckpointMetadata {
                session_id: "imported".to_string(),
                agent_id: None,
                workflow_id: None,
                step_index: None,
                parent_checkpoint: None,
                tags: vec!["imported".to_string()],
            },
        };

        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| CheckpointError::LockError)?;
        checkpoints.insert(checkpoint.id.clone(), checkpoint.clone());

        Ok(checkpoint)
    }

    pub fn export_checkpoint(
        &self,
        checkpoint_id: &str,
        dest_path: &PathBuf,
    ) -> Result<(), CheckpointError> {
        let checkpoint = self.get_checkpoint(checkpoint_id)?;
        fs::copy(&checkpoint.path, dest_path)
            .map_err(|e| CheckpointError::IoError(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum CheckpointError {
    IoError(String),
    NotFound(String),
    LockError,
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::NotFound(id) => write!(f, "Checkpoint not found: {}", id),
            Self::LockError => write!(f, "Failed to acquire lock"),
        }
    }
}

impl std::error::Error for CheckpointError {}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_secs(), now.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let temp_path = std::env::temp_dir().join("checkpoint_test");
        let manager = CheckpointManager::new(temp_path.clone());

        let metadata = CheckpointMetadata {
            session_id: "session_1".to_string(),
            agent_id: None,
            workflow_id: None,
            step_index: None,
            parent_checkpoint: None,
            tags: vec!["test".to_string()],
        };

        let data = b"checkpoint data";
        let cp = manager
            .create_checkpoint("test_cp", CheckpointType::Full, data, metadata)
            .unwrap();

        assert_eq!(cp.name, "test_cp");
        assert_eq!(cp.size_bytes, 15);
    }

    #[test]
    fn test_checkpoint_retrieval() {
        let temp_path = std::env::temp_dir().join("checkpoint_test2");
        let manager = CheckpointManager::new(temp_path);

        let metadata = CheckpointMetadata {
            session_id: "session_1".to_string(),
            agent_id: None,
            workflow_id: None,
            step_index: None,
            parent_checkpoint: None,
            tags: vec![],
        };

        let cp = manager
            .create_checkpoint("test", CheckpointType::Full, b"data", metadata)
            .unwrap();
        let loaded = manager.get_checkpoint(&cp.id).unwrap();

        assert_eq!(loaded.id, cp.id);
    }

    #[test]
    fn test_auto_cleanup() {
        let temp_path = std::env::temp_dir().join("checkpoint_test3");
        let manager = CheckpointManager::new(temp_path)
            .with_max_checkpoints(3)
            .with_auto_cleanup(true);

        let metadata = CheckpointMetadata {
            session_id: "session_1".to_string(),
            agent_id: None,
            workflow_id: None,
            step_index: None,
            parent_checkpoint: None,
            tags: vec![],
        };

        for i in 0..5 {
            let m = CheckpointMetadata {
                session_id: format!("session_{}", i),
                ..metadata.clone()
            };
            let _ =
                manager.create_checkpoint(&format!("cp_{}", i), CheckpointType::Full, b"data", m);
        }

        let checkpoints = manager.list_checkpoints().unwrap();
        assert!(checkpoints.len() <= 3);
    }
}
