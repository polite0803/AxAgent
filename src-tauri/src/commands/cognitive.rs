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
use axagent_harness::workflow_evolution::ToolExecutionStats;
use axagent_harness::workflow_types::Variable;
use axagent_harness::{
    CandidateSummary, CapabilityDomain, CapabilityGapProposal, CapabilityGapType, CapabilityKind,
    CapabilityQuery, ExecutionMode, ModeHint, PatternPromptGuard, PromptAttackCategory,
    PromptGuard, PromptRejection, RouteStageRecord, RoutingDecisionV2,
};
use axagent_runtime::work_engine::{RunOptions, SubWorkflowCallback};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
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

/// 选中的执行专家（Agent 执行路径）视图
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedAgentProfileView {
    /// AgentProfile ID
    pub id: String,
    /// 专家名称
    pub name: String,
    /// 角色名（agent_role，可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// 关联专家（expert_id → agency_experts.name，可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expert: Option<String>,
}

/// 解析选中工作流的可读名称：优先取候选摘要中的 name，否则尝试从能力护照解析。
/// 主流程与 Clarify 二次执行（forced_id）共用，保证两处决策分支展示一致。
async fn resolve_selected_workflow_name(
    state: &AppState,
    capability_id: &str,
    candidate_details: &[CandidateSummary],
) -> Option<String> {
    match candidate_details
        .iter()
        .find(|c| c.capability_id == capability_id)
        .filter(|c| !c.name.is_empty())
        .map(|c| c.name.clone())
    {
        Some(name) => Some(name),
        None if !capability_id.is_empty() => state
            .capability_indexer
            .get_passport(capability_id)
            .await
            .filter(|p| !p.name.is_empty())
            .map(|p| p.name),
        None => None,
    }
}

/// 解析选中的执行专家（Agent 执行路径）：profile 名称 + 角色 + 关联专家名。
/// 主流程与 Clarify 二次执行（forced_id）共用；查询失败不阻断，仅返回 ID 供前端展示。
async fn resolve_selected_agent_profile(
    state: &AppState,
    profile_id: Option<&str>,
) -> Option<SelectedAgentProfileView> {
    let profile_id = profile_id.filter(|s| !s.is_empty())?;
    match axagent_dao::repo::agent_profile::get_agent_profile(state.harness.db(), profile_id).await
    {
        Ok(profile) => {
            // 关联专家名称：expert_id → agency_experts.name（解析失败不阻断主流程）
            let expert = match profile.expert_id.as_deref() {
                Some(eid) if !eid.is_empty() => {
                    axagent_entities::agency_experts::Entity::find_by_id(eid)
                        .one(state.harness.db())
                        .await
                        .ok()
                        .flatten()
                        .map(|e| e.name.clone())
                },
                _ => None,
            };
            Some(SelectedAgentProfileView {
                id: profile.id,
                name: profile.name,
                role: profile.agent_role.clone(),
                expert,
            })
        },
        // 查询失败不阻断：仅返回 ID，前端仍可展示
        Err(_) => Some(SelectedAgentProfileView {
            id: profile_id.to_string(),
            name: profile_id.to_string(),
            role: None,
            expert: None,
        }),
    }
}

