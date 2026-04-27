use sea_orm::ActiveModelTrait;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatusResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub current_node_id: Option<String>,
    pub total_time_ms: u64,
    pub node_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummaryResponse {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub total_time_ms: Option<i32>,
    pub created_at: i64,
}

impl From<axagent_core::entity::workflow_executions::Model> for ExecutionSummaryResponse {
    fn from(m: axagent_core::entity::workflow_executions::Model) -> Self {
        Self {
            id: m.id,
            workflow_id: m.workflow_id,
            status: m.status,
            total_time_ms: m.total_time_ms,
            created_at: m.created_at,
        }
    }
}

// ── Commands ──

#[tauri::command]
pub async fn start_workflow_execution(
    state: State<'_, AppState>,
    workflow_id: String,
    input: serde_json::Value,
) -> Result<String, String> {
    let engine = state.work_engine.read().await;
    engine
        .start_workflow(&workflow_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = state.work_engine.read().await;
    engine
        .pause(&execution_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn resume_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = state.work_engine.read().await;
    engine
        .resume(&execution_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn cancel_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = state.work_engine.read().await;
    engine
        .cancel(&execution_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn get_workflow_execution_status(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<ExecutionStatusResponse, String> {
    let engine = state.work_engine.read().await;
    let status = engine
        .get_status(&execution_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExecutionStatusResponse {
        execution_id: status.execution_id,
        workflow_id: status.workflow_id,
        status: status.status.to_string(),
        current_node_id: status.current_node_id,
        total_time_ms: status.total_time_ms,
        node_count: status.node_records.len(),
    })
}

#[tauri::command]
pub async fn list_workflow_executions(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<ExecutionSummaryResponse>, String> {
    let engine = state.work_engine.read().await;
    let executions = engine
        .list_executions(&workflow_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(executions
        .into_iter()
        .map(ExecutionSummaryResponse::from)
        .collect())
}

// ── Workflow Migration Commands ──

#[tauri::command]
pub async fn migrate_workflow_nodes(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<serde_json::Value, String> {
    use axagent_core::repo::workflow_template;
    use axagent_core::workflow_types::WorkflowMigrator;

    let db = &state.sea_db;
    let workflow = workflow_template::get_workflow_template(db, &workflow_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Workflow {} not found", workflow_id))?;

    let mut nodes: Vec<axagent_core::workflow_types::WorkflowNode> =
        serde_json::from_str(&workflow.nodes).map_err(|e| e.to_string())?;

    let result = WorkflowMigrator::migrate(&mut nodes);

    // Update only the nodes field via a direct query
    let updated_nodes_str = serde_json::to_string(&nodes).map_err(|e| e.to_string())?;

    let active_model = axagent_core::entity::workflow_template::ActiveModel {
        id: sea_orm::ActiveValue::Unchanged(workflow.id.clone()),
        nodes: sea_orm::ActiveValue::Set(updated_nodes_str),
        ..Default::default()
    };
    active_model.update(db).await.map_err(|e| e.to_string())?;

    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn migrate_all_workflows(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_core::repo::workflow_template;
    use axagent_core::workflow_types::WorkflowMigrator;

    let db = &state.sea_db;
    let workflows = workflow_template::list_workflow_templates(db, None)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for wf in &workflows {
        let mut nodes: Vec<axagent_core::workflow_types::WorkflowNode> =
            serde_json::from_str(&wf.nodes).map_err(|e| e.to_string())?;

        if WorkflowMigrator::has_legacy_nodes(&nodes) {
            let result = WorkflowMigrator::migrate(&mut nodes);
            let updated_nodes_str = serde_json::to_string(&nodes).map_err(|e| e.to_string())?;

            let active_model = axagent_core::entity::workflow_template::ActiveModel {
                id: sea_orm::ActiveValue::Unchanged(wf.id.clone()),
                nodes: sea_orm::ActiveValue::Set(updated_nodes_str),
                ..Default::default()
            };
            active_model.update(db).await.map_err(|e| e.to_string())?;

            results.push(serde_json::json!({
                "workflow_id": wf.id,
                "workflow_name": wf.name,
                "migrated_count": result.migrated_nodes.len(),
            }));
        }
    }

    Ok(serde_json::json!({ "migrated_workflows": results, "total": results.len() }))
}
