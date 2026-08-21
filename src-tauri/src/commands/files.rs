// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use agent_macro::agent_command;
use axagent_dao::repo::stored_file::StoredFile;
use axagent_storage::file_authorizer::{
    AuthorizationRequest, AuthorizationResponse, PermissionLevel,
};
use serde::Serialize;
use tauri::{Emitter, State};

#[agent_command(domain = files, safety = Caution, call_mode = StateOnly, description = "上传文件")]
#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    data: String,
    file_name: String,
    mime_type: String,
    conversation_id: Option<String>,
) -> Result<StoredFile, String> {
    const MAX_BASE64_SIZE: usize = 100 * 1024 * 1024;
    // base64 编码膨胀约 33%，实际文件大小上限约为 75MB
    const MAX_FILE_SIZE_MB: usize = MAX_BASE64_SIZE * 3 / 4 / (1024 * 1024);
    if data.len() > MAX_BASE64_SIZE {
        return Err(format!("file too large (max {} MB)", MAX_FILE_SIZE_MB));
    }
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    axagent_storage::storage_paths::ensure_documents_dirs()
        .map_err(|e| format!("Failed to ensure documents dirs: {}", e))?;
    let file_store = axagent_storage::file_store::FileStore::new();

    let saved = file_store.save_file(&bytes, &file_name, &mime_type).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let id = axagent_kit::utils::gen_id();
    let stored = axagent_dao::repo::stored_file::create_stored_file(
        state.harness.db(),
        &id,
        &saved.hash,
        &file_name,
        &mime_type,
        saved.size_bytes,
        &saved.storage_path,
        conversation_id.as_deref(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(stored)
}

#[agent_command(domain = files, safety = Safe, call_mode = StateOnly, description = "下载文件")]
#[tauri::command]
pub async fn download_file(state: State<'_, AppState>, file_id: String) -> Result<String, String> {
    use base64::Engine;
    let file = axagent_dao::repo::stored_file::get_stored_file(state.harness.db(), &file_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let storage_path = file.storage_path.clone();
    // 同步磁盘 IO（std::fs::read / mobile 分支 fetch_file）放入 spawn_blocking，
    // 避免阻塞 tokio runtime worker 线程。
    let data = tokio::task::spawn_blocking(move || {
        let file_store = axagent_storage::file_store::FileStore::new();
        file_store.read_file(&storage_path).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))??;

    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

#[agent_command(domain = files, safety = Safe, call_mode = StateOnly, description = "列出文件")]
#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<StoredFile>, String> {
    axagent_dao::repo::stored_file::list_stored_files_by_conversation(
        state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = files, safety = Dangerous, call_mode = StateOnly, description = "删除文件")]
#[tauri::command]
pub async fn delete_file(state: State<'_, AppState>, file_id: String) -> Result<(), String> {
    let file_store = axagent_storage::file_store::FileStore::new();
    super::file_cleanup::delete_attachment_reference(state.harness.db(), &file_store, &file_id)
        .await
}

#[agent_command(domain = files, safety = Caution, call_mode = StateOnly, description = "请求文件访问授权")]
/// 文件访问授权
#[tauri::command]
pub async fn file_authorize(
    state: State<'_, AppState>,
    request: AuthorizationRequest,
) -> Result<AuthorizationResponse, String> {
    let response = state.file_authorizer.request_authorization(request).await;
    Ok(response)
}

#[agent_command(domain = files, safety = Safe, call_mode = StateOnly, description = "检查文件授权状态")]
/// 检查文件是否有授权
#[tauri::command]
pub async fn file_check_authorization(
    state: State<'_, AppState>,
    path: String,
    level: PermissionLevel,
) -> Result<bool, String> {
    Ok(state.file_authorizer.check_authorization(&path, &level).await)
}

#[agent_command(domain = files, safety = Caution, call_mode = StateOnly, description = "撤销文件授权")]
/// 撤销文件授权
#[tauri::command]
pub async fn file_revoke_authorization(
    state: State<'_, AppState>,
    auth_id: String,
) -> Result<(), String> {
    if state.file_authorizer.revoke_authorization(&auth_id).await {
        Ok(())
    } else {
        Err(format!("Authorization not found: {}", auth_id))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FilePermissionRequestEvent {
    pub path: String,
    pub reason: String,
}

#[agent_command(domain = files, safety = Caution, call_mode = StateOnly, description = "请求文件访问权限")]
/// 请求文件访问权限——向后端事件系统发送请求，触发前端弹窗
#[tauri::command]
pub async fn request_file_permission(
    app: tauri::AppHandle,
    path: String,
    reason: String,
) -> Result<(), String> {
    let event = FilePermissionRequestEvent { path, reason };
    app.emit("file-permission-request", event).map_err(|e| format!("Failed to emit event: {}", e))
}