/// 角色命中且执行载体未组合专家时，通过 RAR 检索动态补全专家（expert_id）。
///
/// "角色 + 专家"默认在 AgentProfile 中两两组装；但当角色护照落到只有角色、
/// 未组合专家的执行载体（如自动补齐的 role-bridge）时，此处按用户输入从 Agent
/// 护照库检索最匹配的专家，实现运行时动态组合，避免外形角色命中丢失专家技能。
///
/// 返回 `None` 表示无需/无法补全（非角色能力、执行载体已组合专家、检索失败降级）。
async fn resolve_dynamic_expert_for_role(
    state: &AppState,
    capability_id: &str,
    input: &str,
    exec_profile_id: Option<&str>,
) -> Option<String> {
    // 仅角色护照（agent_role:*）触发动态补专家；专家/工作流护照本身已组合，无需补全。
    if !capability_id.starts_with("agent_role:") {
        return None;
    }
    // 执行载体已存在且自带专家 → 组合完整，无需补全。
    if let Some(pid) = exec_profile_id {
        if let Ok(profile) =
            axagent_dao::repo::agent_profile::get_agent_profile(state.harness.db(), pid).await
        {
            if profile.expert_id.as_deref().is_some_and(|s| !s.is_empty()) {
                return None;
            }
        }
    }
    // RAR 检索：按用户输入从 Agent 护照召回，取专家护照（agent:*，tags 带 expert:{id}）Top1。
    let query = CapabilityQuery {
        user_input: input.to_string(),
        top_k: 5,
        kind_filter: Some(vec![CapabilityKind::Agent]),
        ..Default::default()
    };
    let Ok(retrieval) = state.capability_router.retriever.retrieve(&query).await else {
        return None;
    };
    retrieval
        .candidates
        .iter()
        .find_map(|c| c.passport.tags.iter().find_map(|t| t.strip_prefix("expert:")))
        .map(str::to_string)
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
    /// 选中工作流的可读名称（从能力护照/候选解析；未命中工作流时为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_workflow_name: Option<String>,
    /// 选中的执行专家（Agent 执行路径自动选专家；未走 Agent 路径时为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_agent_profile: Option<SelectedAgentProfileView>,
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
            selected_workflow_name: None,
            selected_agent_profile: None,
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

/// 构建认知编排决策标签（JSON 对象），持久化到 assistant 消息用于每条消息独立展示。
/// 字段与前端 `CognitiveDecisionInfo` 类型对齐：ExecutionMode / 路由路径 / 命中工作流 / 专家。
fn build_decision_value(
    execution_mode: &str,
    route_path: &str,
    confidence: f64,
    selected_workflow_name: Option<String>,
    selected_agent_profile: Option<&SelectedAgentProfileView>,
) -> serde_json::Value {
    serde_json::json!({
        "executionMode": execution_mode,
        "routePath": route_path,
        "confidence": confidence,
        "selectedWorkflowName": selected_workflow_name,
        "selectedAgentProfile": selected_agent_profile.map(|p| serde_json::json!({
            "id": p.id,
            "name": p.name,
            "role": p.role,
            "expert": p.expert,
        })),
    })
}

/// 将已有响应视图转成决策标签（主流程 / Clarify 二次执行共用）。
fn decision_from_response(response: &CognitiveQueryResponse) -> serde_json::Value {
    build_decision_value(
        &response.execution_mode,
        &response.route_path,
        response.confidence,
        response.selected_workflow_name.clone(),
        response.selected_agent_profile.as_ref(),
    )
}

/// 将决策标签持久化到指定 assistant 消息；失败仅告警不阻断主流程。
async fn persist_decision_to_message(
    db: &sea_orm::DatabaseConnection,
    message_id: &str,
    decision: &serde_json::Value,
) {
    if let Err(e) =
        axagent_dao::repo::message::update_message_decision(db, message_id, Some(decision)).await
    {
        tracing::warn!("[cognitive] 写入消息决策标签失败: {}", e);
    }
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

    // ── 前置 1：安全拦截（拒绝分支 → 能力补齐提议双通道，T0.3）──
    // 检测注入/越狱/敏感指令。命中后：① 保留硬阻断（绝不透传给下游执行器，仍写安全日志）；
    // ② 归类为结构化缺口提议征求用户同意；③ 同意则执行补齐（防护规则/有界豁免/工作流）；
    // ④ 拒绝则保持原拒绝行为，并把拒绝记录为证据（拒绝即证据）。
    let prompt_guard = PatternPromptGuard::new();
    let input = match prompt_guard.process_user_input_structured(&input) {
        Ok(processed) => processed,
        Err(rejection) => {
            tracing::error!(%rejection.reason, "🛡️ 安全拦截命中，进入能力补齐提议通道");
            let proposal = build_capability_gap_proposal(Some(&rejection), &input);
            // 征求用户同意（复用授权事件通道，前端 EvolutionConsentModal 弹窗）
            if !await_user_consent(&app, &state, &proposal).await? {
                // 用户拒绝 → 保持原拒绝行为（不透传）+ 拒绝即证据
                persist_decision_to_message(
                    state.harness.db(),
                    request.conversation_id.as_deref().unwrap_or_default(),
                    &build_rejection_decision(&rejection),
                )
                .await;
                return Err(CommandError::new(
                    axagent_harness::error_codes::cognitive::PROMPT_REJECTED,
                )
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(rejection.reason));
            }
            // 用户同意 → 执行补齐（挂 disposer 可回滚）
            apply_capability_gap_proposal(&state, &proposal).await?;
            persist_decision_to_message(
                state.harness.db(),
                request.conversation_id.as_deref().unwrap_or_default(),
                &build_gap_decision(&proposal),
            )
            .await;
            // 补齐后输入按安全化语义继续：
            // - 误伤豁免（ExemptAuthorize）→ 放行原始输入（有界豁免已生效），继续走三层路由
            // - 其余（GuardRule / CapabilityMissing）→ 提示重新发送，避免绕过防护
            if proposal.gap_type == CapabilityGapType::ExemptAuthorize {
                tracing::info!(%rejection.pattern, "🛡️ 用户同意有界豁免，放行该合法诉求");
                input
            } else {
                return Err(CommandError::new(
                    axagent_harness::error_codes::cognitive::GAP_PROPOSAL_APPLIED,
                )
                .with_category(ErrorCategory::General)
                .with_detail("已按用户同意补齐能力，请重新发送请求".to_string()));
            }
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
            let decision =
                build_decision_value(ExecutionMode::ParameterExtract.as_str(), "", 1.0, None, None);
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
                Some(decision),
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
                selected_workflow_name: None,
                selected_agent_profile: None,
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

        // Clarify 二次执行也解析决策分支，保证前端展示与主流程一致：
        // - 工作流名：取能力护照可读名称
        // - 执行专家：Agent 类型能力按 passport 推荐专家解析
        // 必须在下方 match 移动 state 之前解析（借用）。
        let selected_workflow_name = resolve_selected_workflow_name(&state, &forced_id, &[]).await;
        let selected_agent_profile = resolve_selected_agent_profile(
            &state,
            forced_passport.as_ref().and_then(|p| p.agent_profile_id.as_deref()),
        )
        .await;
        // execution_mode 按能力类型定性（此路径为高置信精确命中，非参数抽取）：
        // Workflow/其他 → workflow；Agent → delegate
        let execution_mode = match forced_kind {
            CapabilityKind::Agent => ExecutionMode::Delegate.as_str().to_string(),
            _ => ExecutionMode::Workflow.as_str().to_string(),
        };

        // 决策标签：Clarify 二次执行同样持久化，与主流程展示一致
        let decision = build_decision_value(
            &execution_mode,
            &format!("forced:{}", forced_kind.as_str()),
            1.0,
            selected_workflow_name.clone(),
            selected_agent_profile.as_ref(),
        );

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
                    Some(decision),
                )
                .await
                .map_err(|e| {
                    executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                })?;
                CognitiveExecutionView::Workflow { workflow_id: forced_id.clone(), execution_id }
            },
            // 其余能力类型（Agent / Tool / KnowledgeBase / Skill）统一委派给 agent 执行，
            // 与主流程 Delegate 分支语义一致：Agent 用 passport 推荐专家，Tool/知识库/技能
            // 由 agent 内部工具系统加载执行；passport 无推荐专家时落回默认专家。
            // （修复：原先 Tool 等能力被误当 Workflow 执行，查无模板报 WORKFLOW_NOT_FOUND）
            _ => {
                let conversation_id = request.conversation_id.clone().ok_or_else(|| {
                    CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                        .with_category(ErrorCategory::Validation)
                        .with_param("field", "conversation_id")
                })?;
                // Clarify 选中的能力：解析 passport 推荐执行载体 profile_id
                let forced_profile_id =
                    forced_passport.as_ref().and_then(|p| p.agent_profile_id.clone());
                // 角色命中且执行载体未组合专家时，动态补全专家（RAR 检索），运行时组合
                let dynamic_expert_id = resolve_dynamic_expert_for_role(
                    &state,
                    &forced_id,
                    &input,
                    forced_profile_id.as_deref(),
                )
                .await;
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
                    // Clarify 选中的能力：用其 passport 推荐专家（用户手选已隐藏），
                    // Tool/知识库/技能无推荐专家时落回默认专家
                    agent_profile_id: forced_profile_id,
                    expert_id: dynamic_expert_id,
                    agent_context: request.agent_context.clone(),
                    // 透传认知编排决策模式：Clarify 二次执行按能力类型定性（Agent→delegate，
                    // 其余→workflow），让 agent 运行时感知当前编排模式
                    execution_mode: Some(execution_mode.clone()),
                };
                let agent_resp =
                    crate::commands::agent::agent_query(app, state.clone(), agent_request)
                        .await
                        .map_err(|e| {
                            executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                        })?;
                persist_decision_to_message(
                    state.harness.db(),
                    &agent_resp.assistant_message_id,
                    &decision,
                )
                .await;
                CognitiveExecutionView::Agent {
                    conversation_id: agent_resp.conversation_id,
                    assistant_message_id: agent_resp.assistant_message_id,
                    status: agent_resp.status,
                }
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
            execution_mode,
            selected_workflow_name,
            selected_agent_profile,
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
    // 主 DAG 无产出时不再静默报错：进入能力补齐提议通道（T0.4），用户同意后补齐
    // 能力并提示重发；拒绝则保持原 NO_CANDIDATE 可恢复错误。
    let l3_value = match workflow.output {
        Some(v) => v,
        None => {
            let proposal = build_capability_gap_proposal(None, &input);
            if await_user_consent(&app, &state, &proposal).await? {
                apply_capability_gap_proposal(&state, &proposal).await?;
                return Err(CommandError::new(
                    axagent_harness::error_codes::cognitive::GAP_PROPOSAL_APPLIED,
                )
                .with_category(ErrorCategory::General)
                .with_detail("已按用户同意补齐能力，请重新发送请求".to_string()));
            }
            return Err(CommandError::new(axagent_harness::error_codes::cognitive::NO_CANDIDATE)
                .with_category(ErrorCategory::Retryable)
                .with_detail("认知编排器主 DAG 未产出路由决策".to_string()));
        },
    };
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

    // 选中工作流的可读名称：优先取候选摘要中的 name，否则尝试从能力护照解析
    let selected_workflow_name =
        resolve_selected_workflow_name(&state, &capability_id, &candidate_details).await;

    // 选中的执行专家（Agent 执行路径）：解析 profile 名称 + 角色 + 关联专家名
    let selected_agent_profile =
        resolve_selected_agent_profile(&state, route_agent_profile.as_deref()).await;

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
        selected_workflow_name,
        selected_agent_profile,
        stage_records,
        total_elapsed_ms: total_start.elapsed().as_millis() as u64,
        execution: None,
    };

    // ── 分支执行：按执行模式复用既有执行器 ──
    // 决策标签：为该轮执行生成，Workflow 分支透传给 workflow_execute 持久化，
    // Agent 分支在此处直接写入 assistant 消息。
    let decision = decision_from_response(&response);

    // ── Clarify 兜底无候选 → 能力补齐提议通道（T0.5）──
    // 主 DAG 决策为 Clarify（置信度模糊）但候选为空（RAR/图谱兜底无命中）时，
    // 不进入空候选展示，而是生成 capability_missing 提议征求用户同意；
    // 拒绝则保持原 Clarify 空候选行为（返回空候选，前端自行兜底）。
    if mode == ExecutionMode::Clarify && response.candidate_details.is_empty() {
        let proposal = build_capability_gap_proposal(None, &input);
        if await_user_consent(&app, &state, &proposal).await? {
            apply_capability_gap_proposal(&state, &proposal).await?;
            return Err(CommandError::new(
                axagent_harness::error_codes::cognitive::GAP_PROPOSAL_APPLIED,
            )
            .with_category(ErrorCategory::General)
            .with_detail("已按用户同意补齐能力，请重新发送请求".to_string()));
        }
        tracing::info!("🧭 Clarify 兜底无候选且用户拒绝补齐，返回空候选（前端自行兜底）");
    }

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
                Some(decision),
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
                Some(decision),
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
            // 角色命中且执行载体未组合专家时，动态补全专家（RAR 检索），
            // 运行时组合"角色 + 专家"，避免角色护照落到无专家的执行载体丢失专家技能。
            let dynamic_expert_id = resolve_dynamic_expert_for_role(
                &state,
                &response.capability_id,
                &input,
                selected_agent_profile.as_deref(),
            )
            .await;
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
                // 认知编排选专家：显式指定优先，路由自动推导兜底；角色命中时动态补专家
                agent_profile_id: selected_agent_profile,
                expert_id: dynamic_expert_id,
                agent_context: request.agent_context.clone(),
                // 透传认知编排决策模式（Ask/Act/Delegate），让 agent 运行时感知当前编排模式
                execution_mode: Some(mode.as_str().to_string()),
            };
            let agent_resp = crate::commands::agent::agent_query(app, state.clone(), agent_request)
                .await
                .map_err(|e| {
                    executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                })?;
            persist_decision_to_message(
                state.harness.db(),
                &agent_resp.assistant_message_id,
                &decision,
            )
            .await;
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

