// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::sync_change_log;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{ChangeLogEntry, ChangeOperation, EntityType};

/// 从字符串解析实体类型
fn parse_entity_type(s: &str) -> EntityType {
    match s.to_lowercase().as_str() {
        "conversation" => EntityType::Conversation,
        "message" => EntityType::Message,
        "setting" => EntityType::Setting,
        "file" => EntityType::File,
        "wiki" => EntityType::Wiki,
        "knowledge" => EntityType::Knowledge,
        "agent" => EntityType::Agent,
        "workflow" => EntityType::Workflow,
        _ => EntityType::Setting,
    }
}

/// 从字符串解析操作类型
fn parse_change_operation(s: &str) -> ChangeOperation {
    match s.to_lowercase().as_str() {
        "create" => ChangeOperation::Create,
        "update" => ChangeOperation::Update,
        "delete" => ChangeOperation::Delete,
        _ => ChangeOperation::Update,
    }
}

/// 从操作类型转换为字符串
fn operation_to_str(op: &ChangeOperation) -> &str {
    match op {
        ChangeOperation::Create => "create",
        ChangeOperation::Update => "update",
        ChangeOperation::Delete => "delete",
    }
}

/// 从实体类型转换为字符串
fn entity_type_to_str(et: &EntityType) -> &str {
    match et {
        EntityType::Conversation => "conversation",
        EntityType::Message => "message",
        EntityType::Setting => "setting",
        EntityType::File => "file",
        EntityType::Wiki => "wiki",
        EntityType::Knowledge => "knowledge",
        EntityType::Agent => "agent",
        EntityType::Workflow => "workflow",
    }
}

/// 将 Sea-ORM 模型转换为 ChangeLogEntry
fn model_to_change_log_entry(model: &sync_change_log::Model) -> ChangeLogEntry {
    let _synced_to: Vec<String> = serde_json::from_str(&model.synced_to).unwrap_or_default();
    let version_vector = vec![axagent_harness::device_sync::VersionVectorEntry {
        device_id: String::new(),
        counter: model.version as u64,
    }];

    ChangeLogEntry {
        id: model.id.clone(),
        entity_type: parse_entity_type(&model.entity_type),
        entity_id: model.entity_id.clone(),
        operation: parse_change_operation(&model.operation),
        timestamp: model.created_at as u64,
        device_id: model.device_id.clone(),
        version_vector,
        data: Some(model.data.clone()),
    }
}

/// 添加变更日志
pub async fn add_change_log(db: &DatabaseConnection, entry: &ChangeLogEntry) -> Result<()> {
    let now = entry.timestamp as i64;
    let synced_to = serde_json::to_string(&Vec::<String>::new()).unwrap_or_default();

    sync_change_log::Entity::insert(sync_change_log::ActiveModel {
        id: Set(entry.id.clone()),
        device_id: Set(entry.device_id.clone()),
        entity_type: Set(entity_type_to_str(&entry.entity_type).to_string()),
        entity_id: Set(entry.entity_id.clone()),
        operation: Set(operation_to_str(&entry.operation).to_string()),
        data: Set(entry.data.clone().unwrap_or_default()),
        version: Set(entry.version_vector.first().map(|v| v.counter as i64).unwrap_or(0)),
        parent_version_id: Set(None),
        created_at: Set(now),
        is_synced: Set(false),
        synced_to: Set(synced_to),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// 批量添加变更日志
pub async fn batch_add_change_logs(
    db: &DatabaseConnection,
    entries: &[ChangeLogEntry],
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let models: Vec<sync_change_log::ActiveModel> = entries
        .iter()
        .map(|entry| {
            let synced_to =
                serde_json::to_string(&Vec::<String>::new()).unwrap_or_default();
            sync_change_log::ActiveModel {
                id: Set(entry.id.clone()),
                device_id: Set(entry.device_id.clone()),
                entity_type: Set(entity_type_to_str(&entry.entity_type).to_string()),
                entity_id: Set(entry.entity_id.clone()),
                operation: Set(operation_to_str(&entry.operation).to_string()),
                data: Set(entry.data.clone().unwrap_or_default()),
                version: Set(entry.version_vector.first().map(|v| v.counter as i64).unwrap_or(0)),
                parent_version_id: Set(None),
                created_at: Set(now),
                is_synced: Set(false),
                synced_to: Set(synced_to),
            }
        })
        .collect();

    sync_change_log::Entity::insert_many(models).exec(db).await?;
    Ok(())
}

/// 根据设备获取变更日志
pub async fn get_change_logs_by_device(
    db: &DatabaseConnection,
    device_id: &str,
    _since_timestamp: Option<u64>,
) -> Result<Vec<ChangeLogEntry>> {
    let rows = sync_change_log::Entity::find()
        .filter(sync_change_log::Column::DeviceId.eq(device_id))
        .order_by_asc(sync_change_log::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_change_log_entry).collect())
}

/// 获取未同步的变更日志
pub async fn get_unsynced_change_logs(
    db: &DatabaseConnection,
    device_id: &str,
) -> Result<Vec<ChangeLogEntry>> {
    let rows = sync_change_log::Entity::find()
        .filter(sync_change_log::Column::DeviceId.eq(device_id))
        .filter(sync_change_log::Column::IsSynced.eq(false))
        .order_by_asc(sync_change_log::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_change_log_entry).collect())
}

/// 标记变更为已同步
pub async fn mark_changes_as_synced(
    db: &DatabaseConnection,
    change_ids: &[String],
    target_device_id: &str,
) -> Result<()> {
    db.transaction::<_, _, sea_orm::DbErr>(|txn| {
        let ids = change_ids.to_vec();
        let target = target_device_id.to_string();
        Box::pin(async move {
            for id in &ids {
                let row = sync_change_log::Entity::find_by_id(id).one(txn).await?;
                if let Some(model) = row {
                    let mut synced_to: Vec<String> =
                        serde_json::from_str(&model.synced_to).unwrap_or_default();
                    if !synced_to.contains(&target) {
                        synced_to.push(target.clone());
                    }
                    let is_synced = !synced_to.is_empty();
                    let synced_to_json = serde_json::to_string(&synced_to).unwrap_or_default();

                    let mut active: sync_change_log::ActiveModel = model.into();
                    active.synced_to = Set(synced_to_json);
                    active.is_synced = Set(is_synced);

                    active.update(txn).await?;
                }
            }
            Ok(())
        })
    })
    .await?;
    Ok(())
}
