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

    let mut map = serde_json::Map::new();
    for row in &rows {
        // 尝试解析 JSON 值；如果是残留的 "null" 字符串（之前错误存储的），
        // 当作 Value::Null 处理，避免将字符串 "null" 误读为有效值。
        let val = if row.value == "null" {
            serde_json::Value::Null
        } else {
            serde_json::from_str::<serde_json::Value>(&row.value)
                .unwrap_or_else(|_| serde_json::Value::String(row.value.clone()))
        };
        // 兼容旧版 snake_case 键：统一转换为 camelCase
        let camel_key = snake_to_camel(&row.key);
        map.insert(camel_key, val);
    }

    let settings: AppSettings =
        serde_json::from_value(serde_json::Value::Object(map)).unwrap_or_default();
    Ok(settings)
}

pub async fn save_settings(db: &DatabaseConnection, settings: &AppSettings) -> Result<()> {
    let value = serde_json::to_value(settings).unwrap_or_default();

    if let serde_json::Value::Object(map) = value {
        // 保存前先清理数据库中可能存在的旧 snake_case 键，避免重复数据。
        // 新版使用 camelCase 键存储，旧版使用 snake_case 键，需要迁移清理。
        let existing_rows = settings::Entity::find().all(db).await?;
        for row in &existing_rows {
            let camel_key = snake_to_camel(&row.key);
            if camel_key != row.key {
                // 旧 snake_case 键：无论新 map 中是否存在对应 camelCase 键，
                // 都必须删除，避免旧数据干扰（如 value="null" 的残留记录）。
                settings::Entity::delete_by_id(row.key.clone()).exec(db).await?;
            }
        }

        // 清理值为 null 的字段在数据库中可能存在的旧 snake_case 记录。
        // 即使 snake_case 键已被上面清理，这里也处理新的 camelCase 键对应的 snake_case 旧记录。
        for (key, val) in &map {
            if val.is_null() {
                let snake_key = camel_to_snake(key);
                if snake_key != *key {
                    settings::Entity::delete_by_id(snake_key).exec(db).await?;
                }
            }
        }

        db.transaction::<_, _, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                for (key, val) in &map {
                    // 跳过 null 值：Option::None 不应存入数据库
                    if val.is_null() {
                        continue;
                    }
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    settings::Entity::insert(settings::ActiveModel {
                        key: Set(key.clone()),
                        value: Set(val_str),
                    })
                    .on_conflict(
                        OnConflict::column(settings::Column::Key)
                            .update_column(settings::Column::Value)
                            .to_owned(),
                    )
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
