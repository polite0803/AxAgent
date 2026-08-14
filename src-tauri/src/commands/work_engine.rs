// SPDX-License-Identifier: AGPL-3.0-only

use agent_macro::agent_command;
use axagent_harness::repo_dtos::WorkflowExecutionData;
use axagent_harness::workflow_types::NodeStatus;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::work_engine as work_engine_err;
use crate::commands::spawn_guard::SpawnGuard;

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

impl From<WorkflowExecutionData> for ExecutionSummaryResponse {
    fn from(m: WorkflowExecutionData) -> Self {
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

/// 后台执行工作流（P0-2：`start_workflow_execution` / `workflow_execute` 统一入口）。
///
/// 流程：execution_id（复用 opts 或新建）→ `tokio::spawn`（SpawnGuard 兜底
/// panic emit failed）→ `run_workflow(opts)` → emit `workflow:execution-completed`
/// （成功/失败均发，字段对齐前端 workEngineStore 契约）→ OPC 自动学习钩子
/// （成功/失败均触发；industry_id 由 `identify_industry_from_template` 从模板 ID
/// 动态识别，P4-4 行业包驱动，无需显式传参）。
///
/// 返回 execution_id。模板存在性不做预校验——`run_workflow` 内部对 DB 模板
/// 有兜底（重启后 `self.workflows` 未填充的场景），失败自然 emit failed。
#[allow(dead_code)]
pub fn spawn_workflow_run(
    app: tauri::AppHandle,
    engine: std::sync::Arc<axagent_runtime::work_engine::WorkEngine>,
    workflow_id: String,
    mut opts: axagent_runtime::work_engine::RunOptions,
    learning: Option<crate::state::learning::LearningEngineState>,
    app_data_dir: Option<std::path::PathBuf>,
) -> String {
    let execution_id =
        opts.execution_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    opts.execution_id = Some(execution_id.clone());

    let wid = workflow_id.clone();
    let eid = execution_id.clone();
    let app_for_completion = app.clone();
    let app_for_panic = app_for_completion.clone();
    let wid_for_panic = wid.clone();
    let eid_for_panic = eid.clone();
    let eng = engine.clone();
    tokio::spawn(async move {
        let _guard = SpawnGuard::new("workflow_run", move || {
            tracing::error!("[workflow_run] PANIC guard fired for workflow={}", wid_for_panic);
            let _ = app_for_panic.emit(
                "workflow:execution-completed",
                serde_json::json!({
                    "workflow_id": wid_for_panic,
                    "execution_id": eid_for_panic,
                    "status": "failed",
                    "total_time_ms": 0,
                    "error": "Internal panic during workflow execution",
                }),
            );
        });
        let started_at = std::time::Instant::now();
        match eng.run_workflow(&wid, opts).await {
            Ok(wf) => {
                let total_time_ms = started_at.elapsed().as_millis() as u64;
                let status_str = format!("{:?}", wf.status).to_lowercase();
                let _ = app_for_completion.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "execution_id": eid,
                        "status": status_str,
                        "total_time_ms": total_time_ms,
                    }),
                );
                // OPC 自动学习钩子（成功路径，携带节点结果与步骤状态）
                if let Some(ref l) = learning {
                    let node_steps: Vec<serde_json::Value> = wf
                        .node_states
                        .iter()
                        .map(|(id, s)| {
                            serde_json::json!({
                                "node_id": id,
                                "status": format!("{:?}", s.status).to_lowercase(),
                            })
                        })
                        .collect();
                    let result_json = serde_json::json!({
                        "status": status_str,
                        "total_time_ms": total_time_ms,
                        "results": wf.results,
                        "steps": node_steps,
                    });
                    crate::commands::opc_learning_hook::try_auto_learn_workflow(
                        &wid,
                        &result_json,
                        l,
                        app_data_dir.as_deref(),
                    )
                    .await;
                }
            },
            Err(e) => {
                tracing::error!("[workflow_run] 执行失败: {}", e);
                let total_time_ms = started_at.elapsed().as_millis() as u64;
                let _ = app_for_completion.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "execution_id": eid,
                        "status": "failed",
                        "total_time_ms": total_time_ms,
                        "error": e.to_string(),
                    }),
                );
                // OPC 自动学习钩子（失败路径，负反馈）
                if let Some(ref l) = learning {
                    let result_json = serde_json::json!({
                        "status": "failed",
                        "error": e.to_string(),
                        "total_time_ms": total_time_ms,
                    });
                    crate::commands::opc_learning_hook::try_auto_learn_workflow(
                        &wid,
                        &result_json,
                        l,
                        app_data_dir.as_deref(),
                    )
                    .await;
                }
            },
        }
        _guard.finish();
    });

    execution_id
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "暂停工作流执行")]
#[tauri::command]
pub async fn pause_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
    engine.pause(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(true)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "恢复工作流执行")]
