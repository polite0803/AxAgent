// SPDX-License-Identifier: AGPL-3.0-only

//! 内存存储实现
//!
//! 实现 SyncStorage trait，用于测试和无数据库场景。
//! 生产环境应使用 SyncStorageDb（Sea-ORM 实现）。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::device_sync::{
    AuditLogEntry, ChangeLogEntry, DeviceInfo, DevicePermissions, SyncHistoryEntry, SyncPolicy,
    SyncStorage,
};
use tokio::sync::RwLock;

/// 内存存储实现
#[derive(Default)]
pub struct MemorySyncStorage {
    devices: RwLock<HashMap<String, DeviceInfo>>,
    change_logs: RwLock<Vec<ChangeLogEntry>>,
    policies: RwLock<HashMap<String, SyncPolicy>>,
    histories: RwLock<Vec<SyncHistoryEntry>>,
    permissions: RwLock<HashMap<String, DevicePermissions>>,
    audit_logs: RwLock<Vec<AuditLogEntry>>,
}

impl MemorySyncStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl SyncStorage for MemorySyncStorage {
    // ─── 设备存储 ──────────────────────────────────────────────────────

    async fn save_device(&self, device: &DeviceInfo) -> Result<(), String> {
        self.devices.write().await.insert(device.device_id.clone(), device.clone());
        Ok(())
    }

