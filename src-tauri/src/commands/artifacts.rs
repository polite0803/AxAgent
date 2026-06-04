use crate::AppState;
use axagent_harness::types::*;
use tauri::State;

#[tauri::command]
pub async fn list_artifacts(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<Artifact>, String> {
    axagent_core::repo::artifact::list_artifacts(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_artifact(
    state: State<'_, AppState>,
    input: CreateArtifactInput,
) -> Result<Artifact, String> {
    axagent_core::repo::artifact::create_artifact(state.harness.db(), &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_artifact(
    state: State<'_, AppState>,
    id: String,
    input: UpdateArtifactInput,
) -> Result<Artifact, String> {
    axagent_core::repo::artifact::update_artifact(state.harness.db(), &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_artifact(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_core::repo::artifact::delete_artifact(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}
