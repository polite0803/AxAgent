// SPDX-License-Identifier: AGPL-3.0-only

//! 同步引擎实现
//!
//! 实现 SyncEngine trait，提供全量/增量同步、
//! 变更推送/拉取、冲突检测与解决等核心功能。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::device_sync::{
    ChangeLogEntry, ChangeOperation, ConflictInfo, ConflictResolutionStrategy, DeviceSyncStatus,
    EntityType, SyncEngine, SyncResult, VersionVectorEntry,
};
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::conflict_resolver::ConflictResolver;
use crate::crdt::{CrdtEngine, OperationType};
use crate::history_store::HistoryStore;
use crate::manager::DeviceStore;
use crate::version_vector::VersionVector;

/// 变更日志存储
#[derive(Default)]
pub struct ChangeLogStore {
    entries: RwLock<Vec<ChangeLogEntry>>,
    last_sync_per_device: RwLock<HashMap<String, u64>>,
}

impl ChangeLogStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add_entry(&self, entry: ChangeLogEntry) {
        self.entries.write().await.push(entry);
    }

    pub async fn get_entries_since(&self, timestamp: u64) -> Vec<ChangeLogEntry> {
        let entries = self.entries.read().await;
        entries.iter().filter(|e| e.timestamp > timestamp).cloned().collect()
    }

    pub async fn get_all_entries(&self) -> Vec<ChangeLogEntry> {
        self.entries.read().await.clone()
    }

    pub async fn set_last_sync(&self, device_id: &str, timestamp: u64) {
        self.last_sync_per_device
            .write()
            .await
            .insert(device_id.to_string(), timestamp);
    }

    pub async fn get_last_sync(&self, device_id: &str) -> u64 {
        self.last_sync_per_device
            .read()
            .await
            .get(device_id)
            .copied()
            .unwrap_or(0)
    }

    pub async fn cleanup_before(&self, timestamp: u64) -> usize {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|e| e.timestamp >= timestamp);
        before - entries.len()
    }
}

/// 同步引擎实现
pub struct SyncEngineImpl {
    change_log: Arc<ChangeLogStore>,
    device_store: Arc<DeviceStore>,
    history_store: Arc<HistoryStore>,
    crdt_engine: Arc<RwLock<CrdtEngine>>,
    local_device_id: String,
}

impl SyncEngineImpl {
    pub fn new(
        change_log: Arc<ChangeLogStore>,
        device_store: Arc<DeviceStore>,
        history_store: Arc<HistoryStore>,
        local_device_id: String,
    ) -> Self {
        Self {
            change_log,
            device_store,
            history_store,
            crdt_engine: Arc::new(RwLock::new(CrdtEngine::new())),
            local_device_id,
        }
    }

    /// 获取变更日志存储引用
    pub fn change_log(&self) -> &Arc<ChangeLogStore> {
        &self.change_log
    }

    /// 获取设备存储引用
    pub fn device_store(&self) -> &Arc<DeviceStore> {
        &self.device_store
    }

    /// 获取历史存储引用
    pub fn history_store(&self) -> &Arc<HistoryStore> {
        &self.history_store
    }

    /// 获取 CRDT 引擎引用
    pub fn crdt_engine(&self) -> &Arc<RwLock<CrdtEngine>> {
        &self.crdt_engine
    }

    /// 初始化 CRDT 文档（如果不存在）
    pub async fn ensure_crdt_document(&self, entity_id: &str, initial_content: &str) {
        let engine = self.crdt_engine.read().await;
        if !engine.has_document(entity_id) {
            drop(engine);
            let mut engine = self.crdt_engine.write().await;
            engine.create_document(entity_id, initial_content);
        }
    }

    /// 应用 CRDT 操作到实体
    pub async fn apply_crdt_operation(
        &self,
        entity_id: &str,
        op_type: OperationType,
        position: usize,
    ) -> Result<(), String> {
        let mut engine = self.crdt_engine.write().await;
        engine.apply_local_operation(entity_id, &self.local_device_id, op_type, position)?;
        Ok(())
    }

    /// 合并远程 CRDT 操作
    pub async fn merge_remote_crdt_operations(
        &self,
        entity_id: &str,
        operations: Vec<crate::crdt::CrdtOperation>,
    ) -> Result<Vec<crate::crdt::CrdtOperation>, String> {
        let mut engine = self.crdt_engine.write().await;
        engine.merge_operations(entity_id, operations)
    }

