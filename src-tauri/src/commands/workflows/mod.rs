// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::agent::skill_execution::{self, SkillStep};
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::agent as agent_err;
use crate::commands::spawn_guard::SpawnGuard;

use axagent_agent_macro::agent_command;
use axagent_dao::repo::{conversation, message, workflow_template};
use axagent_harness::types::{MessageRole, UpdateConversationInput};
use axagent_harness::workflow_types::{Variable, Workflow};
use axagent_runtime::work_engine::{ProgressCallback, StepProgressEvent, node_type_of};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
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
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "创建新工作流DAG")]
#[tauri::command]
pub async fn workflow_create(
    app_state: State<'_, AppState>,
    request: WorkflowCreateRequest,
) -> Result<WorkflowCreateResponse, String> {
    let nodes: Vec<axagent_harness::workflow_types::WorkflowNode> =
        request.nodes.into_iter().filter_map(|n| serde_json::from_value(n).ok()).collect();
    let edges: Vec<axagent_harness::workflow_types::WorkflowEdge> =
        request.edges.into_iter().filter_map(|e| serde_json::from_value(e).ok()).collect();

    let workflow =
        app_state.work_engine.create_workflow(&request.name, nodes, edges).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(WorkflowCreateResponse {
        workflow_id: workflow.id.clone(),
        name: workflow.name,
        step_count: workflow.nodes.len(),
    })
}