// ── 双通道闭环：能力补齐（T0.6 / T0.7）────────────────

/// 能力补齐提议的归类分析器（通道一：能力补齐）。
///
/// 输入：`PromptRejection`（安全拦截命中）或无候选信号（主 DAG / Clarify 兜底），
/// 输出：结构化的 [`CapabilityGapProposal`]，供前端弹窗征求用户同意后执行补齐。
///
/// # 归类规则
/// | 场景 | gap_type | 提议内容 |
/// |------|----------|----------|
/// | 攻击手法不在静态防护列表 | `GuardRule` | 新增防护正则（挂 Disposer 可回滚） |
/// | 本地 IDE 合法诉求误伤（developer mode 等） | `ExemptAuthorize` | 按命中模式 + 作用域有界授权 |
/// | 用户请求系统当前无能力安全处理 / 主 DAG 无候选 / Clarify 兜底无命中 | `CapabilityMissing` | 生成补齐工作流 + 护照 + 图谱 |
fn build_capability_gap_proposal(
    rejection: Option<&PromptRejection>,
    input: &str,
) -> CapabilityGapProposal {
    let now = chrono::Utc::now();
    let id = format!("gap:{}", now.timestamp_millis());

    let Some(rejection) = rejection else {
        // 无候选信号（主 DAG 无产出 / Clarify 兜底无命中）→ 能力缺失补齐
        return CapabilityGapProposal {
            id,
            gap_type: CapabilityGapType::CapabilityMissing,
            category: None,
            title: "能力补齐提议：当前无能力处理该请求".to_string(),
            proposal: format!(
                "为请求「{input}」生成补齐工作流模板，并注册能力护照与工作流图谱，使其可被三层路由发现。"
            ),
            reason: "认知编排器三层路由未产出候选（主 DAG 无候选 / Clarify 兜底无命中）。"
                .to_string(),
            impact: "补齐后该请求可被路由命中并执行；未补齐前保持无候选行为。".to_string(),
            rollback: "可逆：注销能力护照索引 + 从图谱移除节点（挂 Disposer，可随时回滚）。"
                .to_string(),
            created_at: now,
        };
    };

    // 安全拦截命中 → 按攻击类别归类。
    // 误伤豁免：developer mode 等本地 IDE 合法诉求被静态规则命中 → 有界授权（不开放全量）。
    let (gap_type, title, proposal) = match rejection.category {
        PromptAttackCategory::RoleOverride
            if rejection.pattern.to_lowercase().contains("developer mode") =>
        {
            (
                CapabilityGapType::ExemptAuthorize,
                "误伤豁免提议：本地 IDE 合法诉求".to_string(),
                format!(
                    "按命中的具体模式「{}」+ 作用域授权有界豁免（不开放全量），允许该合法诉求通过。",
                    rejection.pattern
                ),
            )
        },
        _ => (
            CapabilityGapType::GuardRule,
            "防护规则补齐提议".to_string(),
            format!(
                "为攻击类别「{:?}」新增防护正则，覆盖未在静态防护列表中的攻击手法。",
                rejection.category
            ),
        ),
    };

    CapabilityGapProposal {
        id,
        gap_type,
        category: Some(rejection.category),
        title,
        proposal,
        reason: rejection.reason.clone(),
        impact: "补齐后该类攻击将被精确拦截 / 合法诉求正常放行，降低误伤。".to_string(),
        rollback: "可逆：防护规则挂 Disposer，可随时回滚。".to_string(),
        created_at: now,
    }
}

