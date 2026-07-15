// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::agent::skill_execution::{self, SkillStep};
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::agent as agent_err;
use crate::commands::spawn_guard::SpawnGuard;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::{Emitter, State};

// ── 类型定义 ──

/// 创建工作流的请求参数
#[derive(Debug, Deserialize)]
pub struct WorkflowCreateRequest {
    pub name: String,
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

/// 创建工作流的响应
#[derive(Debug, Serialize)]
pub struct WorkflowCreateResponse {
    #[serde(rename = "workflowId")]
    pub workflow_id: String,
    pub name: String,
    #[serde(rename = "stepCount")]
    pub step_count: usize,
}

/// 对话工作流预览
#[derive(Debug, Serialize)]
pub struct ConversationWorkflowPreview {
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
    pub skill_execution_order: Vec<String>,
    pub skill_count: usize,
}

// ── 命令函数 ──

/// 从节点和边的 JSON 创建新工作流 DAG
#[tauri::command]
pub async fn workflow_create(
    app_state: State<'_, AppState>,
    request: WorkflowCreateRequest,
) -> Result<WorkflowCreateResponse, String> {
    let nodes: Vec<axagent_harness::workflow_types::WorkflowNode> =
        request.nodes.into_iter().filter_map(|n| serde_json::from_value(n).ok()).collect();
    let edges: Vec<axagent_harness::workflow_types::WorkflowEdge> =
        request.edges.into_iter().filter_map(|e| serde_json::from_value(e).ok()).collect();

    let workflow = app_state
        .work_engine
        .create_workflow(&request.name, nodes, edges)
        .await
        .map_err(|e| e.to_string())?;

    Ok(WorkflowCreateResponse {
        workflow_id: workflow.id.clone(),
        name: workflow.name,
        step_count: workflow.nodes.len(),
    })
}

/// 执行工作流（含 LLM 步骤执行）
#[tauri::command]
pub async fn workflow_execute(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    workflow_id: String,
    model_id: Option<String>,
    provider_id: Option<String>,
    variables: Option<Vec<axagent_harness::workflow_types::Variable>>,
) -> Result<String, String> {
    // 验证工作流存在
    let _ = app_state
        .work_engine
        .get_workflow(&workflow_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| ErrorResponse::err(agent_err::WORKFLOW_NOT_FOUND))?;

    // 工具解析器已由 init/services.rs 在启动期注入（含 builtin / mcp / workflow:: 三种来源），
    // 此处不再 set_tool_resolver 覆盖——否则会静默丢弃 init 阶段注入的 workflow:: 解析。
    let _ = app_state.local_tool_registry; // 保留依赖项以维持签名稳定

    let engine = app_state.work_engine.clone();
    let wid = workflow_id.clone();
    let app_for_emit = app.clone();
    let app_for_panic = app_for_emit.clone();
    let wid_for_panic = wid.clone();
    tokio::spawn(async move {
        // 兜底：panic / 早退路径上 emit execution-completed failed 事件
        let _guard = SpawnGuard::new("workflow_run", move || {
            tracing::error!("[workflow_run] PANIC guard fired for workflow={}", wid_for_panic);
            let _ = app_for_panic.emit(
                "workflow:execution-completed",
                serde_json::json!({
                    "workflow_id": wid_for_panic,
                    "success": false,
                    "error": "Internal panic during workflow execution",
                }),
            );
        });
        let mut opts = axagent_runtime::work_engine::RunOptions::default();
        if let Some(m) = model_id {
            opts = opts.with_model(m);
        }
        if let Some(p) = provider_id {
            opts = opts.with_provider(p);
        }
        if let Some(vars) = variables {
            opts = opts.with_variables(vars);
        }
        match engine.run_workflow(&wid, opts).await {
            Ok(result) => {
                let _ = app_for_emit.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "success": true,
                        "result": result,
                    }),
                );
            },
            Err(e) => {
                tracing::error!("[workflow] 执行失败: {}", e);
                let _ = app_for_emit.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "success": false,
                        "error": e.to_string(),
                    }),
                );
            },
        }
        _guard.finish();
    });

    Ok(workflow_id)
}

/// 获取工作流状态
#[tauri::command]
pub async fn workflow_get_status(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    let workflow =
        app_state.work_engine.get_workflow(&workflow_id).await.map_err(|e| e.to_string())?;

    match workflow {
        Some(w) => Ok(serde_json::to_value(w).map_err(|e| e.to_string())?),
        None => Err(ErrorResponse::new(agent_err::WORKFLOW_NOT_FOUND).into()),
    }
}

/// 取消正在执行的工作流
#[tauri::command]
pub async fn workflow_cancel(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    let workflow =
        app_state.work_engine.cancel_workflow(&workflow_id).await.map_err(|e| e.to_string())?;

    serde_json::to_value(workflow).map_err(|e| e.to_string())
}

/// 列出所有工作流
#[tauri::command]
pub async fn workflow_list(app_state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let workflows = app_state.work_engine.list_workflows().await.map_err(|e| e.to_string())?;

    Ok(workflows.into_iter().filter_map(|w| serde_json::to_value(w).ok()).collect())
}

