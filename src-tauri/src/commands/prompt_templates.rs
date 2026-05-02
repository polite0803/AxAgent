use crate::AppState;
use axagent_core::types::*;
use tauri::State;

#[tauri::command]
pub async fn list_prompt_templates(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplate>, String> {
    axagent_core::repo::prompt_template::list_prompt_templates(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::get_prompt_template(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_prompt_template(
    state: State<'_, AppState>,
    input: CreatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::create_prompt_template(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_prompt_template(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::update_prompt_template(&state.sea_db, &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_core::repo::prompt_template::delete_prompt_template(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template_versions(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<Vec<PromptTemplateVersion>, String> {
    axagent_core::repo::prompt_template::get_prompt_template_versions(&state.sea_db, &template_id)
        .await
        .map_err(|e| e.to_string())
}