/// 能力补齐提议的执行器（通道一落地）。
///
/// 用户同意后调用：
/// - `CapabilityMissing`：生成补齐工作流的能力护照（`evolution:workflow:{id}`，
///   `auto_evolved` 标签 + `/evolution/` 前缀），注册进能力索引（L2 混合检索可见），
///   并同步进工作流图谱（L3 `system_workflow_graph_router` 可见）。
/// - `GuardRule` / `ExemptAuthorize`：动态防护规则 / 有界豁免的注入属阶段二
///   （副作用栈 + Disposer），此处先记录决策标签，不改变安全底线。
///
/// 所有补齐动作必须经用户显式同意（铁律），调用方负责先征求同意再调用本函数。
async fn apply_capability_gap_proposal(
    state: &AppState,
    proposal: &CapabilityGapProposal,
) -> Result<(), CommandError> {
    match proposal.gap_type {
        CapabilityGapType::CapabilityMissing => {
            // 复用 T0.11 统一注册：生成能力护照（auto_evolved 标签 + /evolution/ 前缀，
            // L2 混合检索可见）并同步进工作流图谱（L3 图谱路由可见）。
            let workflow_id = format!("auto_generated:{}", proposal.id);
            let display_name = proposal.title.trim();
            crate::commands::capability::register_evolution_product(
                state,
                &workflow_id,
                display_name,
                &proposal.proposal,
            )
            .await?;
            tracing::info!(
                workflow_id = %workflow_id,
                "🗺️ 能力补齐：工作流护照已注册并同步进工作流图谱"
            );
        },
        CapabilityGapType::GuardRule | CapabilityGapType::ExemptAuthorize => {
            // 阶段二注入动态防护规则 / 有界豁免（挂 Disposer）；阶段零仅记录决策标签
            tracing::info!(
                gap_type = ?proposal.gap_type,
                category = ?proposal.category,
                "🔒 能力补齐：防护类提议已记录（动态注入见阶段二副作用栈）"
            );
        },
    }
    Ok(())
}