/// 执行工作流（含 LLM 步骤执行）
///
/// `max_concurrent`：最大并发节点数（None 使用默认值 3）。
/// 暴露给前端用于按场景调节吞吐：CPU 密集型工作流降低并发避免压垮本机，
/// IO 密集型工作流可提高并发缩短端到端时延。
///
/// `conversation_id` / `input`：对话驱动模式参数（可选）。
/// 传入后，工作流执行期间会把步骤事件（`workflow_start` / `workflow_step_start` /
/// `workflow_step_complete` / `workflow_step_error`）以带 `type` 的 `agent-stream-text`
/// 事件桥接到对应对话；执行完成时结果写回 assistant 消息并 emit `agent-done` +
/// `workflow-complete`，失败时 emit `agent-error` + `workflow-complete(success=false)`。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "执行工作流")]
#[tauri::command]
pub async fn workflow_execute(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    workflow_id: String,
    model_id: Option<String>,
    provider_id: Option<String>,
    mut variables: Option<Vec<axagent_harness::workflow_types::Variable>>,
    max_concurrent: Option<usize>,
    conversation_id: Option<String>,
    input: Option<serde_json::Value>,
    // 认知编排决策标签（JSON 对象）：由 cognitive_query 透传，持久化到本次执行的 assistant 消息
    decision: Option<serde_json::Value>,
) -> Result<String, String> {
    // 验证工作流存在，并预构建节点元信息（node_id → (title, node_type)）。
    // 供 progress_callback 组装步骤事件使用；回调内不再访问 engine 锁，
    // 避免 progress_callback 与 run_workflow 主循环产生死锁。
    let workflow = app_state
        .work_engine
        .get_workflow(&workflow_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| ErrorResponse::err(agent_err::WORKFLOW_NOT_FOUND))?;

    let goal_map: HashMap<String, (String, String)> = workflow
        .nodes
        .iter()
        .map(|n| {
            let base = n.base();
            let kind = node_type_of(n).to_string();
            (base.id.clone(), (base.title.clone(), kind))
        })
        .collect();

    // 工具解析器已由 init/services.rs 在启动期注入（含 builtin / mcp / workflow:: 三种来源），
    // 此处不再 set_tool_resolver 覆盖——否则会静默丢弃 init 阶段注入的 workflow:: 解析。
    let _ = app_state.local_tool_registry; // 保留依赖项以维持签名稳定

    let db = app_state.harness.db().clone();
    let engine = app_state.work_engine.clone();
    let wid = workflow_id.clone();
    let app_for_emit = app.clone();
    let app_for_panic = app_for_emit.clone();
    let wid_for_panic = wid.clone();
    tokio::spawn(async move {
        // 兜底：panic / 早退路径上 emit execution-completed failed 事件
        // WF-P0-2: emit 字段统一为 { workflow_id, execution_id, status, total_time_ms, error? }
        // 与前端 workEngineStore.ts 期望对齐
        let _guard = SpawnGuard::new("workflow_run", move || {
            tracing::error!("[workflow_run] PANIC guard fired for workflow={}", wid_for_panic);
            let _ = app_for_panic.emit(
                "workflow:execution-completed",
                serde_json::json!({
                    "workflow_id": wid_for_panic,
                    "execution_id": null,
                    "status": "failed",
                    "total_time_ms": 0,
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
        if let Some(mc) = max_concurrent {
            // 合理下限保护：至少 1 个并发，避免 0 导致死锁。
            let clamped = mc.max(1);
            opts = opts.with_max_concurrent(clamped);
        }
        let user_input_ref = input.clone();
        opts.input = input;

        // ── 对话驱动模式：自动加载模板变量 ──
        // 当 conversation_id 存在时，从会话获取 workflow_template_id，
        // 从数据库加载模板变量定义（如 stock_code 等），合并到执行选项。
        // 这样即使前端没有显式传递 variables，模板定义的变量也能被正确注入。
        if let Some(ref conv_id) = conversation_id {
            if let Ok(conv) = conversation::get_conversation(&db, conv_id).await {
                if let Some(ref template_id) = conv.workflow_template_id {
                    if let Ok(Some(template_model)) =
                        workflow_template::get_workflow_template(&db, template_id).await
                    {
                        let template_data =
                            workflow_template::template_model_to_data(&template_model);
                        let template_vars = template_data.variables;
                        if !template_vars.is_empty() {
                            // 前端显式传入的变量优先级高于模板默认值
                            let mut merged = template_vars;
                            if let Some(ref mut front_vars) = variables {
                                for fv in front_vars {
                                    if let Some(pos) = merged.iter().position(|m| m.name == fv.name)
                                    {
                                        merged[pos] = fv.clone();
                                    } else {
                                        merged.push(fv.clone());
                                    }
                                }
                            }
                            opts = opts.with_variables(merged);
                        }
                    }
                }
            }
        }

        // ── 参数提取：从用户输入中提取结构化变量 ──
        // 两种模式：
        //   1) JSON 对象输入：直接将 key-value 作为模板变量覆盖（如 {"stock_code": "301302"}）
        //   2) 纯文本输入：尝试用内置规则提取常见参数（股票代码、字数、章节数等）
        if let Some(ref user_input) = user_input_ref {
            let existing_vars = opts.variables.get_or_insert_with(Vec::new);

            // 模式 1：JSON 对象 → 直接提取 key-value
            if let serde_json::Value::Object(map) = user_input {
                for (key, value) in map {
                    // 跳过系统保留 key
                    if key == "input" || key == "user_message" {
                        continue;
                    }
                    // 若模板变量中已定义此 key，则用用户值覆盖
                    if let Some(pos) = existing_vars.iter().position(|v| v.name == *key) {
                        existing_vars[pos].value = value.clone();
                    } else {
                        // 否则新增为动态变量
                        existing_vars.push(axagent_harness::workflow_types::Variable {
                            name: key.clone(),
                            var_type: "string".to_string(),
                            value: value.clone(),
                            description: None,
                            is_secret: false,
                        });
                    }
                }
            }
            // 模式 2：纯文本 → 用内置规则提取
            else if let Some(text) = user_input.as_str() {
                let existing_vars = opts.variables.get_or_insert_with(Vec::new);
                extract_params_from_text(text, existing_vars);
            }
        }

        // 前端显式传入的变量（若非对话驱动模式或模板无变量时生效）
        if let Some(vars) = variables {
            if opts.variables.is_none() {
                opts = opts.with_variables(vars);
            }
        }

        // ── 对话驱动模式：创建 assistant 占位消息 + 桥接步骤事件 ──
        // 步骤文本累积缓冲：与前端实时事件同格式，最终与结果一并写入 DB 消息，
        // 保证 fetchMessages 回读的内容与对话区显示一致（步骤保留 + 结果在尾部）。
        let steps_buf: Arc<parking_lot::Mutex<String>> =
            Arc::new(parking_lot::Mutex::new(String::new()));
        let assistant_message_id: Option<String> = if let Some(conv) = &conversation_id {
            match message::create_message_with_parts(
                &db,
                conv,
                MessageRole::Assistant,
                "",
                &[],
                None,
                0,
                None,
                None,
            )
            .await
            {
                Ok(m) => {
                    // 前端用真实 ID 替换流式占位消息
                    let _ = app_for_emit.emit(
                        "agent-message-id",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": m.id,
                        }),
                    );
                    let mut buf = steps_buf.lock();
                    buf.push_str(&format!("\n[Workflow Started: {}]\n", wid));

                    let _ = app_for_emit.emit(
                        "agent-stream-text",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": m.id,
                            "type": "workflow_start",
                            "workflowId": wid.clone(),
                        }),
                    );
                    Some(m.id)
                },
                Err(e) => {
                    tracing::warn!("[workflow_run] 创建 assistant 消息失败: {}", e);
                    None
                },
            }
        } else {
            None
        };

        // 持久化认知编排决策标签到本次执行的 assistant 消息（若存在）
        if let (Some(msg_id), Some(decision)) = (&assistant_message_id, &decision) {
            if let Err(e) = message::update_message_decision(&db, msg_id, Some(decision)).await {
                tracing::warn!("[workflow_run] 写入决策标签失败: {}", e);
            }
        }

        if let (Some(conv), Some(msg_id)) = (&conversation_id, &assistant_message_id) {
            let cb_app = app_for_emit.clone();
            let cb_conv = conv.clone();
            let cb_msg = msg_id.clone();
            let cb_goals = goal_map.clone();
            let cb_steps = steps_buf.clone();
            let progress_cb: ProgressCallback = Arc::new(move |evt: StepProgressEvent| {
                let app = cb_app.clone();
                let conv = cb_conv.clone();
                let msg = cb_msg.clone();
                let goals = cb_goals.clone();
                let steps = cb_steps.clone();
                Box::pin(async move {
                    let (title, kind) = goals
                        .get(&evt.node_id)
                        .cloned()
                        .unwrap_or_else(|| (evt.node_id.clone(), "node".to_string()));
                    match evt.status.as_str() {
                        "running" => {
                            let mut buf = steps.lock();
                            buf.push_str(&format!("\n[Step Start] {}: {}\n", kind, title));

                            let _ = app.emit(
                                "agent-stream-text",
                                serde_json::json!({
                                    "conversationId": conv,
                                    "assistantMessageId": msg,
                                    "type": "workflow_step_start",
                                    "stepId": evt.node_id,
                                    "stepGoal": title,
                                    "agentRole": kind,
                                }),
                            );
                        },
                        "completed" => {
                            let mut buf = steps.lock();
                            buf.push_str(&format!("[Step Complete] {}: ✓\n", title));

                            let _ = app.emit(
                                "agent-stream-text",
                                serde_json::json!({
                                    "conversationId": conv,
                                    "assistantMessageId": msg,
                                    "type": "workflow_step_complete",
                                    "stepId": evt.node_id,
                                    "stepGoal": title,
                                    "result": "✓",
                                }),
                            );
                        },
                        "failed" => {
                            let mut buf = steps.lock();
                            buf.push_str(&format!("[Step Error] {}: 节点执行失败\n", evt.node_id));

                            let _ = app.emit(
                                "agent-stream-text",
                                serde_json::json!({
                                    "conversationId": conv,
                                    "assistantMessageId": msg,
                                    "type": "workflow_step_error",
                                    "stepId": evt.node_id,
                                    "error": "节点执行失败",
                                }),
                            );
                        },
                        _ => {},
                    }
                })
            });
            opts = opts.with_progress_callback(progress_cb);
        }

        let started_at = std::time::Instant::now();
        match engine.run_workflow(&wid, opts).await {
            Ok(workflow) => {
                let total_time_ms = started_at.elapsed().as_millis() as u64;
                // run_workflow 不生成独立 execution_id，使用 workflow.id 作为标识
                let execution_id = workflow.id.clone();
                let status_str = format!("{:?}", workflow.status).to_lowercase();
                let _ = app_for_emit.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "execution_id": execution_id,
                        "status": status_str,
                        "total_time_ms": total_time_ms,
                    }),
                );

                // 对话驱动模式：步骤文本保留 + 结果追加尾部，写入 DB 与 agent-done
                if let (Some(conv), Some(msg_id)) = (&conversation_id, &assistant_message_id) {
                    let output_text = format_workflow_output(&workflow);
                    let steps_text = steps_buf.lock().clone();
                    let full_text = if steps_text.trim().is_empty() {
                        output_text.clone()
                    } else {
                        format!(
                            "{}{}{}",
                            steps_text.trim_end(),
                            if output_text.trim().is_empty() {
                                ""
                            } else {
                                "\n\n"
                            },
                            output_text
                        )
                    };
                    // DB 落库完整内容（步骤 + 结果），保证 fetchMessages 回读与显示一致
                    let _ = message::update_message_content(&db, msg_id, &full_text).await;
                    // agent-done 只带结果部分：前端在 workflow 场景追加到已流式的步骤事件尾部
                    let _ = app_for_emit.emit(
                        "agent-done",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": msg_id,
                            "text": output_text,
                            "thinking": serde_json::Value::Null,
                            "usage": serde_json::Value::Null,
                            "numTurns": serde_json::Value::Null,
                            "costUsd": serde_json::Value::Null,
                            "blocks": serde_json::Value::Null,
                        }),
                    );
                    let _ = app_for_emit.emit(
                        "workflow-complete",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": msg_id,
                            "workflowId": wid,
                            "success": true,
                        }),
                    );
                    let _ = conversation::update_conversation(
                        &db,
                        conv,
                        UpdateConversationInput {
                            workflow_status: Some(Some("completed".to_string())),
                            ..Default::default()
                        },
                    )
                    .await;
                }
            },
            Err(e) => {
                tracing::error!("[workflow] 执行失败: {}", e);
                let total_time_ms = started_at.elapsed().as_millis() as u64;
                let _ = app_for_emit.emit(
                    "workflow:execution-completed",
                    serde_json::json!({
                        "workflow_id": wid,
                        "execution_id": null,
                        "status": "failed",
                        "total_time_ms": total_time_ms,
                        "error": e.to_string(),
                    }),
                );

                // 对话驱动模式：失败事件 + 会话状态
                if let Some(conv) = &conversation_id {
                    let _ = app_for_emit.emit(
                        "agent-error",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": assistant_message_id,
                            "message": e.to_string(),
                        }),
                    );
                    let _ = app_for_emit.emit(
                        "workflow-complete",
                        serde_json::json!({
                            "conversationId": conv,
                            "assistantMessageId": assistant_message_id,
                            "workflowId": wid,
                            "success": false,
                        }),
                    );
                    let _ = conversation::update_conversation(
                        &db,
                        conv,
                        UpdateConversationInput {
                            workflow_status: Some(Some("failed".to_string())),
                            ..Default::default()
                        },
                    )
                    .await;
                }
            },
        }
        _guard.finish();
    });

    Ok(workflow_id)
}

