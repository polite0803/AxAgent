// SPDX-License-Identifier: AGPL-3.0-only

//! 设备权限存储
//!
//! 管理各设备的操作权限配置。

use std::collections::HashMap;

use axagent_harness::device_sync::{
    DevicePermissions, PermissionUpdate, TrustLevel,
};
use axagent_harness::util_fns::current_rfc3339;
use tokio::sync::RwLock;

/// 设备权限存储
#[derive(Default)]
pub struct PermissionStore {
    permissions: RwLock<HashMap<String, DevicePermissions>>,
}

impl PermissionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置设备权限
    pub async fn set_permissions(&self, permissions: DevicePermissions) {
        self.permissions
            .write()
            .await
            .insert(permissions.device_id.clone(), permissions);
    }

    /// 获取设备权限
    pub async fn get_permissions(&self, device_id: &str) -> Option<DevicePermissions> {
        self.permissions.read().await.get(device_id).cloned()
    }

    /// 获取所有设备权限
    pub async fn get_all_permissions(&self) -> Vec<DevicePermissions> {
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
        let perm = permissions
            .get_mut(device_id)
            .ok_or_else(|| "设备权限不存在".to_string())?;

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
        Ok(perm.clone())
    }

    /// 初始化设备权限（使用默认值）
    pub async fn init_device_permissions(&self, device_id: &str, trust_level: TrustLevel) -> DevicePermissions {
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
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_push)
            .unwrap_or(false)
    }

    pub async fn can_pull(&self, device_id: &str) -> bool {
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_pull)
            .unwrap_or(false)
    }

    pub async fn can_full_sync(&self, device_id: &str) -> bool {
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_full_sync)
            .unwrap_or(false)
    }

    pub async fn can_resolve_conflicts(&self, device_id: &str) -> bool {
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_resolve_conflicts)
            .unwrap_or(false)
    }

    pub async fn can_manage_devices(&self, device_id: &str) -> bool {
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_manage_devices)
            .unwrap_or(false)
    }

    pub async fn can_modify_policy(&self, device_id: &str) -> bool {
        self.permissions
            .read()
            .await
            .get(device_id)
            .map(|p| p.allow_modify_policy)
            .unwrap_or(false)
    }

    /// 移除设备权限
    pub async fn remove_permissions(&self, device_id: &str) {
        self.permissions.write().await.remove(device_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_and_get_permissions() {
        let store = PermissionStore::new();
        let perms = store
            .init_device_permissions("device-1", TrustLevel::Standard)
            .await;

        assert_eq!(perms.device_id, "device-1");
        assert_eq!(perms.trust_level, TrustLevel::Standard);
        assert!(perms.allow_push);
        assert!(!perms.allow_full_sync); // Standard 不允许全量同步
    }

    #[tokio::test]
    async fn test_update_permissions() {
        let store = PermissionStore::new();
        store
            .init_device_permissions("device-1", TrustLevel::Standard)
            .await;

        let updated = store
            .update_permissions(
                "device-1",
                PermissionUpdate {
                    allow_full_sync: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(updated.allow_full_sync);
    }

    #[tokio::test]
    async fn test_check_permissions() {
        let store = PermissionStore::new();
        store
            .init_device_permissions("device-1", TrustLevel::Full)
            .await;

        assert!(store.can_push("device-1").await);
        assert!(store.can_full_sync("device-1").await);
        assert!(store.can_manage_devices("device-1").await);
        assert!(!store.can_push("unknown-device").await);
    }
}