/// 用户同意等待超时（秒）。超时视为拒绝，保持原安全行为。
const CONSENT_TIMEOUT: Duration = Duration::from_secs(180);
/// 前端同意弹窗事件名（T0.13 EvolutionConsentModal 监听）。
const EVOLUTION_CONSENT_EVENT: &str = "evolution-consent-request";

/// 征求用户同意：通过事件通道下发提议，阻塞等待前端弹窗回传（T0.13 配套）。
///
/// 复用 `agent_plan_approvals` 同款挂起审批槽模式：
/// 1. 插入 `oneshot` sender 到 `evolution_consent_senders`（proposalId → sender）
/// 2. emit `evolution-consent-request` 事件（携带 camelCase 提议）
/// 3. await receiver（180s 超时，超时视为拒绝）
/// 4. 前端弹窗由 `capability_gap_consent` 命令回传结果
///
/// 返回 `true` = 用户同意；`false` = 用户拒绝 / 超时 / 前端无监听。
async fn await_user_consent(
    app: &tauri::AppHandle,
    state: &AppState,
    proposal: &CapabilityGapProposal,
) -> Result<bool, CommandError> {
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    state.evolution_consent_senders.lock().await.insert(proposal.id.clone(), tx);
    // 事件下发失败不阻断：视为拒绝（保持原安全行为），并清理挂起槽
    if let Err(e) = app.emit(EVOLUTION_CONSENT_EVENT, proposal) {
        tracing::warn!(%e, "🧭 能力补齐提议事件下发失败，视为用户拒绝");
        state.evolution_consent_senders.lock().await.remove(&proposal.id);
        return Ok(false);
    }
    tracing::info!(proposal_id = %proposal.id, "🧭 能力补齐提议已下发，等待用户同意/拒绝");
    let approved = match tokio::time::timeout(CONSENT_TIMEOUT, rx).await {
        Ok(Ok(approved)) => approved,
        // sender 被 drop（前端从未回传）或超时 → 视为拒绝
        Ok(Err(_)) | Err(_) => false,
    };
    // 清理残留挂起槽（前端可能从未回传）
    state.evolution_consent_senders.lock().await.remove(&proposal.id);
    tracing::info!(proposal_id = %proposal.id, approved = %approved, "🧭 能力补齐提议审批结果");
    Ok(approved)
}

/// 能力补齐提议的用户审批回传请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGapConsentRequest {
    /// 提议 ID（与 `CapabilityGapProposal.id` 对应）
    #[serde(rename = "proposalId")]
    pub proposal_id: String,
    /// 用户是否同意补齐
    pub approved: bool,
}

/// 能力补齐提议的用户审批回传 — 前端 EvolutionConsentModal 同意/拒绝后调用。
///
/// 从挂起审批槽取出对应 sender 回传结果，唤醒认知编排器中阻塞的 `await_user_consent`。
#[agent_command(domain = cognitive, safety = Caution, call_mode = StateInput, description = "回传能力补齐提议的用户同意/拒绝结果")]
#[tauri::command]
pub async fn capability_gap_consent(
    state: State<'_, AppState>,
    request: CapabilityGapConsentRequest,
) -> Result<(), CommandError> {
    let mut senders = state.evolution_consent_senders.lock().await;
    if let Some(sender) = senders.remove(&request.proposal_id) {
        let _ = sender.send(request.approved);
        tracing::info!(
            proposal_id = %request.proposal_id,
            approved = %request.approved,
            "🧭 能力补齐提议审批回传"
        );
    } else {
        tracing::warn!(
            proposal_id = %request.proposal_id,
            "🧭 能力补齐提议审批回传：无挂起的提议（可能已超时）"
        );
    }
    Ok(())
}

/// 拒绝即证据：安全拦截拒绝的结构化决策标签（T0.3 用户拒绝分支持久化用）。
fn build_rejection_decision(rejection: &PromptRejection) -> serde_json::Value {
    serde_json::json!({
        "executionMode": "rejected",
        "routePath": "security:rejected",
        "confidence": 0.0,
        "rejection": {
            "category": serde_json::to_value(rejection.category).unwrap_or_default(),
            "pattern": rejection.pattern,
            "reason": rejection.reason,
            "suggestion": rejection.suggestion,
        },
    })
}

