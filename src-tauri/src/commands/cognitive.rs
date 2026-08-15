// SPDX-License-Identifier: AGPL-3.0-only

//! 认知编排器 Tauri 命令集
//!
//! 认知编排器（路由工作流）是全局唯一被用户消息触发的工作流，位于用户输入框
//! 输入消息后触发，掌管未来执行任务的分支。
//!
//! # 架构
//! ```text
//! 用户输入框 ──> cognitive_query（统一入口命令）
//!                    │
//!                    ▼
//!       WorkEngine.run_workflow(cognitive_router_main)   ← 主 DAG 驱动
//!                    │  注入动态分类目录 __l1/__l2_categories
//!                    │  注入系统能力回调（system_* 节点 → CognitiveRouter）
//!                    ▼
//!            主 DAG（L1 子工作流 → L2 子工作流 → L3 子工作流）
//!                    │  L1 域路由 → L2 簇路由 → L3 RAR+图谱路由
//!                    ▼
//!             EndNode 输出 l3_result（含 execution_mode / candidates）
//!                    │
//!                    ▼
//!        Workflow → WorkEngine 执行 / Delegate → agent 执行
//!        Ask / Plan / Act → agent_query 模式决策
//! ```
//!
//! # 与 agent_query 的关系
//! cognitive_query 是用户消息的统一入口，先完成能力发现与路由决策；
//! agent_query 作为执行器承接 Delegate / Ask / Plan / Act 模式的实际执行。

use crate::AppState;
use crate::commands::agent::{AgentContextPayload, AgentOptions, AgentQueryRequest};
use crate::commands::error::{CommandError, ErrorCategory, ErrorResponse};
use crate::init::COGNITIVE_ROUTER_MAIN_ID;
use agent_macro::agent_command;
use axagent_harness::workflow_types::Variable;
use axagent_harness::{
    CandidateSummary, CapabilityDomain, CapabilityKind, ExecutionMode, ModeHint,
    PatternPromptGuard, PromptGuard, RouteStageRecord, RoutingDecisionV2,
};
use axagent_runtime::work_engine::{RunOptions, SubWorkflowCallback};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

// ── DTO 类型 ──────────────────────────────────────

/// 认知编排统一入口的请求
///
/// `input` 之外的字段均为执行参数，按 `execution_mode` 分发给对应执行器：
/// - `Workflow` 模式：`provider_id` / `model_id` / `max_concurrent` 透传给 WorkEngine
/// - `Delegate` / `Ask` / `Plan` / `Act` 模式：透传给 `agent_query`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveQueryRequest {
    /// 用户输入
    pub input: String,
    /// 目标会话 ID（执行必需）
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    /// 提供商 ID
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    /// 模型 ID
    #[serde(rename = "model_id")]
    pub model_id: Option<String>,
    /// Agent 画像 ID（Agent 执行模式透传）
    #[serde(rename = "agentProfileId")]
    pub agent_profile_id: Option<String>,
    /// 用户自定义系统提示（Agent 执行模式透传）
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    /// Web 搜索提供商 ID（Agent 执行模式透传）
    #[serde(rename = "searchProviderId")]
    pub search_provider_id: Option<String>,
    /// 前端注入的页面上下文（Agent 执行模式透传）
    #[serde(rename = "agentContext")]
    pub agent_context: Option<AgentContextPayload>,
    /// Agent 执行选项（禁用工具 / 活跃域等）
    pub options: Option<AgentOptions>,
    /// 工作流最大并发节点数（Workflow 模式透传）
    #[serde(rename = "maxConcurrent")]
    pub max_concurrent: Option<usize>,
    /// 用户意图提示（覆盖执行模式）：auto / ask / plan / act，缺省视为 auto。
    /// 前端手动选择的模式降级为意图提示，路由决策优先尊重但可被自动决策覆盖。
    #[serde(rename = "modeHint")]
    pub mode_hint: Option<String>,
    /// 强制目标能力 ID（Clarify 二次执行）：用户从澄清候选中选择后，跳过三层路由，
    /// 直接执行该能力（Workflow 类型 → WorkEngine；Agent 类型 → agent_query）。
    #[serde(rename = "forcedCapabilityId")]
    pub forced_capability_id: Option<String>,
}

