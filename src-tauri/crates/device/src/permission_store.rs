// SPDX-License-Identifier: AGPL-3.0-only

//! 设备权限存储
//!
//! 管理各设备的操作权限配置。
//! 支持内存存储（默认）和数据库持久化两种模式。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::device_sync::{DevicePermissions, PermissionUpdate, SyncStorage, TrustLevel};
use axagent_harness::util_fns::current_rfc3339;
use tokio::sync::RwLock;

/// 设备权限存储
pub struct PermissionStore {
    /// 数据库存储实现（可选）
    sync_storage: Option<Arc<dyn SyncStorage>>,
    /// 内存权限存储（作为回退或缓存）
    permissions: RwLock<HashMap<String, DevicePermissions>>,
}

impl PermissionStore {
    /// 创建内存模式的权限存储（用于测试或无数据库场景）
    pub fn new() -> Self {
        Self { sync_storage: None, permissions: RwLock::new(HashMap::new()) }
    }

    /// 创建数据库模式的权限存储
    pub fn with_storage(sync_storage: Arc<dyn SyncStorage>) -> Self {
        Self { sync_storage: Some(sync_storage), permissions: RwLock::new(HashMap::new()) }
    }

    /// 从数据库加载所有权限到内存缓存
    pub async fn load_from_db(&self) -> Result<(), String> {
        if let Some(storage) = &self.sync_storage {
            let perms = storage.get_all_permissions().await?;
            let mut cache = self.permissions.write().await;
            for perm in perms {
                cache.insert(perm.device_id.clone(), perm);
            }
        }
        Ok(())
    }

    /// 检查是否使用数据库存储
    pub fn uses_database(&self) -> bool {
        self.sync_storage.is_some()
    }

    /// 设置设备权限
    pub async fn set_permissions(&self, permissions: DevicePermissions) {
        // 先写入内存缓存
        self.permissions.write().await.insert(permissions.device_id.clone(), permissions.clone());

        // 如果使用数据库，同时持久化
        if let Some(storage) = &self.sync_storage {
            let _ = storage.save_permissions(&permissions).await;
        }
    }

    /// 获取设备权限
    pub async fn get_permissions(&self, device_id: &str) -> Option<DevicePermissions> {
        // 先检查内存缓存
        if let Some(perm) = self.permissions.read().await.get(device_id).cloned() {
            return Some(perm);
        }

        // 如果使用数据库，尝试从数据库获取
        if let Some(storage) = &self.sync_storage
            && let Ok(Some(perm)) = storage.get_permissions_by_device(device_id).await
        {
            // 回填到缓存
            self.permissions.write().await.insert(device_id.to_string(), perm.clone());
            return Some(perm);
        }

        None
    }

    /// 获取所有设备权限
    pub async fn get_all_permissions(&self) -> Vec<DevicePermissions> {
        if let Some(storage) = &self.sync_storage {
            // 使用数据库存储时，从数据库获取最新数据
            if let Ok(perms) = storage.get_all_permissions().await {
                return perms;
            }
        }

        let permissions = self.permissions.read().await;
        let mut list: Vec<_> = permissions.values().cloned().collect();
        list.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        list
    }

    /// 更新设备权限
    pub async fn update_permissions(
        &self,
        device_id: &str,
        update: PermissionUpdate,
    ) -> Result<DevicePermissions, String> {
        let mut permissions = self.permissions.write().await;
        let perm = permissions.get_mut(device_id).ok_or_else(|| "设备权限不存在".to_string())?;

        if let Some(trust_level) = update.trust_level {
            perm.trust_level = trust_level;
        }
        if let Some(allow_push) = update.allow_push {
            perm.allow_push = allow_push;
        }
        if let Some(allow_pull) = update.allow_pull {
            perm.allow_pull = allow_pull;
        }
        if let Some(allow_full_sync) = update.allow_full_sync {
            perm.allow_full_sync = allow_full_sync;
        }
        if let Some(allow_resolve_conflicts) = update.allow_resolve_conflicts {
            perm.allow_resolve_conflicts = allow_resolve_conflicts;
        }
        if let Some(allow_manage_devices) = update.allow_manage_devices {
            perm.allow_manage_devices = allow_manage_devices;
        }
        if let Some(allow_modify_policy) = update.allow_modify_policy {
            perm.allow_modify_policy = allow_modify_policy;
        }

        perm.updated_at = current_rfc3339();
        let updated = perm.clone();

        // 如果使用数据库，同时更新
        if let Some(storage) = &self.sync_storage {
            let _ = storage.save_permissions(&updated).await;
        }

        Ok(updated)
    }