/// 补齐产物决策标签（T0.3/T0.4/T0.5 用户同意补齐后持久化用，
/// 供阶段三贝叶斯后验消费：记录补齐原因 + 缺口类型）。
fn build_gap_decision(proposal: &CapabilityGapProposal) -> serde_json::Value {
    serde_json::json!({
        "executionMode": "gap_proposal",
        "routePath": "evolution:gap_proposal",
        "confidence": 0.0,
        "gapProposal": {
            "id": proposal.id,
            "gapType": serde_json::to_value(proposal.gap_type).unwrap_or_default(),
            "category": proposal.category.map(|c| serde_json::to_value(c).unwrap_or_default()),
            "title": proposal.title,
            "proposal": proposal.proposal,
            "reason": proposal.reason,
            "impact": proposal.impact,
            "rollback": proposal.rollback,
        },
    })
}

// ── 阶段三 T3.3：决策标签作为证据源 ─────────────────

/// 进化产物真实执行反馈对照明细（按 tool_id，T5A.4）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionDetail {
    /// 进化产物标识（`GeneratedTool.id`）。
    pub tool_id: String,
    /// 真实执行次数。
    pub usage_count: u32,
    /// 真实成功次数。
    pub successes: u32,
    /// 真实失败次数。
    pub failures: u32,
}

/// 进化产物真实执行反馈汇总（T5A.4 决策视图的真实证据对照）。
///
/// 与决策标签流（按 executionMode 推断成败）对照展示，
/// 真实执行结果由 wiring 层 `EvolutionFeedbackSinkImpl` 累计。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionFeedbackView {
    /// 有真实执行反馈的进化产物数。
    pub tool_count: usize,
    /// 真实执行总次数。
    pub total_runs: u32,
    /// 真实成功总次数。
    pub total_successes: u32,
    /// 真实失败总次数。
    pub total_failures: u32,
    /// 真实成功率（0~1，无执行时 0）。
    pub success_rate: f64,
    /// 按产物的明细（按 tool_id 排序）。
    pub details: Vec<ToolExecutionDetail>,
}

/// 会话决策标签流的贝叶斯进化评估结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionEvidenceView {
    /// 进化决策（evolve / stable / observe）。
    pub decision: String,
    /// 决策原因（中文）。
    pub reason: String,
    /// 贝叶斯后验 `P(success)`（决策标签流 + 真实执行反馈融合后）。
    pub p_success: f64,
    /// 已积累的（置信度加权）证据量（含真实执行反馈）。
    pub evidence_volume: f64,
    /// 消费的证据条数（有效决策标签，排除 clarify/ask 中立）。
    pub consumed_labels: usize,
    /// 决策标签总数。
    pub total_labels: usize,
    /// 证据来源的路由路径（去重），供用户查看命中能力。
    pub route_paths: Vec<String>,
    /// 进化产物真实执行反馈汇总（真实成败证据，与决策标签推断对照）。
    pub execution_feedback: ExecutionFeedbackView,
}

/// T3.3：将「决策标签流」作为证据源接入贝叶斯后验，输出进化决策。
///
/// 读取指定会话内所有 assistant 消息已持久化的 `decision` 字段
/// （每条含 `executionMode` / `routePath` / `confidence` / 选中能力），
/// 逐条经 `axagent_trajectory::EvolutionDecider` 消费：
/// - `workflow`/`direct`/`parameter_extract`/`agent`/`plan` → 成功证据
/// - `rejected`/`gap_proposal` → 失败证据（安全拦截拒绝、补齐提议）
/// - `clarify`/`ask` → 中立，不污染后验
///
/// 低于 `evolve_threshold` 且证据足够 → 输出 `evolve`（进入用户同意通道）。
#[agent_command(
    domain = cognitive,
    safety = Safe,
    call_mode = StateOnly,
    description = "将会话决策标签流接入贝叶斯后验，评估进化决策"
)]
#[tauri::command]
pub async fn cognitive_evolution_decision(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<EvolutionEvidenceView, CommandError> {
    let conversation_id = conversation_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::EMPTY_INPUT)
            .with_category(ErrorCategory::Validation));
    }
    // T5A.4：读取进化产物真实执行统计快照（wiring 层累计），融合进决策视图
    let execution_stats = state.evolution_execution_stats.lock().await.clone();
    evaluate_evolution_evidence(state.harness.db(), &conversation_id, &execution_stats)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))
}