#[tauri::command]
pub async fn resume_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
    engine.resume(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(true)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "取消工作流执行")]
#[tauri::command]
pub async fn cancel_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
    engine.cancel(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(true)
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取工作流执行状态")]
#[tauri::command]
pub async fn get_workflow_execution_status(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<ExecutionStatusResponse, String> {
    let engine = &*state.work_engine;
    let status = engine.get_status(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(ExecutionStatusResponse {
        execution_id: status.execution_id,
        workflow_id: status.workflow_id,
        status: status.status.to_string(),
        current_node_id: status.current_node_id,
        total_time_ms: status.total_time_ms,
        node_count: status.node_records.len(),
        node_records: status.node_records.into_iter().map(NodeRecordResponse::from).collect(),
        variables: serde_json::to_value(&status.variables).unwrap_or(serde_json::json!({})),
        parent_execution_id: status.parent_execution_id,
    })
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出工作流执行记录")]
#[tauri::command]
pub async fn list_workflow_executions(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<ExecutionSummaryResponse>, String> {
    let engine = &*state.work_engine;
    let executions = engine.list_executions(&workflow_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(executions.into_iter().map(ExecutionSummaryResponse::from).collect())
}

// ── 可视化工作流节点执行 ──

/// P1-14: 节点类型白名单。**仅允许这些节点类型通过 IPC 直接触发执行**，
/// 其余类型（agent / databaseQuery / storage / email / notification /
/// approval / webhookSend / debate / fallback / llmClassifier /
/// documentParser / vectorRetrieve 等）必须走完整的 `run_workflow` 流程，
/// 不能被可视化调试接口"借壳"执行。
///
/// 节点分两类：
/// - **纯函数式节点**（trigger/end/logging/validation/dataTransformer/switch/
///   merge/delay/aggregator）—— 单次 execute 不会写库、发网络、占资源。
/// - **单步调试增强节点**（llm/tool/httpRequest/code/subWorkflow）—— 原本必须
///   走完整 run_workflow 流程；现开放给可视化调试接口，但要求执行器在
///   `context.dry_run = true` 时短路返回模拟输出，避免真实副作用（发网络请求、
///   调用 LLM、执行子工作流等）。
const DEBUGGABLE_NODE_TYPES: &[&str] = &[
    // 纯函数式节点
    "trigger",
    "end",
    "logging",
    "validation",
    "dataTransformer",
    "switch",
    "merge",
    "delay",
    "aggregator",
    // 单步调试增强节点（执行器需支持 dry_run 短路）
    "llm",
    "tool",
    "httpRequest",
    "code",
    "subWorkflow",
];

/// P1-14: input 大小上限（bytes），超过直接拒绝 —— 防止通过 input 注入
/// 巨大 JSON 触发 OOM。
const MAX_DEBUG_INPUT_BYTES: usize = 64 * 1024;

#[agent_command(domain = workflow, safety = Caution, call_mode = Manual, description = "执行单个工作流节点")]
#[tauri::command]
pub async fn execute_workflow_node(
    state: State<'_, AppState>,
    execution_id: String,
    node_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // P1-14: 限制 input 大小
    let serialized_len = serde_json::to_string(&node_json).map(|s| s.len()).unwrap_or(0);
    if serialized_len > MAX_DEBUG_INPUT_BYTES {
        return Err(format!(
            "node_json 超过大小限制 ({} > {})",
            serialized_len, MAX_DEBUG_INPUT_BYTES
        ));
    }

    let node: axagent_harness::workflow_types::WorkflowNode =
        serde_json::from_value(node_json).map_err(|e| format!("节点 JSON 解析失败: {}", e))?;

    // P1-14: 节点类型白名单校验 —— 拒绝执行危险节点类型
    let node_type_str = axagent_runtime::work_engine::node_type_of(&node);
    if !DEBUGGABLE_NODE_TYPES.contains(&node_type_str) {
        return Err(format!(
            "节点类型 '{}' 不在可调试白名单内（仅允许：{}）",
            node_type_str,
            DEBUGGABLE_NODE_TYPES.join(", ")
        ));
    }

    let engine = &*state.work_engine;
    // P1-14: 校验 execution_id 归属 —— 防止任意调用方探测他人工作流
    let status = engine.get_status(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    if status.workflow_id.is_empty() {
        return Err(ErrorResponse::err(work_engine_err::EXECUTION_NOT_FOUND));
    }

    match engine.execute_node(&node, &status).await {
        Ok(output) => serde_json::to_value(output).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        }),
        // C-3: 迁移到 ErrorResponse，节点执行错误归类为 Unrecoverable
        Err(e) => Err(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        )
        .to_string()),
    }
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateOnly, description = "列出节点执行器类型")]
#[tauri::command]
pub async fn list_node_executor_types(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let engine = &*state.work_engine;
    Ok(engine.registered_executor_types().await.into_iter().map(String::from).collect())
}

// ── Debug Commands ──

#[agent_command(domain = workflow, safety = Caution, call_mode = Manual, description = "调试运行工作流")]
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
    use axagent_dao::repo::workflow_template;

    let db = state.harness.db();
    let template = workflow_template::get_workflow_template(db, &template_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| format!("Template {} not found", template_id))?;

    let nodes: Vec<axagent_harness::workflow_types::WorkflowNode> =
        serde_json::from_str(&template.nodes).map_err(|e| format!("节点解析失败: {}", e))?;
    for (i, n) in nodes.iter().enumerate() {
        let typ = axagent_rt_workflow::work_engine::node_executor_trait::node_type_name(n);
        tracing::info!(i, node_id = %n.base_id(), node_type = typ, "deserialized node");
    }
    let edges: Vec<axagent_harness::workflow_types::WorkflowEdge> =
        serde_json::from_str(&template.edges).map_err(|e| format!("边解析失败: {}", e))?;

    let variables: Vec<axagent_harness::workflow_types::Variable> = template
        .variables
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("变量解析失败: {}", e))?
        .unwrap_or_default();

    let input_schema: Option<axagent_harness::workflow_types::JsonSchema> = template
        .input_schema
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("input_schema 解析失败: {}", e))?;
    let output_schema: Option<axagent_harness::workflow_types::JsonSchema> = template
        .output_schema
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("output_schema 解析失败: {}", e))?;

    let engine = state.work_engine.clone();
    let workflow = engine.create_workflow(&template.name, nodes, edges).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
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
    let engine_for_progress = engine.clone();
    let progress_cb: axagent_runtime::work_engine::ProgressCallback = std::sync::Arc::new(
        move |evt| {
            let app = app_clone.clone();
            let node_id = evt.node_id.clone();
            let status = evt.status.clone();
            let total = evt.total_nodes;
            let completed = evt.completed_nodes;
            let wf_id = wid_for_progress.clone();
            let exec_id = evt.execution_id.clone().unwrap_or_else(|| eid_for_progress.clone());
            let eng = engine_for_progress.clone();
            Box::pin(async move {
                // ── 轻量级节点状态事件（实时）──
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

                // ── 全量状态同步事件（消除 2s 轮询依赖）──
                // 从引擎获取当前完整执行状态，序列化后发送
                if let Ok(full_state) = eng.get_status(&exec_id).await {
                    let node_records: Vec<serde_json::Value> = full_state
                        .node_records
                        .into_iter()
                        .map(|nr| {
                            serde_json::json!({
                                "node_id": nr.node_id,
                                "node_type": nr.node_type,
                                "node_name": nr.node_name,
                                "status": nr.status,
                                "input": nr.input,
                                "output": nr.output,
                                "execution_time_ms": nr.execution_time_ms,
                                "error": nr.error,
                                "started_at": nr.started_at,
                                "completed_at": nr.completed_at,
                                "sub_workflow_id": nr.sub_workflow_id,
                            })
                        })
                        .collect::<Vec<_>>();
                    let _ = app.emit(
                        "workflow:state-changed",
                        serde_json::json!({
                            "execution_id": full_state.execution_id,
                            "workflow_id": full_state.workflow_id,
                            "status": full_state.status.to_string(),
                            "current_node_id": full_state.current_node_id,
                            "total_time_ms": full_state.total_time_ms,
                            "node_count": node_records.len(),
                            "node_records": node_records,
                            "variables": serde_json::to_value(&full_state.variables).unwrap_or(serde_json::json!({})),
                        }),
                    );
                }
            })
        },
    );

    let wid = workflow_id.clone();
    let eid = execution_id.clone();
    let app_for_completion = app.clone();
    let app_for_panic = app_for_completion.clone();
    let wid_for_panic = wid.clone();
    let eid_for_panic = eid.clone();
    tokio::spawn(async move {
        // 兜底：panic / 早退路径上 emit execution-completed failed 事件,
        // 前端能感知 workflow 异常退出, 不会卡在 running
        let _guard = SpawnGuard::new("debug_run_workflow", move || {
            tracing::error!(
                "[debug_run_workflow] PANIC guard fired for workflow={}",
                wid_for_panic
            );
            let _ = app_for_panic.emit(
                "workflow:execution-completed",
                serde_json::json!({
                    "workflow_id": wid_for_panic,
                    "execution_id": eid_for_panic,
                    "status": "failed",
                    "total_time_ms": 0,
                    "error": "Internal panic during workflow execution",
                }),
            );
        });
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
        _guard.finish();
    });

    Ok(execution_id)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "设置工作流断点")]
