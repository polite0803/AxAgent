// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;

use axagent_runtime::profile_manager::ProfileManager;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[agent_command(domain = profile, safety = Safe, call_mode = StateOnly, description = "列出所有配置档案")]
#[tauri::command]
pub async fn profile_list(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
) -> Result<Vec<axagent_runtime::profile::ProfileInfo>, String> {
    let mgr = manager.lock().await;
    mgr.list().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = profile, safety = Caution, call_mode = StateInput, description = "创建新配置档案")]
#[tauri::command]
pub async fn profile_create(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
    display_name: String,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.create(&name, &display_name).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = profile, safety = Dangerous, call_mode = StateInput, description = "删除指定配置档案")]
#[tauri::command]
pub async fn profile_delete(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.delete(&name).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = profile, safety = Caution, call_mode = StateInput, description = "切换活动配置档案")]
#[tauri::command]
pub async fn profile_switch(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.set_active(&name).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = profile, safety = Safe, call_mode = StateOnly, description = "获取当前活动配置档案")]
#[tauri::command]
pub async fn profile_active(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.active_info().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