/// 认知编排执行结果视图 — 路由决策落地的执行分支句柄
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CognitiveExecutionView {
    /// WorkEngine 执行工作流模板（命中 Workflow 能力）
    Workflow {
        #[serde(rename = "workflowId")]
        workflow_id: String,
        #[serde(rename = "executionId")]
        execution_id: String,
    },
    /// agent_query 执行（Delegate / Ask / Act 模式）
    Agent {
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "assistantMessageId")]
        assistant_message_id: String,
        /// 计划确认被拒绝时返回 "rejected"，正常执行时 None（透传 agent_query）
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    /// plan_generate 执行（Plan 模式）：已触发计划生成，前端监听 plan-generated 事件渲染 PlanCard
    Plan {
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "planId")]
        plan_id: String,
    },
    /// 意图澄清（Clarify 模式）：模糊命中（置信度 0.60 ~ 0.90），返回 Top2 候选交用户选择。
    /// 前端渲染候选卡片，用户选择后携带所选 capability_id 二次路由/执行。
    Clarify {
        #[serde(rename = "candidates")]
        candidates: Vec<CandidateSummary>,
    },
}

/// 单个路由阶段的对外视图
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStageView {
    pub stage: String,
    pub success: bool,
    pub confidence: f64,
    pub elapsed_ms: u64,
    pub summary: String,
}

impl From<&RouteStageRecord> for RouteStageView {
    fn from(r: &RouteStageRecord) -> Self {
        Self {
            stage: r.stage.as_str().to_string(),
            success: r.success,
            confidence: r.confidence,
            elapsed_ms: r.elapsed_ms,
            summary: r.summary.clone(),
        }
    }
}

/// 认知编排统一入口的响应
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveQueryResponse {
    /// 三层路由地址（确定性路径），如 "invest/stock_analysis/tech"
    pub route_path: String,
    /// 业务域
    pub domain: String,
    /// 功能集群
    pub cluster: String,
    /// 具体能力/工作流 ID
    pub capability_id: String,
    /// 路由置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 是否通过 LLM 兜底
    pub is_llm_fallback: bool,
    /// 是否触发熔断
    pub circuit_broken: bool,
    /// 熔断原因
    pub circuit_break_reason: Option<String>,
    /// 备选路径
    pub fallback_path: Option<String>,
    /// 候选列表（Top-K，仅 ID）
    pub candidates: Vec<String>,
    /// 候选摘要（Top-K，含名称/描述/置信度，Clarify 分支展示用）
    pub candidate_details: Vec<CandidateSummary>,
    /// 执行模式（ask / plan / act / workflow / delegate / parameter_extract / clarify）
    pub execution_mode: String,
    /// 各阶段执行记录
    pub stage_records: Vec<RouteStageView>,
    /// 总耗时（毫秒）
    pub total_elapsed_ms: u64,
    /// 执行分支结果（Workflow → WorkEngine；其余 → agent_query）
    pub execution: Option<CognitiveExecutionView>,
}

impl From<RoutingDecisionV2> for CognitiveQueryResponse {
    fn from(d: RoutingDecisionV2) -> Self {
        Self {
            route_path: d.route_path,
            domain: d.domain,
            cluster: d.cluster,
            capability_id: d.capability_id,
            confidence: d.confidence,
            is_llm_fallback: d.is_llm_fallback,
            circuit_broken: d.circuit_broken,
            circuit_break_reason: d.circuit_break_reason,
            fallback_path: d.fallback_path,
            candidates: d.candidates,
            candidate_details: d.candidate_details,
            execution_mode: d.execution_mode.as_str().to_string(),
            stage_records: d.stage_records.iter().map(RouteStageView::from).collect(),
            total_elapsed_ms: d.total_elapsed_ms,
            execution: None,
        }
    }
}