#[tauri::command]
pub async fn set_workflow_breakpoints(
    state: State<'_, AppState>,
    node_ids: Vec<String>,
    execution_id: Option<String>,
) -> Result<bool, String> {
    let bp: std::collections::HashSet<String> = node_ids.into_iter().collect();
    if let Some(eid) = execution_id {
        state.work_engine.set_breakpoints_for_execution(&eid, bp).await;
    } else {
        state.work_engine.set_breakpoints(bp).await;
    }
    Ok(true)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "恢复断点执行")]
#[tauri::command]
pub async fn resume_workflow_breakpoint(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    state.work_engine.resume_breakpoints(&execution_id).await;
    Ok(true)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "单步执行断点")]
#[tauri::command]
pub async fn step_workflow_breakpoint(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    state.work_engine.step_breakpoint(&execution_id).await;
    Ok(true)
}

// ── Loop 节点人工审查 resume ──────────────────────────────────────────

/// 前端在人工审查（审批、修订 iteratee）后调用此 command 唤醒被挂起的 Loop 节点。
///
/// - `approved = true`  → 继续迭代，LoopExecutor 从 checkpoint.cursor 继续
/// - `approved = false` → 取消整个 execution（复用 `cancel_workflow_execution` 路径）
/// - `modified_iteratee` + `iteratee_var` → 可选地把当前迭代的 iteratee 改写成
///   新值，body 节点在 resume 后看到的就是修改后的版本
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "恢复循环迭代")]
#[tauri::command]
pub async fn resume_loop_iteration(
    state: State<'_, AppState>,
    execution_id: String,
    node_id: String,
    decision: serde_json::Value,
) -> Result<bool, String> {
    use axagent_runtime::work_engine::LoopResumeDecision;
    let decision: LoopResumeDecision =
        serde_json::from_value(decision).map_err(|e| format!("decision 解析失败: {e}"))?;
    state.work_engine.resume_loop_iteration(&execution_id, &node_id, decision).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;
    Ok(true)
}