    async fn get_all_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let devices = self.devices.read().await;
        Ok(devices.values().cloned().collect())
    }

    async fn get_paired_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let devices = self.devices.read().await;
        Ok(devices.values().filter(|d| d.is_paired).cloned().collect())
    }

    async fn get_device_by_id(&self, device_id: &str) -> Result<Option<DeviceInfo>, String> {
        let devices = self.devices.read().await;
        Ok(devices.get(device_id).cloned())
    }

    async fn update_device(&self, device: &DeviceInfo) -> Result<(), String> {
        self.devices.write().await.insert(device.device_id.clone(), device.clone());
        Ok(())
    }

    async fn delete_device(&self, device_id: &str) -> Result<(), String> {
        self.devices.write().await.remove(device_id);
        Ok(())
    }

    // ─── 变更日志存储 ──────────────────────────────────────────────────

    async fn add_change_log(&self, entry: &ChangeLogEntry) -> Result<(), String> {
        self.change_logs.write().await.push(entry.clone());
        Ok(())
    }

    async fn batch_add_change_logs(&self, entries: &[ChangeLogEntry]) -> Result<(), String> {
        self.change_logs.write().await.extend(entries.iter().cloned());
        Ok(())
    }

    async fn get_change_logs_by_device(
        &self,
        device_id: &str,
        since_timestamp: Option<u64>,
    ) -> Result<Vec<ChangeLogEntry>, String> {
        let logs = self.change_logs.read().await;
        let filtered: Vec<_> = logs
            .iter()
            .filter(|e| e.device_id == device_id)
            .filter(|e| {
                if let Some(ts) = since_timestamp {
                    e.timestamp >= ts
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn get_unsynced_change_logs(
        &self,
        device_id: &str,
    ) -> Result<Vec<ChangeLogEntry>, String> {
        let logs = self.change_logs.read().await;
        let filtered: Vec<_> =
            logs.iter().filter(|e| e.device_id == device_id && !e.is_synced).cloned().collect();
        Ok(filtered)
    }

    async fn mark_changes_as_synced(
        &self,
        change_ids: &[String],
        target_device_id: &str,
    ) -> Result<(), String> {
        let mut logs = self.change_logs.write().await;
        for log in logs.iter_mut() {
            if change_ids.contains(&log.id) {
                if !log.synced_to.contains(&target_device_id.to_string()) {
                    log.synced_to.push(target_device_id.to_string());
                }
                log.is_synced = !log.synced_to.is_empty();
            }
        }
        Ok(())
    }

    // ─── 策略存储 ──────────────────────────────────────────────────────

    async fn save_policy(&self, policy: &SyncPolicy) -> Result<(), String> {
        self.policies.write().await.insert(policy.id.clone(), policy.clone());
        Ok(())
    }

    async fn get_all_policies(&self) -> Result<Vec<SyncPolicy>, String> {
        let policies = self.policies.read().await;
        Ok(policies.values().cloned().collect())
    }

    async fn get_enabled_policies(&self) -> Result<Vec<SyncPolicy>, String> {
        let policies = self.policies.read().await;
        Ok(policies.values().filter(|p| p.enabled).cloned().collect())
    }

    async fn get_policy_by_id(&self, policy_id: &str) -> Result<Option<SyncPolicy>, String> {
        let policies = self.policies.read().await;
        Ok(policies.get(policy_id).cloned())
    }

    async fn delete_policy(&self, policy_id: &str) -> Result<(), String> {
        self.policies.write().await.remove(policy_id);
        Ok(())
    }

    // ─── 历史记录存储 ──────────────────────────────────────────────────

    async fn add_history_entry(&self, entry: &SyncHistoryEntry) -> Result<(), String> {
        self.histories.write().await.push(entry.clone());
        Ok(())
    }

    async fn get_history_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<SyncHistoryEntry>, String> {
        let histories = self.histories.read().await;
        let filtered: Vec<_> =
            histories.iter().filter(|h| h.device_id == device_id).cloned().collect();
        if let Some(lim) = limit {
            Ok(filtered.into_iter().rev().take(lim as usize).collect())
        } else {
            Ok(filtered)
        }
    }

    // ─── 权限存储 ──────────────────────────────────────────────────────

    async fn save_permissions(&self, permissions: &DevicePermissions) -> Result<(), String> {
        self.permissions.write().await.insert(permissions.device_id.clone(), permissions.clone());
        Ok(())
    }

    async fn get_permissions_by_device(
        &self,
        device_id: &str,
    ) -> Result<Option<DevicePermissions>, String> {
        let permissions = self.permissions.read().await;
        Ok(permissions.get(device_id).cloned())
    }

    async fn get_all_permissions(&self) -> Result<Vec<DevicePermissions>, String> {
        let permissions = self.permissions.read().await;
        Ok(permissions.values().cloned().collect())
    }

    async fn delete_permissions(&self, device_id: &str) -> Result<(), String> {
        self.permissions.write().await.remove(device_id);
        Ok(())
    }

    // ─── 审计日志存储 ──────────────────────────────────────────────────

    async fn add_audit_log(&self, entry: &AuditLogEntry) -> Result<(), String> {
        self.audit_logs.write().await.push(entry.clone());
        Ok(())
    }

    async fn batch_add_audit_logs(&self, entries: &[AuditLogEntry]) -> Result<(), String> {
        self.audit_logs.write().await.extend(entries.iter().cloned());
        Ok(())
    }

    async fn get_audit_logs_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        let logs = self.audit_logs.read().await;
        let filtered: Vec<_> = logs.iter().filter(|l| l.device_id == device_id).cloned().collect();
        if let Some(lim) = limit {
            Ok(filtered.into_iter().rev().take(lim as usize).collect())
        } else {
            Ok(filtered)
        }
    }

    async fn get_audit_logs_by_action(
        &self,
        action: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        let logs = self.audit_logs.read().await;
        let filtered: Vec<_> =
            logs.iter().filter(|l| l.action.to_string() == action).cloned().collect();
        if let Some(lim) = limit {
            Ok(filtered.into_iter().rev().take(lim as usize).collect())
        } else {
            Ok(filtered)
        }
    }
}

/// 共享的内存存储实例（用于无数据库场景）
pub fn create_memory_storage() -> Arc<dyn SyncStorage> {
    Arc::new(MemorySyncStorage::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_device_storage() {
        let storage = MemorySyncStorage::new();
        let device = DeviceInfo {
            device_id: "test-001".to_string(),
            name: "Test Device".to_string(),
            hostname: "test-host".to_string(),
            os: "Windows".to_string(),
            device_type: axagent_harness::device_sync::DeviceType::Desktop,
            app_version: "1.0.0".to_string(),
            registered_at: String::new(),
            last_active_at: String::new(),
            is_paired: false,
            trust_level: axagent_harness::device_sync::TrustLevel::Standard,
        };

        storage.save_device(&device).await.expect("测试：异步操作应成功");
        let fetched = storage.get_device_by_id("test-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_some());
        assert_eq!(fetched.expect("测试应成功").name, "Test Device");

        let all = storage.get_all_devices().await.expect("测试：异步操作应成功");
        assert_eq!(all.len(), 1);

        storage.delete_device("test-001").await.expect("测试：异步操作应成功");
        let fetched = storage.get_device_by_id("test-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_memory_policy_storage() {
        let storage = MemorySyncStorage::new();
        let policy = SyncPolicy {
            id: "pol-001".to_string(),
            name: "Test Policy".to_string(),
            conflict_strategy:
                axagent_harness::device_sync::ConflictResolutionStrategy::LastWriteWins,
            auto_sync_interval_secs: 300,
            sync_scope: vec![],
            auto_resolve_conflicts: true,
            max_conflict_threshold: 100,
            change_log_retention_enabled: true,
            change_log_retention_days: 30,
            enabled: true,
            updated_at: String::new(),
        };

        storage.save_policy(&policy).await.expect("测试：异步操作应成功");
        let fetched = storage.get_policy_by_id("pol-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_some());
        assert_eq!(fetched.expect("测试应成功").name, "Test Policy");

        let all = storage.get_all_policies().await.expect("测试：异步操作应成功");
        assert_eq!(all.len(), 1);

        storage.delete_policy("pol-001").await.expect("测试：异步操作应成功");
        let fetched = storage.get_policy_by_id("pol-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_memory_permission_storage() {
        let storage = MemorySyncStorage::new();
        let perms = DevicePermissions {
            device_id: "device-001".to_string(),
            trust_level: axagent_harness::device_sync::TrustLevel::Standard,
            allow_push: true,
            allow_pull: true,
            allow_full_sync: false,
            allow_resolve_conflicts: true,
            allow_manage_devices: false,
            allow_modify_policy: false,
            updated_at: String::new(),
        };

        storage.save_permissions(&perms).await.expect("测试：异步操作应成功");
        let fetched =
            storage.get_permissions_by_device("device-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_some());
        assert!(fetched.expect("测试应成功").allow_push);

        let all = storage.get_all_permissions().await.expect("测试：异步操作应成功");
        assert_eq!(all.len(), 1);

        storage.delete_permissions("device-001").await.expect("测试：异步操作应成功");
        let fetched =
            storage.get_permissions_by_device("device-001").await.expect("测试：异步操作应成功");
        assert!(fetched.is_none());
    }
}