/// 从用户纯文本中提取结构化参数的内置规则集。
///
/// 支持的提取模式：
/// - `stock_code`: A 股 6 位股票代码（如 301302、600519）
/// - `word_count`: "不超过 X 字"/"约 X 字"/"X 万字" 等字数限制
/// - `chapter_count`: "分 X 章"/"X 章" 等章节数
/// - `topic`: 从 "XX 题材"/"XX 主题" 中提取主题关键词
/// - `genre`: 体裁识别（小说/散文/诗歌等）
///
/// 提取后若 vars 中已有同名变量则覆盖，否则新增。
fn extract_params_from_text(text: &str, vars: &mut Vec<Variable>) {
    // 1. 股票代码：6 位纯数字，独立出现
    if let Ok(re) = Regex::new(r"\b(\d{6})\b") {
        if let Some(caps) = re.captures(text) {
            if let Some(code) = caps.get(1) {
                upsert_var(vars, "stock_code", code.as_str());
            }
        }
    }

    // 2. 字数限制：支持 "不超过50万字"/"约10万字"/"5万字" 等
    if let Ok(re) = Regex::new(r"(?:不超过|约|共计|总计)?(\d+(?:\.\d+)?)\s*(万?字)") {
        if let Some(caps) = re.captures(text) {
            let num = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("字");
            let value = if unit.contains("万") {
                format!("{}0000", num.replace('.', ""))
            } else {
                num.to_string()
            };
            upsert_var(vars, "word_count", &value);
        }
    }

    // 3. 章节数：支持 "分10章"/"共10章"/"10章进行"
    if let Ok(re) = Regex::new(r"(?:分|共|共计)?\s*(\d+)\s*章") {
        if let Some(caps) = re.captures(text) {
            if let Some(num) = caps.get(1) {
                upsert_var(vars, "chapter_count", num.as_str());
            }
        }
    }

    // 4. 题材/主题提取：识别 "海军题材"/"军旅题材"/"XX 题材"/"XX 主题"
    if let Ok(re) = Regex::new(r#"([\u4e00-\u9fa5A-Za-z]+?)(?:题材|主题|小说)"#) {
        if let Some(caps) = re.captures(text) {
            if let Some(topic) = caps.get(1) {
                let topic_val = topic.as_str().trim();
                if !topic_val.is_empty() && topic_val.len() >= 2 {
                    upsert_var(vars, "topic", topic_val);
                }
            }
        }
    }

    // 5. 体裁识别：小说/散文/诗歌/报告等
    if let Ok(re) = Regex::new(r#"(小说|散文|诗歌|报告文学|传记|剧本)"#) {
        if let Some(caps) = re.captures(text) {
            if let Some(genre) = caps.get(1) {
                upsert_var(vars, "genre", genre.as_str());
            }
        }
    }
}

