// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::narrative_structure as db_repo;
use axagent_harness::narrative::NarrativeStructure;
use serde::Deserialize;
use tauri::State;

/// 创建叙事结构请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNarrativeRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    pub structure: NarrativeStructure,
    pub is_template: Option<bool>,
}

/// 更新叙事结构请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNarrativeRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub structure: Option<NarrativeStructure>,
}

/// 叙事结构响应
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeStructureResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    pub structure: NarrativeStructure,
    pub is_template: bool,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

fn model_to_response(
    model: &axagent_entities::narrative_structure::Model,
) -> NarrativeStructureResponse {
    let structure = db_repo::model_to_dto(model);
    NarrativeStructureResponse {
        id: model.id.clone(),
        name: model.name.clone(),
        description: model.description.clone(),
        genre: model.genre.clone(),
        structure,
        is_template: model.is_template,
        version: model.version,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出叙事结构")]
#[tauri::command]
pub async fn list_narrative_structures(
    state: State<'_, AppState>,
    is_template: Option<bool>,
    genre: Option<String>,
) -> Result<Vec<NarrativeStructureResponse>, String> {
    let db = state.harness.db();
    let results =
        db_repo::list_narrative_structures(db, is_template, genre).await.map_err(|e| {
            String::from(ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(results.iter().map(model_to_response).collect())
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取叙事结构详情")]
#[tauri::command]
pub async fn get_narrative_structure(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<NarrativeStructureResponse>, String> {
    let db = state.harness.db();
    let result = db_repo::get_narrative_structure(db, &id).await.map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(result.as_ref().map(model_to_response))
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "创建叙事结构")]
#[tauri::command]
pub async fn create_narrative_structure(
    state: State<'_, AppState>,
    input: CreateNarrativeRequest,
) -> Result<NarrativeStructureResponse, String> {
    let db = state.harness.db();
    let model = db_repo::create_narrative_structure(
        db,
        input.id,
        input.name,
        input.description,
        input.genre,
        &input.structure,
        input.is_template.unwrap_or(false),
    )
    .await
    .map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(model_to_response(&model))
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "更新叙事结构")]
#[tauri::command]
pub async fn update_narrative_structure(
    state: State<'_, AppState>,
    input: UpdateNarrativeRequest,
) -> Result<NarrativeStructureResponse, String> {
    let db = state.harness.db();
    let model = db_repo::update_narrative_structure(
        db,
        &input.id,
        input.name,
        input.description,
        input.genre,
        input.structure.as_ref(),
    )
    .await
    .map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(model_to_response(&model))
}

#[agent_command(domain = workflow, safety = Dangerous, call_mode = StateInput, description = "删除叙事结构")]
#[tauri::command]
pub async fn delete_narrative_structure(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    db_repo::delete_narrative_structure(db, &id).await.map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(())
}
