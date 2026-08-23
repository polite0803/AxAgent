// SPDX-License-Identifier: AGPL-3.0-only

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::skill as skill_err;
use axagent_agent_macro::agent_command;
use axagent_runtime::theme_engine::{Theme, ThemeEngine, ThemeMetadata, XTermTheme};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

pub struct ThemeState {
    pub engine: ThemeEngine,
}

#[agent_command(domain = theme, safety = Safe, call_mode = StateOnly, description = "列出所有主题")]
#[tauri::command]
pub async fn list_themes(
    state: State<'_, Arc<RwLock<ThemeState>>>,
) -> Result<Vec<ThemeMetadata>, String> {
    let state = state.read().await;
    Ok(state.engine.list_themes())
}

#[agent_command(domain = theme, safety = Safe, call_mode = StateInput, description = "获取指定主题")]
#[tauri::command]
pub async fn get_theme(
    state: State<'_, Arc<RwLock<ThemeState>>>,
    name: String,
) -> Result<Theme, String> {
    let state = state.read().await;
    state.engine.get_theme(&name).ok_or_else(|| {
        ErrorResponse::err_with_detail(skill_err::NOT_FOUND, format!("Theme '{}' not found", name))
    })
}

#[agent_command(domain = theme, safety = Safe, call_mode = StateInput, description = "获取 XTerm 主题")]
#[tauri::command]
pub async fn get_xterm_theme(
    state: State<'_, Arc<RwLock<ThemeState>>>,
    name: String,
) -> Result<XTermTheme, String> {
    let state = state.read().await;
    let theme = state.engine.get_theme(&name).ok_or_else(|| {
        ErrorResponse::err_with_detail(skill_err::NOT_FOUND, format!("Theme '{}' not found", name))
    })?;
    Ok(theme.to_xterm_theme())
}

#[agent_command(domain = theme, safety = Caution, call_mode = StateInput, description = "保存主题")]
#[tauri::command]
pub async fn save_theme(
    state: State<'_, Arc<RwLock<ThemeState>>>,
    theme: Theme,
) -> Result<(), String> {
    let state = state.read().await;
    state.engine.save_theme(&theme)
}

#[agent_command(domain = theme, safety = Dangerous, call_mode = StateInput, description = "删除主题")]
#[tauri::command]
pub async fn delete_theme(
    state: State<'_, Arc<RwLock<ThemeState>>>,
    name: String,
) -> Result<(), String> {
    let state = state.read().await;
    state.engine.delete_theme(&name)
}

#[agent_command(domain = theme, safety = Safe, call_mode = StateOnly, description = "加载用户主题")]
#[tauri::command]
pub async fn load_user_themes(
    state: State<'_, Arc<RwLock<ThemeState>>>,
) -> Result<Vec<Theme>, String> {
    let state = state.read().await;
    Ok(state.engine.load_user_themes())
}
