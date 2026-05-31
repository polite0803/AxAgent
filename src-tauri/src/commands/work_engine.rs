use sea_orm::ActiveModelTrait;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

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
    pub node_records: Vec<NodeRecordResponse>,
    pub variables: serde_json::Value,
    pub parent_execution_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecordResponse {
    pub node_id: String,
    pub node_type: String,
    pub node_name: Option<String>,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub parent_execution_id: Option<String>,
    pub sub_workflow_id: Option<String>,
}

impl From<axagent_runtime::work_engine::execution_state::NodeExecutionRecord>
    for NodeRecordResponse
{
    fn from(r: axagent_runtime::work_engine::execution_state::NodeExecutionRecord) -> Self {
        Self {
            node_id: r.node_id,
            node_type: r.node_type,
            node_name: r.node_name,
            status: r.status,
            input: r.input,
            output: r.output,
            execution_time_ms: r.execution_time_ms,
            error: r.error,
            started_at: r.started_at,
            completed_at: r.completed_at,
            parent_execution_id: r.parent_execution_id,
            sub_workflow_id: r.sub_workflow_id,
        }
    }
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
    let engine = &*state.work_engine;
    engine
        .start_workflow(&workflow_id, input, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
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
    let engine = &*state.work_engine;
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
    let engine = &*state.work_engine;
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
    let engine = &*state.work_engine;
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
        node_records: status
            .node_records
            .into_iter()
            .map(NodeRecordResponse::from)
            .collect(),
        variables: serde_json::to_value(&status.variables).unwrap_or(serde_json::json!({})),
        parent_execution_id: status.parent_execution_id,
    })
}

