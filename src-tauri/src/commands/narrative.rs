// SPDX-License-Identifier: AGPL-3.0-only

//! 叙事结构命令（v126）—— 补齐前端 `src/lib/narrativeStructure.ts` 的后端。
//!
//! 契约（与前端 `NarrativeStructureRecord` / `CreateNarrativeRequest` /
//! `UpdateNarrativeRequest` 一一对应，全 camelCase）：
//! - `list_narrative_structures(isTemplate?, genre?)`
//! - `get_narrative_structure(id)`
//! - `create_narrative_structure(input)`
//! - `update_narrative_structure(input)`
//! - `delete_narrative_structure(id)`

use axagent_agent_macro::agent_command;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;

/// 叙事结构记录（对应前端 `NarrativeStructureRecord`）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeStructureRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    /// JSON 对象：arcs / confluences / foreshadows（见 src/types/narrative.ts）
    pub structure: serde_json::Value,
    pub is_template: bool,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNarrativeInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    pub structure: serde_json::Value,
    pub is_template: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNarrativeInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub structure: Option<serde_json::Value>,
}

fn record_from_model(
    m: axagent_entities::narrative_structures::Model,
) -> Result<NarrativeStructureRecord, String> {
    let structure = serde_json::from_str(&m.structure).map_err(|e| {
        String::from(ErrorResponse::from_error(
            format!("叙事结构 JSON 损坏（id={}）: {e}", m.id),
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(NarrativeStructureRecord {
        id: m.id,
        name: m.name,
        description: m.description,
        genre: m.genre,
        structure,
        is_template: m.is_template,
        version: m.version,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

fn err(e: impl std::fmt::Display) -> String {
    String::from(ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable))
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Safe,
    call_mode = StateInput,
    description = "列出叙事结构（可按模板/体裁过滤）"
)]
pub async fn list_narrative_structures(
    state: State<'_, AppState>,
    is_template: Option<bool>,
    genre: Option<String>,
) -> Result<Vec<NarrativeStructureRecord>, String> {
    let rows = axagent_dao::repo::narrative::list_narrative_structures(
        state.harness.db(),
        is_template,
        genre.as_deref(),
    )
    .await
    .map_err(err)?;

    rows.into_iter().map(record_from_model).collect()
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Safe,
    call_mode = StateInput,
    description = "获取单个叙事结构"
)]
pub async fn get_narrative_structure(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<NarrativeStructureRecord>, String> {
    let row = axagent_dao::repo::narrative::get_narrative_structure(state.harness.db(), &id)
        .await
        .map_err(err)?;
    row.map(record_from_model).transpose()
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Caution,
    call_mode = StateInput,
    description = "创建叙事结构"
)]
pub async fn create_narrative_structure(
    state: State<'_, AppState>,
    input: CreateNarrativeInput,
) -> Result<NarrativeStructureRecord, String> {
    if input.name.trim().is_empty() {
        return Err(String::from(ErrorResponse::from_error(
            "叙事结构名称不能为空",
            crate::commands::error::ErrorCategory::Validation,
        )));
    }
    let now = chrono::Utc::now().timestamp_millis();
    let active = axagent_entities::narrative_structures::ActiveModel {
        id: Set(input.id),
        name: Set(input.name),
        description: Set(input.description),
        genre: Set(input.genre),
        structure: Set(serde_json::to_string(&input.structure).map_err(err)?),
        is_template: Set(input.is_template.unwrap_or(false)),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let row = axagent_dao::repo::narrative::insert_narrative_structure(state.harness.db(), active)
        .await
        .map_err(err)?;
    record_from_model(row)
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Caution,
    call_mode = StateInput,
    description = "更新叙事结构（version 自动递增）"
)]
pub async fn update_narrative_structure(
    state: State<'_, AppState>,
    input: UpdateNarrativeInput,
) -> Result<NarrativeStructureRecord, String> {
    let structure = match input.structure {
        Some(v) => Some(serde_json::to_string(&v).map_err(err)?),
        None => None,
    };
    let row = axagent_dao::repo::narrative::update_narrative_structure(
        state.harness.db(),
        &input.id,
        input.name,
        input.description,
        input.genre,
        structure,
    )
    .await
    .map_err(err)?;
    record_from_model(row)
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Caution,
    call_mode = StateInput,
    description = "删除叙事结构"
)]
pub async fn delete_narrative_structure(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_dao::repo::narrative::delete_narrative_structure(state.harness.db(), &id)
        .await
        .map_err(err)
}