/// 获取工作流步骤详情（用于 DAG 可视化）
#[tauri::command]
pub async fn workflow_get_steps(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<Value>, String> {
    let workflow = app_state
        .work_engine
        .get_workflow(&workflow_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| ErrorResponse::err(agent_err::WORKFLOW_NOT_FOUND))?;
    Ok(workflow.nodes.iter().filter_map(|s| serde_json::to_value(s).ok()).collect())
}

/// 从对话工具执行记录获取工作流预览
#[tauri::command]
pub async fn get_conversation_workflow_preview(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationWorkflowPreview, String> {
    let db = app_state.harness.db();

    let executions = axagent_dao::repo::tool_execution::list_tool_executions(db, &conversation_id)
        .await
        .map_err(|e| {
            ErrorResponse::new(agent_err::INTERNAL)
                .with_detail(format!("Failed to list tool executions: {}", e))
        })?;

    let mut all_nodes: Vec<Value> = Vec::new();
    let mut all_edges: Vec<Value> = Vec::new();
    let mut skill_execution_order: Vec<String> = Vec::new();
    let mut skill_node_ids: HashMap<String, Vec<String>> = HashMap::new();

    for execution in &executions {
        if execution.tool_name.starts_with("skill_") || execution.tool_name == "skill_executor" {
            if let Some(ref skill_steps_json) = execution.skill_steps_json {
                if let Ok(skill_steps) = serde_json::from_str::<Vec<SkillStep>>(skill_steps_json) {
                    let skill_id = execution.tool_name.clone();
                    let base_y = all_nodes.len() as f64 * 200.0;

                    let (nodes, edges) =
                        skill_steps_to_nodes_edges_with_offset(&skill_steps, &skill_id, base_y);

                    let node_ids: Vec<String> = nodes
                        .iter()
                        .filter_map(|n| n.get("id").and_then(|id| id.as_str()).map(String::from))
                        .collect();

                    skill_node_ids.insert(skill_id.clone(), node_ids);
                    skill_execution_order.push(skill_id.clone());

                    all_nodes.extend(nodes);
                    all_edges.extend(edges);
                }
            }

            if let Some(ref depends_on_json) = execution.depends_on {
                if let Ok(depends_on) = serde_json::from_str::<Vec<String>>(depends_on_json) {
                    for dep_skill in depends_on {
                        if let Some(dep_nodes) = skill_node_ids.get(&dep_skill) {
                            if let Some(current_nodes) = skill_node_ids.get(&execution.tool_name) {
                                if let (Some(first_dep), Some(first_current)) =
                                    (dep_nodes.first(), current_nodes.first())
                                {
                                    let edge = serde_json::json!({
                                        "id": format!("inter_edge_{}_{}", dep_skill, execution.tool_name),
                                        "source": first_dep,
                                        "target": first_current,
                                        "edge_type": "dependency",
                                        "data": {
                                            "dependency_type": "inter_skill",
                                            "from_skill": dep_skill,
                                            "to_skill": execution.tool_name,
                                        }
                                    });
                                    all_edges.push(edge);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ConversationWorkflowPreview {
        nodes: all_nodes,
        edges: all_edges,
        skill_execution_order: skill_execution_order.clone(),
        skill_count: skill_execution_order.len(),
    })
}

// ── 辅助函数 ──

fn skill_steps_to_nodes_edges_with_offset(
    skill_steps: &[SkillStep],
    skill_id: &str,
    base_y: f64,
) -> (Vec<Value>, Vec<Value>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let trigger_node_id = format!("trigger_{}", skill_id);
    let trigger_node = serde_json::json!({
        "id": trigger_node_id,
        "type": "trigger",
        "position": { "x": 250, "y": base_y },
        "data": {
            "id": trigger_node_id,
            "title": format!("Trigger: {}", skill_id),
            "description": format!("Skill trigger for {}", skill_id),
            "node_type": "trigger",
            "config": {
                "type": "manual",
                "skill_id": skill_id,
            },
            "enabled": true,
        },
    });
    nodes.push(trigger_node);

    let mut step_offset_map: HashMap<usize, String> = HashMap::new();

    for s in skill_steps {
        let step_id = format!("{}_step_{}", skill_id, s.step);
        step_offset_map.insert(s.step, step_id.clone());

        let role_str = skill_execution::infer_agent_role(&s.action, &s.description);

        let node = serde_json::json!({
            "id": step_id,
            "type": "agent",
            "position": { "x": 250, "y": base_y + (s.step as f64 + 1.0) * 150.0 },
            "data": {
                "id": step_id,
                "title": s.action,
                "description": s.description,
                "node_type": "agent",
                "config": {
                    "role": role_str,
                    "system_prompt": format!("You are a {}. Task: {}", role_str, s.description),
                    "output_var": "result",
                    "context_sources": [],
                },
                "retry": {
                    "max_attempts": 2,
                    "delay_ms": 1000,
                },
                "enabled": true,
                "skill_id": skill_id,
            },
        });
        nodes.push(node);

        let edge = serde_json::json!({
            "id": format!("edge_{}_{}", trigger_node_id, step_id),
            "source": trigger_node_id,
            "target": step_id,
            "edge_type": "default",
        });
        edges.push(edge);

        for need in &s.needs {
            if let Some(prev_step_id) = step_offset_map.get(need) {
                let need_edge = serde_json::json!({
                    "id": format!("need_edge_{}_{}", prev_step_id, step_id),
                    "source": prev_step_id,
                    "target": step_id,
                    "edge_type": "dependency",
                    "data": {
                        "dependency_type": "intra_skill",
                        "from_step": need,
                        "to_step": s.step,
                    }
                });
                edges.push(need_edge);
            }
        }
    }

    (nodes, edges)
}
