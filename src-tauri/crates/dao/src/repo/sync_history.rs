// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::sync_history;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{
    ConflictInfo, SyncDirection, SyncHistoryEntry, SyncResult, SyncType,
};

/// 从字符串解析同步方向
fn parse_direction(s: &str) -> SyncDirection {
    match s.to_lowercase().as_str() {
        "pull" => SyncDirection::Pull,
        "both" => SyncDirection::Both,
        _ => SyncDirection::Push,
    }
}

/// 从同步方向转换为字符串
fn direction_to_str(d: &SyncDirection) -> &str {
    match d {
        SyncDirection::Push => "push",
        SyncDirection::Pull => "pull",
        SyncDirection::Both => "both",
    }
}

/// 从字符串解析同步类型
fn parse_sync_type(s: &str) -> SyncType {
    match s.to_lowercase().as_str() {
        "incremental" => SyncType::Incremental,
        "manual" => SyncType::Manual,
        _ => SyncType::Full,
    }
}

/// 从同步类型转换为字符串
fn sync_type_to_str(t: &SyncType) -> &str {
    match t {
        SyncType::Full => "full",
        SyncType::Incremental => "incremental",
        SyncType::Manual => "manual",
        SyncType::Scheduled => "scheduled",
    }
}

/// 从实体模型转换为领域模型
fn model_to_history_entry(model: &sync_history::Model) -> SyncHistoryEntry {
    let result: SyncResult = serde_json::from_str(&model.result).unwrap_or_else(|_| SyncResult::default());
    let conflicts: Vec<ConflictInfo> = serde_json::from_str(&model.conflicts).unwrap_or_default();

    let started_at = chrono::DateTime::from_timestamp(model.started_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default();

    let completed_at = chrono::DateTime::from_timestamp(model.completed_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default();

    SyncHistoryEntry {
        id: model.id.clone(),
        device_id: model.device_id.clone(),
        direction: parse_direction(&model.direction),
        sync_type: parse_sync_type(&model.sync_type),
        result,
        conflicts,
        started_at,
        completed_at,
        initiated_by: model.initiated_by.clone(),
    }
}

/// 添加同步历史记录
pub async fn add_history_entry(db: &DatabaseConnection, entry: &SyncHistoryEntry) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let started_at = chrono::DateTime::parse_from_rfc3339(&entry.started_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(now);
    let completed_at = chrono::DateTime::parse_from_rfc3339(&entry.completed_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(now);

    let result_json = serde_json::to_string(&entry.result).unwrap_or_default();
    let conflicts_json = serde_json::to_string(&entry.conflicts).unwrap_or_else(|_| "[]".to_string());

    sync_history::Entity::insert(sync_history::ActiveModel {
        id: Set(entry.id.clone()),
        device_id: Set(entry.device_id.clone()),
        direction: Set(direction_to_str(&entry.direction).to_string()),
        sync_type: Set(sync_type_to_str(&entry.sync_type).to_string()),
        result: Set(result_json),
        conflicts: Set(conflicts_json),
        started_at: Set(started_at),
        completed_at: Set(completed_at),
        initiated_by: Set(entry.initiated_by.clone()),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// 根据设备获取同步历史
pub async fn get_history_by_device(
    db: &DatabaseConnection,
    device_id: &str,
    limit: Option<u64>,
) -> Result<Vec<SyncHistoryEntry>> {
    let mut query = sync_history::Entity::find()
        .filter(sync_history::Column::DeviceId.eq(device_id))
        .order_by_desc(sync_history::Column::StartedAt);

    if let Some(l) = limit {
        query = query.limit(l);
    }

    let rows = query.all(db).await?;
    Ok(rows.iter().map(model_to_history_entry).collect())
}

/// 获取指定时间范围的历史记录
pub async fn get_history_by_time_range(
    db: &DatabaseConnection,
    start: i64,
    end: i64,
) -> Result<Vec<SyncHistoryEntry>> {
    let rows = sync_history::Entity::find()
        .filter(sync_history::Column::StartedAt.gte(start))
        .filter(sync_history::Column::StartedAt.lte(end))
        .order_by_desc(sync_history::Column::StartedAt)
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_history_entry).collect())
}

/// 删除旧的历史记录
pub async fn delete_old_history(
    db: &DatabaseConnection,
    before_timestamp: i64,
) -> Result<u64> {
    let result = sync_history::Entity::delete_many()
        .filter(sync_history::Column::CompletedAt.lt(before_timestamp))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// 获取设备同步统计
pub async fn get_sync_stats(db: &DatabaseConnection, device_id: &str) -> Result<SyncStats> {
    let total = sync_history::Entity::find()
        .filter(sync_history::Column::DeviceId.eq(device_id))
        .count(db)
        .await?;

    let rows = sync_history::Entity::find()
        .filter(sync_history::Column::DeviceId.eq(device_id))
        .all(db)
        .await?;

    let successful = rows
        .iter()
        .filter(|r| {
            let result: SyncResult =
                serde_json::from_str(&r.result).unwrap_or_default();
            result.success
        })
        .count() as u64;

    Ok(SyncStats {
        total_syncs: total,
        successful_syncs: successful,
        failed_syncs: total - successful,
    })
}

/// 同步统计
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SyncStats {
    pub total_syncs: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
}
