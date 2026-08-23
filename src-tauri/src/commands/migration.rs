// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_migration::{DetectedPlatform, MigrationReport};
use serde::Deserialize;
use std::path::Path;
use tauri::State;

#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "检测可迁移平台")]
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

#[agent_command(domain = system, safety = Safe, call_mode = StateInput, description = "预览迁移项目")]
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

#[agent_command(domain = system, safety = Caution, call_mode = StateInput, description = "执行数据迁移")]
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

#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "列出迁移备份")]
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

#[agent_command(domain = system, safety = Caution, call_mode = StateInput, description = "回滚迁移")]
#[tauri::command]
pub async fn migration_rollback(
    payload: MigrationRollbackPayload,
    _state: State<'_, AppState>,
) -> Result<MigrationReport, String> {
    let backup_path = Path::new(&payload.backup_id);
    axagent_migration::rollback(backup_path).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