    /// 初始化设备权限（使用默认值）
    pub async fn init_device_permissions(
        &self,
        device_id: &str,
        trust_level: TrustLevel,
    ) -> DevicePermissions {
        let now = current_rfc3339();
        let permissions = DevicePermissions {
            device_id: device_id.to_string(),
            trust_level,
            allow_push: true,
            allow_pull: true,
            allow_full_sync: matches!(trust_level, TrustLevel::Full),
            allow_resolve_conflicts: true,
            allow_manage_devices: matches!(trust_level, TrustLevel::Full),
            allow_modify_policy: matches!(trust_level, TrustLevel::Full),
            updated_at: now,
        };
        self.set_permissions(permissions.clone()).await;
        permissions
    }

    /// 检查操作是否被允许
    pub async fn can_push(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_push).unwrap_or(false)
    }

    pub async fn can_pull(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_pull).unwrap_or(false)
    }

    pub async fn can_full_sync(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_full_sync).unwrap_or(false)
    }

    pub async fn can_resolve_conflicts(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_resolve_conflicts).unwrap_or(false)
    }

    pub async fn can_manage_devices(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_manage_devices).unwrap_or(false)
    }

    pub async fn can_modify_policy(&self, device_id: &str) -> bool {
        self.get_permissions(device_id).await.map(|p| p.allow_modify_policy).unwrap_or(false)
    }

    /// 移除设备权限
    pub async fn remove_permissions(&self, device_id: &str) {
        self.permissions.write().await.remove(device_id);

        if let Some(storage) = &self.sync_storage {
            let _ = storage.delete_permissions(device_id).await;
        }
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemorySyncStorage;

    #[tokio::test]
    async fn test_init_and_get_permissions() {
        let store = PermissionStore::new();
        let perms = store.init_device_permissions("device-1", TrustLevel::Standard).await;

        assert_eq!(perms.device_id, "device-1");
        assert_eq!(perms.trust_level, TrustLevel::Standard);
        assert!(perms.allow_push);
        assert!(!perms.allow_full_sync);
    }

    #[tokio::test]
    async fn test_update_permissions() {
        let store = PermissionStore::new();
        store.init_device_permissions("device-1", TrustLevel::Standard).await;

        let updated = store
            .update_permissions(
                "device-1",
                PermissionUpdate { allow_full_sync: Some(true), ..Default::default() },
            )
            .await
            .unwrap();

        assert!(updated.allow_full_sync);
    }

    #[tokio::test]
    async fn test_check_permissions() {
        let store = PermissionStore::new();
        store.init_device_permissions("device-1", TrustLevel::Full).await;

        assert!(store.can_push("device-1").await);
        assert!(store.can_full_sync("device-1").await);
        assert!(store.can_manage_devices("device-1").await);
        assert!(!store.can_push("unknown-device").await);
    }

    #[tokio::test]
    async fn test_database_persistence() {
        let storage = Arc::new(MemorySyncStorage::new());
        let store = PermissionStore::with_storage(storage);

        store.init_device_permissions("device-1", TrustLevel::Full).await;

        // 验证数据库存储
        let perms = store.get_permissions("device-1").await.unwrap();
        assert!(perms.allow_full_sync);

        // 验证列表
        let all = store.get_all_permissions().await;
        assert_eq!(all.len(), 1);
    }
}
