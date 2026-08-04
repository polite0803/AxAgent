// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorCategory;
use crate::commands::error::ErrorResponse;
use agent_macro::agent_command;
use axagent_search::model_downloader::{LocalModelInfo, ModelDownloader, PresetModel};
use tauri::State;

#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "列出本地模型")]
#[tauri::command]
pub async fn list_local_models() -> Result<Vec<LocalModelInfo>, String> {
    let dl = ModelDownloader::new();
    Ok(dl.list_all_models())
}

#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "下载模型")]
#[tauri::command]
pub async fn download_model(filename: String) -> Result<(), String> {
    axagent_search::inference::download_and_load_model(&filename)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

#[agent_command(domain = model, safety = Dangerous, call_mode = StateInput, description = "删除模型")]
#[tauri::command]
pub async fn delete_model(filename: String) -> Result<(), String> {
    axagent_search::inference::delete_and_unload_model(&filename)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "获取预设模型列表")]
#[tauri::command]
pub async fn get_preset_models() -> Result<Vec<PresetModel>, String> {
    Ok(ModelDownloader::preset_models())
}

/// 设置是否在下载模型后自动加载到内存（同时持久化到 DB settings）。
///
/// `enabled` — true: 下载后自动加载; false: 仅下载,不加载（节省内存）。
/// 运行时立即生效，无需重启。
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "设置自动加载模型")]
#[tauri::command]
pub async fn set_auto_load_models(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    // 更新运行时原子标志（立即生效）
    axagent_search::inference::set_auto_load_models(enabled);

    // 持久化到 DB settings
    axagent_dao::repo::settings::set_setting(
        state.harness.db(),
        "auto_load_models",
        if enabled { "true" } else { "false" },
    )
    .await
    .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;

    Ok(())
}
