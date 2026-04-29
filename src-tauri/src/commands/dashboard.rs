use axagent_runtime::dashboard_registry::DashboardPluginInfo;
use axagent_runtime::dashboard_plugin::{DashboardPluginAdapter, DashboardPluginManifest};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn dashboard_list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<DashboardPluginInfo>, String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    Ok(registry.list_plugins().await)
}

#[tauri::command]
pub async fn dashboard_register_plugin(
    state: State<'_, AppState>,
    manifest_json: String,
) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    let manifest: DashboardPluginManifest =
        serde_json::from_str(&manifest_json).map_err(|e| e.to_string())?;

    let plugin = DashboardPluginAdapter::new(manifest, |_, _| {
        "".to_string()
    });

    registry.register(Box::new(plugin)).await
}

#[tauri::command]
pub async fn dashboard_unregister_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.unregister(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_enable_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.enable(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_disable_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.disable(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_render_panel(
    state: State<'_, AppState>,
    plugin_id: String,
    panel_id: String,
    props: std::collections::HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.render_panel(&plugin_id, &panel_id, props).await
}

#[tauri::command]
pub async fn dashboard_reload_plugins(state: State<'_, AppState>) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.reload().await
}