    /// 获取 CRDT 文档版本
    pub async fn get_crdt_version(&self, entity_id: &str) -> Result<u64, String> {
        let engine = self.crdt_engine.read().await;
        engine.get_document_version(entity_id)
    }

    /// 记录本地变更
    pub async fn record_change(
        &self,
        entity_type: EntityType,
        entity_id: &str,
        operation: ChangeOperation,
        data: Option<String>,
    ) -> ChangeLogEntry {
        // 获取本地版本向量并递增
        let mut vv = self.get_local_version_vector().await;
        vv.increment(&self.local_device_id);

        let entry = ChangeLogEntry {
            id: Uuid::new_v4().to_string(),
            entity_type,
            entity_id: entity_id.to_string(),
            operation,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            device_id: self.local_device_id.clone(),
            version_vector: vv.to_entries(),
            data,
        };

        self.change_log.add_entry(entry.clone()).await;
        entry
    }

    /// 获取本地版本向量
    async fn get_local_version_vector(&self) -> VersionVector {
        let entries = self.change_log.get_all_entries().await;
        let mut vv = VersionVector::new();

        for entry in &entries {
            for vv_entry in &entry.version_vector {
                let current = vv.get(&vv_entry.device_id);
                if vv_entry.counter > current {
                    // 更新最大值
                    vv.increment(&vv_entry.device_id);
                    // 可能需要多次递增
                    while vv.get(&vv_entry.device_id) < vv_entry.counter {
                        vv.increment(&vv_entry.device_id);
                    }
                }
            }
        }

        vv
    }

    /// 合并远程变更到本地版本向量
    async fn merge_remote_version_vector(&self, remote_entries: &[VersionVectorEntry]) {
        let mut local_vv = self.get_local_version_vector().await;
        let remote_vv = VersionVector::from_entries(remote_entries);
        local_vv.merge(&remote_vv);

        // 存储合成的版本向量
        let merged_entries = local_vv.to_entries();
        let now = chrono::Utc::now().timestamp_millis() as u64;

        let merge_entry = ChangeLogEntry {
            id: Uuid::new_v4().to_string(),
            entity_type: EntityType::Conversation, // 系统事件
            entity_id: "__version_vector_sync__".to_string(),
            operation: ChangeOperation::Update,
            timestamp: now,
            device_id: self.local_device_id.clone(),
            version_vector: merged_entries,
            data: None,
        };

        self.change_log.add_entry(merge_entry).await;
    }
}

