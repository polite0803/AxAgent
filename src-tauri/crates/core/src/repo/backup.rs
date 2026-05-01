use sea_orm::*;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::entity::backup_manifests;
use crate::error::{AxAgentError, Result};
use crate::types::BackupManifest;
use crate::utils::gen_id;

fn model_to_manifest(m: backup_manifests::Model) -> BackupManifest {
    BackupManifest {
        id: m.id,
        version: m.version,
        created_at: m.created_at,
        encrypted: m.encrypted != 0,
        checksum: m.checksum,
        object_counts_json: m.object_counts_json,
        source_app_version: m.source_app_version,
        file_path: m
            .file_path
            .as_ref()
            .map(|p| crate::path_vars::decode_path(p)),
        file_size: m.file_size,
    }
}

/// Get the backup directory, using the configured path or defaulting to the AxAgent home backups dir.
pub fn resolve_backup_dir(backup_dir_setting: Option<&str>, app_data_dir: &Path) -> PathBuf {
    if let Some(dir) = backup_dir_setting {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    app_data_dir.join("backups")
}

/// Ensure the backup directory exists
pub fn ensure_backup_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to create backup directory: {}", e)))
}

/// Create a real backup file (SQLite copy or JSON export)
pub async fn create_backup(
    db: &DatabaseConnection,
    format: &str,
    backup_dir: &Path,
) -> Result<BackupManifest> {
    ensure_backup_dir(backup_dir)?;

    let id = gen_id();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let extension = match format {
        "sqlite" => "db",
        _ => "json",
    };
    let filename = format!("axagent-backup-{}.{}", timestamp, extension);
    let file_path = backup_dir.join(&filename);

    match format {
        "sqlite" => {
            create_sqlite_backup(db, &file_path).await?;
        }
        _ => {
            create_json_backup(db, &file_path).await?;
        }
    }

    let file_size = std::fs::metadata(&file_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let checksum = compute_file_checksum(&file_path)?;

    // Count objects for manifest
    let object_counts = count_objects(db).await?;

    let am = backup_manifests::ActiveModel {
        id: Set(id.clone()),
        version: Set(format.to_string()),
        encrypted: Set(0),
        checksum: Set(checksum),
        object_counts_json: Set(object_counts),
        source_app_version: Set(env!("CARGO_PKG_VERSION").to_string()),
        file_path: Set(Some(crate::path_vars::encode_path(
            &file_path.to_string_lossy(),
        ))),
        file_size: Set(file_size),
        ..Default::default()
    };

    am.insert(db).await?;

    get_backup(db, &id).await
}

/// Create a SQLite backup using VACUUM INTO
async fn create_sqlite_backup(db: &DatabaseConnection, dest: &Path) -> Result<()> {
    let dest_str = dest.to_string_lossy().to_string();
    // Remove existing file if present (VACUUM INTO fails otherwise)
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|e| {
            AxAgentError::Gateway(format!("Failed to remove existing backup file: {}", e))
        })?;
    }
    db.execute(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        format!("VACUUM INTO '{}'", dest_str.replace('\'', "''")),
    ))
    .await
    .map_err(|e| AxAgentError::Gateway(format!("VACUUM INTO failed: {}", e)))?;
    Ok(())
}

/// Create a JSON backup by exporting all important tables
async fn create_json_backup(db: &DatabaseConnection, dest: &Path) -> Result<()> {
    use crate::entity::*;

    let conversations = conversations::Entity::find().all(db).await?;
    let messages = messages::Entity::find().all(db).await?;
    let providers = providers::Entity::find().all(db).await?;
    let provider_keys = provider_keys::Entity::find().all(db).await?;
    let models = models::Entity::find().all(db).await?;
    let settings = settings::Entity::find().all(db).await?;
    let gateway_keys = gateway_keys::Entity::find().all(db).await?;

    let data = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "tables": {
            "conversations": conversations,
            "messages": messages,
            "providers": providers,
            "provider_keys": provider_keys,
            "models": models,
            "settings": settings,
            "gateway_keys": gateway_keys,
        }
    });

    let json_str = serde_json::to_string_pretty(&data)
        .map_err(|e| AxAgentError::Gateway(format!("JSON serialization failed: {}", e)))?;
    std::fs::write(dest, json_str)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to write backup file: {}", e)))?;
    Ok(())
}

