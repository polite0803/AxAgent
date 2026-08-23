// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;

use crate::AppState;
use axagent_harness::types::*;
use tauri::State;

#[agent_command(domain = prompt, safety = Safe, call_mode = StateOnly, description = "列出所有提示词模板")]
#[tauri::command]
pub async fn list_prompt_templates(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplate>, String> {
    axagent_dao::repo::prompt_template::list_prompt_templates(state.harness.db()).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[agent_command(domain = prompt, safety = Safe, call_mode = StateInput, description = "获取单个提示词模板详情")]
#[tauri::command]
pub async fn get_prompt_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<PromptTemplate, String> {
    axagent_dao::repo::prompt_template::get_prompt_template(state.harness.db(), &id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "创建新提示词模板")]
#[tauri::command]
pub async fn create_prompt_template(
    state: State<'_, AppState>,
    input: CreatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_dao::repo::prompt_template::create_prompt_template(state.harness.db(), input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "更新提示词模板")]
#[tauri::command]
pub async fn update_prompt_template(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePromptTemplateInput,
) -> Result<PromptTemplate, String> {
    axagent_dao::repo::prompt_template::update_prompt_template(state.harness.db(), &id, input)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = prompt, safety = Dangerous, call_mode = StateInput, description = "删除提示词模板")]
#[tauri::command]
pub async fn delete_prompt_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::prompt_template::delete_prompt_template(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = prompt, safety = Safe, call_mode = StateInput, description = "获取提示词模板的历史版本列表")]
#[tauri::command]
pub async fn get_prompt_template_versions(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<Vec<PromptTemplateVersion>, String> {
    axagent_dao::repo::prompt_template::get_prompt_template_versions(
        state.harness.db(),
        &template_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "回滚提示词模板到指定版本")]
#[tauri::command]
pub async fn rollback_prompt_template(
    state: State<'_, AppState>,
    id: String,
    target_version: i32,
) -> Result<PromptTemplate, String> {
    axagent_dao::repo::prompt_template::rollback_prompt_template(
        state.harness.db(),
        &id,
        target_version,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "批量导入提示词模板")]
#[tauri::command]
pub async fn import_prompt_templates(
    state: State<'_, AppState>,
    inputs: Vec<ImportPromptTemplateInput>,
) -> Result<ImportPromptResult, String> {
    axagent_dao::repo::prompt_template::import_prompt_templates(state.harness.db(), inputs)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = prompt, safety = Safe, call_mode = StateInput, description = "导出指定的提示词模板")]
#[tauri::command]
pub async fn export_prompt_templates(
    state: State<'_, AppState>,
    ids: Vec<String>,
    format: ExportPromptFormat,
) -> Result<String, String> {
    axagent_dao::repo::prompt_template::export_prompt_templates(state.harness.db(), ids, format)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "从URL导入提示词模板")]
#[tauri::command]
pub async fn import_prompt_from_url(
    state: State<'_, AppState>,
    input: ImportFromUrlInput,
) -> Result<ImportPromptResult, String> {
    axagent_dao::repo::prompt_template::import_from_url(state.harness.db(), input).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "从文件夹批量导入提示词模板")]
#[tauri::command]
pub async fn import_prompt_from_folder(
    state: State<'_, AppState>,
    folder_path: String,
    category_filter: Option<String>,
) -> Result<ImportPromptResult, String> {
    axagent_dao::repo::prompt_template::import_from_folder(
        state.harness.db(),
        &folder_path,
        category_filter,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = prompt, safety = Caution, call_mode = StateInput, description = "递增提示词模板使用次数")]
#[tauri::command]
pub async fn increment_prompt_usage(
    state: State<'_, AppState>,
    id: String,
) -> Result<PromptTemplate, String> {
    axagent_dao::repo::prompt_template::increment_usage_count(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}
