// SPDX-License-Identifier: AGPL-3.0-only

//! 权限检查中间件
//!
//! 在执行同步操作前进行权限验证，
//! 防止未授权的设备执行危险操作。

use std::sync::Arc;

use axagent_harness::device_sync::{DevicePermissions, PermissionType};
use tokio::sync::RwLock;

use crate::permission_store::PermissionStore;

/// 权限检查结果
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionCheckResult {
    /// 允许操作
    Allowed,
    /// 权限不足
    Denied { reason: String, required_permission: PermissionType },
    /// 设备未注册
    DeviceNotRegistered,
    /// 设备已被禁用
    DeviceDisabled,
}

impl PermissionCheckResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionCheckResult::Allowed)
    }

    pub fn denied(reason: impl Into<String>, permission: PermissionType) -> Self {
        PermissionCheckResult::Denied { reason: reason.into(), required_permission: permission }
    }
}

/// 权限检查器
#[derive(Clone)]
pub struct PermissionChecker {
    permission_store: Arc<RwLock<PermissionStore>>,
}

impl PermissionChecker {
    pub fn new(permission_store: Arc<RwLock<PermissionStore>>) -> Self {
        Self { permission_store }
    }

    /// 检查设备是否可以推送变更
    pub async fn check_push(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::Push).await
    }

    /// 检查设备是否可以拉取变更
    pub async fn check_pull(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::Pull).await
    }

    /// 检查设备是否可以执行全量同步
    pub async fn check_full_sync(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::FullSync).await
    }

    /// 检查设备是否可以解决冲突
    pub async fn check_resolve_conflicts(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::ResolveConflicts).await
    }

    /// 检查设备是否可以管理其他设备
    pub async fn check_manage_devices(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::ManageDevices).await
    }

    /// 检查设备是否可以修改策略
    pub async fn check_modify_policy(&self, device_id: &str) -> PermissionCheckResult {
        self.check_permission(device_id, PermissionType::ModifyPolicy).await
    }

    /// 检查特定权限
    pub async fn check_permission(
        &self,
        device_id: &str,
        permission: PermissionType,
    ) -> PermissionCheckResult {
        let store = self.permission_store.read().await;

        match store.get_permissions(device_id).await {
            None => PermissionCheckResult::DeviceNotRegistered,
            Some(permissions) => {
                if !is_permission_granted(&permissions, permission) {
                    PermissionCheckResult::denied(
                        format!(
                            "设备 '{}' 没有 {} 权限",
                            device_id,
                            permission_display_name(permission)
                        ),
                        permission,
                    )
                } else {
                    PermissionCheckResult::Allowed
                }
            },
        }
    }

    /// 批量检查多个设备的权限
    pub async fn batch_check(
        &self,
        device_ids: &[String],
        permission: PermissionType,
    ) -> Vec<(String, PermissionCheckResult)> {
        let mut results = Vec::new();
        for device_id in device_ids {
            let result = self.check_permission(device_id, permission).await;
            results.push((device_id.clone(), result));
        }
        results
    }

    /// 获取设备权限详情
    pub async fn get_device_permissions(&self, device_id: &str) -> Option<DevicePermissions> {
        let store = self.permission_store.read().await;
        store.get_permissions(device_id).await
    }

    /// 转换权限存储
    pub fn permission_store(&self) -> &Arc<RwLock<PermissionStore>> {
        &self.permission_store
    }
}

/// 检查权限是否被授予
fn is_permission_granted(permissions: &DevicePermissions, permission: PermissionType) -> bool {
    match permission {
        PermissionType::Push => permissions.allow_push,
        PermissionType::Pull => permissions.allow_pull,
        PermissionType::FullSync => permissions.allow_full_sync,
        PermissionType::ResolveConflicts => permissions.allow_resolve_conflicts,
        PermissionType::ManageDevices => permissions.allow_manage_devices,
        PermissionType::ModifyPolicy => permissions.allow_modify_policy,
    }
}

/// 获取权限的显示名称
fn permission_display_name(permission: PermissionType) -> &'static str {
    match permission {
        PermissionType::Push => "推送变更",
        PermissionType::Pull => "拉取变更",
        PermissionType::FullSync => "全量同步",
        PermissionType::ResolveConflicts => "解决冲突",
        PermissionType::ManageDevices => "管理设备",
        PermissionType::ModifyPolicy => "修改策略",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::device_sync::TrustLevel;

    async fn setup_checker() -> PermissionChecker {
        let store = Arc::new(RwLock::new(PermissionStore::new()));
        {
            let s = store.write().await;
            s.init_device_permissions("device-1", TrustLevel::Full).await;
            s.init_device_permissions("device-2", TrustLevel::Standard).await;
        }
        PermissionChecker::new(store)
    }

    #[tokio::test]
    async fn test_full_permission_device() {
        let checker = setup_checker().await;

        assert!(checker.check_push("device-1").await.is_allowed());
        assert!(checker.check_pull("device-1").await.is_allowed());
        assert!(checker.check_full_sync("device-1").await.is_allowed());
        assert!(checker.check_resolve_conflicts("device-1").await.is_allowed());
        assert!(checker.check_manage_devices("device-1").await.is_allowed());
        assert!(checker.check_modify_policy("device-1").await.is_allowed());
    }

    #[tokio::test]
    async fn test_standard_permission_device() {
        let checker = setup_checker().await;

        assert!(checker.check_push("device-2").await.is_allowed());
        assert!(checker.check_pull("device-2").await.is_allowed());
        assert!(!checker.check_full_sync("device-2").await.is_allowed());
        assert!(checker.check_resolve_conflicts("device-2").await.is_allowed());
        assert!(!checker.check_manage_devices("device-2").await.is_allowed());
        assert!(!checker.check_modify_policy("device-2").await.is_allowed());
    }

    #[tokio::test]
    async fn test_unregistered_device() {
        let checker = setup_checker().await;

        let result = checker.check_push("unknown-device").await;
        assert_eq!(result, PermissionCheckResult::DeviceNotRegistered);
    }

    #[tokio::test]
    async fn test_batch_check() {
        let checker = setup_checker().await;

        let results = checker
            .batch_check(
                &["device-1".to_string(), "device-2".to_string(), "device-3".to_string()],
                PermissionType::Push,
            )
            .await;

        assert_eq!(results.len(), 3);
        assert!(results[0].1.is_allowed());
        assert!(results[1].1.is_allowed());
        assert_eq!(results[2].1, PermissionCheckResult::DeviceNotRegistered);
    }
}
