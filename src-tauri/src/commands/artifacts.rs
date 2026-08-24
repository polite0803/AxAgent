// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_harness::types::*;
use tauri::State;

#[tauri::command]
#[agent_command(
    domain = artifact,
    safety = Safe,
    call_mode = StateInput,
    description = "列出会话的所有产物"
)]
pub async fn list_artifacts(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<Artifact>, String> {
    axagent_dao::repo::artifact::list_artifacts(state.harness.db(), &conversation_id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
#[agent_command(
    domain = artifact,
    safety = Caution,
    call_mode = StateInput,
    description = "创建新产物"
)]
pub async fn create_artifact(
    state: State<'_, AppState>,
    input: CreateArtifactInput,
) -> Result<Artifact, String> {
    axagent_dao::repo::artifact::create_artifact(state.harness.db(), &input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
#[agent_command(
    domain = artifact,
    safety = Caution,
    call_mode = StateInput,
    description = "更新产物信息"
)]
pub async fn update_artifact(
    state: State<'_, AppState>,
    id: String,
    input: UpdateArtifactInput,
) -> Result<Artifact, String> {
    axagent_dao::repo::artifact::update_artifact(state.harness.db(), &id, &input).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
#[agent_command(
    domain = artifact,
    safety = Dangerous,
    call_mode = StateInput,
    description = "删除产物"
)]
pub async fn delete_artifact(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::artifact::delete_artifact(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
