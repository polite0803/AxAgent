// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use agent_macro::agent_command;
use axagent_harness::types::*;
use serde_json;
use tauri::AppHandle;
use tauri::State;

#[tauri::command]
#[agent_command(
    domain = settings,
    safety = Safe,
    call_mode = StateOnly,
    description = "获取应用设置"
)]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings =
        axagent_dao::repo::settings::get_settings(state.harness.db()).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    settings.backup_dir = axagent_storage::path_vars::decode_path_opt(&settings.backup_dir);
    settings.gateway_ssl_cert_path =
        axagent_storage::path_vars::decode_path_opt(&settings.gateway_ssl_cert_path);
    settings.gateway_ssl_key_path =
        axagent_storage::path_vars::decode_path_opt(&settings.gateway_ssl_key_path);

    // 调试日志：检查返回的设置
    let settings_json = serde_json::to_string_pretty(&settings)
        .unwrap_or_else(|_| String::from("无法序列化 settings"));
    tracing::info!("[get_settings] 返回完整 settings JSON:\n{}", settings_json);

    tracing::info!(
        "[get_settings] 关键模型字段 default_provider_id={:?} default_model_id={:?} title_summary_provider_id={:?} compression_provider_id={:?}",
        settings.default_provider_id,
        settings.default_model_id,
        settings.title_summary_provider_id,
        settings.compression_provider_id,
    );

    Ok(settings)
}

#[tauri::command]
#[agent_command(
    domain = settings,
    safety = Caution,
    call_mode = StateOnly,
    description = "保存应用设置"
)]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<(), String> {
    // 调试日志：检查接收到的设置
    let settings_json = serde_json::to_string_pretty(&settings)
        .unwrap_or_else(|_| String::from("无法序列化 settings"));
    tracing::info!("[save_settings] 接收到完整 settings JSON:\n{}", settings_json);

    tracing::info!(
        "[save_settings] 关键模型字段 default_provider_id={:?} default_model_id={:?} title_summary_provider_id={:?} compression_provider_id={:?}",
        settings.default_provider_id,
        settings.default_model_id,
        settings.title_summary_provider_id,
        settings.compression_provider_id,
    );

    settings.backup_dir = axagent_storage::path_vars::encode_path_opt(&settings.backup_dir);
    settings.gateway_ssl_cert_path =
        axagent_storage::path_vars::encode_path_opt(&settings.gateway_ssl_cert_path);
    settings.gateway_ssl_key_path =
        axagent_storage::path_vars::encode_path_opt(&settings.gateway_ssl_key_path);
    axagent_dao::repo::settings::save_settings(state.harness.db(), &settings).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;

    // 2.7 P1:telemetry_level 变更后同步更新共享级别句柄。
    //
    // `FilteringSink` 通过 `level_handle()` 引用同一 `Arc<RwLock<TelemetryLevel>>`,
    // 这里更新后所有正在运行的 sink 都会立即按新级别过滤事件,无需重建 sink 链。
    // 容错:解析失败时回退到 `Off`,保守保护用户隐私。
    {
        let new_level =
            axagent_telemetry::TelemetryLevel::from_str_or_off(&settings.telemetry_level);
        if let Ok(mut guard) = state.telemetry_level_handle.write() {
            *guard = new_level;
        }
    }

    #[cfg(not(mobile))]
    {
        crate::tray::sync_tray_language(&app, &settings.language).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    }
    #[cfg(mobile)]
    {
        let _ = &app;
        Ok(())
    }
}
