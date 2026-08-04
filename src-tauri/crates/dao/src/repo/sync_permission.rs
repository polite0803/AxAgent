// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use sea_query::OnConflict;

use axagent_entities::sync_permission;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{DevicePermissions, PermissionType, TrustLevel};

/// 从字符串解析信任级别
fn parse_trust_level(s: &str) -> TrustLevel {
    match s.to_lowercase().as_str() {
        "backup_only" => TrustLevel::BackupOnly,
        "full" => TrustLevel::Full,
        _ => TrustLevel::Standard,
    }
}

/// 从信任级别转换为字符串
fn trust_level_to_str(tl: &TrustLevel) -> &str {
    match tl {
        TrustLevel::BackupOnly => "backup_only",
        TrustLevel::Standard => "standard",
        TrustLevel::Full => "full",
    }
}

/// 将 Sea-ORM 模型转换为 DevicePermissions
fn model_to_device_permissions(model: &sync_permission::Model) -> DevicePermissions {
    let updated_at = chrono::DateTime::from_timestamp(model.updated_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    DevicePermissions {
        device_id: model.device_id.clone(),
        trust_level: parse_trust_level(&model.trust_level),
        allow_push: model.can_push,
        allow_pull: model.can_pull,
        allow_full_sync: model.can_full_sync,
        allow_resolve_conflicts: model.can_resolve_conflicts,
        allow_manage_devices: model.can_manage_devices,
        allow_modify_policy: model.can_modify_policy,
        updated_at,
    }
}

/// 从字符串解析权限类型
fn parse_permission_type(s: &str) -> PermissionType {
    match s.to_lowercase().as_str() {
        "push" => PermissionType::Push,
        "pull" => PermissionType::Pull,
        "full_sync" => PermissionType::FullSync,
        "resolve_conflicts" => PermissionType::ResolveConflicts,
        "manage_devices" => PermissionType::ManageDevices,
        "modify_policy" => PermissionType::ModifyPolicy,
        _ => PermissionType::Pull,
    }
}

/// 保存设备权限
pub async fn save_permissions(
    db: &DatabaseConnection,
    permissions: &DevicePermissions,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = uuid::Uuid::new_v4().to_string();

    // 获取现有权限记录的 ID
    let existing = sync_permission::Entity::find()
        .filter(sync_permission::Column::DeviceId.eq(&permissions.device_id))
        .one(db)
        .await?;

    let pk_id = existing.map(|e| e.id).unwrap_or_else(|| id);

    let trust_level_str = trust_level_to_str(&permissions.trust_level);

    sync_permission::Entity::insert(sync_permission::ActiveModel {
        id: Set(pk_id),
        device_id: Set(permissions.device_id.clone()),
        trust_level: Set(trust_level_str.to_string()),
        can_push: Set(permissions.allow_push),
        can_pull: Set(permissions.allow_pull),
        can_full_sync: Set(permissions.allow_full_sync),
        can_resolve_conflicts: Set(permissions.allow_resolve_conflicts),
        can_manage_devices: Set(permissions.allow_manage_devices),
        can_modify_policy: Set(permissions.allow_modify_policy),
        expires_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .on_conflict(
        OnConflict::column(sync_permission::Column::DeviceId)
            .update_columns([
                sync_permission::Column::TrustLevel,
                sync_permission::Column::CanPush,
                sync_permission::Column::CanPull,
                sync_permission::Column::CanFullSync,
                sync_permission::Column::CanResolveConflicts,
                sync_permission::Column::CanManageDevices,
                sync_permission::Column::CanModifyPolicy,
                sync_permission::Column::ExpiresAt,
                sync_permission::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

/// 获取设备权限
pub async fn get_permissions_by_device(
    db: &DatabaseConnection,
    device_id: &str,
) -> Result<Option<DevicePermissions>> {
    let row = sync_permission::Entity::find()
        .filter(sync_permission::Column::DeviceId.eq(device_id))
        .one(db)
        .await?;
    Ok(row.as_ref().map(model_to_device_permissions))
}

/// 获取所有权限配置
pub async fn get_all_permissions(
    db: &DatabaseConnection,
) -> Result<Vec<DevicePermissions>> {
    let rows = sync_permission::Entity::find().all(db).await?;
    Ok(rows.iter().map(model_to_device_permissions).collect())
}

/// 删除设备权限
pub async fn delete_permissions(db: &DatabaseConnection, device_id: &str) -> Result<()> {
    sync_permission::Entity::delete_many()
        .filter(sync_permission::Column::DeviceId.eq(device_id))
        .exec(db)
        .await?;
    Ok(())
}

/// 检查设备是否有特定权限
pub async fn check_permission(
    db: &DatabaseConnection,
    device_id: &str,
    permission: &str,
) -> Result<bool> {
    let permissions = get_permissions_by_device(db, device_id).await?;
    match permissions {
        Some(p) => {
            let perm_type = parse_permission_type(permission);
            Ok(match perm_type {
                PermissionType::Push => p.allow_push,
                PermissionType::Pull => p.allow_pull,
                PermissionType::FullSync => p.allow_full_sync,
                PermissionType::ResolveConflicts => p.allow_resolve_conflicts,
                PermissionType::ManageDevices => p.allow_manage_devices,
                PermissionType::ModifyPolicy => p.allow_modify_policy,
            })
        },
        None => Ok(false),
    }
}
