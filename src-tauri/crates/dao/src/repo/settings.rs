// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use sea_query::OnConflict;

use axagent_entities::settings;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::AppSettings;

/// 将 snake_case 键转换为 camelCase，兼容数据库中旧版 snake_case 存储。
/// 新版（添加 rename_all = "camelCase" 后）存储使用 camelCase 键。
fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut next_upper = false;
    for ch in s.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(ch.to_ascii_uppercase());
            next_upper = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// 将 camelCase 键转换为 snake_case，用于清理数据库中旧格式的键。
fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 8);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

pub async fn get_settings(db: &DatabaseConnection) -> Result<AppSettings> {
    let rows = settings::Entity::find().all(db).await?;

    // 打印数据库中所有记录
    tracing::info!("[get_settings] 数据库中共有 {} 条记录", rows.len());
    for row in &rows {
        tracing::info!("[get_settings] DB记录 key={} value={}", row.key, row.value);
    }

    let mut map = serde_json::Map::new();
    for row in &rows {
        // 尝试解析 JSON 值；如果是残留的 "null" 字符串、空字符串或 "undefined" 字符串，
        // 当作 Value::Null 处理，避免将无效值误读为有效值。
        let val = if row.value == "null" || row.value.is_empty() || row.value == "undefined" {
            if row.value == "undefined" {
                tracing::warn!(
                    "[get_settings] 发现无效值 'undefined' key={}，将其视为 null",
                    row.key
                );
            }
            serde_json::Value::Null
        } else {
            serde_json::from_str::<serde_json::Value>(&row.value)
                .unwrap_or_else(|_| serde_json::Value::String(row.value.clone()))
        };
        // 兼容旧版 snake_case 键：统一转换为 camelCase
        let camel_key = snake_to_camel(&row.key);
        tracing::debug!(
            "[get_settings] 处理记录 key={} camel_key={} value={} parsed={:?}",
            row.key,
            camel_key,
            row.value,
            val
        );
        map.insert(camel_key, val);
    }

    // 打印构建的 map 中的关键字段
    let dp_id = map.get("defaultProviderId").and_then(|v| v.as_str());
    let dm_id = map.get("defaultModelId").and_then(|v| v.as_str());
    tracing::info!(
        "[get_settings] 构建的map中 defaultProviderId={:?} defaultModelId={:?}",
        dp_id,
        dm_id
    );

    tracing::info!("[get_settings] 从数据库读取 {} 条记录", rows.len());

    let settings: AppSettings =
        serde_json::from_value(serde_json::Value::Object(map)).unwrap_or_default();

    tracing::info!(
        "[get_settings] 反序列化后 default_provider_id={:?} default_model_id={:?}",
        settings.default_provider_id,
        settings.default_model_id,
    );

    Ok(settings)
}

pub async fn save_settings(db: &DatabaseConnection, settings: &AppSettings) -> Result<()> {
    let value = serde_json::to_value(settings).unwrap_or_default();

    tracing::info!(
        "[save_settings] 序列化设置 default_provider_id={:?} default_model_id={:?}",
        settings.default_provider_id,
        settings.default_model_id,
    );

    // 打印序列化后的关键模型字段
    if let serde_json::Value::Object(ref map) = value {
        let dp_id = map.get("defaultProviderId").and_then(|v| v.as_str());
        let dm_id = map.get("defaultModelId").and_then(|v| v.as_str());
        tracing::info!(
            "[save_settings] 序列化后 defaultProviderId={:?} defaultModelId={:?}",
            dp_id,
            dm_id
        );

        // 保存前：先删除所有旧记录（彻底清理，避免残留）
        let existing_rows = settings::Entity::find().all(db).await?;
        tracing::info!("[save_settings] 保存前数据库中有 {} 条旧记录", existing_rows.len());
        for row in &existing_rows {
            tracing::debug!("[save_settings] 删除旧记录 key={} value={}", row.key, row.value);
            settings::Entity::delete_by_id(row.key.clone()).exec(db).await?;
        }

        // 也清理所有可能的 snake_case 旧键（防极端情况）
        let all_camel_keys: Vec<String> = map.keys().cloned().collect();
        for camel_key in &all_camel_keys {
            let snake_key = camel_to_snake(camel_key);
            if snake_key != *camel_key {
                settings::Entity::delete_by_id(snake_key).exec(db).await?;
            }
        }

        // 收集需要保存的键值对（在事务外完成，避免所有权问题）
        let entries_to_save: Vec<(String, String)> = map
            .iter()
            .filter_map(|(key, val)| {
                // 跳过 null 值和空字符串值
                if val.is_null() {
                    tracing::debug!("[save_settings] 跳过 null 值 key={}", key);
                    return None;
                }
                if let Some(s) = val.as_str()
                    && s.is_empty()
                {
                    tracing::debug!("[save_settings] 跳过空字符串 key={}", key);
                    return None;
                }
                // 防御：跳过 "undefined" 和 "null" 字面量字符串
                // 这是前端传来的脏数据，不能存入数据库
                if let Some(s) = val.as_str()
                    && (s == "undefined" || s == "null")
                {
                    tracing::warn!("[save_settings] 跳过无效字符串 key={} value={}", key, s);
                    return None;
                }
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                Some((key.clone(), val_str))
            })
            .collect();

        tracing::info!("[save_settings] 准备保存 {} 个键值对", entries_to_save.len());

        // 在事务中保存所有非 null、非空字符串的值
        db.transaction::<_, _, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                for (key, val_str) in &entries_to_save {
                    tracing::debug!("[save_settings] 保存键值对 key={} value={}", key, val_str);
                    settings::Entity::insert(settings::ActiveModel {
                        key: Set(key.clone()),
                        value: Set(val_str.clone()),
                    })
                    .exec(txn)
                    .await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|e| match e {
            sea_orm::TransactionError::Connection(db_err) => AxAgentError::from(db_err),
            sea_orm::TransactionError::Transaction(db_err) => AxAgentError::from(db_err),
        })?;

        // 验证保存结果
        let saved_rows = settings::Entity::find().all(db).await?;
        tracing::info!("[save_settings] 保存完成，数据库中有 {} 条记录", saved_rows.len());
        for row in &saved_rows {
            tracing::debug!("[save_settings] 已保存 key={} value={}", row.key, row.value);
        }
    }
    Ok(())
}

pub async fn get_setting(db: &DatabaseConnection, key: &str) -> Result<Option<String>> {
    let row = settings::Entity::find_by_id(key).one(db).await?;
    Ok(row.map(|r| r.value))
}

pub async fn set_setting(db: &DatabaseConnection, key: &str, value: &str) -> Result<()> {
    settings::Entity::insert(settings::ActiveModel {
        key: Set(key.to_string()),
        value: Set(value.to_string()),
    })
    .on_conflict(
        OnConflict::column(settings::Column::Key).update_column(settings::Column::Value).to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}
