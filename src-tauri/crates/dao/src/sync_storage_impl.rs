// SPDX-License-Identifier: AGPL-3.0-only

//! 同步存储的数据库实现
//!
//! 实现 harness 中定义的 SyncStorage trait，
//! 通过 Sea-ORM 抽象层操作数据库，自动兼容 PostgreSQL 和 SQLite。

use sea_orm::DatabaseConnection;

use crate::repo::{
    sync_audit_log, sync_change_log, sync_device, sync_history, sync_permission, sync_policy,
};
use axagent_harness::device_sync::{
    AuditLogEntry, ChangeLogEntry, DeviceInfo, DevicePermissions, SyncHistoryEntry, SyncPolicy,
    SyncStorage,
};

/// 基于数据库的同步存储实现
pub struct SyncStorageDb {
    db: DatabaseConnection,
}

impl SyncStorageDb {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl SyncStorage for SyncStorageDb {
    // ─── 设备存储 ──────────────────────────────────────────────────────

    async fn save_device(&self, device: &DeviceInfo) -> Result<(), String> {
        sync_device::save_device(&self.db, device).await.map_err(|e| e.to_string())
    }

    async fn get_all_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        sync_device::get_all_devices(&self.db).await.map_err(|e| e.to_string())
    }

    async fn get_paired_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        sync_device::get_paired_devices(&self.db).await.map_err(|e| e.to_string())
    }

    async fn get_device_by_id(&self, device_id: &str) -> Result<Option<DeviceInfo>, String> {
        sync_device::get_device_by_id(&self.db, device_id).await.map_err(|e| e.to_string())
    }

    async fn update_device(&self, device: &DeviceInfo) -> Result<(), String> {
        sync_device::update_device(&self.db, device).await.map_err(|e| e.to_string())
    }

    async fn delete_device(&self, device_id: &str) -> Result<(), String> {
        sync_device::delete_device(&self.db, device_id).await.map_err(|e| e.to_string())
    }

    // ─── 变更日志存储 ──────────────────────────────────────────────────

    async fn add_change_log(&self, entry: &ChangeLogEntry) -> Result<(), String> {
        sync_change_log::add_change_log(&self.db, entry).await.map_err(|e| e.to_string())
    }

    async fn batch_add_change_logs(&self, entries: &[ChangeLogEntry]) -> Result<(), String> {
        sync_change_log::batch_add_change_logs(&self.db, entries).await.map_err(|e| e.to_string())
    }

    async fn get_change_logs_by_device(
        &self,
        device_id: &str,
        _since_timestamp: Option<u64>,
    ) -> Result<Vec<ChangeLogEntry>, String> {
        // 将 since_timestamp 转换为版本号（简化处理）
        sync_change_log::get_change_logs_by_device(&self.db, device_id, None)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_unsynced_change_logs(
        &self,
        device_id: &str,
    ) -> Result<Vec<ChangeLogEntry>, String> {
        sync_change_log::get_unsynced_change_logs(&self.db, device_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn mark_changes_as_synced(
        &self,
        change_ids: &[String],
        target_device_id: &str,
    ) -> Result<(), String> {
        sync_change_log::mark_changes_as_synced(&self.db, change_ids, target_device_id)
            .await
            .map_err(|e| e.to_string())
    }

    // ─── 策略存储 ──────────────────────────────────────────────────────

    async fn save_policy(&self, policy: &SyncPolicy) -> Result<(), String> {
        sync_policy::save_policy(&self.db, policy).await.map_err(|e| e.to_string())
    }

    async fn get_all_policies(&self) -> Result<Vec<SyncPolicy>, String> {
        sync_policy::get_all_policies(&self.db).await.map_err(|e| e.to_string())
    }

    async fn get_enabled_policies(&self) -> Result<Vec<SyncPolicy>, String> {
        sync_policy::get_enabled_policies(&self.db).await.map_err(|e| e.to_string())
    }

    async fn get_policy_by_id(&self, policy_id: &str) -> Result<Option<SyncPolicy>, String> {
        sync_policy::get_policy_by_id(&self.db, policy_id).await.map_err(|e| e.to_string())
    }

    async fn delete_policy(&self, policy_id: &str) -> Result<(), String> {
        sync_policy::delete_policy(&self.db, policy_id).await.map_err(|e| e.to_string())
    }

    // ─── 历史记录存储 ──────────────────────────────────────────────────

    async fn add_history_entry(&self, entry: &SyncHistoryEntry) -> Result<(), String> {
        sync_history::add_history_entry(&self.db, entry).await.map_err(|e| e.to_string())
    }

    async fn get_history_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<SyncHistoryEntry>, String> {
        sync_history::get_history_by_device(&self.db, device_id, limit)
            .await
            .map_err(|e| e.to_string())
    }

    // ─── 权限存储 ──────────────────────────────────────────────────────

    async fn save_permissions(&self, permissions: &DevicePermissions) -> Result<(), String> {
        sync_permission::save_permissions(&self.db, permissions).await.map_err(|e| e.to_string())
    }

    async fn get_permissions_by_device(
        &self,
        device_id: &str,
    ) -> Result<Option<DevicePermissions>, String> {
        sync_permission::get_permissions_by_device(&self.db, device_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_all_permissions(&self) -> Result<Vec<DevicePermissions>, String> {
        sync_permission::get_all_permissions(&self.db).await.map_err(|e| e.to_string())
    }

    async fn delete_permissions(&self, device_id: &str) -> Result<(), String> {
        sync_permission::delete_permissions(&self.db, device_id).await.map_err(|e| e.to_string())
    }

    // ─── 审计日志存储 ──────────────────────────────────────────────────

    async fn add_audit_log(&self, entry: &AuditLogEntry) -> Result<(), String> {
        sync_audit_log::add_audit_log(&self.db, entry).await.map_err(|e| e.to_string())
    }

    async fn batch_add_audit_logs(&self, entries: &[AuditLogEntry]) -> Result<(), String> {
        sync_audit_log::batch_add_audit_logs(&self.db, entries).await.map_err(|e| e.to_string())
    }

    async fn get_audit_logs_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        sync_audit_log::get_audit_logs_by_device(&self.db, device_id, limit)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_audit_logs_by_action(
        &self,
        action: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        sync_audit_log::get_audit_logs_by_action(&self.db, action, limit)
            .await
            .map_err(|e| e.to_string())
    }
}
