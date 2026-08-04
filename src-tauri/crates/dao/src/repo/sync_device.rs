// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use sea_query::OnConflict;

use axagent_entities::sync_device;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{DeviceInfo, DeviceType, TrustLevel};

/// 从字符串解析设备类型
fn parse_device_type(s: &str) -> DeviceType {
    match s.to_lowercase().as_str() {
        "desktop" => DeviceType::Desktop,
        "mobile" => DeviceType::Mobile,
        "server" => DeviceType::Server,
        _ => DeviceType::Desktop,
    }
}

/// 从字符串解析信任级别
fn parse_trust_level(s: &str) -> TrustLevel {
    match s.to_lowercase().as_str() {
        "backup_only" => TrustLevel::BackupOnly,
        "full" => TrustLevel::Full,
        _ => TrustLevel::Standard,
    }
}

/// 将 Sea-ORM 模型转换为 DeviceInfo
fn model_to_device_info(model: &sync_device::Model) -> DeviceInfo {
    let registered_at = chrono::DateTime::from_timestamp(model.created_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default();

    let last_active_at = model
        .last_heartbeat_at
        .and_then(|ts| chrono::DateTime::from_timestamp(ts / 1000, 0))
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    DeviceInfo {
        device_id: model.id.clone(),
        name: model.name.clone(),
        hostname: model.unique_id.clone(), // 使用 unique_id 作为 hostname
        os: model.os.clone(),
        device_type: parse_device_type(&model.device_type),
        app_version: model.app_version.clone(),
        registered_at,
        last_active_at,
        is_paired: model.is_paired,
        trust_level: parse_trust_level(&model.trust_level),
    }
}

/// 创建或更新设备
pub async fn save_device(db: &DatabaseConnection, device: &DeviceInfo) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let created_at = chrono::DateTime::parse_from_rfc3339(&device.registered_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(now);

    let trust_level_str = match device.trust_level {
        TrustLevel::BackupOnly => "backup_only",
        TrustLevel::Standard => "standard",
        TrustLevel::Full => "full",
    };

    let device_type_str = match device.device_type {
        DeviceType::Desktop => "desktop",
        DeviceType::Mobile => "mobile",
        DeviceType::Server => "server",
    };

    sync_device::Entity::insert(sync_device::ActiveModel {
        id: Set(device.device_id.clone()),
        name: Set(device.name.clone()),
        device_type: Set(device_type_str.to_string()),
        os: Set(device.os.clone()),
        app_version: Set(device.app_version.clone()),
        unique_id: Set(device.hostname.clone()),
        public_key: Set(String::new()), // 默认空
        ip_address: Set(None),
        is_paired: Set(device.is_paired),
        trust_level: Set(trust_level_str.to_string()),
        last_synced_at: Set(None),
        last_heartbeat_at: Set(Some(now)),
        is_enabled: Set(true),
        created_at: Set(created_at),
        updated_at: Set(now),
    })
    .on_conflict(
        OnConflict::column(sync_device::Column::Id)
            .update_columns([
                sync_device::Column::Name,
                sync_device::Column::Os,
                sync_device::Column::AppVersion,
                sync_device::Column::IsPaired,
                sync_device::Column::TrustLevel,
                sync_device::Column::LastHeartbeatAt,
                sync_device::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

/// 获取所有设备
pub async fn get_all_devices(db: &DatabaseConnection) -> Result<Vec<DeviceInfo>> {
    let rows = sync_device::Entity::find().all(db).await?;
    Ok(rows.iter().map(model_to_device_info).collect())
}

/// 获取已配对设备
pub async fn get_paired_devices(db: &DatabaseConnection) -> Result<Vec<DeviceInfo>> {
    let rows = sync_device::Entity::find()
        .filter(sync_device::Column::IsPaired.eq(true))
        .filter(sync_device::Column::IsEnabled.eq(true))
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_device_info).collect())
}

/// 根据 ID 获取设备
pub async fn get_device_by_id(db: &DatabaseConnection, id: &str) -> Result<Option<DeviceInfo>> {
    let row = sync_device::Entity::find_by_id(id).one(db).await?;
    Ok(row.as_ref().map(model_to_device_info))
}

/// 更新设备
pub async fn update_device(db: &DatabaseConnection, device: &DeviceInfo) -> Result<()> {
    // 使用 save_device 实现 upsert
    save_device(db, device).await
}

/// 删除设备
pub async fn delete_device(db: &DatabaseConnection, id: &str) -> Result<()> {
    sync_device::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