/// 辅助函数：插入或覆盖变量
fn upsert_var(vars: &mut Vec<Variable>, name: &str, value: &str) {
    if let Some(pos) = vars.iter().position(|v| v.name == name) {
        vars[pos].value = Value::String(value.to_string());
    } else {
        vars.push(Variable {
            name: name.to_string(),
            var_type: "string".to_string(),
            value: Value::String(value.to_string()),
            description: None,
            is_secret: false,
        });
    }
}

/// 提取工作流执行结果文本（不含步骤清单）。
///
/// 步骤清单由 progress_callback 累积（与前端实时事件同格式），最终由调用方
/// 拼成「步骤 + 结果」写入 DB 消息；本函数只负责结果部分。
/// 优先级：`workflow.output`（EndNode 聚合）> 节点 results 聚合 > 状态兜底。
fn format_workflow_output(workflow: &Workflow) -> String {
    let mut out = String::new();
    if let Some(output) = &workflow.output {
        match output {
            Value::String(s) if !s.is_empty() => out.push_str(s),
            _ => {
                let pretty = serde_json::to_string_pretty(output).unwrap_or_default();
                if !pretty.is_empty() && pretty != "null" {
                    out.push_str(&pretty);
                }
            },
        }
    }
    if out.trim().is_empty() && !workflow.results.is_empty() {
        let mut parts: Vec<String> = Vec::new();
        for (node_id, val) in &workflow.results {
            let title = workflow
                .nodes
                .iter()
                .find(|n| n.base_id() == node_id)
                .map(|n| n.base().title.clone())
                .unwrap_or_else(|| node_id.clone());
            let text = serde_json::to_string_pretty(val).unwrap_or_default();
            if !text.is_empty() && text != "null" {
                parts.push(format!("【{}】\n{}", title, text));
            }
        }
        if !parts.is_empty() {
            out.push_str(&parts.join("\n\n"));
        }
    }
    if out.trim().is_empty() {
        out = format!("工作流执行完成（状态：{:?}）", workflow.status);
    }
    out
}