/// T3.3：决策标签流 → 贝叶斯后验 → 进化视图 的纯执行逻辑。
///
/// 抽成独立函数便于单元测试（无需构造完整 AppState，仅依赖数据库连接）。
/// T5A.4 起额外接收进化产物真实执行统计快照，与决策标签流融合为双证据后验。
/// D2 起统计快照按会话维度组织（`conversation_id → tool_id → stats`），
/// 本函数只消费当前会话产物的真实反馈，避免跨会话污染决策。
pub(crate) async fn evaluate_evolution_evidence(
    db: &axagent_harness::DatabaseConnection,
    conversation_id: &str,
    execution_stats: &HashMap<String, HashMap<String, ToolExecutionStats>>,
) -> Result<EvolutionEvidenceView, axagent_harness::core_error::AxAgentError> {
    // 读取会话内消息的决策标签流（每条 assistant 消息的 decision 字段）
    let messages = axagent_dao::repo::message::list_messages(db, conversation_id).await?;

    let labels: Vec<serde_json::Value> =
        messages.iter().filter_map(|m| m.decision.clone()).collect();
    let total_labels = labels.len();

    // D2 会话隔离：仅取本会话产物的真实执行反馈（空会话 → 空表，退化为冷启动推断）
    let empty: HashMap<String, ToolExecutionStats> = HashMap::new();
    let session_stats: &HashMap<String, ToolExecutionStats> =
        execution_stats.get(conversation_id).unwrap_or(&empty);

    // 逐条消费为贝叶斯证据（决策标签流：按 executionMode 推断成败）
    let mut decider = axagent_trajectory::EvolutionDecider::new();
    let consumed_labels = decider.consume_decision_labels(&labels);

    // T5A.4：融合真实执行反馈（进化产物真实成败，校正「按模式推断」的偏差）。
    // D1 真实优先：decide() 在存在真实反馈时仅以真实后验判定，推断证据不稀释。
    for stats in session_stats.values() {
        decider.consume_execution_stats(stats);
    }

    // T5A.4：真实执行反馈汇总视图（供前端对照展示，仅含本会话产物）
    let mut details: Vec<ToolExecutionDetail> = session_stats
        .iter()
        .map(|(tool_id, s)| ToolExecutionDetail {
            tool_id: tool_id.clone(),
            usage_count: s.usage_count,
            successes: s.successes,
            failures: s.failures,
        })
        .collect();
    details.sort_by(|a, b| a.tool_id.cmp(&b.tool_id));
    let total_runs: u32 = session_stats.values().map(|s| s.usage_count).sum();
    let total_successes: u32 = session_stats.values().map(|s| s.successes).sum();
    let total_failures: u32 = session_stats.values().map(|s| s.failures).sum();

    let (decision, reason) = decider.describe();

    // 证据来源路由路径（去重）
    let mut route_paths: Vec<String> = labels
        .iter()
        .map(axagent_trajectory::DecisionEvidence::from_json)
        .filter(|e| e.is_evidential())
        .map(|e| e.route_path)
        .filter(|p| !p.is_empty())
        .collect();
    route_paths.sort();
    route_paths.dedup();

    Ok(EvolutionEvidenceView {
        decision: match decision {
            axagent_trajectory::EvolutionDecision::Evolve => "evolve".to_string(),
            axagent_trajectory::EvolutionDecision::Stable => "stable".to_string(),
            axagent_trajectory::EvolutionDecision::Observe => "observe".to_string(),
        },
        reason,
        p_success: decider.p_success(),
        evidence_volume: decider.evidence_volume(),
        consumed_labels,
        total_labels,
        route_paths,
        execution_feedback: ExecutionFeedbackView {
            tool_count: session_stats.len(),
            total_runs,
            total_successes,
            total_failures,
            success_rate: if total_runs > 0 {
                total_successes as f64 / total_runs as f64
            } else {
                0.0
            },
            details,
        },
    })
}