fn compute_file_checksum(path: &Path) -> Result<String> {
    let data = std::fs::read(path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read file for checksum: {}", e)))?;
    let hash = Sha256::digest(&data);
    Ok(format!("{:x}", hash))
}

async fn count_objects(db: &DatabaseConnection) -> Result<String> {
    use crate::entity::*;

    let conv_count = conversations::Entity::find().count(db).await.unwrap_or(0);
    let msg_count = messages::Entity::find().count(db).await.unwrap_or(0);
    let provider_count = providers::Entity::find().count(db).await.unwrap_or(0);

    let counts = serde_json::json!({
        "conversations": conv_count,
        "messages": msg_count,
        "providers": provider_count,
    });
    Ok(counts.to_string())
}

pub async fn list_backups(db: &DatabaseConnection) -> Result<Vec<BackupManifest>> {
    let models = backup_manifests::Entity::find()
        .order_by_desc(backup_manifests::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_manifest).collect())
}

pub async fn get_backup(db: &DatabaseConnection, id: &str) -> Result<BackupManifest> {
    let model = backup_manifests::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("BackupManifest {}", id)))?;

    Ok(model_to_manifest(model))
}

pub async fn delete_backup(db: &DatabaseConnection, id: &str) -> Result<()> {
    let manifest = get_backup(db, id).await?;

    // Delete the file from disk if it exists
    if let Some(ref path) = manifest.file_path {
        let p = Path::new(path);
        if p.exists() {
            std::fs::remove_file(p).ok();
        }
    }

    let result = backup_manifests::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("BackupManifest {}", id)));
    }
    Ok(())
}

pub async fn batch_delete_backups(db: &DatabaseConnection, ids: &[String]) -> Result<()> {
    for id in ids {
        delete_backup(db, id).await?;
    }
    Ok(())
}

/// Restore from a SQLite backup by replacing the current database file
pub async fn restore_sqlite_backup(backup_path: &str, current_db_path: &str) -> Result<()> {
    let src = Path::new(backup_path);
    if !src.exists() {
        return Err(AxAgentError::NotFound(format!(
            "Backup file not found: {}",
            backup_path
        )));
    }
    std::fs::copy(src, current_db_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to restore backup: {}", e)))?;
    Ok(())
}

/// 将 JSON 值转换为 sea_orm Value 以便动态构建 INSERT
fn json_value_to_sea_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::String(None),
        serde_json::Value::Bool(b) => Value::Bool(Some(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::BigInt(Some(i))
            } else if let Some(f) = n.as_f64() {
                Value::Double(Some(f))
            } else {
                Value::BigInt(None)
            }
        }
        serde_json::Value::String(s) => Value::String(Some(Box::new(s.clone()))),
        serde_json::Value::Array(a) => {
            Value::String(Some(Box::new(serde_json::to_string(a).unwrap_or_default())))
        }
        serde_json::Value::Object(o) => {
            Value::String(Some(Box::new(serde_json::to_string(o).unwrap_or_default())))
        }
    }
}