#[tauri::command]
pub async fn list_workflow_executions(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<ExecutionSummaryResponse>, String> {
    let engine = &*state.work_engine;
    let executions = engine
        .list_executions(&workflow_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(executions
        .into_iter()
        .map(ExecutionSummaryResponse::from)
        .collect())
}

// ── 可视化工作流节点执行 ──

#[tauri::command]
pub async fn execute_workflow_node(
    state: State<'_, AppState>,
    execution_id: String,
    node_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let node: axagent_core::workflow_types::WorkflowNode =
        serde_json::from_value(node_json).map_err(|e| format!("节点 JSON 解析失败: {}", e))?;

    let engine = &*state.work_engine;
    let context = engine
        .get_status(&execution_id)
        .await
        .map_err(|e| e.to_string())?;

    match engine.execute_node(&node, &context).await {
        Ok(output) => serde_json::to_value(output).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn list_node_executor_types(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let engine = &*state.work_engine;
    Ok(engine
        .registered_executor_types()
        .await
        .into_iter()
        .map(String::from)
        .collect())
}

// ── Debug Commands ──

#[tauri::command]
pub async fn debug_run_workflow(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    template_id: String,
    input: Option<serde_json::Value>,
    breakpoints: Option<Vec<String>>,
    dry_run: Option<bool>,
    model_id: Option<String>,
    provider_id: Option<String>,
) -> Result<String, String> {
    use axagent_core::repo::workflow_template;

    let db = &state.sea_db;
    let template = workflow_template::get_workflow_template(db, &template_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Template {} not found", template_id))?;

    let nodes_json: Vec<serde_json::Value> =
        serde_json::from_str(&template.nodes).map_err(|e| format!("节点解析失败: {}", e))?;
    // 逐条反序列化以排查 Vec 整体反序列化的变体混淆
    let mut nodes: Vec<axagent_core::workflow_types::WorkflowNode> = Vec::new();
    for (i, val) in nodes_json.iter().enumerate() {
        let n: axagent_core::workflow_types::WorkflowNode = serde_json::from_value(val.clone())
            .map_err(|e| format!("节点[{i}]反序列化失败: {e}"))?;
        let typ = axagent_rt_workflow::work_engine::node_executor_trait::node_type_name(&n);
        tracing::info!(i, node_id = %n.base_id(), node_type = typ, "deserialized node");
        nodes.push(n);
    }
    let edges: Vec<axagent_core::workflow_types::WorkflowEdge> =
        serde_json::from_str(&template.edges).map_err(|e| format!("边解析失败: {}", e))?;

    let variables: Vec<axagent_core::workflow_types::Variable> = template
        .variables
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("变量解析失败: {}", e))?
        .unwrap_or_default();

    let input_schema: Option<axagent_core::workflow_types::JsonSchema> = template
        .input_schema
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("input_schema 解析失败: {}", e))?;
    let output_schema: Option<axagent_core::workflow_types::JsonSchema> = template
        .output_schema
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("output_schema 解析失败: {}", e))?;

    let engine = state.work_engine.clone();
    let workflow = engine
        .create_workflow(&template.name, nodes, edges)
        .await
        .map_err(|e| e.to_string())?;
    let workflow_id = workflow.id.clone();
    let execution_id = uuid::Uuid::new_v4().to_string();

    if let Some(bp) = breakpoints {
        let bp_set: std::collections::HashSet<String> = bp.into_iter().collect();
        engine.set_breakpoints(bp_set).await;
    }

    engine.clear_node_breakers().await;

    let app_clone = app.clone();
    let wid_for_progress = workflow_id.clone();
    let eid_for_progress = execution_id.clone();
    let progress_cb: axagent_runtime::work_engine::ProgressCallback =
        std::sync::Arc::new(move |evt| {
            let app = app_clone.clone();
            let node_id = evt.node_id.clone();
            let status = evt.status.clone();
            let total = evt.total_nodes;
            let completed = evt.completed_nodes;
            let wf_id = wid_for_progress.clone();
            let exec_id = evt
                .execution_id
                .clone()
                .unwrap_or_else(|| eid_for_progress.clone());
            Box::pin(async move {
                let _ = app.emit(
                    "workflow:node-status-changed",
                    serde_json::json!({
                        "workflow_id": wf_id,
                        "execution_id": exec_id,
                        "node_id": node_id,
                        "status": status,
                        "total_nodes": total,
                        "completed_nodes": completed,
                    }),
                );
            })
        });

    let wid = workflow_id.clone();
    let eid = execution_id.clone();
    let app_for_completion = app.clone();
    tokio::spawn(async move {
        let mut opts =
            axagent_runtime::work_engine::RunOptions::default().with_progress_callback(progress_cb);
        opts.execution_id = Some(eid.clone());
        if let Some(m) = model_id {
            opts = opts.with_model(m);
        }
        if let Some(p) = provider_id {
            opts = opts.with_provider(p);
        }
        if !variables.is_empty() {
            opts = opts.with_variables(variables);
        }
        opts.input = input;
        opts.input_schema = input_schema;
        opts.output_schema = output_schema;
        opts.dry_run = dry_run.unwrap_or(false);

        let result = engine.run_workflow(&wid, opts).await;
        match &result {
            Ok(wf) => {
                let _ = app_for_completion.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wf.id,
                        "execution_id": eid,
                        "status": match wf.status {
                            axagent_runtime::workflow_engine::WorkflowStatus::Completed => "completed",
                            axagent_runtime::workflow_engine::WorkflowStatus::PartiallyCompleted => "partially_completed",
                            axagent_runtime::workflow_engine::WorkflowStatus::Failed => "failed",
                            axagent_runtime::workflow_engine::WorkflowStatus::Cancelled => "cancelled",
                            _ => "unknown",
                        },
                        "total_time_ms": wf.completed_at
                            .map(|end| end.saturating_sub(wf.created_at) * 1000)
                            .unwrap_or(0),
                    }),
                );
            },
            Err(e) => {
                tracing::error!("[debug_run_workflow] 执行失败: {}", e);
                let _ = app_for_completion.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "execution_id": eid,
                        "status": "failed",
                        "total_time_ms": 0,
                        "error": e.to_string(),
                    }),
                );
            },
        }
    });

    Ok(execution_id)
}

#[tauri::command]
pub async fn set_workflow_breakpoints(
    state: State<'_, AppState>,
    node_ids: Vec<String>,
    execution_id: Option<String>,
) -> Result<bool, String> {
    let bp: std::collections::HashSet<String> = node_ids.into_iter().collect();
    if let Some(eid) = execution_id {
        state
            .work_engine
            .set_breakpoints_for_execution(&eid, bp)
            .await;
    } else {
        state.work_engine.set_breakpoints(bp).await;
    }
    Ok(true)
}

#[tauri::command]
pub async fn resume_workflow_breakpoint(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    state.work_engine.resume_breakpoints(&execution_id).await;
    Ok(true)
}

#[tauri::command]
pub async fn step_workflow_breakpoint(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    state.work_engine.step_breakpoint(&execution_id).await;
    Ok(true)
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