// ── Tauri 命令 ────────────────────────────────────

/// 将嵌套执行器（workflow_execute / agent_query）返回的错误串转为 CommandError。
///
/// 执行器以 `ErrorResponse` JSON 序列化返回错误，此处解析还原 code/category/params，
/// 保留前端 i18n 错误码；解析失败时回退到 fallback_code 并保留原始 detail。
fn executor_error(e: String, fallback_code: &'static str) -> CommandError {
    serde_json::from_str::<ErrorResponse>(&e)
        .unwrap_or_else(|_| CommandError::new(fallback_code).with_detail(e))
}

/// 认知编排统一入口 — 用户消息触发，完成三层路由决策并按执行模式分发执行
///
/// # 分支执行（复用既有执行器，不新增）
/// - `Workflow`：调用 `workflow_execute`，由 WorkEngine 执行命中的工作流模板
///   （`capability_id` 即工作流模板 ID，`SubWorkflowExecutor` 支持嵌套）
/// - `Delegate` / `Ask` / `Plan` / `Act`：调用 `agent_query`，交给通用 agent
///   （Ask/Plan/Act 对应 agent 引擎原有的三种执行模式，由认知编排器自动决策）
#[agent_command(domain = cognitive, safety = Caution, call_mode = StateInput, description = "认知编排统一入口（三层路由决策并按执行模式分发）")]
#[tauri::command]
pub async fn cognitive_query(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: CognitiveQueryRequest,
) -> Result<CognitiveQueryResponse, CommandError> {
    let input = request.input.trim().to_string();
    if input.is_empty() {
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::EMPTY_INPUT)
            .with_category(ErrorCategory::Validation)
            .with_param("field", "input"));
    }

    // ── 前置 1：安全拦截（拒绝分支）──
    // 检测注入/越狱/敏感指令。命中即拒绝执行并记录安全日志，绝不透传给下游执行器。
    let prompt_guard = PatternPromptGuard::new();
    let input = match prompt_guard.process_user_input(&input) {
        Ok(processed) => processed,
        Err(reason) => {
            tracing::error!(%reason, "🛡️ 安全拦截：检测到注入/越狱，拒绝执行并记录安全日志");
            return Err(CommandError::new(
                axagent_harness::error_codes::cognitive::PROMPT_REJECTED,
            )
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(reason));
        },
    };

    // ── 前置 2：结构化 JSON 检测（参数自带完整 → 跳过路由模型，直发参数抽取）──
    // 用户输入包含完整 JSON 对象且携带目标工作流/能力 ID（workflow_id / capability_id）时，
    // 直接定位并执行，省一次路由 LLM 调用；否则仍按文本走正常三层路由。
    if let Some(json_params) = extract_json_object(&input) {
        if let Some(target) = json_params
            .get("workflow_id")
            .or_else(|| json_params.get("capability_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            let execution_id = crate::commands::workflows::workflow_execute(
                app.clone(),
                state,
                target.clone(),
                request.model_id.clone(),
                request.provider_id.clone(),
                None,
                request.max_concurrent,
                request.conversation_id.clone(),
                Some(json_params),
            )
            .await
            .map_err(|e| {
                executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            })?;
            return Ok(CognitiveQueryResponse {
                route_path: String::new(),
                domain: String::new(),
                cluster: String::new(),
                capability_id: target.clone(),
                confidence: 1.0,
                is_llm_fallback: false,
                circuit_broken: false,
                circuit_break_reason: None,
                fallback_path: None,
                candidates: Vec::new(),
                candidate_details: Vec::new(),
                execution_mode: ExecutionMode::ParameterExtract.as_str().to_string(),
                stage_records: Vec::new(),
                total_elapsed_ms: 0,
                execution: Some(CognitiveExecutionView::Workflow {
                    workflow_id: target,
                    execution_id,
                }),
            });
        }
    }

    // ── 前置 3：Clarify 二次执行（forcedCapabilityId 快速路径）──
    // 用户在 Clarify 候选中选定能力后，携带 forcedCapabilityId 重新调用本命令：
    // 跳过三层路由，按能力类型直接分发给对应执行器（Workflow → WorkEngine；Agent → agent_query）。
    if let Some(forced_id) = request.forced_capability_id.as_deref().filter(|s| !s.is_empty()) {
        let forced_id = forced_id.to_string();
        let forced_passport = state.capability_indexer.get_passport(&forced_id).await;
        let forced_kind =
            forced_passport.as_ref().map(|p| p.kind).unwrap_or(CapabilityKind::Workflow);
        let execution = match forced_kind {
            CapabilityKind::Workflow => {
                let execution_id = crate::commands::workflows::workflow_execute(
                    app.clone(),
                    state,
                    forced_id.clone(),
                    request.model_id.clone(),
                    request.provider_id.clone(),
                    None,
                    request.max_concurrent,
                    request.conversation_id.clone(),
                    Some(serde_json::Value::String(input.clone())),
                )
                .await
                .map_err(|e| {
                    executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                })?;
                CognitiveExecutionView::Workflow { workflow_id: forced_id.clone(), execution_id }
            },
            CapabilityKind::Agent => {
                let conversation_id = request.conversation_id.clone().ok_or_else(|| {
                    CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                        .with_category(ErrorCategory::Validation)
                        .with_param("field", "conversation_id")
                })?;
                let agent_request = AgentQueryRequest {
                    conversation_id,
                    input,
                    provider_id: request.provider_id.clone().unwrap_or_default(),
                    model_id: request.model_id.clone().unwrap_or_default(),
                    enabled_mcp_server_ids: None,
                    enabled_knowledge_base_ids: None,
                    enabled_memory_namespace_ids: None,
                    enabled_wiki_ids: None,
                    system_prompt: request.system_prompt.clone(),
                    thinking_budget: None,
                    search_provider_id: request.search_provider_id.clone(),
                    attachments: None,
                    options: request.options.clone(),
                    // Clarify 选中的 Agent 类型能力：用其 passport 推荐专家（用户手选已隐藏）
                    agent_profile_id: forced_passport
                        .as_ref()
                        .and_then(|p| p.agent_profile_id.clone()),
                    agent_context: request.agent_context.clone(),
                };
                let agent_resp = crate::commands::agent::agent_query(app, state, agent_request)
                    .await
                    .map_err(|e| {
                        executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                    })?;
                CognitiveExecutionView::Agent {
                    conversation_id: agent_resp.conversation_id,
                    assistant_message_id: agent_resp.assistant_message_id,
                    status: agent_resp.status,
                }
            },
            // 其余能力类型（Tool / KnowledgeBase / Skill）当前无可执行执行器，
            // 按 Workflow 语义回退（绝大多数 Clarify 候选来自工作流检索）
            _ => {
                let execution_id = crate::commands::workflows::workflow_execute(
                    app.clone(),
                    state,
                    forced_id.clone(),
                    request.model_id.clone(),
                    request.provider_id.clone(),
                    None,
                    request.max_concurrent,
                    request.conversation_id.clone(),
                    Some(serde_json::Value::String(input.clone())),
                )
                .await
                .map_err(|e| {
                    executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                })?;
                CognitiveExecutionView::Workflow { workflow_id: forced_id.clone(), execution_id }
            },
        };
        return Ok(CognitiveQueryResponse {
            route_path: format!("forced:{}", forced_kind.as_str()),
            domain: String::new(),
            cluster: String::new(),
            capability_id: forced_id,
            confidence: 1.0,
            is_llm_fallback: false,
            circuit_broken: false,
            circuit_break_reason: None,
            fallback_path: None,
            candidates: Vec::new(),
            candidate_details: Vec::new(),
            execution_mode: ExecutionMode::ParameterExtract.as_str().to_string(),
            stage_records: Vec::new(),
            total_elapsed_ms: 0,
            execution: Some(execution),
        });
    }

    // ── 三层路由决策：主 DAG 驱动（WorkEngine 同步执行认知编排器）──
    // 由 work_engine.run_workflow(cognitive_router_main) 执行完整路由工作流，
    // 返回的 EndNode 输出即 l3_result（含 route_path / capability_id / execution_mode /
    // candidates / 熔断标记等），替代原先 CognitiveRouter.route_with_hint 的硬编码三层调用。
    let total_start = std::time::Instant::now();
    let mode_hint = ModeHint::parse_str(request.mode_hint.as_deref().unwrap_or("auto"));

    // 1. 动态分类目录：L1 全量业务域；L2 按预路由 L1 域实时生成（纯规则匹配，零 LLM 成本）
    let l1_categories = state.cognitive_router.list_l1_categories().await;
    // L2 动态目录按预路由 L1 域实时生成：纯规则匹配（零 LLM 成本），未命中回退 General 域，
    // 避免与主 DAG 内 L1 子工作流重复触发 LLM 兜底（重复调用）。
    let l1_pre = state
        .cognitive_router
        .route_l1_rules_only(&input)
        .await
        .unwrap_or(CapabilityDomain::General);
    let l2_categories = state.cognitive_router.list_l2_categories(l1_pre.as_str()).await;
    let variables = vec![
        // 用户输入作为顶层变量注入，供主 DAG 各子工作流 input_mapping 引用 `user_input`。
        // 注意：引擎 run_workflow 仅把 with_input 的对象存为 `input` 变量，不会展开为顶层
        // `user_input`，若不在此注入，call_l1 的 input_mapping 解析 `user_input` 将失败。
        Variable {
            name: "user_input".to_string(),
            var_type: "string".to_string(),
            value: serde_json::json!(input.clone()),
            description: None,
            is_secret: false,
        },
        Variable {
            name: "__l1_categories".to_string(),
            var_type: "array".to_string(),
            value: serde_json::json!(l1_categories),
            description: None,
            is_secret: false,
        },
        Variable {
            name: "__l2_categories".to_string(),
            var_type: "array".to_string(),
            value: serde_json::json!(l2_categories),
            description: None,
            is_secret: false,
        },
    ];

    // 2. 系统能力回调：L1/L2/RAR/图谱等 `system_*` 节点统一走 CognitiveRouter.execute_system_capability
    //    （L3 子工作流内 system_rar_retriever / system_workflow_graph_router 也经此回调透传执行）
    let cognitive_router = state.cognitive_router.clone();
    let system_capability_cb: Option<SubWorkflowCallback> = Some(Arc::new(
        move |capability_id: String,
              _parent_execution_id: String,
              cap_input: HashMap<String, serde_json::Value>| {
            let cognitive_router = cognitive_router.clone();
            Box::pin(async move {
                let result =
                    cognitive_router.execute_system_capability(&capability_id, cap_input).await?;
                Ok((String::new(), result))
            })
        },
    ));

    // 3. 执行主 DAG（同步执行，输出即 l3_result）
    // 主 DAG 各子工作流 input_mapping 引用 `user_input` 变量 → 以对象形式注入
    let mut opts = RunOptions::new()
        .with_variables(variables)
        .with_system_capability_callback(system_capability_cb)
        .with_input(serde_json::json!({ "user_input": input.clone() }));
    if let Some(model_id) = &request.model_id {
        opts = opts.with_model(model_id.clone());
    }
    if let Some(provider_id) = &request.provider_id {
        opts = opts.with_provider(provider_id.clone());
    }
    if let Some(mc) = request.max_concurrent {
        opts = opts.with_max_concurrent(mc.max(1));
    }

    let workflow =
        state.work_engine.run_workflow(COGNITIVE_ROUTER_MAIN_ID, opts).await.map_err(|e| {
            CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                .with_category(ErrorCategory::Retryable)
                .with_detail(format!("认知编排器主 DAG 执行失败: {e}"))
        })?;

    // 4. 解析 l3_result（主 DAG EndNode 输出）
    let l3_value = workflow.output.ok_or_else(|| {
        CommandError::new(axagent_harness::error_codes::cognitive::NO_CANDIDATE)
            .with_category(ErrorCategory::Retryable)
            .with_detail("认知编排器主 DAG 未产出路由决策".to_string())
    })?;
    let l3 = l3_value.as_object().cloned().unwrap_or_default();
    let get_str = |key: &str, default: &str| {
        l3.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string()
    };

    let route_path = get_str("route_path", "");
    let domain = get_str("domain", "");
    let cluster = get_str("cluster", "");
    let capability_id = get_str("capability_id", "");
    let confidence = l3.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let circuit_break_reason = l3.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string());
    // candidates：RAR 检索候选（id/name/description/score/kind/domain/cluster）→ CandidateSummary
    // （RAR 候选用 `id` 承载能力 ID，转成 CandidateSummary 的 `capability_id` 字段）
    let candidate_details: Vec<CandidateSummary> = l3
        .get("candidates")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let id = c
                        .get("id")
                        .or_else(|| c.get("capability_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    serde_json::from_value::<CandidateSummary>(serde_json::json!({
                        "capability_id": id,
                        "name": c.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "description": c.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                        "score": c.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "kind": c.get("kind").and_then(|v| v.as_str()).unwrap_or("workflow"),
                        "domain": c.get("domain").and_then(|v| v.as_str()).unwrap_or(""),
                        "cluster": c.get("cluster").and_then(|v| v.as_str()).map(str::to_string),
                        "agent_profile_id": c
                            .get("agent_profile_id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                    }))
                    .ok()
                })
                .collect()
        })
        .unwrap_or_default();
    let candidates: Vec<String> =
        candidate_details.iter().map(|c| c.capability_id.clone()).collect();

    // 认知编排自动选专家：取选中能力（capability_id）候选上标注的推荐专家。
    // 命中工作流路径时由 WorkEngine/AgentNode 自行绑定专家，此处仅服务于
    // Ask/Act/Delegate 等 Agent 执行路径；选中能力无推荐专家时兜底为 None，
    // 由 agent_query 落回默认专家（单专家数据约束下即为默认 profile）。
    let route_agent_profile = candidate_details
        .iter()
        .find(|c| c.capability_id == capability_id)
        .and_then(|c| c.agent_profile_id.clone());
    tracing::debug!(
        capability_id = %capability_id,
        route_agent_profile = ?route_agent_profile,
        "认知编排自动选专家（Agent 执行路径）"
    );

    // 5. 熔断检查（自指/系统能力层熔断 → 可恢复错误，前端引导重试或降级）
    if l3.get("is_circuit_broken").and_then(|v| v.as_bool()).unwrap_or(false) {
        let mut params = HashMap::new();
        // 熔断原因可能缺失，兜底默认值避免前端渲染出字面量 `{reason}`
        params.insert(
            "reason".to_string(),
            circuit_break_reason.unwrap_or_else(|| "self-reference circuit breaker".to_string()),
        );
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::CIRCUIT_BROKEN)
            .with_category(ErrorCategory::Retryable)
            .with_params(params));
    }

    // 6. 执行模式决策：优先尊重用户意图提示（mode_hint），否则采用主 DAG 决策的 execution_mode
    let mut mode = serde_json::from_str::<ExecutionMode>(&get_str("execution_mode", "ask"))
        .unwrap_or(ExecutionMode::Ask);
    if mode_hint != ModeHint::Auto {
        mode = match mode_hint {
            ModeHint::Ask => ExecutionMode::Ask,
            ModeHint::Plan => ExecutionMode::Plan,
            ModeHint::Act => ExecutionMode::Act,
            ModeHint::Auto => mode,
        };
    }

    // 观测字段：图谱路由透传 is_llm_fallback / fallback_path / stage_records，供前端观测完整路由过程
    let is_llm_fallback = l3.get("is_llm_fallback").and_then(|v| v.as_bool()).unwrap_or(false);
    let fallback_path = l3.get("fallback_path").and_then(|v| v.as_str()).map(str::to_string);
    let stage_records: Vec<RouteStageView> = l3
        .get("stage_records")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|r| RouteStageView {
                    stage: r.get("stage").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    success: r.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    confidence: r.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    elapsed_ms: r.get("elapsed_ms").and_then(|v| v.as_u64()).unwrap_or(0),
                    summary: r.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut response: CognitiveQueryResponse = CognitiveQueryResponse {
        route_path,
        domain,
        cluster,
        capability_id,
        confidence,
        is_llm_fallback,
        circuit_broken: false,
        circuit_break_reason,
        fallback_path,
        candidates,
        candidate_details,
        execution_mode: mode.as_str().to_string(),
        stage_records,
        total_elapsed_ms: total_start.elapsed().as_millis() as u64,
        execution: None,
    };

    // ── 分支执行：按执行模式复用既有执行器 ──
    response.execution = Some(match mode {
        // Workflow / Direct：capability_id 即工作流模板 ID，交给 WorkEngine 执行
        ExecutionMode::Workflow | ExecutionMode::Direct => {
            let workflow_id = response.capability_id.clone();
            let execution_id = crate::commands::workflows::workflow_execute(
                app.clone(),
                state,
                workflow_id.clone(),
                request.model_id.clone(),
                request.provider_id.clone(),
                None,
                request.max_concurrent,
                request.conversation_id.clone(),
                Some(serde_json::Value::String(input.clone())),
            )
            .await
            .map_err(|e| {
                executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            })?;
            CognitiveExecutionView::Workflow { workflow_id, execution_id }
        },
        // ParameterExtract：精准命中（置信度 > 0.90），跳过澄清直接执行目标工作流；
        // 文本输入交给 WorkEngine 内置规则抽取参数（JSON 对象已在快速路径直发）
        ExecutionMode::ParameterExtract => {
            let workflow_id = response.capability_id.clone();
            let execution_id = crate::commands::workflows::workflow_execute(
                app.clone(),
                state,
                workflow_id.clone(),
                request.model_id.clone(),
                request.provider_id.clone(),
                None,
                request.max_concurrent,
                request.conversation_id.clone(),
                Some(serde_json::Value::String(input.clone())),
            )
            .await
            .map_err(|e| {
                executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            })?;
            CognitiveExecutionView::Workflow { workflow_id, execution_id }
        },
        // Plan：域明确但无具体工作流命中，触发 plan_generate 拆解任务（前端监听 plan-generated）
        ExecutionMode::Plan => {
            let conversation_id = request.conversation_id.clone().ok_or_else(|| {
                CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                    .with_category(ErrorCategory::Validation)
                    .with_param("field", "conversation_id")
            })?;
            let plan = crate::commands::plan::plan_generate(
                state,
                app.clone(),
                crate::commands::plan::PlanGenerateRequest {
                    conversation_id: conversation_id.clone(),
                    content: input,
                },
            )
            .await
            .map_err(|e| {
                executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            })?;
            CognitiveExecutionView::Plan { conversation_id, plan_id: plan.id }
        },
        // Clarify：模糊命中（置信度 0.60 ~ 0.90），返回 Top2 候选交用户选择，前端二次路由
        ExecutionMode::Clarify => {
            let mut candidates = response.candidate_details.clone();
            candidates.truncate(2);
            CognitiveExecutionView::Clarify { candidates }
        },
        // Delegate / Ask / Act：交给 agent_query 执行
        _ => {
            let conversation_id = request.conversation_id.clone().ok_or_else(|| {
                CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                    .with_category(ErrorCategory::Validation)
                    .with_param("field", "conversation_id")
            })?;
            // 专家选择优先级：调用方显式指定（request.agent_profile_id）优先，
            // 否则回退到认知编排路由自动选专家（route_agent_profile，从命中能力候选推导）。
            // 单专家数据约束下两者通常一致；显式指定供外部调用方/未来恢复手选时覆盖。
            let selected_agent_profile =
                request.agent_profile_id.clone().or(route_agent_profile.clone());
            let agent_request = AgentQueryRequest {
                conversation_id,
                input,
                provider_id: request.provider_id.clone().unwrap_or_default(),
                model_id: request.model_id.clone().unwrap_or_default(),
                enabled_mcp_server_ids: None,
                enabled_knowledge_base_ids: None,
                enabled_memory_namespace_ids: None,
                enabled_wiki_ids: None,
                system_prompt: request.system_prompt.clone(),
                thinking_budget: None,
                search_provider_id: request.search_provider_id.clone(),
                attachments: None,
                options: request.options.clone(),
                // 认知编排选专家：显式指定优先，路由自动推导兜底
                agent_profile_id: selected_agent_profile,
                agent_context: request.agent_context.clone(),
            };
            let agent_resp =
                crate::commands::agent::agent_query(app, state, agent_request).await.map_err(
                    |e| executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED),
                )?;
            CognitiveExecutionView::Agent {
                conversation_id: agent_resp.conversation_id,
                assistant_message_id: agent_resp.assistant_message_id,
                status: agent_resp.status,
            }
        },
    });

    Ok(response)
}