/// 获取工作流状态
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取工作流状态")]
#[tauri::command]
pub async fn workflow_get_status(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    let workflow = app_state.work_engine.get_workflow(&workflow_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    match workflow {
        Some(w) => Ok(serde_json::to_value(w).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?),
        None => Err(ErrorResponse::new(agent_err::WORKFLOW_NOT_FOUND).into()),
    }
}

/// 取消正在执行的工作流
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "取消正在执行的工作流")]
#[tauri::command]
pub async fn workflow_cancel(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    let workflow = app_state.work_engine.cancel_workflow(&workflow_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    serde_json::to_value(workflow).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 列出所有工作流
#[agent_command(domain = workflow, safety = Safe, call_mode = StateOnly, description = "列出所有工作流")]
#[tauri::command]
pub async fn workflow_list(app_state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let workflows = app_state.work_engine.list_workflows().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(workflows.into_iter().filter_map(|w| serde_json::to_value(w).ok()).collect())
}

/// 列出当前内存中所有活跃执行（status = running / paused）。
///
/// 可观测性用途：前端轮询此接口渲染"正在执行的工作流"列表，
/// 配合 `workflow_cancel_execution` 实现按 execution_id 取消。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateOnly, description = "列出当前活跃执行的工作流")]
#[tauri::command]
pub async fn workflow_list_active_executions(
    app_state: State<'_, AppState>,
) -> Result<Vec<Value>, String> {
    let active = app_state.work_engine.list_active_executions().await;
    Ok(active.into_iter().filter_map(|v| serde_json::to_value(v).ok()).collect())
}

/// 获取工作流步骤详情（用于 DAG 可视化）
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取工作流步骤详情")]
#[tauri::command]
pub async fn workflow_get_steps(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<Value>, String> {
    let workflow = app_state
        .work_engine
        .get_workflow(&workflow_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| ErrorResponse::err(agent_err::WORKFLOW_NOT_FOUND))?;
    Ok(workflow.nodes.iter().filter_map(|s| serde_json::to_value(s).ok()).collect())
}

/// 从对话工具执行记录获取工作流预览
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取对话工作流预览")]
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
