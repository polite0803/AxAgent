use crate::AppState;
use axagent_migration::{DetectedPlatform, MigrationReport};
use serde::Deserialize;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn migration_detect(
    _state: State<'_, AppState>,
) -> Result<Vec<DetectedPlatform>, String> {
    Ok(axagent_migration::detect_platforms())
}

#[derive(Debug, Deserialize)]
pub struct MigrationPreviewPayload {
    pub platform: String,
}

#[tauri::command]
pub async fn migration_preview(
    payload: MigrationPreviewPayload,
    _state: State<'_, AppState>,
) -> Result<Vec<axagent_migration::MigrationItem>, String> {
    match payload.platform.as_str() {
        "openclaw" => Ok(axagent_migration::preview_openclaw()),
        "hermes" => Ok(axagent_migration::preview_hermes()),
        _ => Err(format!("Unknown platform: {}", payload.platform)),
    }
}

#[derive(Debug, Deserialize)]
pub struct MigrationExecutePayload {
    pub platform: String,
    #[serde(default)]
    pub overwrite: bool,
}

#[tauri::command]
pub async fn migration_execute(
    payload: MigrationExecutePayload,
    _state: State<'_, AppState>,
) -> Result<MigrationReport, String> {
    match payload.platform.as_str() {
        "openclaw" => Ok(axagent_migration::migrate_openclaw(payload.overwrite)),
        "hermes" => Ok(axagent_migration::migrate_hermes(payload.overwrite)),
        _ => Err(format!("Unknown platform: {}", payload.platform)),
    }
}

#[tauri::command]
pub async fn migration_list_backups(
    _state: State<'_, AppState>,
) -> Result<Vec<axagent_migration::BackupInfo>, String> {
    Ok(axagent_migration::list_backups())
}

#[derive(Debug, Deserialize)]
pub struct MigrationRollbackPayload {
    pub backup_id: String,
}

#[tauri::command]
pub async fn migration_rollback(
    payload: MigrationRollbackPayload,
    _state: State<'_, AppState>,
) -> Result<MigrationReport, String> {
    let backup_path = Path::new(&payload.backup_id);
    axagent_migration::rollback(backup_path).map_err(|e| e.to_string())
}