/// 从输入中提取完整 JSON 对象（结构化参数检测）。
///
/// 识别两种形态：
/// 1. 输入整体即为 JSON 对象（如直接粘贴 `{"stock_code": "301302"}`）
/// 2. 输入包含 ```json ... ``` 代码块包裹的 JSON 对象
///
/// 仅在对象内命中目标能力 ID 字段（workflow_id / capability_id）时启用快速路径。
fn extract_json_object(input: &str) -> Option<serde_json::Value> {
    // 形态 1：整体解析
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(input.trim()) {
        if value.is_object() {
            return Some(value);
        }
    }
    // 形态 2：提取 ```json ... ``` 代码块
    let start = input.find("```json")?;
    let body_start = start + "```json".len();
    let body_end = input[body_start..].find("```")? + body_start;
    let body = &input[body_start..body_end];
    serde_json::from_str::<serde_json::Value>(body.trim()).ok().filter(|v| v.is_object())
}

/// 快速路径 — 仅执行 L1 域路由（供调试与展示）
#[agent_command(domain = cognitive, safety = Safe, call_mode = StateInput, description = "认知编排 L1 域路由")]
#[tauri::command]
pub async fn cognitive_route_l1(
    state: State<'_, AppState>,
    input: String,
) -> Result<axagent_harness::DomainRoutingResult, CommandError> {
    let input = input.trim().to_string();
    if input.is_empty() {
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::EMPTY_INPUT)
            .with_category(ErrorCategory::Validation));
    }
    Ok(state.cognitive_router.route_l1(&input).await)
}

/// 查询执行模式说明（供前端展示/调试）
#[agent_command(domain = cognitive, safety = Safe, call_mode = StateOnly, description = "查询认知编排执行模式列表")]
#[tauri::command]
pub async fn cognitive_list_execution_modes() -> Result<Vec<&'static str>, CommandError> {
    Ok(vec![
        ExecutionMode::Ask.as_str(),
        ExecutionMode::Plan.as_str(),
        ExecutionMode::Act.as_str(),
        ExecutionMode::Workflow.as_str(),
        ExecutionMode::Direct.as_str(),
        ExecutionMode::Delegate.as_str(),
        ExecutionMode::ParameterExtract.as_str(),
        ExecutionMode::Clarify.as_str(),
    ])
}
