// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::sync_audit_log;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{AuditAction, AuditLogEntry};

/// 将 AuditAction 枚举转换为字符串
fn audit_action_to_str(action: &AuditAction) -> String {
    match action {
        AuditAction::DeviceRegistered => "device_registered",
        AuditAction::DevicePaired => "device_paired",
        AuditAction::DeviceUnpaired => "device_unpaired",
        AuditAction::SyncStarted => "sync_started",
        AuditAction::SyncCompleted => "sync_completed",
        AuditAction::SyncFailed => "sync_failed",
        AuditAction::ConflictDetected => "conflict_detected",
        AuditAction::ConflictResolved => "conflict_resolved",
        AuditAction::PolicyUpdated => "policy_updated",
        AuditAction::PermissionChanged => "permission_changed",
        AuditAction::EncryptionEnabled => "encryption_enabled",
        AuditAction::EncryptionDisabled => "encryption_disabled",
    }
    .to_string()
}

/// 将字符串转换为 AuditAction 枚举
fn str_to_audit_action(s: &str) -> AuditAction {
    match s {
        "device_registered" => AuditAction::DeviceRegistered,
        "device_paired" => AuditAction::DevicePaired,
        "device_unpaired" => AuditAction::DeviceUnpaired,
        "sync_started" => AuditAction::SyncStarted,
        "sync_completed" => AuditAction::SyncCompleted,
        "sync_failed" => AuditAction::SyncFailed,
        "conflict_detected" => AuditAction::ConflictDetected,
        "conflict_resolved" => AuditAction::ConflictResolved,
        "policy_updated" => AuditAction::PolicyUpdated,
        "permission_changed" => AuditAction::PermissionChanged,
        "encryption_enabled" => AuditAction::EncryptionEnabled,
        "encryption_disabled" => AuditAction::EncryptionDisabled,
        _ => AuditAction::SyncFailed, // 回退
    }
}

/// 从实体模型转换为领域模型
fn model_to_audit_entry(model: &sync_audit_log::Model) -> AuditLogEntry {
    let timestamp = chrono::DateTime::from_timestamp(model.created_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    AuditLogEntry {
        id: model.id.clone(),
        action: str_to_audit_action(&model.action),
        entity_type: model.target_type.clone(),
        entity_id: model.target_id.clone(),
        device_id: model.actor_device_id.clone(),
        details: model.details.as_ref().and_then(|d| serde_json::from_str(d).ok()),
        success: model.is_successful,
        error_message: model.error_message.clone(),
        timestamp,
    }
}

/// 添加审计日志
pub async fn add_audit_log(db: &DatabaseConnection, entry: &AuditLogEntry) -> Result<()> {
    let created_at = chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());

    sync_audit_log::Entity::insert(sync_audit_log::ActiveModel {
        id: Set(entry.id.clone()),
        action: Set(audit_action_to_str(&entry.action)),
        target_type: Set(entry.entity_type.clone()),
        target_id: Set(entry.entity_id.clone()),
        actor_device_id: Set(entry.device_id.clone()),
        is_successful: Set(entry.success),
        details: Set(entry.details.as_ref().and_then(|d| serde_json::to_string(d).ok())),
        error_message: Set(entry.error_message.clone()),
        created_at: Set(created_at),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// 批量添加审计日志
pub async fn batch_add_audit_logs(
    db: &DatabaseConnection,
    entries: &[AuditLogEntry],
) -> Result<()> {
    let models: Vec<sync_audit_log::ActiveModel> = entries
        .iter()
        .map(|entry| {
            let created_at = chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());

            sync_audit_log::ActiveModel {
                id: Set(entry.id.clone()),
                action: Set(audit_action_to_str(&entry.action)),
                target_type: Set(entry.entity_type.clone()),
                target_id: Set(entry.entity_id.clone()),
                actor_device_id: Set(entry.device_id.clone()),
                is_successful: Set(entry.success),
                details: Set(entry.details.as_ref().and_then(|d| serde_json::to_string(d).ok())),
                error_message: Set(entry.error_message.clone()),
                created_at: Set(created_at),
            }
        })
        .collect();

    sync_audit_log::Entity::insert_many(models).exec(db).await?;
    Ok(())
}

/// 根据设备获取审计日志
pub async fn get_audit_logs_by_device(
    db: &DatabaseConnection,
    device_id: &str,
    limit: Option<u64>,
) -> Result<Vec<AuditLogEntry>> {
    let mut query = sync_audit_log::Entity::find()
        .filter(sync_audit_log::Column::ActorDeviceId.eq(device_id))
        .order_by_desc(sync_audit_log::Column::CreatedAt);

    if let Some(l) = limit {
        query = query.limit(l);
    }

    let rows = query.all(db).await?;
    Ok(rows.iter().map(model_to_audit_entry).collect())
}

/// 根据操作类型获取审计日志
pub async fn get_audit_logs_by_action(
    db: &DatabaseConnection,
    action: &str,
    limit: Option<u64>,
) -> Result<Vec<AuditLogEntry>> {
    let mut query = sync_audit_log::Entity::find()
        .filter(sync_audit_log::Column::Action.eq(action))
        .order_by_desc(sync_audit_log::Column::CreatedAt);

    if let Some(l) = limit {
        query = query.limit(l);
    }

    let rows = query.all(db).await?;
    Ok(rows.iter().map(model_to_audit_entry).collect())
}

/// 获取指定时间范围的审计日志
pub async fn get_audit_logs_by_time_range(
    db: &DatabaseConnection,
    start: i64,
    end: i64,
) -> Result<Vec<AuditLogEntry>> {
    let rows = sync_audit_log::Entity::find()
        .filter(sync_audit_log::Column::CreatedAt.gte(start))
        .filter(sync_audit_log::Column::CreatedAt.lte(end))
        .order_by_desc(sync_audit_log::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_audit_entry).collect())
}

/// 获取失败的审计日志
pub async fn get_failed_audit_logs(
    db: &DatabaseConnection,
    limit: Option<u64>,
) -> Result<Vec<AuditLogEntry>> {
    let mut query = sync_audit_log::Entity::find()
        .filter(sync_audit_log::Column::IsSuccessful.eq(false))
        .order_by_desc(sync_audit_log::Column::CreatedAt);

    if let Some(l) = limit {
        query = query.limit(l);
    }

    let rows = query.all(db).await?;
    Ok(rows.iter().map(model_to_audit_entry).collect())
}

/// 删除旧的审计日志
pub async fn delete_old_audit_logs(db: &DatabaseConnection, before_timestamp: i64) -> Result<u64> {
    let result = sync_audit_log::Entity::delete_many()
        .filter(sync_audit_log::Column::CreatedAt.lt(before_timestamp))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// 获取审计日志数量
pub async fn count_audit_logs(db: &DatabaseConnection, device_id: Option<&str>) -> Result<u64> {
    let mut query = sync_audit_log::Entity::find();

    if let Some(id) = device_id {
        query = query.filter(sync_audit_log::Column::ActorDeviceId.eq(id));
    }

    let count = query.count(db).await?;
    Ok(count)
}