#[async_trait]
impl SyncEngine for SyncEngineImpl {
    async fn full_sync(&self, _device_id: &str) -> Result<SyncResult, String> {
        let start = std::time::Instant::now();

        // 1. 获取本地所有变更
        let local_entries = self.change_log.get_all_entries().await;

        // 2. 检测冲突
        let conflicts = ConflictResolver::detect_conflicts(&local_entries);

        // 3. 自动解决可解决的冲突
        let mut conflicts_detected = conflicts.len() as u64;
        if !conflicts.is_empty() {
            let auto_result = ConflictResolver::auto_resolve(&local_entries);
            conflicts_detected = auto_result.total_conflicts as u64;
        }

        // 4. 更新最后同步时间
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        self.change_log
            .set_last_sync(&self.local_device_id, now_ms)
            .await;

        Ok(SyncResult {
            success: true,
            files_synced: local_entries.len() as u64,
            files_uploaded: 0,
            files_downloaded: 0,
            conflicts_detected,
            error_message: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn incremental_sync(&self, device_id: &str) -> Result<SyncResult, String> {
        let start = std::time::Instant::now();

        // 1. 获取自上次同步以来的变更
        let last_sync = self.change_log.get_last_sync(device_id).await;
        let changes = self.change_log.get_entries_since(last_sync).await;

        // 2. 检测冲突
        let conflicts = ConflictResolver::detect_conflicts(&changes);
        let conflicts_detected = conflicts.len() as u64;

        // 3. 更新最后同步时间
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        self.change_log.set_last_sync(device_id, now_ms).await;

        Ok(SyncResult {
            success: true,
            files_synced: changes.len() as u64,
            files_uploaded: 0,
            files_downloaded: 0,
            conflicts_detected,
            error_message: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn push_changes(
        &self,
        changes: Vec<ChangeLogEntry>,
    ) -> Result<Vec<ConflictInfo>, String> {
        // 1. 检查与本地变更的冲突
        let local_entries = self.change_log.get_all_entries().await;
        let mut all_entries = local_entries;
        all_entries.extend(changes.iter().cloned());

        let conflicts = ConflictResolver::detect_conflicts(&all_entries);

        // 2. 如果没有冲突，合并变更日志
        if conflicts.is_empty() {
            for change in &changes {
                self.change_log.add_entry(change.clone()).await;
            }

            // 3. 更新版本向量
            if let Some(first) = changes.first() {
                self.merge_remote_version_vector(&first.version_vector).await;
            }
        }

        Ok(conflicts)
    }

    async fn pull_changes(
        &self,
        since_timestamp: u64,
    ) -> Result<Vec<ChangeLogEntry>, String> {
        let changes = self.change_log.get_entries_since(since_timestamp).await;

        // 过滤掉系统事件
        let filtered: Vec<_> = changes
            .into_iter()
            .filter(|e| e.entity_id != "__version_vector_sync__")
            .collect();

        Ok(filtered)
    }

    async fn resolve_conflict(
        &self,
        conflict: &ConflictInfo,
        strategy: ConflictResolutionStrategy,
    ) -> Result<(), String> {
        let result = ConflictResolver::resolve(conflict, strategy);

        // 如果成功解决，记录解决事件
        if result.resolved_data.is_some() {
            let now = chrono::Utc::now().timestamp_millis() as u64;
            let resolution_entry = ChangeLogEntry {
                id: Uuid::new_v4().to_string(),
                entity_type: conflict.entity_type,
                entity_id: conflict.entity_id.clone(),
                operation: ChangeOperation::Update,
                timestamp: now,
                device_id: self.local_device_id.clone(),
                version_vector: vec![],
                data: Some(format!(
                    r#"{{"resolution":"{}","conflict_id":"{}"}}"#,
                    result.resolution_applied, result.conflict_id
                )),
            };

            self.change_log.add_entry(resolution_entry).await;
        }

        Ok(())
    }

    async fn get_sync_status(&self, device_id: &str) -> Result<DeviceSyncStatus, String> {
        let last_sync = self.change_log.get_last_sync(device_id).await;
        let pending = self.change_log.get_entries_since(last_sync).await;

        Ok(DeviceSyncStatus {
            local_device_id: device_id.to_string(),
            last_sync_at: if last_sync > 0 { Some(last_sync) } else { None },
            last_sync_result: None,
            pending_changes: pending.len() as u64,
            sync_progress: 100,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_engine() -> SyncEngineImpl {
        let change_log = Arc::new(ChangeLogStore::new());
        let device_store = Arc::new(DeviceStore::new());
        let history_store = Arc::new(HistoryStore::new());
        SyncEngineImpl::new(change_log, device_store, history_store, "test-device".to_string())
    }

    #[tokio::test]
    async fn test_record_change() {
        let engine = create_test_engine().await;

        let entry = engine
            .record_change(
                EntityType::Conversation,
                "conv-1",
                ChangeOperation::Create,
                Some(r#"{"title":"test"}"#.to_string()),
            )
            .await;

        assert_eq!(entry.device_id, "test-device");
        assert_eq!(entry.entity_id, "conv-1");
        assert!(!entry.version_vector.is_empty());
    }

    #[tokio::test]
    async fn test_incremental_sync() {
        let engine = create_test_engine().await;

        // 记录一些变更
        engine
            .record_change(
                EntityType::Conversation,
                "conv-1",
                ChangeOperation::Create,
                None,
            )
            .await;

        let result = engine.incremental_sync("test-device").await.unwrap();
        assert!(result.success);
        assert_eq!(result.files_synced, 1);
    }

    #[tokio::test]
    async fn test_push_changes_no_conflict() {
        let engine = create_test_engine().await;

        let remote_changes = vec![ChangeLogEntry {
            id: Uuid::new_v4().to_string(),
            entity_type: EntityType::Conversation,
            entity_id: "conv-remote".to_string(),
            operation: ChangeOperation::Create,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            device_id: "remote-device".to_string(),
            version_vector: vec![VersionVectorEntry {
                device_id: "remote-device".to_string(),
                counter: 1,
            }],
            data: None,
        }];

        let conflicts = engine.push_changes(remote_changes).await.unwrap();
        assert!(conflicts.is_empty()); // 不同实体，无冲突
    }

    #[tokio::test]
    async fn test_get_sync_status() {
        let engine = create_test_engine().await;

        let status = engine.get_sync_status("test-device").await.unwrap();
        assert_eq!(status.local_device_id, "test-device");
    }
}
