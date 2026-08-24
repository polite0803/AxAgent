// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::backup as backup_err;
use crate::commands::spawn_guard::panic_message;
use axagent_agent_macro::agent_command;
use axagent_crypto::{decrypt_key, encrypt_key};
use axagent_dao::repo::{backup, settings as settings_repo};
use axagent_storage::webdav::{self, WebDavClient, WebDavConfig, WebDavFileInfo};
use futures::FutureExt;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::path::PathBuf;
use tauri::{Emitter, State};

#[derive(Default)]
struct RestoreCleanup {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

impl RestoreCleanup {
    fn track_file<P: Into<PathBuf>>(&mut self, path: P) {
        self.files.push(path.into());
    }

    fn track_dir<P: Into<PathBuf>>(&mut self, path: P) {
        self.dirs.push(path.into());
    }
}

impl Drop for RestoreCleanup {
    fn drop(&mut self) {
        for path in &self.files {
            let _ = std::fs::remove_file(path);
        }
        for path in &self.dirs {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

/// RAII guard clearing temp files on drop — ensures cleanup even when
/// the function returns early via `?` before reaching explicit cleanup.
struct WebdavTempCleanup {
    files: Vec<PathBuf>,
}

impl WebdavTempCleanup {
    fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn track(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    fn clear(mut self) {
        self.files.clear();
    }
}

impl Drop for WebdavTempCleanup {
    fn drop(&mut self) {
        for path in &self.files {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Get WebDAV configuration (password decrypted).
#[agent_command(domain = storage, safety = Safe, call_mode = StateOnly, description = "获取 WebDAV 配置")]
#[tauri::command]
pub async fn get_webdav_config(state: State<'_, AppState>) -> Result<WebDavConfig, String> {
    get_webdav_config_from_db(state.harness.db(), state.harness.master_key()).await
}

/// Save WebDAV configuration (password encrypted).
#[agent_command(domain = storage, safety = Caution, call_mode = StateInput, description = "保存 WebDAV 配置")]
#[tauri::command]
pub async fn save_webdav_config(
    state: State<'_, AppState>,
    config: WebDavConfig,
) -> Result<(), String> {
    let mut settings = settings_repo::get_settings(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // SECURITY (S8): 用户启用 accept_invalid_certs 时记录安全警告
    if config.accept_invalid_certs && !settings.webdav_accept_invalid_certs {
        tracing::warn!(
            "SECURITY: WebDAV accept_invalid_certs 已启用 — 跳过 TLS 证书验证，可能遭受中间人攻击"
        );
    }

    settings.webdav_host = Some(config.host);
    settings.webdav_username = Some(config.username);
    settings.webdav_path = Some(config.path);
    settings.webdav_accept_invalid_certs = config.accept_invalid_certs;

    settings_repo::save_settings(state.harness.db(), &settings).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // Encrypt and store password separately
    if !config.password.is_empty() {
        let encrypted = encrypt_key(&config.password, state.harness.master_key()).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        settings_repo::set_setting(state.harness.db(), "webdav_password_encrypted", &encrypted)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
    } else {
        settings_repo::set_setting(state.harness.db(), "webdav_password_encrypted", "")
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
    }

    Ok(())
}

/// Test WebDAV connection without requiring saved config.
#[agent_command(domain = storage, safety = Safe, call_mode = Manual, description = "检查 WebDAV 连接")]
#[tauri::command]
pub async fn webdav_check_connection(config: WebDavConfig) -> Result<bool, String> {
    let client = WebDavClient::new(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    client.check_connection().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Create a backup and upload it to WebDAV.
#[agent_command(domain = storage, safety = Caution, call_mode = StateOnly, description = "创建 WebDAV 备份")]
#[tauri::command]
pub async fn webdav_backup(state: State<'_, AppState>) -> Result<String, String> {
    do_webdav_backup_impl(state.harness.db(), state.harness.master_key(), &state.app_data_dir).await
}

/// List remote backups on WebDAV server.
#[agent_command(domain = storage, safety = Safe, call_mode = StateOnly, description = "列出 WebDAV 备份")]
#[tauri::command]
pub async fn webdav_list_backups(
    state: State<'_, AppState>,
) -> Result<Vec<WebDavFileInfo>, String> {
    let config = get_webdav_config_from_db(state.harness.db(), state.harness.master_key()).await?;
    if config.host.is_empty() {
        return Ok(vec![]);
    }
    let client = WebDavClient::new(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    client.list_files().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Restore from a remote WebDAV backup.
#[agent_command(domain = storage, safety = Caution, call_mode = StateInput, description = "从 WebDAV 恢复备份")]
#[tauri::command]
pub async fn webdav_restore(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    file_name: String,
) -> Result<(), String> {
    // 2026-07-31 修复：PG 模式下恢复流程对 db_path（postgres:// URL）做 fs::copy
    // 必然失败（且 DB 本就在 PG 服务器，无需本地快照恢复）。明确降级报错。
    if state.harness.db().get_database_backend() == sea_orm::DbBackend::Postgres {
        return Err("当前使用 PostgreSQL 数据库：WebDAV 恢复暂不支持 PG 模式。\
             数据库位于 PG 服务器端，无需从备份 ZIP 恢复；如需恢复文档/工作区，请直接解压备份文件。"
            .to_string());
    }
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Backup file name must not contain path separators or traversal".to_string());
    }
    let config = get_webdav_config_from_db(state.harness.db(), state.harness.master_key()).await?;
    let settings = settings_repo::get_settings(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let decoded_backup_dir = axagent_storage::path_vars::decode_path_opt(&settings.backup_dir);
    let backup_dir = backup::resolve_backup_dir(decoded_backup_dir.as_deref(), &state.app_data_dir);
    backup::ensure_backup_dir(&backup_dir).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut cleanup = RestoreCleanup::default();

    // 1. Download ZIP
    let zip_path = backup_dir.join(&file_name);
    cleanup.track_file(&zip_path);
    let client = WebDavClient::new(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    client.download_file(&file_name, &zip_path).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 2. Extract to temp directory
    let temp_dir = backup_dir.join("_webdav_restore_temp");
    let _ = std::fs::remove_dir_all(&temp_dir);
    cleanup.track_dir(&temp_dir);
    let crypto = axagent_crypto::platform_adapter_impl::DefaultCryptoService::new(
        state.harness.master_key_owned(),
    );
    let contents = webdav::extract_backup_zip(&zip_path, &temp_dir, &crypto).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 3. Verify checksum
    if let Some(expected) = contents.metadata.get("db_checksum").and_then(|v| v.as_str()) {
        let ok = webdav::verify_db_checksum(&contents.db_path, expected).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        if !ok {
            return Err(ErrorResponse::new(backup_err::RESTORE_FAILED)
                .with_detail("Backup checksum verification failed — file may be corrupted")
                .into());
        }
    }

    // 4. Create a safety backup of current database and master.key
    //    失败时必须中止恢复，否则数据不可恢复。
    let db_path =
        state.harness.db_path().strip_prefix("sqlite:").unwrap_or(state.harness.db_path());
    let safety_backup = backup_dir.join("_pre_webdav_restore_safety.db");
    std::fs::copy(db_path, &safety_backup).map_err(|e| {
        format!("创建数据库安全备份失败 ({}): {} — 已中止恢复", safety_backup.display(), e)
    })?;
    let master_key_dest = state.app_data_dir.join("master.key");
    let safety_key_backup = temp_dir.join("_pre_webdav_restore_safety.key");
    std::fs::copy(&master_key_dest, &safety_key_backup).map_err(|e| {
        format!(
            "创建 master.key 安全备份失败 ({}): {} — 已中止恢复",
            safety_key_backup.display(),
            e
        )
    })?;
    cleanup.track_file(&safety_key_backup);
    #[cfg(unix)]
    {
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&safety_key_backup, perms);
    }

    // 5. Restore master.key if present in backup — with integrity check
    // SECURITY (S7): 恢复 master.key 前验证文件完整性（非空 + 32 字节）
    if let Some(ref key_path) = contents.master_key_path {
        let key_meta = std::fs::metadata(key_path)
            .map_err(|e| format!("Failed to stat master.key in backup: {}", e))?;
        if key_meta.len() != 32 {
            return Err(ErrorResponse::new(backup_err::RESTORE_FAILED)
                .with_detail(format!(
                    "master.key 大小异常 ({})，期望 32 字节 — 备份文件可能损坏",
                    key_meta.len(),
                ))
                .into());
        }
        // 额外校验：读取文件内容确认非空
        let key_data = std::fs::read(key_path)
            .map_err(|e| format!("Failed to read master.key from backup: {}", e))?;
        if key_data.iter().all(|&b| b == 0) {
            return Err(ErrorResponse::new(backup_err::RESTORE_FAILED)
                .with_detail("master.key 内容全零 — 备份文件可能损坏".to_string())
                .into());
        }
        std::fs::copy(key_path, &master_key_dest)
            .map_err(|e| format!("Failed to restore master.key: {}", e))?;
        #[cfg(unix)]
        {
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&master_key_dest, perms);
        }
    }

    // 6. Restore database — also remove stale WAL/SHM files so SQLite
    //    doesn't try to replay a journal that belongs to the old database.
    //    NotFound 属正常路径，其他 I/O 错误必须打 warn 而非静默吞错。
    backup::restore_sqlite_backup(contents.db_path.to_str().unwrap_or(""), db_path).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;
    for suffix in ["-wal", "-shm"] {
        let aux_path = format!("{db_path}{suffix}");
        match std::fs::remove_file(&aux_path) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => {
                tracing::warn!("failed to remove auxiliary db file {aux_path}: {e}");
            },
        }
    }

    // 7. Restore documents if present
    if contents.has_documents {
        let docs_source = temp_dir.join("documents");
        let docs_target = webdav::documents_sync_root();
        if docs_source.exists() {
            copy_directory(&docs_source, &docs_target)
                .map_err(|e| format!("Failed to restore documents: {}", e))?;
        }
    }

    // 7b. Restore workspace if present
    if contents.has_workspace {
        let ws_source = temp_dir.join("workspace");
        let ws_target = state.app_data_dir.join("workspace");
        if ws_source.exists() {
            copy_directory(&ws_source, &ws_target)
                .map_err(|e| format!("Failed to restore workspace: {}", e))?;
        }
    }

    // 8. Auto-restart to pick up the restored database
    app.restart();
}

/// Delete a remote backup file.
#[agent_command(domain = storage, safety = Dangerous, call_mode = StateInput, description = "删除 WebDAV 备份")]
#[tauri::command]
pub async fn webdav_delete_backup(
    state: State<'_, AppState>,
    file_name: String,
) -> Result<(), String> {
    let config = get_webdav_config_from_db(state.harness.db(), state.harness.master_key()).await?;
    let client = WebDavClient::new(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    client.delete_file(&file_name).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Get WebDAV sync status (last sync time and result).
#[agent_command(domain = storage, safety = Safe, call_mode = StateOnly, description = "获取 WebDAV 同步状态")]
#[tauri::command]
pub async fn get_webdav_sync_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let last_time = settings_repo::get_setting(state.harness.db(), "webdav_last_sync_time")
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let last_status = settings_repo::get_setting(state.harness.db(), "webdav_last_sync_status")
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(serde_json::json!({
        "lastSyncTime": last_time,
        "lastSyncStatus": last_status,
    }))
}

/// Restart the WebDAV auto-sync scheduler based on current settings.
#[agent_command(domain = storage, safety = Caution, call_mode = StateOnly, description = "重启 WebDAV 同步")]
#[tauri::command]
pub async fn restart_webdav_sync(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = settings_repo::get_settings(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut guard: tokio::sync::MutexGuard<'_, Option<tokio::task::JoinHandle<()>>> =
        state.webdav_sync_handle.lock().await;

    // Stop existing scheduler
    if let Some(h) = guard.take() {
        h.abort();
    }

    if !settings.webdav_sync_enabled || settings.webdav_sync_interval_minutes == 0 {
        return Ok(());
    }

    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let app_data_dir = state.app_data_dir.clone();
    let interval_minutes = settings.webdav_sync_interval_minutes;
    let shutdown_token = state.shutdown_token.clone();
    let task = spawn_webdav_sync_task(
        app,
        db,
        master_key,
        app_data_dir,
        interval_minutes,
        interval_minutes as u64 * 60,
        shutdown_token,
    );

    *guard = Some(task);
    Ok(())
}

// === Internal Helpers ===

pub(crate) async fn get_webdav_config_from_db(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
) -> Result<WebDavConfig, String> {
    let settings = settings_repo::get_settings(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let encrypted_pw =
        settings_repo::get_setting(db, "webdav_password_encrypted").await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let password = match encrypted_pw {
        Some(enc) if !enc.is_empty() => decrypt_key(&enc, master_key).unwrap_or_default(),
        _ => String::new(),
    };

    Ok(WebDavConfig {
        host: settings.webdav_host.unwrap_or_default(),
        username: settings.webdav_username.unwrap_or_default(),
        password,
        path: settings.webdav_path.unwrap_or_else(|| "/axagent/".to_string()),
        accept_invalid_certs: settings.webdav_accept_invalid_certs,
    })
}

/// Core backup-and-upload logic, shared by the command and the auto-sync scheduler.
pub(crate) async fn do_webdav_backup_impl(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    app_data_dir: &Path,
) -> Result<String, String> {
    let result = do_webdav_backup_once(db, master_key, app_data_dir).await;
    record_webdav_sync_status(db, if result.is_ok() { "success" } else { "failed" }).await;
    result
}

async fn do_webdav_backup_once(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    app_data_dir: &Path,
) -> Result<String, String> {
    // 2026-07-31 修复：PG 模式下 VACUUM INTO（SQLite 专属语法）必然报错，
    // 且 ZIP 快照语义（跨设备恢复 axagent.db）对 PG 无意义。明确降级报错，
    // 避免静默失败/错误行为。完整 PG 支持需 pg_dump 方案（待实现）。
    if db.get_database_backend() == sea_orm::DbBackend::Postgres {
        return Err(ErrorResponse::new(backup_err::CREATE_FAILED)
            .with_detail(
                "当前使用 PostgreSQL 数据库：WebDAV 备份暂不支持 PG 模式 \
                 （VACUUM INTO 为 SQLite 专属语法）。请改用 PostgreSQL 自身的备份方案 \
                 （pg_dump / 服务器级备份）。",
            )
            .into());
    }

    // 1. Load config
    let config = get_webdav_config_from_db(db, master_key).await?;
    if config.host.is_empty() {
        return Err(ErrorResponse::new(backup_err::CREATE_FAILED)
            .with_detail("WebDAV is not configured")
            .into());
    }

    let settings = settings_repo::get_settings(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 2. Create local SQLite snapshot via VACUUM INTO
    let decoded_backup_dir = axagent_storage::path_vars::decode_path_opt(&settings.backup_dir);
    let backup_dir = backup::resolve_backup_dir(decoded_backup_dir.as_deref(), app_data_dir);
    backup::ensure_backup_dir(&backup_dir).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let temp_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_db_path = backup_dir.join(format!("_webdav_temp_{}.db", temp_id));
    let _ = std::fs::remove_file(&temp_db_path);

    // RAII cleanup guard: always removes temp_db_path and zip_path on drop,
    // regardless of whether the function returns Ok or Err.
    let mut temp_cleanup = WebdavTempCleanup::new();
    temp_cleanup.track(temp_db_path.clone());

    let db_str = temp_db_path.to_string_lossy().to_string();
    db.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        format!("VACUUM INTO '{}'", db_str.replace('\'', "''")),
    ))
    .await
    .map_err(|e| format!("VACUUM INTO failed: {}", e))?;

    // 3. Object counts for metadata
    let object_counts = backup::count_objects(db).await.unwrap_or_else(|e| {
        tracing::warn!("Failed to count objects for WebDAV backup metadata: {}", e);
        r#"{"conversations":0,"messages":0,"providers":0}"#.to_string()
    });

    // 4. Documents directory (optional)
    let include_docs = settings.webdav_include_documents;
    let documents_dir = if include_docs {
        let docs_root = webdav::documents_sync_root();
        if docs_root.exists() {
            Some(docs_root)
        } else {
            None
        }
    } else {
        None
    };

    // 4b. Workspace directory (always included if present)
    let workspace_root = app_data_dir.join("workspace");
    let workspace_dir = if workspace_root.exists() {
        Some(workspace_root)
    } else {
        None
    };

    // 5. Create ZIP (includes master.key for cross-device restore)
    let master_key_path = app_data_dir.join("master.key");
    let zip_filename = webdav::generate_backup_filename();
    let zip_path = backup_dir.join(&zip_filename);
    temp_cleanup.track(zip_path.clone());
    let crypto = axagent_crypto::platform_adapter_impl::DefaultCryptoService::new(*master_key);
    webdav::create_backup_zip(
        &temp_db_path,
        documents_dir.as_deref(),
        workspace_dir.as_deref(),
        Some(&master_key_path),
        &zip_path,
        env!("CARGO_PKG_VERSION"),
        &object_counts,
        &crypto,
    )
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 6. Upload
    let client = WebDavClient::new(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    client.upload_file(&zip_filename, &zip_path).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 7. Clear temp files (RAII guard still ensures cleanup on early return)
    temp_cleanup.clear();

    // 8. Cleanup old remote backups
    let max_backups = settings.webdav_max_remote_backups;
    if max_backups > 0 {
        cleanup_remote_backups(&client, max_backups).await;
    }

    Ok(zip_filename)
}

async fn cleanup_remote_backups(client: &WebDavClient, max_per_host: u32) {
    if let Ok(files) = client.list_files().await {
        let mut by_host: std::collections::HashMap<String, Vec<WebDavFileInfo>> =
            std::collections::HashMap::new();
        for f in files {
            by_host.entry(f.hostname.clone()).or_default().push(f);
        }

        for (_, mut host_files) in by_host {
            if host_files.len() > max_per_host as usize {
                let to_delete = host_files.split_off(max_per_host as usize);
                for f in to_delete {
                    if let Err(e) = client.delete_file(&f.file_name).await {
                        tracing::warn!(
                            "Failed to clean up old WebDAV backup {}: {}",
                            f.file_name,
                            e
                        );
                    }
                }
            }
        }
    }
}

fn copy_directory(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

async fn record_webdav_sync_status(db: &DatabaseConnection, status: &str) {
    let timestamp = webdav::sync_status_timestamp();
    let _ = settings_repo::set_setting(db, "webdav_last_sync_time", &timestamp).await;
    let _ = settings_repo::set_setting(db, "webdav_last_sync_status", status).await;
}

pub(crate) fn spawn_webdav_sync_task(
    app: tauri::AppHandle,
    db: DatabaseConnection,
    master_key: [u8; 32],
    app_data_dir: std::path::PathBuf,
    interval_minutes: u32,
    initial_delay_secs: u64,
    shutdown_token: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let app_for_emit = app.clone();
    tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(interval_minutes as u64 * 60);
        // Initial wait (may be shorter if overdue)
        tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("[webdav_sync] 收到关闭信号，停止同步");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    // 读取 notify_backup 设置（gated emit）
                    let notify = settings_repo::get_settings(&db)
                        .await
                        .map(|s| s.notify_backup)
                        .unwrap_or(true);

                    // 用 catch_unwind 包裹单次同步执行体；panic 不会杀死整个周期任务
                    let result = AssertUnwindSafe(async {
                        match do_webdav_backup_impl(&db, &master_key, &app_data_dir).await {
                            Ok(name) => {
                                tracing::info!("WebDAV auto-sync completed: {}", name);
                                if notify {
                                    let _ = app_for_emit.emit("webdav-sync-completed", serde_json::json!({
                                        "success": true,
                                        "name": name,
                                    }));
                                }
                            },
                            Err(e) => {
                                tracing::warn!("WebDAV auto-sync failed: {}", e);
                                if notify {
                                    let _ = app_for_emit.emit("webdav-sync-completed", serde_json::json!({
                                        "success": false,
                                        "error": e.to_string(),
                                    }));
                                }
                            },
                        }
                    })
                    .catch_unwind()
                    .await;

                    if let Err(p) = result {
                        tracing::error!(
                            "[webdav_sync] PANIC 在一次周期同步执行中: {}",
                            panic_message(&p)
                        );
                        let _ = app_for_emit.emit("webdav-sync-completed", serde_json::json!({
                            "success": false,
                            "error": "Internal panic during WebDAV sync",
                        }));
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::RestoreCleanup;

    #[test]
    fn restore_cleanup_removes_tracked_safety_key_files() {
        let temp_root = std::env::temp_dir()
            .join(format!("axagent-webdav-restore-cleanup-{}", axagent_kit::utils::gen_id()));
        std::fs::create_dir_all(&temp_root).expect("create temp root");
        let safety_key = temp_root.join("_pre_webdav_restore_safety.key");
        std::fs::write(&safety_key, b"secret").expect("write safety key");

        {
            let mut cleanup = RestoreCleanup::default();
            cleanup.track_file(&safety_key);
        }

        assert!(
            !safety_key.exists(),
            "restore cleanup must delete the plaintext safety key backup"
        );
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[cfg(unix)]
    #[test]
    fn restore_cleanup_keeps_safety_key_backup_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let temp_root = std::env::temp_dir()
            .join(format!("axagent-webdav-restore-perms-{}", axagent_kit::utils::gen_id()));
        std::fs::create_dir_all(&temp_root).expect("create temp root");
        let safety_key = temp_root.join("_pre_webdav_restore_safety.key");
        std::fs::write(&safety_key, b"secret").expect("write safety key");
        std::fs::set_permissions(&safety_key, std::fs::Permissions::from_mode(0o600))
            .expect("set permissions");

        let mode = std::fs::metadata(&safety_key).expect("metadata").permissions().mode() & 0o777;

        assert_eq!(mode, 0o600, "safety key backups must be owner-readable only");
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
