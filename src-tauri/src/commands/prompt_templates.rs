// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_harness::types::*;
use tauri::State;

#[tauri::command]
pub async fn list_prompt_templates(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplate>, String> {
    axagent_core::repo::prompt_template::list_prompt_templates(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::get_prompt_template(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_prompt_template(
    state: State<'_, AppState>,
    input: CreatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::create_prompt_template(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_prompt_template(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::update_prompt_template(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_core::repo::prompt_template::delete_prompt_template(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template_versions(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<Vec<PromptTemplateVersion>, String> {
    axagent_core::repo::prompt_template::get_prompt_template_versions(
        state.harness.db(),
        &template_id,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rollback_prompt_template(
    state: State<'_, AppState>,
    id: String,
    target_version: i32,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::rollback_prompt_template(
        state.harness.db(),
        &id,
        target_version,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_prompt_templates(
    state: State<'_, AppState>,
    inputs: Vec<ImportPromptTemplateInput>,
) -> Result<ImportPromptResult, String> {
    axagent_core::repo::prompt_template::import_prompt_templates(state.harness.db(), inputs)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_prompt_templates(
    state: State<'_, AppState>,
    ids: Vec<String>,
    format: ExportPromptFormat,
) -> Result<String, String> {
    axagent_core::repo::prompt_template::export_prompt_templates(state.harness.db(), ids, format)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_prompt_from_url(
    state: State<'_, AppState>,
    input: ImportFromUrlInput,
) -> Result<ImportPromptResult, String> {
    axagent_core::repo::prompt_template::import_from_url(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_prompt_from_folder(
    state: State<'_, AppState>,
    folder_path: String,
    category_filter: Option<String>,
) -> Result<ImportPromptResult, String> {
    axagent_core::repo::prompt_template::import_from_folder(
        state.harness.db(),
        &folder_path,
        category_filter,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn increment_prompt_usage(
    state: State<'_, AppState>,
    id: String,
) -> Result<PromptTemplate, String> {
    axagent_core::repo::prompt_template::increment_usage_count(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}