// ── 阶段三 T3.5：决策标签流 → 贝叶斯后验 的集成测试 ─────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::MessageRole;
    use serde_json::json;

    /// 创建一条 assistant 消息并写入决策标签（T3.3 证据源）。
    ///
    /// 消息表对会话存在外键约束，需先创建会话（`create_test_pool` 开启了
    /// `PRAGMA foreign_keys=ON`），否则插入消息会报 FOREIGN KEY constraint failed。
    async fn create_message_with_decision(
        db: &axagent_harness::DatabaseConnection,
        conversation_id: &str,
        decision: &serde_json::Value,
    ) -> String {
        let msg = axagent_dao::repo::message::create_message(
            db,
            conversation_id,
            MessageRole::Assistant,
            "test",
            &[],
            None,
            0,
        )
        .await
        .expect("测试：创建消息应成功");
        axagent_dao::repo::message::update_message_decision(db, &msg.id, Some(decision))
            .await
            .expect("测试：写入决策标签应成功");
        msg.id
    }

    /// 创建测试会话（消息外键依赖）。
    async fn create_test_conversation(
        db: &axagent_harness::DatabaseConnection,
        title: &str,
    ) -> String {
        axagent_dao::repo::conversation::create_conversation(
            db,
            title,
            "model-1",
            "provider-1",
            None,
        )
        .await
        .expect("测试：创建会话应成功")
        .id
    }

    /// 强成功流：5 次高置信成功（0.9+0.95+0.8+0.9+0.85=4.4）+ 1 次安全拦截拒绝（1.0）。
    /// 后验 P(success)=5.4/7.4≈0.729 > 0.7 → stable；routePath 去重。
    #[tokio::test]
    async fn evolution_evidence_stable_on_strong_success_stream() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let conversation_id = create_test_conversation(&db, "Evolution stable").await;

        for (i, (mode, confidence)) in [
            ("workflow", 0.9),
            ("workflow", 0.95),
            ("direct", 0.8),
            ("parameter_extract", 0.9),
            ("agent", 0.85),
        ]
        .into_iter()
        .enumerate()
        {
            let label = json!({
                "executionMode": mode,
                "routePath": format!("/trade/refund/auto{}", if i == 3 { "x" } else { "" }),
                "confidence": confidence,
                "selectedWorkflowName": "refund-auto",
            });
            create_message_with_decision(&db, &conversation_id, &label).await;
        }
        // 1 条 rejected（routePath 与前重复，验证去重）
        create_message_with_decision(
            &db,
            &conversation_id,
            &json!({
                "executionMode": "rejected",
                "routePath": "/trade/refund/auto",
                "confidence": 1.0,
            }),
        )
        .await;

        let view = evaluate_evolution_evidence(&db, &conversation_id, &HashMap::new())
            .await
            .expect("测试：进化评估应成功");

        assert_eq!(view.total_labels, 6);
        assert_eq!(view.consumed_labels, 6);
        assert_eq!(view.decision, "stable");
        assert!((view.p_success - 5.4 / 7.4).abs() < 1e-9, "p_success={}", view.p_success);
        assert!(view.evidence_volume >= 3.0);
        assert_eq!(view.route_paths, vec!["/trade/refund/auto", "/trade/refund/autox"]);
    }

    /// 失败流：4 条安全拦截拒绝（0.9×4）。后验 P(success)=1/5.6≈0.179 < 0.4 → evolve。
    #[tokio::test]
    async fn evolution_evidence_evolves_on_rejection_stream() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let conversation_id = create_test_conversation(&db, "Evolution evolve").await;

        for i in 0..4 {
            create_message_with_decision(
                &db,
                &conversation_id,
                &json!({
                    "executionMode": "rejected",
                    "routePath": format!("/gate/guard/{i}"),
                    "confidence": 0.9,
                }),
            )
            .await;
        }

        let view = evaluate_evolution_evidence(&db, &conversation_id, &HashMap::new())
            .await
            .expect("测试：进化评估应成功");

        assert_eq!(view.total_labels, 4);
        assert_eq!(view.consumed_labels, 4);
        assert_eq!(view.decision, "evolve");
        assert!(view.p_success < 0.4, "p_success={}", view.p_success);
        assert!(view.evidence_volume >= 3.0);
        assert_eq!(view.route_paths.len(), 4);
    }

    /// 中立流：仅 clarify/ask 决策标签 → 不贡献证据 → observe。
    #[tokio::test]
    async fn evolution_evidence_ignores_neutral_labels() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let conversation_id = create_test_conversation(&db, "Evolution neutral").await;

        for mode in ["clarify", "ask"] {
            create_message_with_decision(
                &db,
                &conversation_id,
                &json!({ "executionMode": mode, "confidence": 0.7 }),
            )
            .await;
        }

        let view = evaluate_evolution_evidence(&db, &conversation_id, &HashMap::new())
            .await
            .expect("测试：进化评估应成功");

        assert_eq!(view.total_labels, 2);
        assert_eq!(view.consumed_labels, 0);
        assert_eq!(view.decision, "observe");
        assert_eq!(view.route_paths, Vec::<String>::new());
    }

    /// 空会话：无决策标签 → 证据量 0 → observe。
    #[tokio::test]
    async fn evolution_evidence_empty_conversation_observes() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;

        let view = evaluate_evolution_evidence(&db, "conv-evolution-empty", &HashMap::new())
            .await
            .expect("测试：进化评估应成功");

        assert_eq!(view.total_labels, 0);
        assert_eq!(view.consumed_labels, 0);
        assert_eq!(view.decision, "observe");
        // T5A.4：无真实执行反馈时汇总为空
        assert_eq!(view.execution_feedback.tool_count, 0);
        assert_eq!(view.execution_feedback.total_runs, 0);
        assert_eq!(view.execution_feedback.success_rate, 0.0);
    }

    /// T5A.4：真实执行反馈融合 — 决策标签流推断为 stable，真实执行反馈（全失败）
    /// 在 D1 真实优先下不稀释：真实后验 P=0 → 触发 evolve。
    #[tokio::test]
    async fn evolution_evidence_fuses_real_execution_feedback() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let conversation_id = create_test_conversation(&db, "Evolution fuse").await;

        // 决策标签流：5 次高置信执行成功 → 单看标签流推断为 stable
        for _ in 0..5 {
            create_message_with_decision(
                &db,
                &conversation_id,
                &json!({ "executionMode": "workflow", "confidence": 1.0 }),
            )
            .await;
        }

        // D2 会话隔离：按 (conversation_id → tool_id → stats) 嵌套组织
        // 真实执行反馈：tool-a 真实 3 次全失败 → D1 真实优先，推断成功不稀释 → evolve
        let mut execution_stats: HashMap<String, HashMap<String, ToolExecutionStats>> =
            HashMap::new();
        execution_stats.insert(
            conversation_id.clone(),
            HashMap::from([(
                "tool-a".to_string(),
                ToolExecutionStats { usage_count: 3, successes: 0, failures: 3 },
            )]),
        );

        let view = evaluate_evolution_evidence(&db, &conversation_id, &execution_stats)
            .await
            .expect("测试：进化评估应成功");

        // D1 真实优先：真实后验 P(success)=0/3=0 < 0.4 → evolve
        assert_eq!(view.decision, "evolve");
        assert!((view.p_success - 0.0).abs() < 1e-9, "p_success={}", view.p_success);
        // 真实反馈对照视图
        assert_eq!(view.execution_feedback.tool_count, 1);
        assert_eq!(view.execution_feedback.total_runs, 3);
        assert_eq!(view.execution_feedback.total_successes, 0);
        assert_eq!(view.execution_feedback.total_failures, 3);
        assert_eq!(view.execution_feedback.success_rate, 0.0);
        assert_eq!(view.execution_feedback.details.len(), 1);
        assert_eq!(view.execution_feedback.details[0].tool_id, "tool-a");
        assert_eq!(view.execution_feedback.details[0].failures, 3);
    }
}
