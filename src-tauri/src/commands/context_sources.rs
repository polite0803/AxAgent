// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_harness::types::*;
use tauri::State;

#[tauri::command]
pub async fn list_context_sources(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ContextSource>, String> {
    axagent_dao::repo::context_source::list_context_sources(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn add_context_source(
    state: State<'_, AppState>,
    input: CreateContextSourceInput,
) -> Result<ContextSource, String> {
    axagent_dao::repo::context_source::add_context_source(state.harness.db(), &input).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn remove_context_source(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::context_source::remove_context_source(state.harness.db(), &id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn toggle_context_source(
    state: State<'_, AppState>,
    id: String,
) -> Result<ContextSource, String> {
    axagent_dao::repo::context_source::toggle_context_source(state.harness.db(), &id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

/// 多文档协同：根据 (conversation_id, source_type, ref_id) 定位 context_source 行，
/// 更新其 doc_ids 字段。前端用户在 ContextSourcePicker 中勾选/取消勾选文档时调用。
#[tauri::command]
pub async fn set_context_source_doc_ids(
    state: State<'_, AppState>,
    conversation_id: String,
    source_type: String,
    ref_id: String,
    doc_ids: Vec<String>,
) -> Result<ContextSource, String> {
    axagent_dao::repo::context_source::set_doc_ids_by_ref(
        state.harness.db(),
        &conversation_id,
        &source_type,
        &ref_id,
        &doc_ids,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
