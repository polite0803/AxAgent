use axagent_runtime::theme_engine::{Theme, ThemeEngine, ThemeMetadata, XTermTheme};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

pub struct ThemeState {
    pub engine: ThemeEngine,
}

#[tauri::command]
pub async fn list_themes(state: State<'_, Arc<RwLock<ThemeState>>>) -> Result<Vec<ThemeMetadata>, String> {
    let state = state.read().await;
    Ok(state.engine.list_themes())
}

#[tauri::command]
pub async fn get_theme(state: State<'_, Arc<RwLock<ThemeState>>>, name: String) -> Result<Theme, String> {
    let state = state.read().await;
    state
        .engine
        .get_theme(&name)
        .ok_or_else(|| format!("Theme '{}' not found", name))
}

#[tauri::command]
pub async fn get_xterm_theme(state: State<'_, Arc<RwLock<ThemeState>>>, name: String) -> Result<XTermTheme, String> {
    let state = state.read().await;
    let theme = state
        .engine
        .get_theme(&name)
        .ok_or_else(|| format!("Theme '{}' not found", name))?;
    Ok(theme.to_xterm_theme())
}

#[tauri::command]
pub async fn save_theme(state: State<'_, Arc<RwLock<ThemeState>>>, theme: Theme) -> Result<(), String> {
    let state = state.read().await;
    state.engine.save_theme(&theme)
}

#[tauri::command]
pub async fn delete_theme(state: State<'_, Arc<RwLock<ThemeState>>>, name: String) -> Result<(), String> {
    let state = state.read().await;
    state.engine.delete_theme(&name)
}

#[tauri::command]
pub async fn load_user_themes(state: State<'_, Arc<RwLock<ThemeState>>>) -> Result<Vec<Theme>, String> {
    let state = state.read().await;
    Ok(state.engine.load_user_themes())
}
