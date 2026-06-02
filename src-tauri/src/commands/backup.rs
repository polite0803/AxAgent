use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::backup as backup_err;
use axagent_core::repo::backup;
use axagent_core::repo::settings::get_settings;
use axagent_core::types::*;
use sea_orm::DatabaseConnection;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupManifest>, String> {
    backup::list_backups(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_backup(
    state: State<'_, AppState>,
    format: String,
) -> Result<BackupManifest, String> {
    let settings = get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    let decoded_backup_dir = axagent_core::path_vars::decode_path_opt(&settings.backup_dir);
    let backup_dir = backup::resolve_backup_dir(decoded_backup_dir.as_deref(), &state.app_data_dir);
    backup::create_backup(&state.sea_db, &format, &backup_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    backup_id: String,
    strategy: Option<String>,
) -> Result<serde_json::Value, String> {
    let manifest = backup::get_backup(&state.sea_db, &backup_id)
        .await
        .map_err(|e| e.to_string())?;

    let backup_path = manifest.file_path.ok_or("Backup file path not available")?;

    match manifest.version.as_str() {
        "sqlite" => {
            let db_path = state
                .db_path
                .strip_prefix("sqlite:")
                .unwrap_or(&state.db_path);
            backup::restore_sqlite_backup(&backup_path, db_path)
                .await
                .map_err(|e| e.to_string())?;
            // 移除残留的 WAL/SHM 文件，防止 SQLite 在重启后回放不兼容的日志
            let _ = std::fs::remove_file(format!("{}-wal", db_path));
            let _ = std::fs::remove_file(format!("{}-shm", db_path));

            // SQLite 恢复后需要重启应用
            app.restart();
        },
        "json" => {
            let strategy = match strategy.as_deref() {
                Some("merge") => axagent_core::types::RestoreStrategy::Merge,
                Some("dry_run") => axagent_core::types::RestoreStrategy::DryRun,
                _ => axagent_core::types::RestoreStrategy::Overwrite,
            };

            let report = backup::restore_json_backup(&state.sea_db, &backup_path, &strategy)
                .await
                .map_err(|e| e.to_string())?;

            Ok(serde_json::to_value(&report).map_err(|e| e.to_string())?)
        },
        other => Err(ErrorResponse::new(backup_err::FORMAT_UNSUPPORTED)
            .with_detail(format!("不支持的备份格式: {}。仅支持 sqlite 和 json 格式。", other))
            .to_string()),
    }
}

#[tauri::command]
pub async fn delete_backup(state: State<'_, AppState>, backup_id: String) -> Result<(), String> {
    backup::delete_backup(&state.sea_db, &backup_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn batch_delete_backups(
    state: State<'_, AppState>,
    backup_ids: Vec<String>,
) -> Result<(), String> {
    backup::batch_delete_backups(&state.sea_db, &backup_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_backup_settings(state: State<'_, AppState>) -> Result<AutoBackupSettings, String> {
    let settings = get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    let decoded_backup_dir = axagent_core::path_vars::decode_path_opt(&settings.backup_dir);
    let default_dir = backup::resolve_backup_dir(None, &state.app_data_dir);
    Ok(AutoBackupSettings {
        enabled: settings.auto_backup_enabled,
        interval_hours: settings.auto_backup_interval_hours,
        max_count: settings.auto_backup_max_count,
        backup_dir: Some(
            decoded_backup_dir.unwrap_or_else(|| default_dir.to_string_lossy().to_string()),
        ),
    })
}

#[tauri::command]
pub async fn update_backup_settings(
    state: State<'_, AppState>,
    backup_settings: AutoBackupSettings,
) -> Result<(), String> {
    let mut settings = get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    settings.auto_backup_enabled = backup_settings.enabled;
    settings.auto_backup_interval_hours = backup_settings.interval_hours;
    settings.auto_backup_max_count = backup_settings.max_count;
    settings.backup_dir = axagent_core::path_vars::encode_path_opt(&backup_settings.backup_dir);

    axagent_core::repo::settings::save_settings(&state.sea_db, &settings)
        .await
        .map_err(|e| e.to_string())?;

    // Restart scheduler with new settings
    restart_auto_backup(
        &state.auto_backup_handle,
        &state.sea_db,
        &state.app_data_dir,
        &backup_settings,
    )
    .await;

    Ok(())
}

/// Start or restart the auto-backup scheduler
async fn restart_auto_backup(
    handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    db: &DatabaseConnection,
    app_data_dir: &Path,
    settings: &AutoBackupSettings,
) {
    let mut guard = handle.lock().await;

    // Stop existing scheduler
    if let Some(h) = guard.take() {
        h.abort();
    }

    if !settings.enabled || settings.interval_hours == 0 {
        return;
    }

    let db = db.clone();
    let app_dir = app_data_dir.to_path_buf();
    let interval_hours = settings.interval_hours;
    let max_count = settings.max_count;
    let interval_secs = interval_hours as u64 * 3600;

    // Calculate initial delay: catch up if overdue
    let initial_delay_secs = match backup::list_backups(&db).await {
        Ok(backups) if !backups.is_empty() => {
            let last_ts = &backups[0].created_at;
            if let Ok(last_time) =
                chrono::NaiveDateTime::parse_from_str(last_ts, "%Y-%m-%d %H:%M:%S")
            {
                let elapsed = chrono::Utc::now()
                    .naive_utc()
                    .signed_duration_since(last_time)
                    .num_seconds()
                    .max(0) as u64;
                interval_secs.saturating_sub(elapsed)
            } else {
                interval_secs
            }
        },
        _ => interval_secs,
    };

    let task = tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(interval_secs);
        // Initial wait (may be shorter if overdue)
        tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
        loop {
            // Read current settings to get backup_dir
            let backup_dir = match get_settings(&db).await {
                Ok(s) => {
                    let decoded = axagent_core::path_vars::decode_path_opt(&s.backup_dir);
                    backup::resolve_backup_dir(decoded.as_deref(), &app_dir)
                },
                Err(_) => backup::resolve_backup_dir(None, &app_dir),
            };

            // Create auto backup (SQLite format for speed)
            if let Err(e) = backup::create_backup(&db, "sqlite", &backup_dir).await {
                tracing::warn!("Auto-backup failed: {}", e);
            } else {
                tracing::info!("Auto-backup created successfully");
                // Cleanup old backups
                if let Err(e) = backup::cleanup_old_backups(&db, max_count).await {
                    tracing::warn!("Auto-backup cleanup failed: {}", e);
                }
            }
            tokio::time::sleep(interval).await;
        }
    });

    *guard = Some(task);
}

#[derive(Debug, serde::Deserialize)]
pub struct UploadBackupToCloudRequest {
    pub backup_id: String,
}

#[tauri::command]
pub async fn upload_backup_to_cloud(
    state: State<'_, AppState>,
    request: UploadBackupToCloudRequest,
) -> Result<(), String> {
    let backend = state
        .sync_engine
        .as_ref()
        .ok_or("云存储未配置，请先在设置中配置 S3 或 WebDAV")?
        .backend
        .clone();

    let manifest = backup::get_backup(&state.sea_db, &request.backup_id)
        .await
        .map_err(|e| e.to_string())?;

    let backup_path = manifest.file_path.ok_or("备份文件路径不可用")?;
    let data = std::fs::read(&backup_path).map_err(|e| format!("读取备份文件失败: {}", e))?;

    let cloud_key = format!(
        "backups/{}/{}",
        chrono::Utc::now().format("%Y%m%d"),
        Path::new(&backup_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "backup.db".to_string())
    );

    backend
        .put(&cloud_key, &data, "application/x-sqlite3")
        .await
        .map_err(|e| format!("上传备份到云端失败: {}", e))?;

    tracing::info!("备份已上传到云端: {}", cloud_key);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct ListCloudBackupsRequest {
    pub prefix: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudBackupEntry {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

#[tauri::command]
pub async fn list_cloud_backups(
    state: State<'_, AppState>,
    request: ListCloudBackupsRequest,
) -> Result<Vec<CloudBackupEntry>, String> {
    let backend = state
        .sync_engine
        .as_ref()
        .ok_or("云存储未配置")?
        .backend
        .clone();

    let prefix = request.prefix.unwrap_or_else(|| "backups/".to_string());
    let result = backend
        .list(&prefix, 100, None)
        .await
        .map_err(|e| format!("列出云端备份失败: {}", e))?;

    let entries = result
        .objects
        .into_iter()
        .filter(|o| !o.key.ends_with('/') && o.size > 0)
        .map(|o| CloudBackupEntry {
            key: o.key,
            size: o.size,
            last_modified: o.last_modified,
            etag: o.etag,
        })
        .collect();

    Ok(entries)
}

#[derive(Debug, serde::Deserialize)]
pub struct DownloadCloudBackupRequest {
    pub cloud_key: String,
}

#[tauri::command]
pub async fn download_cloud_backup(
    state: State<'_, AppState>,
    request: DownloadCloudBackupRequest,
) -> Result<String, String> {
    let backend = state
        .sync_engine
        .as_ref()
        .ok_or("云存储未配置")?
        .backend
        .clone();

    let obj = backend
        .get(&request.cloud_key)
        .await
        .map_err(|e| format!("从云端下载备份失败: {}", e))?;

    let settings = get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    let decoded_backup_dir = axagent_core::path_vars::decode_path_opt(&settings.backup_dir);
    let backup_dir = backup::resolve_backup_dir(decoded_backup_dir.as_deref(), &state.app_data_dir);
    std::fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {}", e))?;

    let file_name = Path::new(&request.cloud_key)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "cloud_backup.db".to_string());
    let local_path = backup_dir.join(&file_name);

    std::fs::write(&local_path, &obj.data).map_err(|e| format!("写入备份文件失败: {}", e))?;

    tracing::info!("云端备份已下载到: {:?}", local_path);
    Ok(local_path.to_string_lossy().to_string())
}
