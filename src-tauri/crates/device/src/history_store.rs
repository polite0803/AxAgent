// SPDX-License-Identifier: AGPL-3.0-only

//! 同步历史记录与审计日志存储
//!
//! 记录同步操作历史和系统审计日志。
//! 支持内存存储（默认）和数据库持久化两种模式。

use std::sync::Arc;

use axagent_harness::device_sync::{AuditAction, AuditLogEntry, SyncHistoryEntry, SyncStorage};
use axagent_harness::util_fns::current_rfc3339;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 历史记录与审计存储
pub struct HistoryStore {
    /// 数据库存储实现（可选）
    sync_storage: Option<Arc<dyn SyncStorage>>,
    /// 内存历史记录存储（作为回退或缓存）
    history: RwLock<Vec<SyncHistoryEntry>>,
    /// 内存审计日志存储（作为回退或缓存）
    audit_logs: RwLock<Vec<AuditLogEntry>>,
    /// 最大历史记录数量
    max_history_entries: usize,
    /// 最大审计日志数量
    max_audit_entries: usize,
}

impl HistoryStore {
    /// 创建内存模式的历史存储
    pub fn new() -> Self {
        Self {
            sync_storage: None,
            history: RwLock::new(Vec::new()),
            audit_logs: RwLock::new(Vec::new()),
            max_history_entries: 1000,
            max_audit_entries: 5000,
        }
    }

    /// 创建数据库模式的历史存储
    pub fn with_storage(sync_storage: Arc<dyn SyncStorage>) -> Self {
        Self {
            sync_storage: Some(sync_storage),
            history: RwLock::new(Vec::new()),
            audit_logs: RwLock::new(Vec::new()),
            max_history_entries: 1000,
            max_audit_entries: 5000,
        }
    }

    /// 检查是否使用数据库存储
    pub fn uses_database(&self) -> bool {
        self.sync_storage.is_some()
    }

    /// 添加同步历史记录
    pub async fn add_history_entry(&self, mut entry: SyncHistoryEntry) {
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }

        // 写入内存缓存
        {
            let mut history = self.history.write().await;
            history.push(entry.clone());

            // 裁剪超出限制的旧记录
            while history.len() > self.max_history_entries {
                history.remove(0);
            }
        }