/// 从 JSON 备份恢复数据到数据库
///
/// # 参数
/// - `db`: 数据库连接
/// - `backup_path`: JSON 备份文件路径
/// - `strategy`: 恢复策略（Overwrite 清空后导入, Merge 跳过已存在, DryRun 仅验证）
///
/// # 恢复顺序
/// 按外键依赖顺序导入: settings → providers → provider_keys → models →
/// gateway_keys → conversations → messages
pub async fn restore_json_backup(
    db: &DatabaseConnection,
    backup_path: &str,
    strategy: &crate::types::RestoreStrategy,
) -> Result<crate::types::RestoreReport> {
    use crate::types::{RestoreReport, TableRestoreResult};

    let path = Path::new(backup_path);
    if !path.exists() {
        return Err(AxAgentError::NotFound(format!(
            "备份文件未找到: {}",
            backup_path
        )));
    }

    let data = std::fs::read_to_string(path)
        .map_err(|e| AxAgentError::Gateway(format!("读取备份文件失败: {}", e)))?;

    let backup: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| AxAgentError::Gateway(format!("解析备份 JSON 失败: {}", e)))?;

    let version = backup["version"].as_str().unwrap_or("unknown").to_string();

    let tables = backup["tables"]
        .as_object()
        .ok_or_else(|| AxAgentError::Gateway("备份文件格式无效：缺少 tables 字段".to_string()))?;

    // 按外键依赖顺序恢复
    let table_order = [
        "settings",
        "providers",
        "provider_keys",
        "models",
        "gateway_keys",
        "conversations",
        "messages",
    ];

    let mut report = RestoreReport {
        backup_version: version,
        strategy: format!("{}", strategy),
        tables_restored: Vec::new(),
        total_imported: 0,
        total_skipped: 0,
        total_errored: 0,
    };

    if matches!(strategy, crate::types::RestoreStrategy::DryRun) {
        // 空跑模式：仅统计行数
        for table_name in &table_order {
            if let Some(rows) = tables.get(*table_name).and_then(|v| v.as_array()) {
                report.tables_restored.push(TableRestoreResult {
                    table: String::from(*table_name),
                    rows_imported: rows.len(),
                    rows_skipped: 0,
                    rows_errored: 0,
                });
            }
        }
        report.total_imported = report.tables_restored.iter().map(|t| t.rows_imported).sum();
        return Ok(report);
    }

    // 开始事务
    let txn = db
        .begin()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("开始事务失败: {}", e)))?;

    for table_name in &table_order {
        let rows = match tables.get(*table_name).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };

        if rows.is_empty() {
            continue;
        }

        // Overwrite 策略：先清空表
        if matches!(strategy, crate::types::RestoreStrategy::Overwrite) {
            txn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("DELETE FROM \"{}\"", table_name),
            ))
            .await
            .map_err(|e| AxAgentError::Gateway(format!("清空表 {} 失败: {}", table_name, e)))?;
        }

        let mut imported = 0usize;
        let skipped = 0usize;
        let mut errored = 0usize;

        for row in rows {
            let obj = match row.as_object() {
                Some(o) => o,
                None => {
                    errored += 1;
                    continue;
                }
            };

            let columns: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            if columns.is_empty() {
                errored += 1;
                continue;
            }

            let placeholders: Vec<&str> = vec!["?"; columns.len()];
            let conflict_clause = match strategy {
                crate::types::RestoreStrategy::Merge => "OR IGNORE",
                _ => "OR REPLACE",
            };

            let sql = format!(
                "INSERT {} INTO \"{}\" ({}) VALUES ({})",
                conflict_clause,
                table_name,
                columns.join(", "),
                placeholders.join(", "),
            );

            let values: Vec<Value> = columns
                .iter()
                .map(|col| json_value_to_sea_value(&obj[*col]))
                .collect();

            match txn
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    &sql,
                    values,
                ))
                .await
            {
                Ok(_) => imported += 1,
                Err(e) => {
                    tracing::warn!("恢复表 {} 的行失败 (列: {:?}): {}", table_name, columns, e);
                    errored += 1;
                }
            }
        }

        report.tables_restored.push(TableRestoreResult {
            table: String::from(*table_name),
            rows_imported: imported,
            rows_skipped: skipped,
            rows_errored: errored,
        });
    }

    txn.commit()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("提交事务失败: {}", e)))?;

    report.total_imported = report.tables_restored.iter().map(|t| t.rows_imported).sum();
    report.total_skipped = report.tables_restored.iter().map(|t| t.rows_skipped).sum();
    report.total_errored = report.tables_restored.iter().map(|t| t.rows_errored).sum();

    Ok(report)
}

/// Clean up old backups exceeding max_count (keeps most recent)
pub async fn cleanup_old_backups(db: &DatabaseConnection, max_count: u32) -> Result<u32> {
    let all = list_backups(db).await?;
    if all.len() <= max_count as usize {
        return Ok(0);
    }

    let to_delete = &all[max_count as usize..];
    let mut deleted = 0u32;
    for backup in to_delete {
        delete_backup(db, &backup.id).await?;
        deleted += 1;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::resolve_backup_dir;
    use std::path::PathBuf;

    #[test]
    fn resolve_backup_dir_defaults_to_axagent_backups_subdir() {
        let axagent_home = PathBuf::from("/Users/test/.axagent");

        assert_eq!(
            resolve_backup_dir(None, &axagent_home),
            axagent_home.join("backups")
        );
        assert_eq!(
            resolve_backup_dir(Some(""), &axagent_home),
            axagent_home.join("backups")
        );
    }

    #[test]
    fn resolve_backup_dir_honors_explicit_absolute_override() {
        let axagent_home = PathBuf::from("/Users/test/.axagent");
        let override_dir = PathBuf::from("/Volumes/external/axagent-backups");

        assert_eq!(
            resolve_backup_dir(Some(override_dir.to_str().unwrap()), &axagent_home),
            override_dir
        );
    }
}