/// 前端订阅某次执行的 partial_result 流式事件（每次 Loop 迭代完成一条）。
/// 返回 broadcast::Receiver 的订阅句柄；调用方用 `invoke` 拿到的是
/// `(Vec<PartialResultEvent>, ReceiverId)` 形式的事件流。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "加载循环检查点")]
#[tauri::command]
pub async fn load_loop_checkpoint(
    state: State<'_, AppState>,
    execution_id: String,
    node_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let cp =
        state.work_engine.load_loop_checkpoint(&execution_id, &node_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    cp.map(serde_json::to_value).transpose().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Approval (HITL) Commands ──────────────────────────────────────────

/// 列出所有待审批的工作流审批请求。
/// 可选按 execution_id 过滤。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出待审批请求")]
#[tauri::command]
pub async fn list_pending_approvals(
    state: State<'_, AppState>,
    execution_id: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.harness.db();
    // 先处理超时自动裁决
    let now_ms = chrono::Utc::now().timestamp_millis();
    let _ = axagent_dao::repo::workflow_approval::auto_resolve_timeouts(db, now_ms)
        .await
        .map_err(|e| format!("超时裁决处理失败: {}", e));

    let records =
        axagent_dao::repo::workflow_approval::list_pending_approvals(db, execution_id.as_deref())
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

    records
        .into_iter()
        .map(|r| {
            serde_json::to_value(serde_json::json!({
                "id": r.id,
                "execution_id": r.execution_id,
                "node_id": r.node_id,
                "workflow_id": "",
                "title": r.title,
                "message": r.message,
                "status": r.status,
                "approver": r.approver,
                "timeout_secs": r.timeout_secs,
                "expires_at": r.expires_at,
                "created_at": r.created_at,
            }))
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

/// 批准或拒绝一个审批请求。
/// 决策后恢复对应工作流的执行。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "批准或拒绝审批")]
#[tauri::command]
pub async fn resume_approval(
    state: State<'_, AppState>,
    approval_id: String,
    decision: String,
    decided_by: Option<String>,
    note: Option<String>,
) -> Result<bool, String> {
    let db = state.harness.db();
    let engine = &*state.work_engine;

    // 1. 读取审批记录
    let record = axagent_dao::repo::workflow_approval::get_approval_by_id(db, &approval_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| format!("审批记录 {} 不存在", approval_id))?;

    if record.status != "pending" {
        return Err(format!(
            "审批记录 {} 状态不是 pending（当前: {}）",
            approval_id, record.status
        ));
    }

    let is_approved = decision == "approved" || decision == "approve";

    // 2. 更新审批记录
    axagent_dao::repo::workflow_approval::resolve_approval(
        db,
        &approval_id,
        if is_approved { "approved" } else { "rejected" },
        decided_by.as_deref(),
        note.as_deref(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 3. 恢复工作流执行
    // 修复 P0-1：审批决策必须注入节点结果（result:true/false）驱动条件边，
    // 拒绝不再取消整个工作流——走 approval 的 false 分支（end）正常收尾。
    let execution_id = &record.execution_id;
    let node_id = &record.node_id;
    let approval_result = serde_json::json!({
        "status": if is_approved { "approved" } else { "rejected" },
        "result": is_approved,
        "message": record.message,
    });
    let _ = engine
        .update_node_status_for_execution(
            execution_id,
            node_id,
            NodeStatus::Completed,
            Some(approval_result),
            None,
            None,
        )
        .await;
    engine.resume_breakpoints(execution_id).await;

    Ok(true)
}

/// 取消（撤回）一个审批请求。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "取消审批请求")]
#[tauri::command]
pub async fn cancel_approval(
    state: State<'_, AppState>,
    approval_id: String,
) -> Result<bool, String> {
    let db = state.harness.db();
    let engine = &*state.work_engine;

    let record = axagent_dao::repo::workflow_approval::get_approval_by_id(db, &approval_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| format!("审批记录 {} 不存在", approval_id))?;

    if record.status != "pending" {
        return Err(format!("审批记录 {} 状态不是 pending", approval_id));
    }

    axagent_dao::repo::workflow_approval::resolve_approval(
        db,
        &approval_id,
        "cancelled",
        None,
        None,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 取消对应的工作流
    let _ = engine.cancel(&record.execution_id).await;

    Ok(true)
}

// ── 崩溃恢复命令 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PausedExecutionInfo {
    pub execution_id: String,
    pub workflow_id: String,
    pub snapshot: serde_json::Value,
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出所有暂停状态的工作流执行（用于崩溃后恢复）")]
#[tauri::command]
pub async fn list_paused_workflow_executions(
    state: State<'_, AppState>,
) -> Result<Vec<PausedExecutionInfo>, String> {
    let engine = &*state.work_engine;
    let records = engine.list_paused_executions().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(records
        .into_iter()
        .map(|(execution_id, workflow_id, snapshot)| PausedExecutionInfo {
            execution_id,
            workflow_id,
            snapshot,
        })
        .collect())
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "从快照恢复指定的暂停工作流执行")]
#[tauri::command]
pub async fn recover_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
    engine.recover_execution(&execution_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(true)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "批量恢复所有暂停状态的工作流执行")]
#[tauri::command]
pub async fn recover_all_paused_workflow_executions(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let engine = &*state.work_engine;
    let recovered = engine.recover_all_paused_executions().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(recovered)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "取消所有暂停的工作流执行（放弃恢复）")]
#[tauri::command]
pub async fn cancel_all_paused_workflow_executions(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let engine = &*state.work_engine;
    engine.cancel_all_paused_executions().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(true)
}
