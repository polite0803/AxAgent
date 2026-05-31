use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_active: bool,
}

#[tauri::command]
pub async fn personality_list(_state: State<'_, AppState>) -> Result<Vec<PersonalityInfo>, String> {
    let active = axagent_agent::personality::PersonalityManager::get_active()
        .ok()
        .flatten()
        .map(|p| p.name);
    let personalities =
        axagent_agent::personality::PersonalityManager::list().map_err(|e| e.to_string())?;
    Ok(personalities
        .into_iter()
        .map(|name| {
            let is_active = active.as_ref() == Some(&name);
            let info = axagent_agent::personality::PersonalityManager::load(&name)
                .ok()
                .map(|p| PersonalityInfo {
                    name: p.name,
                    version: p.version,
                    description: p.description,
                    is_active,
                })
                .unwrap_or_else(|| PersonalityInfo {
                    name,
                    version: "?".to_string(),
                    description: String::new(),
                    is_active,
                });
            info
        })
        .collect())
}

#[tauri::command]
pub async fn personality_get(
    name: String,
    _state: State<'_, AppState>,
) -> Result<axagent_agent::personality::Personality, String> {
    axagent_agent::personality::PersonalityManager::load(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn personality_switch(name: String, _state: State<'_, AppState>) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::set_active(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn personality_current(
    _state: State<'_, AppState>,
) -> Result<Option<axagent_agent::personality::Personality>, String> {
    axagent_agent::personality::PersonalityManager::get_active().map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct PersonalityCreatePayload {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[tauri::command]
pub async fn personality_create(
    payload: PersonalityCreatePayload,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let personality = axagent_agent::personality::Personality {
        name: payload.name,
        version: payload.version.unwrap_or_else(|| "1.0.0".to_string()),
        description: payload.description.unwrap_or_default(),
        content: payload.content,
        created_at: chrono::Utc::now(),
    };
    axagent_agent::personality::PersonalityManager::save(&personality).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn personality_delete(name: String, _state: State<'_, AppState>) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::delete(&name).map_err(|e| e.to_string())
}
