use crate::AppState;
use axagent_core::file_authorizer::{AuthorizationRequest, AuthorizationResponse, PermissionLevel};
use axagent_core::repo::stored_file::StoredFile;
use serde::Serialize;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    data: String,
    file_name: String,
    mime_type: String,
    conversation_id: Option<String>,
) -> Result<StoredFile, String> {
    const MAX_BASE64_SIZE: usize = 100 * 1024 * 1024;
    if data.len() > MAX_BASE64_SIZE {
        return Err(format!("file too large (max {} MB)", MAX_BASE64_SIZE / (1024 * 1024)));
    }
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    axagent_core::storage_paths::ensure_documents_dirs()
        .map_err(|e| format!("Failed to ensure documents dirs: {}", e))?;
    let file_store = axagent_core::file_store::FileStore::new();

    let saved = file_store
        .save_file(&bytes, &file_name, &mime_type)
        .map_err(|e| e.to_string())?;

    let id = axagent_core::utils::gen_id();
    let stored = axagent_core::repo::stored_file::create_stored_file(
        &state.sea_db,
        &id,
        &saved.hash,
        &file_name,
        &mime_type,
        saved.size_bytes,
        &saved.storage_path,
        conversation_id.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(stored)
}

#[tauri::command]
pub async fn download_file(state: State<'_, AppState>, file_id: String) -> Result<String, String> {
    use base64::Engine;
    let file = axagent_core::repo::stored_file::get_stored_file(&state.sea_db, &file_id)
        .await
        .map_err(|e| e.to_string())?;

    let file_store = axagent_core::file_store::FileStore::new();

    let data = file_store
        .read_file(&file.storage_path)
        .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<StoredFile>, String> {
    axagent_core::repo::stored_file::list_stored_files_by_conversation(
        &state.sea_db,
        &conversation_id,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_file(state: State<'_, AppState>, file_id: String) -> Result<(), String> {
    let file_store = axagent_core::file_store::FileStore::new();
    super::file_cleanup::delete_attachment_reference(&state.sea_db, &file_store, &file_id).await
}

/// 文件访问授权
#[tauri::command]
pub async fn file_authorize(
    state: State<'_, AppState>,
    request: AuthorizationRequest,
) -> Result<AuthorizationResponse, String> {
    let response = state.file_authorizer.request_authorization(request);
    Ok(response)
}

/// 检查文件是否有授权
#[tauri::command]
pub async fn file_check_authorization(
    state: State<'_, AppState>,
    path: String,
    level: PermissionLevel,
) -> Result<bool, String> {
    Ok(state.file_authorizer.check_authorization(&path, &level))
}

/// 撤销文件授权
#[tauri::command]
pub async fn file_revoke_authorization(
    state: State<'_, AppState>,
    auth_id: String,
) -> Result<(), String> {
    if state.file_authorizer.revoke_authorization(&auth_id) {
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

/// 请求文件访问权限——向后端事件系统发送请求，触发前端弹窗
#[tauri::command]
pub async fn request_file_permission(
    app: tauri::AppHandle,
    path: String,
    reason: String,
) -> Result<(), String> {
    let event = FilePermissionRequestEvent { path, reason };
    app.emit("file-permission-request", event)
        .map_err(|e| format!("Failed to emit event: {}", e))
}
