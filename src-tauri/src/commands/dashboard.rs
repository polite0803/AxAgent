use crate::AppState;
use axagent_runtime::dashboard_plugin::{DashboardPluginAdapter, DashboardPluginManifest};
use axagent_runtime::dashboard_registry::DashboardPluginInfo;
use std::path::PathBuf;
use tauri::State;

fn default_plugins_dir() -> PathBuf {
    axagent_core::storage_paths::documents_root().join("dashboard-plugins")
}

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

    let frontend_entry = manifest.frontend_entry.clone();
    let plugin = DashboardPluginAdapter::new(manifest, move |panel_id, props| {
        let panel_info = serde_json::json!({
            "panel_id": panel_id,
            "props": props,
            "frontend_entry": frontend_entry,
        });
        axagent_runtime::dashboard_plugin::RenderOutput::Html {
            content: panel_info.to_string(),
        }
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
    registry
        .render_panel(&plugin_id, &panel_id, props)
        .await
        .map(|r| match r {
            axagent_runtime::dashboard_plugin::RenderOutput::Html { content } => content,
            axagent_runtime::dashboard_plugin::RenderOutput::Data { payload } => {
                payload.to_string()
            },
            axagent_runtime::dashboard_plugin::RenderOutput::Directive(d) => {
                serde_json::to_string(&d).unwrap_or_default()
            },
        })
}

#[tauri::command]
pub async fn dashboard_reload_plugins(state: State<'_, AppState>) -> Result<(), String> {
    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.reload().await
}

#[tauri::command]
pub async fn dashboard_open_plugins_folder(app: tauri::AppHandle) -> Result<(), String> {
    let dir = default_plugins_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create plugins dir: {}", e))?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_install_plugin(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<(), String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    let plugins_dir = default_plugins_dir();
    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Failed to create plugins dir: {}", e))?;

    let plugin_dir_name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin")
        .to_string();
    let dest_dir = plugins_dir.join(&plugin_dir_name);

    if source.is_dir() {
        if source.join("manifest.json").exists() {
            let dest = dest_dir;
            copy_dir_recursive(&source, &dest)?;
        } else {
            return Err("Selected directory does not contain a manifest.json".to_string());
        }
    } else if source.extension().and_then(|e| e.to_str()) == Some("json") {
        let manifest_str = std::fs::read_to_string(&source)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest: DashboardPluginManifest =
            serde_json::from_str(&manifest_str).map_err(|e| format!("Invalid manifest: {}", e))?;
        let dest_dir = plugins_dir.join(&manifest.id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;
        std::fs::copy(&source, dest_dir.join("manifest.json"))
            .map_err(|e| format!("Failed to copy manifest: {}", e))?;
    } else {
        return Err("Please select a directory containing manifest.json or a manifest.json file"
            .to_string());
    }

    let registry = state
        .dashboard_registry
        .as_ref()
        .ok_or("Dashboard registry not initialized")?;
    registry.reload().await
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir: {}", e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}