        // 如果使用数据库，同时持久化
        if let Some(storage) = &self.sync_storage {
            let _ = storage.add_history_entry(&entry).await;
        }
    }

    /// 获取同步历史记录
    pub async fn get_history(&self, limit: usize) -> Vec<SyncHistoryEntry> {
        if let Some(storage) = &self.sync_storage {
            let lim = if limit == 0 { None } else { Some(limit as u64) };
            if let Ok(entries) = storage.get_history_by_device("__all__", lim).await
                && !entries.is_empty()
            {
                return entries;
            }
        }

        let history = self.history.read().await;
        let mut list: Vec<_> = history.iter().rev().take(limit).cloned().collect();
        list.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        list
    }

    /// 按设备获取同步历史
    pub async fn get_history_by_device(
        &self,
        device_id: &str,
        limit: usize,
    ) -> Vec<SyncHistoryEntry> {
        if let Some(storage) = &self.sync_storage {
            let lim = if limit == 0 { None } else { Some(limit as u64) };
            if let Ok(entries) = storage.get_history_by_device(device_id, lim).await {
                return entries;
            }
        }

        let history = self.history.read().await;
        let mut list: Vec<_> = history
            .iter()
            .filter(|h| h.device_id == device_id)
            .rev()
            .take(limit)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        list
    }

    /// 添加审计日志
    pub async fn add_audit_log(&self, mut entry: AuditLogEntry) {
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }

        // 写入内存缓存
        {
            let mut logs = self.audit_logs.write().await;
            logs.push(entry.clone());

            // 裁剪超出限制的旧记录
            while logs.len() > self.max_audit_entries {
                logs.remove(0);
            }
        }

        // 如果使用数据库，同时持久化
        if let Some(storage) = &self.sync_storage {
            let _ = storage.add_audit_log(&entry).await;
        }
    }

    /// 记录审计操作（便捷方法）
    #[allow(clippy::too_many_arguments)]
    pub async fn log_audit(
        &self,
        action: AuditAction,
        entity_type: &str,
        entity_id: &str,
        device_id: &str,
        success: bool,
        details: Option<String>,
        error_message: Option<String>,
    ) {
        let entry = AuditLogEntry {
            id: Uuid::new_v4().to_string(),
            action,
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            device_id: device_id.to_string(),
            details,
            success,
            error_message,
            timestamp: current_rfc3339(),
        };
        self.add_audit_log(entry).await;
    }

    /// 获取审计日志
    pub async fn get_audit_logs(&self, limit: usize) -> Vec<AuditLogEntry> {
        let logs = self.audit_logs.read().await;
        let mut list: Vec<_> = logs.iter().rev().take(limit).cloned().collect();
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        list
    }

    /// 按操作类型获取审计日志
    pub async fn get_audit_logs_by_action(
        &self,
        action: AuditAction,
        limit: usize,
    ) -> Vec<AuditLogEntry> {
        if let Some(storage) = &self.sync_storage {
            let lim = if limit == 0 { None } else { Some(limit as u64) };
            if let Ok(entries) = storage.get_audit_logs_by_action(&action.to_string(), lim).await {
                return entries;
            }
        }

        let logs = self.audit_logs.read().await;
        let mut list: Vec<_> =
            logs.iter().filter(|l| l.action == action).rev().take(limit).cloned().collect();
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        list
    }

    /// 按设备获取审计日志
    pub async fn get_audit_logs_by_device(
        &self,
        device_id: &str,
        limit: usize,
    ) -> Vec<AuditLogEntry> {
        if let Some(storage) = &self.sync_storage {
            let lim = if limit == 0 { None } else { Some(limit as u64) };
            if let Ok(entries) = storage.get_audit_logs_by_device(device_id, lim).await {
                return entries;
            }
        }

        let logs = self.audit_logs.read().await;
        let mut list: Vec<_> =
            logs.iter().filter(|l| l.device_id == device_id).rev().take(limit).cloned().collect();
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        list
    }

    /// 清理过期记录
    pub async fn cleanup_old_entries(&self, days: u32) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let mut history = self.history.write().await;
        let before = history.len();
        history.retain(|h| h.completed_at >= cutoff_str);
        let history_removed = before - history.len();

        let mut logs = self.audit_logs.write().await;
        let before_logs = logs.len();
        logs.retain(|l| l.timestamp >= cutoff_str);
        let logs_removed = before_logs - logs.len();

        history_removed + logs_removed
    }
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::device_sync::{SyncDirection, SyncResult, SyncType};

    fn create_test_history_entry() -> SyncHistoryEntry {
        SyncHistoryEntry {
            id: Uuid::new_v4().to_string(),
            device_id: "test-device".to_string(),
            direction: SyncDirection::Both,
            sync_type: SyncType::Incremental,
            result: SyncResult {
                success: true,
                files_synced: 10,
                files_uploaded: 5,
                files_downloaded: 5,
                conflicts_detected: 0,
                error_message: None,
                duration_ms: 100,
            },
            conflicts: vec![],
            started_at: current_rfc3339(),
            completed_at: current_rfc3339(),
            initiated_by: "user".to_string(),
        }
    }

    #[tokio::test]
    async fn test_add_and_get_history() {
        let store = HistoryStore::new();
        let entry = create_test_history_entry();
        store.add_history_entry(entry).await;

        let history = store.get_history(10).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].device_id, "test-device");
    }

    #[tokio::test]
    async fn test_add_and_get_audit_log() {
        let store = HistoryStore::new();
        store
            .log_audit(
                AuditAction::SyncCompleted,
                "sync",
                "sync-001",
                "test-device",
                true,
                Some(r#"{"count": 10}"#.to_string()),
                None,
            )
            .await;

        let logs = store.get_audit_logs(10).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::SyncCompleted);
    }

    #[tokio::test]
    async fn test_get_history_by_device() {
        let store = HistoryStore::new();
        store.add_history_entry(create_test_history_entry()).await;

        let history = store.get_history_by_device("test-device", 10).await;
        assert_eq!(history.len(), 1);

        let empty = store.get_history_by_device("other-device", 10).await;
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_database_persistence() {
        let storage = Arc::new(MemorySyncStorage::new());
        let store = HistoryStore::with_storage(storage);

        store.add_history_entry(create_test_history_entry()).await;

        let history = store.get_history_by_device("test-device", 10).await;
        assert_eq!(history.len(), 1);

        store
            .log_audit(
                AuditAction::SyncCompleted,
                "sync",
                "sync-001",
                "test-device",
                true,
                None,
                None,
            )
            .await;

        let logs = store.get_audit_logs(10).await;
        assert_eq!(logs.len(), 1);
    }
}
