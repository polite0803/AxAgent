// SPDX-License-Identifier: AGPL-3.0-only
//! Tauri commands for YAML workflow import/export.
//!
//! Delegates to `rt_workflow::yaml_io` for serialization/deserialization.

use crate::AppState;
use agent_macro::agent_command;
use axagent_rt_workflow::workflow_engine::Workflow;
use axagent_rt_workflow::yaml_io::{
    WorkflowYamlMetadata, YamlIoError, export_workflow_yaml as io_export,
    import_workflow_yaml as io_import,
};

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "导出工作流为YAML")]
#[tauri::command]
pub async fn export_workflow_yaml(
    _app: tauri::AppHandle,
    _state: tauri::State<'_, AppState>,
    workflow_json: String,
) -> Result<String, String> {
    let workflow: Workflow = serde_json::from_str(&workflow_json).map_err(|e| {
        // C-3: 迁移到 ErrorResponse，JSON 解析错误归类为 Validation
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Validation,
        )
        .to_string()
    })?;

    // C-3: 迁移到 ErrorResponse，YAML 序列化错误归类为 Unrecoverable
    io_export(&workflow).map_err(|e: YamlIoError| {
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        )
        .to_string()
    })
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "从YAML导入工作流")]
#[tauri::command]
pub async fn import_workflow_yaml(
    _app: tauri::AppHandle,
    _state: tauri::State<'_, AppState>,
    yaml_str: String,
) -> Result<String, String> {
    // C-3: 迁移到 ErrorResponse，YAML 解析错误归类为 Validation
    let (workflow, metadata) = io_import(&yaml_str).map_err(|e: YamlIoError| {
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Validation,
        )
        .to_string()
    })?;

    // Return both the parsed Workflow and metadata as a JSON string
    // so the frontend can destructure them.
    #[derive(serde::Serialize)]
    struct ImportResult {
        workflow: Workflow,
        metadata: WorkflowYamlMetadata,
    }

    let result = ImportResult { workflow, metadata };
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize import result: {e}"))
}
