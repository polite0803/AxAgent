// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use sea_query::OnConflict;

use axagent_entities::sync_policy;
use axagent_harness::core_error::Result;
use axagent_harness::device_sync::{ConflictResolutionStrategy, EntityType, SyncPolicy};

/// 从字符串解析冲突解决策略
fn parse_conflict_strategy(s: &str) -> ConflictResolutionStrategy {
    match s.to_lowercase().as_str() {
        "keep_local" => ConflictResolutionStrategy::KeepLocal,
        "keep_remote" => ConflictResolutionStrategy::KeepRemote,
        "keep_both" => ConflictResolutionStrategy::KeepBoth,
        "last_write_wins" => ConflictResolutionStrategy::LastWriteWins,
        _ => ConflictResolutionStrategy::LastWriteWins,
    }
}

/// 从策略类型转换为字符串
fn conflict_strategy_to_str(s: &ConflictResolutionStrategy) -> &str {
    match s {
        ConflictResolutionStrategy::KeepLocal => "keep_local",
        ConflictResolutionStrategy::KeepRemote => "keep_remote",
        ConflictResolutionStrategy::KeepBoth => "keep_both",
        ConflictResolutionStrategy::LastWriteWins => "last_write_wins",
        ConflictResolutionStrategy::CustomMerge => "custom_merge",
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

/// 将 Sea-ORM 模型转换为 SyncPolicy
fn model_to_sync_policy(model: &sync_policy::Model) -> SyncPolicy {
    let sync_scope: Vec<EntityType> =
        serde_json::from_str::<Vec<String>>(&model.allowed_entity_types)
            .unwrap_or_default()
            .into_iter()
            .map(|s| parse_entity_type(&s))
            .collect();

    let updated_at = chrono::DateTime::from_timestamp(model.updated_at / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    SyncPolicy {
        id: model.id.clone(),
        name: model.name.clone(),
        conflict_strategy: parse_conflict_strategy(&model.conflict_strategy),
        auto_sync_interval_secs: model.sync_interval_ms as u64 / 1000,
        sync_scope,
        auto_resolve_conflicts: false, // 默认 false
        max_conflict_threshold: 100,   // 默认 100
        change_log_retention_enabled: false,
        change_log_retention_days: 30,
        enabled: model.is_enabled,
        updated_at,
    }
}

/// 创建或更新策略
pub async fn save_policy(db: &DatabaseConnection, policy: &SyncPolicy) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();

    let sync_scope: Vec<String> =
        policy.sync_scope.iter().map(|et| entity_type_to_str(et).to_string()).collect();
    let sync_scope_json = serde_json::to_string(&sync_scope).unwrap_or_default();
    let conflict_strategy_str = conflict_strategy_to_str(&policy.conflict_strategy);

    sync_policy::Entity::insert(sync_policy::ActiveModel {
        id: Set(policy.id.clone()),
        name: Set(policy.name.clone()),
        description: Set(None),
        sync_mode: Set("manual".to_string()),
        conflict_strategy: Set(conflict_strategy_str.to_string()),
        sync_interval_ms: Set((policy.auto_sync_interval_secs * 1000) as i64),
        allowed_entity_types: Set(sync_scope_json),
        excluded_entity_types: Set("[]".to_string()),
        compression_algorithm: Set("none".to_string()),
        max_transfer_size: Set(104857600),
        encryption_enabled: Set(false),
        is_enabled: Set(policy.enabled),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .on_conflict(
        OnConflict::column(sync_policy::Column::Id)
            .update_columns([
                sync_policy::Column::Name,
                sync_policy::Column::ConflictStrategy,
                sync_policy::Column::SyncIntervalMs,
                sync_policy::Column::AllowedEntityTypes,
                sync_policy::Column::IsEnabled,
                sync_policy::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

/// 获取所有策略
pub async fn get_all_policies(db: &DatabaseConnection) -> Result<Vec<SyncPolicy>> {
    let rows = sync_policy::Entity::find().all(db).await?;
    Ok(rows.iter().map(model_to_sync_policy).collect())
}

/// 获取启用的策略
pub async fn get_enabled_policies(db: &DatabaseConnection) -> Result<Vec<SyncPolicy>> {
    let rows =
        sync_policy::Entity::find().filter(sync_policy::Column::IsEnabled.eq(true)).all(db).await?;
    Ok(rows.iter().map(model_to_sync_policy).collect())
}

/// 根据 ID 获取策略
pub async fn get_policy_by_id(db: &DatabaseConnection, id: &str) -> Result<Option<SyncPolicy>> {
    let row = sync_policy::Entity::find_by_id(id).one(db).await?;
    Ok(row.as_ref().map(model_to_sync_policy))
}

/// 删除策略
pub async fn delete_policy(db: &DatabaseConnection, id: &str) -> Result<()> {
    sync_policy::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
