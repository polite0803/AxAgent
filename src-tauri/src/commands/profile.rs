use axagent_runtime::profile_manager::ProfileManager;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn profile_list(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
) -> Result<Vec<axagent_runtime::profile::ProfileInfo>, String> {
    let mgr = manager.lock().await;
    mgr.list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_create(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
    display_name: String,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.create(&name, &display_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_delete(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.delete(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_switch(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.set_active(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_active(
    manager: State<'_, Arc<Mutex<ProfileManager>>>,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.active_info().await.map_err(|e| e.to_string())
}
