// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_harness::types::*;
use tauri::AppHandle;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = axagent_core::repo::settings::get_settings(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    settings.backup_dir = axagent_core::path_vars::decode_path_opt(&settings.backup_dir);
    settings.gateway_ssl_cert_path =
        axagent_core::path_vars::decode_path_opt(&settings.gateway_ssl_cert_path);
    settings.gateway_ssl_key_path =
        axagent_core::path_vars::decode_path_opt(&settings.gateway_ssl_key_path);
    Ok(settings)
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<(), String> {
    settings.backup_dir = axagent_core::path_vars::encode_path_opt(&settings.backup_dir);
    settings.gateway_ssl_cert_path =
        axagent_core::path_vars::encode_path_opt(&settings.gateway_ssl_cert_path);
    settings.gateway_ssl_key_path =
        axagent_core::path_vars::encode_path_opt(&settings.gateway_ssl_key_path);
    axagent_core::repo::settings::save_settings(state.harness.db(), &settings)
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(not(mobile))]
    {
        crate::tray::sync_tray_language(&app, &settings.language).map_err(|e| e.to_string())
    }
    #[cfg(mobile)]
    {
        Ok(())
    }
}
