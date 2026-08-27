// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;
use axagent_harness::repo_dtos::WorkflowExecutionData;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::work_engine as work_engine_err;
use crate::commands::spawn_guard::SpawnGuard;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
    pub total_time_ms: Option<i64>,
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

/// 工作流运行时工具的执行反馈接收器 —— 把工具真实执行成败回写到
/// `workflow_tools` 表的 `usage_count` / `success_rate`。
///
/// 模式与 `EvolutionFeedbackSinkImpl` 对称：`record` 是同步回调（工具执行
/// 完成瞬间调用），内部通过当前 tokio runtime 异步落库，不跨 await 持锁。
/// `tool_id` 即 `GeneratedToolAdapter` 上报的 `tool.name`（= workflow_tools.tool_name），
/// 按 `(workflow_id, tool_name)` 定位回写。
struct WorkflowToolFeedbackSink {
    db: sea_orm::DatabaseConnection,
    workflow_id: String,
}

impl axagent_harness::workflow_evolution::ExecutionFeedbackSink for WorkflowToolFeedbackSink {
    fn record(&self, _conversation_id: Option<&str>, tool_id: &str, success: bool) {
        let db = self.db.clone();
        let workflow_id = self.workflow_id.clone();
        let tool_name = tool_id.to_string();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let now = chrono::Utc::now().timestamp_millis();
                if let Err(e) = axagent_dao::repo::workflow_tool::record_execution_feedback_by_name(
                    &db,
                    &workflow_id,
                    &tool_name,
                    success,
                    now,
                )
                .await
                {
                    tracing::debug!("[workflow_tool] 执行反馈回写失败 {tool_name}: {e}");
                }
            });
        } else {
            tracing::debug!("[workflow_tool] 无 runtime 上下文，跳过反馈回写");
        }
    }
}

/// 启动工作流前，从 `workflow_tools` 表加载 active 工具并注册到运行时注册表。
///
/// 每个工具构造 `GeneratedTool` → `GeneratedToolAdapter`（注入反馈 sink）→
/// `register_runtime_tool`，来源标记 `workflow:{workflow_id}`。注册幂等：同名
/// 已存在（重复注册/重启后已在注册表）时跳过，不覆盖既有工具。注册失败仅
/// warn，不阻塞工作流启动（工具节点执行时若缺失会走 ToolExecutor 的"未注册"
/// 错误路径，由上层发现闭环处理）。
async fn register_workflow_runtime_tools(state: &AppState, workflow_id: &str) {
    use axagent_dao::repo::workflow_tool as wt_repo;
    use axagent_harness::trajectory_types::{EvolutionArtifactKind, GeneratedTool};
    use axagent_tools::generated_tool::GeneratedToolAdapter;
    use std::sync::Arc;

    let db = state.harness.db();
    let Ok(tools) = wt_repo::list_by_workflow(db, workflow_id, Some(wt_repo::STATUS_ACTIVE)).await
    else {
        return;
    };
    if tools.is_empty() {
        return;
    }

    // 反馈 sink 与 db 克隆：每个 adapter 注入同一 sink（sink 内部按 tool_name 定位）
    let sink: Arc<dyn axagent_harness::workflow_evolution::ExecutionFeedbackSink> =
        Arc::new(WorkflowToolFeedbackSink { db: db.clone(), workflow_id: workflow_id.to_string() });

    let mut registry = state.local_tool_registry.lock().await;
    for t in tools {
        let Some(code) = t.code.clone() else {
            continue;
        };
        let kind = if t.tool_type == wt_repo::TYPE_WORKFLOW_DAG {
            EvolutionArtifactKind::WorkflowDag
        } else {
            EvolutionArtifactKind::RhaiScript
        };
        let gen_tool = GeneratedTool::with_artifact_kind(
            &t.tool_name,
            &code,
            t.description.as_deref().unwrap_or(""),
            kind,
        );
        let adapter =
            Arc::new(GeneratedToolAdapter::new(gen_tool).with_feedback_sink(sink.clone()));
        match registry.register_runtime_tool(adapter, format!("workflow:{workflow_id}")) {
            Ok(()) => {
                tracing::info!(
                    "[workflow_tool] 已注册运行时工具 '{}'（workflow:{}）",
                    t.tool_name,
                    workflow_id
                );
            },
            Err(e) => {
                tracing::warn!("[workflow_tool] 注册工具 '{}' 跳过: {}", t.tool_name, e.message);
            },
        }
    }
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "启动工作流执行")]
#[tauri::command]
pub async fn start_workflow_execution(
    state: State<'_, AppState>,
    workflow_id: String,
    input: serde_json::Value,
) -> Result<String, String> {
    // 启动前加载并注册工作流运行时工具（幂等；失败不阻塞启动）
    register_workflow_runtime_tools(&state, &workflow_id).await;

    let engine = &*state.work_engine;
    engine.start_workflow(&workflow_id, input, None).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
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
    // 先处理超时自动裁决，并按策略联动引擎（拒→cancel / 放→resume）
    let now_ms = chrono::Utc::now().timestamp_millis();
    let engine = &*state.work_engine;
    let resolved = axagent_dao::repo::workflow_approval::auto_resolve_timeouts(db, now_ms)
        .await
        .map_err(|e| format!("超时裁决处理失败: {}", e))?;
    for res in resolved {
        match res {
            axagent_dao::repo::workflow_approval::TimeoutResolution::Rejected {
                execution_id: eid,
                ..
            } => {
                tracing::warn!(execution_id = %eid, "[Approval] 超时默认拒绝，取消工作流");
                // 拒绝：永久停摆。cancel 本身幂等，失败仅告警不阻塞列表
                if let Err(e) = engine.cancel(&eid).await {
                    tracing::error!(execution_id = %eid, error = %e, "[Approval] 超时拒绝后取消工作流失败");
                }
            },
            axagent_dao::repo::workflow_approval::TimeoutResolution::Approved {
                execution_id: eid,
                ..
            } => {
                tracing::info!(execution_id = %eid, "[Approval] 超时自动批准，恢复工作流");
                engine.resume_breakpoints(&eid).await;
            },
        }
    }

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
                "executionId": r.execution_id,
                "nodeId": r.node_id,
                "workflowId": "",
                "title": r.title,
                "message": r.message,
                "status": r.status,
                "approver": r.approver,
                "timeoutAction": r.timeout_action,
                "timeoutSecs": r.timeout_secs,
                "expiresAt": r.expires_at,
                "createdAt": r.created_at,
                "decision": r.decision,
                "comment": r.comment,
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
    let execution_id = &record.execution_id;
    if is_approved {
        engine.resume_breakpoints(execution_id).await;
    } else {
        // 拒绝时取消工作流
        let _ = engine.cancel(execution_id).await;
    }

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
