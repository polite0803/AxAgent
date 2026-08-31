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
use axagent_agent_macro::agent_command;
use axagent_harness::workflow_evolution::ToolExecutionStats;
use axagent_harness::workflow_types::Variable;
use axagent_harness::{
    CandidateSummary, CapabilityDomain, CapabilityGapProposal, CapabilityGapType, CapabilityKind,
    CapabilityQuery, DynamicGuardRule, ExecutionMode, ModeHint, PatternPromptGuard,
    PromptAttackCategory, PromptGuard, PromptRejection, RouteStageRecord, RoutingDecisionV2,
    TaskShapeDecision,
};
// 遗留边界③：任务拆解（RuleBasedDecomposer，纯规则无 LLM，不违反 orchestrator 运行时边界）
use axagent_orchestrator::OrchestrationStrategy;
use axagent_orchestrator::decomposer::{MissionDecomposer, RuleBasedDecomposer};
use axagent_runtime::work_engine::{
    RunOptions, SubWorkflowCallback, SubWorkflowLaunch, unwrap_end_envelope,
};
use dashmap::DashMap;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tauri::State;

// ── 多轮短路缓存 ────────────────────────────────────────
// 首轮完整三层路由，后续轮次复用上一轮的 capability_id + execution_mode，跳过路由。
// key = conversation_id，value = 上一次成功路由的决策快照。
// 注意：缓存仅内存（进程生命周期），重启后自然失效。
#[derive(Clone)]
struct LastRouteDecision {
    capability_id: String,
    execution_mode: String,
    route_path: String,
    domain: String,
    cluster: String,
    /// 决策时的会话消息数，用于检测用户是否清过会话
    msg_count: u64,
    /// 决策时间戳，超时（>10min）自动失效
    timestamp: Instant,
}

/// 多轮短路缓存：conversation_id → 上一次路由决策
static ROUTE_SHORT_CIRCUIT: LazyLock<DashMap<String, LastRouteDecision>> =
    LazyLock::new(DashMap::new);

/// 短路缓存有效期（10 分钟），超过此时间强制重新路由
const SHORT_CIRCUIT_TTL: Duration = Duration::from_secs(10 * 60);

/// 会话消息数变化阈值：会话消息数与缓存记录差异超过此值时强制重新路由
/// （用户清空/删除过消息后，上下文已变，不能沿用旧决策）
const MSG_COUNT_TOLERANCE: u64 = 2;

// ── P1: 任务形态分类器（原则三标尺） ──────────────────────────
//
// 在三层路由前调用，产出 `TaskShapeDecision` 注入到：
// - 主 DAG variables（`__task_shape`）供路由管线消费
// - `RoutingDecisionV2.task_shape`（决策留痕）
// - `AgentQueryRequest.task_shape`（运行时按任务覆盖权限初值）
// - `CognitiveQueryResponse.task_shape`（前端展示决策标签）
//
// flag 关闭时（`UNITY_P0_TASK_SHAPE_ENABLED = false`）不注入，走旧链路。
const UNITY_P0_TASK_SHAPE_ENABLED: bool = true;

/// 在三层路由前执行任务形态分类。
///
/// 当 `UNITY_P0_TASK_SHAPE_ENABLED = true` 时调用 `classify_hybrid`（规则优先 +
/// LLM 兜底），返回 `Some(TaskShapeDecision)`；否则返回 `None`，走旧链路。
///
/// # 参数
/// - `state`: AppState（用于获取 LLM 分类器）
/// - `input`: 用户原始输入（已过安全拦截）
/// - `options`: Agent 执行选项（提取活跃域用于权限模式推断）
async fn classify_task_shape(
    state: &AppState,
    input: &str,
    options: Option<&AgentOptions>,
) -> Option<TaskShapeDecision> {
    if !UNITY_P0_TASK_SHAPE_ENABLED {
        return None;
    }
    // 推断当前权限模式：从 options.active_domains 推断，缺省 WorkspaceWrite
    let permission = axagent_harness::runtime_types::permissions::PermissionMode::WorkspaceWrite;
    let _ = options; // 预留：后续可从 options 推断更精确的权限模式
    // P3: 走 classify_hybrid（规则优先 + LLM 兜底）
    let decision = axagent_orchestrator::classify_hybrid(
        input,
        permission,
        Some(state.task_shape_llm_classifier.as_ref()),
    )
    .await
    .ok()?;
    tracing::debug!(
        context_cost = ?decision.context_cost,
        isolation_need = ?decision.isolation_need,
        strategy = ?decision.recommended_strategy,
        merge_score = decision.merge_score,
        split_score = decision.split_score,
        "🧭 任务形态分类完成（原则三标尺 + LLM 兜底）"
    );
    Some(decision)
}

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
    /// 熔断过滤数量（RAR 原始候选数 - 最终候选数，0 表示无过滤）
    #[serde(default)]
    pub filtered_count: usize,
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
    /// P1: 任务形态决策（原则三标尺输出，Step 0 产出）
    ///
    /// 当 `UNITY_P0_TASK_SHAPE` flag 启用时由 `DefaultTaskShapeClassifier` 在路由前产出，
    /// 随响应返回前端展示决策标签（两条标尺 + 推荐策略 + 合并/拆分倾向）。
    /// `None` 表示 flag 未启用或分类失败已回退。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_shape: Option<axagent_harness::TaskShapeDecision>,
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
            filtered_count: 0,
            execution_mode: d.execution_mode.as_str().to_string(),
            selected_workflow_name: None,
            selected_agent_profile: None,
            stage_records: d.stage_records.iter().map(RouteStageView::from).collect(),
            total_elapsed_ms: d.total_elapsed_ms,
            execution: None,
            task_shape: d.task_shape,
        }
    }
}

// ── 能力补齐自动重试配置 ────────────────────────────────────

/// 能力补齐后自动重试的最大次数。
/// 防止因重复补齐导致的无限递归。
const MAX_CAPABILITY_GAP_RETRIES: usize = 2;

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
/// 字段与前端 `CognitiveDecisionInfo` 类型对齐：ExecutionMode / 路由路径 / 命中工作流 / 专家 / 任务形态。
fn build_decision_value(
    execution_mode: &str,
    route_path: &str,
    confidence: f64,
    selected_workflow_name: Option<String>,
    selected_agent_profile: Option<&SelectedAgentProfileView>,
    task_shape: Option<&TaskShapeDecision>,
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
        // P1: 任务形态决策（原则三标尺输出），前端 CognitiveDecisionCard 展示
        "taskShape": task_shape,
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
        response.task_shape.as_ref(),
    )
}

/// 把路由选中的能力信息合并进 agent_context 的 `routing_hint` 字段。
///
/// 渐进式披露（P1-4）：认知编排器已用三层路由完成能力精化，委派 agent_query 时
/// 必须把精化结果（capability_id / 名称 / 描述 / 类型）随行传递，作为
/// `<routing-hint>` slot 注入系统提示，agent 无需重新走索引层发现。
/// 按 `CapabilityKind` 附加对应的定义层加载指引。
fn merge_routing_hint(
    agent_context: Option<AgentContextPayload>,
    capability_id: &str,
    name: &str,
    description: &str,
    kind: &str,
) -> AgentContextPayload {
    let mut ctx = agent_context.unwrap_or_default();
    let mut hint = format!(
        "The cognitive orchestrator routed this request to capability `{}` (kind: {}).",
        capability_id, kind
    );
    if !name.is_empty() {
        hint.push_str(&format!("\n- Name: {}", name));
    }
    if !description.is_empty() {
        hint.push_str(&format!("\n- Description: {}", description));
    }
    hint.push_str(match kind {
        // 技能：定义层按需加载（SkillView 按 skill 名取 SKILL.md，SkillReference 取 references/）
        "skill" => "\nLoad this skill's full instructions with the \"SkillView\" tool (try the capability name as \"skill\") before executing the task.",
        "tool" => "\nLocate and call the matching tool to execute the task.",
        "knowledge_base" => "\nRetrieve relevant material from the matching knowledge base when answering.",
        "agent" => "\nFollow the recommended expert profile for this task.",
        _ => "",
    });
    ctx.routing_hint = Some(hint);
    ctx
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
/// 自动重试机制：能力补齐完成（`GAP_PROPOSAL_APPLIED`）后自动重试，最多重试 2 次，
/// 避免要求用户手动重发请求，提升用户体验。
#[agent_command(domain = cognitive, safety = Caution, call_mode = StateInput, description = "认知编排统一入口（三层路由决策并按执行模式分发）")]
#[tauri::command]
pub async fn cognitive_query(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: CognitiveQueryRequest,
) -> Result<CognitiveQueryResponse, CommandError> {
    let mut retry_count = 0;
    const MAX_RETRIES: usize = MAX_CAPABILITY_GAP_RETRIES;

    // ── 多轮短路前置：在调用 inner 之前检查缓存 ──
    // 命中条件：mode_hint=auto + 有缓存 + TTL 内 + 消息数未剧变 → 注入 forced_capability_id
    // 同时保存缓存值，拿到 response 后覆盖 domain/cluster/route_path/execution_mode（forced 路径会把这些
    // 字段设为空/推断值，不是真实路由决策值）。
    let mut request = request; // 可变，短路命中时注入 forced_capability_id
    let shortcut_override: Option<LastRouteDecision> = if request
        .mode_hint
        .as_deref()
        .is_none_or(|m| m.eq_ignore_ascii_case("auto"))
        && request.forced_capability_id.is_none()
        && request.conversation_id.is_some()
    {
        let conv_id = request.conversation_id.clone().unwrap_or_default();
        if let Some(cached) = ROUTE_SHORT_CIRCUIT.get(&conv_id) {
            if cached.timestamp.elapsed() < SHORT_CIRCUIT_TTL && !cached.capability_id.is_empty() {
                let current_msg_count = axagent_dao::repo::message::get_conversation_stats(
                    state.harness.db(),
                    &conv_id,
                )
                .await
                .map(|s| s.total_messages)
                .unwrap_or(0);

                let msg_diff = (current_msg_count as i64 - cached.msg_count as i64).unsigned_abs();
                if msg_diff <= MSG_COUNT_TOLERANCE {
                    tracing::info!(
                        target: "axagent.cognitive.shortcut",
                        "⚡ 多轮短路命中 conv_id={} capability_id={} exec_mode={} route_path={} domain={} cluster={} cached_msgs={} current_msgs={} elapsed_ms={}",
                        conv_id,
                        cached.capability_id,
                        cached.execution_mode,
                        cached.route_path,
                        cached.domain,
                        cached.cluster,
                        cached.msg_count,
                        current_msg_count,
                        cached.timestamp.elapsed().as_millis()
                    );
                    request.forced_capability_id = Some(cached.capability_id.clone());
                    Some(cached.clone())
                } else {
                    tracing::info!(
                        target: "axagent.cognitive.shortcut",
                        "⏭️ 短路跳过（消息数变化 {} > 阈值 {}）conv_id={}",
                        msg_diff, MSG_COUNT_TOLERANCE, conv_id
                    );
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    loop {
        match cognitive_query_inner(&app, state.clone(), &request, retry_count).await {
            Ok(mut response) => {
                // 短路命中后，用缓存的真实路由值覆盖 forced 路径的占位值
                if let Some(ovr) = &shortcut_override {
                    apply_shortcut_override(&mut response, ovr);
                }
                return Ok(response);
            },
            Err(e) => {
                if e.code == axagent_harness::error_codes::cognitive::GAP_PROPOSAL_APPLIED
                    && retry_count < MAX_RETRIES
                {
                    retry_count += 1;
                    tracing::info!(
                        retry_count = %retry_count,
                        "能力补齐完成，自动重试路由决策"
                    );
                    continue;
                }
                return Err(e);
            },
        }
    }
}

/// 短路命中后，用缓存的真实路由值覆盖 forced 路径的占位响应（F2）。
///
/// 短路缓存里存的 `execution_mode` 是上一轮主 DAG 按 confidence 分档的结果，
/// 而短路后实际走 forced 路径（按能力 kind 分派，见 `cognitive_query_inner`：
/// Workflow kind → WorkEngine，Agent kind → agent_query）。两者口径不同——
/// 缓存 mode=plan 而能力是 Workflow kind 时，若不覆盖，响应会报 plan 但实际
/// 跑了工作流，前端按 `execution.kind` 分发与 `executionMode` 标签就会打架。
///
/// 覆盖规则：
/// - `route_path` / `domain` / `cluster` 取缓存真实路由值（forced 路径的占位值不含这些）
/// - `execution_mode` 不取缓存值，统一以实际执行视图反推（`derive_mode_from_execution_view`），
///   保证响应字段 / decision 标签 / execution 视图三者一致
/// - `confidence` 保持 1.0（forced 路径的值，合理，不覆盖）
fn apply_shortcut_override(response: &mut CognitiveQueryResponse, ovr: &LastRouteDecision) {
    response.route_path = ovr.route_path.clone();
    response.domain = ovr.domain.clone();
    response.cluster = ovr.cluster.clone();
    response.execution_mode = derive_mode_from_execution_view(&response.execution).to_string();
}

/// 按实际执行视图反推 `execution_mode`（F2：短路缓存口径统一）。
///
/// 视图只有 4 种执行结果，对应 4 个真实分派目标，反推是确定性映射：
/// - Workflow 视图 → workflow（WorkEngine 执行）
/// - Plan 视图 → plan（plan_generate 执行）
/// - Clarify 视图 → clarify（候选卡片交用户选择）
/// - Agent 视图（含 execution 为 None 的极端情形）→ delegate（agent_query 执行）
fn derive_mode_from_execution_view(view: &Option<CognitiveExecutionView>) -> &'static str {
    match view {
        Some(CognitiveExecutionView::Workflow { .. }) => ExecutionMode::Workflow.as_str(),
        Some(CognitiveExecutionView::Plan { .. }) => ExecutionMode::Plan.as_str(),
        Some(CognitiveExecutionView::Clarify { .. }) => ExecutionMode::Clarify.as_str(),
        Some(CognitiveExecutionView::Agent { .. }) | None => ExecutionMode::Delegate.as_str(),
    }
}

/// 按能力护照的 exposure / kind 解析要注入 chat_tools 的工具与技能。
///
/// 原为通配分支内联逻辑，抽出后供三处复用：
/// - 通配分支（Ask/Act/Delegate）路由命中能力
/// - Clarify 二次执行（forced_capability_id，F4）
/// - 会话已加载能力集合（F3：合并注入，避免"已加载的能力组合"被路由丢下）
///
/// `CapabilityExposure` 三态语义（此前只判 Managed，Auto/OnDemand 走同一分支）：
/// - Auto：命中即把定义注入 —— 编排器替 LLM 做完披露，适合必然要用上的能力。
/// - OnDemand：不注入。能力已在 `<capability-index>` 目录里露出摘要，由 LLM 自己
///   调 CapabilityView 展开定义后再驱动 —— 披露的主动权交给 LLM，省 schema token。
/// - Managed：不注入，且属元能力，不得回灌给 LLM 上下文。
async fn resolve_exposure_injection(
    state: &AppState,
    capability_ids: &[String],
) -> (Option<Vec<String>>, Option<Vec<String>>) {
    let mut tools: Vec<String> = Vec::new();
    let mut skills: Vec<String> = Vec::new();
    for cid in capability_ids {
        let Some(p) = state.capability_indexer.get_passport(cid).await else {
            continue;
        };
        match p.exposure {
            axagent_harness::CapabilityExposure::OnDemand => {
                tracing::info!(
                    capability_id = %p.capability_id,
                    "🧭 [exposure] OnDemand：跳过定义注入，交由 LLM 经 CapabilityView 按需展开"
                );
            },
            axagent_harness::CapabilityExposure::Managed => {
                tracing::debug!(
                    capability_id = %p.capability_id,
                    "🧭 [exposure] Managed：元能力不回灌，仅保留路由结论"
                );
            },
            axagent_harness::CapabilityExposure::Auto => match p.kind {
                axagent_harness::CapabilityKind::Skill => {
                    let name = p
                        .capability_id
                        .strip_prefix("skill:")
                        .unwrap_or(&p.capability_id)
                        .to_string();
                    skills.push(name);
                },
                axagent_harness::CapabilityKind::Toolchain => {
                    for step in &p.steps {
                        if let Some(step_p) = state.capability_indexer.get_passport(step).await
                            && let Some(tr) = step_p.tool_ref
                        {
                            tools.push(tr.tool_name);
                        }
                    }
                },
                _ => {
                    if let Some(tr) = p.tool_ref {
                        tools.push(tr.tool_name);
                    }
                },
            },
        }
    }
    ((!tools.is_empty()).then_some(tools), (!skills.is_empty()).then_some(skills))
}

/// 合并两组注入结果：主列表优先，次列表去重追加（F3 已加载能力合并用）。
fn merge_injection(
    primary: (Option<Vec<String>>, Option<Vec<String>>),
    secondary: (Option<Vec<String>>, Option<Vec<String>>),
) -> (Option<Vec<String>>, Option<Vec<String>>) {
    let merge = |mut p: Vec<String>, s: Vec<String>| {
        for t in s {
            if !p.contains(&t) {
                p.push(t);
            }
        }
        p
    };
    let tools = match (primary.0, secondary.0) {
        (Some(p), Some(s)) => Some(merge(p, s)),
        (p, s) => p.or(s),
    };
    let skills = match (primary.1, secondary.1) {
        (Some(p), Some(s)) => Some(merge(p, s)),
        (p, s) => p.or(s),
    };
    (tools, skills)
}

/// 认知编排查询内部实现，被 `cognitive_query` 循环包装器调用以支持自动重试。
///
/// # 参数
/// - `app`: Tauri 应用句柄引用
/// - `state`: 全局应用状态（Tauri State，可 clone）
/// - `request`: 查询请求引用
/// - `retry_count`: 当前重试次数（0 = 首次调用，1 = 第一次重试，以此类推），
///   仅用于日志记录，不改变核心逻辑。
async fn cognitive_query_inner(
    app: &tauri::AppHandle,
    state: State<'_, AppState>,
    request: &CognitiveQueryRequest,
    _retry_count: usize,
) -> Result<CognitiveQueryResponse, CommandError> {
    let input = request.input.trim().to_string();
    if input.is_empty() {
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::EMPTY_INPUT)
            .with_category(ErrorCategory::Validation)
            .with_param("field", "input"));
    }

    // ── 前置 1：安全拦截（非阻塞式：存缺口 → 返回错误提示，T0.3）──
    // 检测注入/越狱/敏感指令。命中后：① 保留硬阻断（绝不透传给下游执行器）；
    // ② 生成结构化缺口提议，静默存储（不弹窗）；③ 返回错误提示，用户可在能力管理中审核后重新发送。
    let prompt_guard = PatternPromptGuard::new();
    let input = match prompt_guard.process_user_input_structured(&input) {
        Ok(processed) => processed,
        Err(rejection) => {
            tracing::error!(%rejection.reason, "🛡️ 安全拦截命中，存储能力缺口（非阻塞）");
            let proposal = build_capability_gap_proposal(Some(&rejection), &input);
            store_capability_gap(app, &state, &proposal).await?;
            // 默认拒绝：安全策略优先，用户审核通过后需重新发送
            return Err(CommandError::new(
                axagent_harness::error_codes::cognitive::PROMPT_REJECTED,
            )
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(format!(
                "{}。能力缺口已存储，可在能力管理中审核通过后重新发送请求。",
                rejection.reason
            )));
        },
    };

    // ── 前置 2.5：会话已加载能力感知（F3 软档）──
    // 本会话已通过 CapabilityLoad 加载过能力时，用户意图大概率是"用这些已加载能力组合做事"，
    // 不应被路由命中的现成工作流模板抢走执行权 —— 命中 domain 交集时转 agent 路径，
    // 让 LLM 在已加载能力上下文中编排（注入 extra_tools / extra_skills）。
    // 读不到会话状态或解析失败 → 视为未加载（不阻断路由）。
    let loaded_capability_ids: Vec<String> = match request.conversation_id.as_deref() {
        Some(cid) if !cid.trim().is_empty() => {
            let prefix = axagent_harness::session_state::namespace_prefix(
                axagent_harness::session_state::StateScope::Temp,
                axagent_harness::session_state::NS_SKILL_LOADED,
                cid,
                None,
            );
            state
                .session_state_store
                .list_by_prefix(&prefix)
                .await
                .unwrap_or_default()
                .iter()
                .filter_map(|e| {
                    serde_json::from_str::<serde_json::Value>(&e.value)
                        .ok()
                        .and_then(|v| v["capabilityId"].as_str().map(str::to_string))
                })
                .collect()
        },
        _ => Vec::new(),
    };
    if !loaded_capability_ids.is_empty() {
        tracing::info!(
            conversation_id = %request.conversation_id.clone().unwrap_or_default(),
            loaded = ?loaded_capability_ids,
            "🧭 会话已加载 {} 个能力，路由命中模板时按 domain 交集决定是否转 agent 路径",
            loaded_capability_ids.len()
        );
    }

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
            let decision = build_decision_value(
                ExecutionMode::ParameterExtract.as_str(),
                "",
                1.0,
                None,
                None,
                None,
            );
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
                filtered_count: 0,
                execution_mode: ExecutionMode::ParameterExtract.as_str().to_string(),
                selected_workflow_name: None,
                selected_agent_profile: None,
                stage_records: Vec::new(),
                total_elapsed_ms: 0,
                execution: Some(CognitiveExecutionView::Workflow {
                    workflow_id: target,
                    execution_id,
                }),
                // JSON 快速路径：用户已显式提供目标 ID，跳过路由，不产出 task_shape
                task_shape: None,
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
            // Clarify 二次执行：task_shape 已在前一次主调用中产出，此处不重复分类
            None,
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
                // F4：Clarify 二次执行把选中能力的定义按 exposure/kind 注入 chat_tools，
                // 免去 LLM 先调 CapabilityView 展开再多一轮往返（schema 已由目录摘要提供）。
                let (forced_tools, forced_skills) =
                    resolve_exposure_injection(&state, std::slice::from_ref(&forced_id)).await;
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
                    // P1-4: Clarify 精选结果随行（passport 名称/描述/kind）
                    agent_context: Some(merge_routing_hint(
                        request.agent_context.clone(),
                        &forced_id,
                        forced_passport.as_ref().map(|p| p.name.as_str()).unwrap_or(""),
                        forced_passport.as_ref().map(|p| p.description.as_str()).unwrap_or(""),
                        forced_kind.as_str(),
                    )),
                    // 透传认知编排决策模式：Clarify 二次执行按能力类型定性（Agent→delegate，
                    // 其余→workflow），让 agent 运行时感知当前编排模式
                    execution_mode: Some(execution_mode.clone()),
                    // Clarify 二次执行：task_shape 已在前一次主调用中产出，此处不重复分类
                    task_shape: None,
                    // Clarify 二次执行（F4）：命中能力的定义按 exposure/kind 注入，
                    // 免去 LLM 先 CapabilityView 展开再多一轮往返；无 tool_ref 时与旧行为一致。
                    extra_tools: forced_tools,
                    extra_skills: forced_skills,
                };
                let agent_resp =
                    crate::commands::agent::agent_query(app.clone(), state.clone(), agent_request)
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
            filtered_count: 0,
            execution_mode,
            selected_workflow_name,
            selected_agent_profile,
            stage_records: Vec::new(),
            total_elapsed_ms: 0,
            execution: Some(execution),
            // Clarify 二次执行：task_shape 已在前一次主调用中产出，此处不重复分类
            task_shape: None,
        });
    }

    // ── 三层路由决策：主 DAG 驱动（WorkEngine 同步执行认知编排器）──
    // 由 work_engine.run_workflow(cognitive_router_main) 执行完整路由工作流，
    // 返回的 EndNode 输出即 l3_result（含 route_path / capability_id / execution_mode /
    // candidates / 熔断标记等），替代原先 CognitiveRouter.route_with_hint 的硬编码三层调用。
    let total_start = std::time::Instant::now();
    let mode_hint = ModeHint::parse_str(request.mode_hint.as_deref().unwrap_or("auto"));

    tracing::info!(
        conversation_id = %request.conversation_id.as_deref().unwrap_or("none"),
        input_len = input.len(),
        mode_hint = ?mode_hint,
        provider_id = %request.provider_id.as_deref().unwrap_or("default"),
        model_id = %request.model_id.as_deref().unwrap_or("default"),
        "🚦 [DIAG] cognitive_query 入口 — 三层路由开始"
    );

    // ── P1 Step 0：任务形态分类（原则三标尺，在三层路由前产出）──
    // 产出 `TaskShapeDecision` 注入主 DAG variables + 最终响应，
    // 供路由管线 / AgentQueryRequest / 前端决策标签消费。
    // flag 关闭时返回 None，完全不影响旧链路。
    let task_shape_decision = classify_task_shape(&state, &input, request.options.as_ref()).await;

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
        // P1: 任务形态决策（原则三标尺输出），供主 DAG 各子工作流按需消费。
        // flag 关闭时为 null，路由管线应跳过此变量。
        Variable {
            name: "__task_shape".to_string(),
            var_type: "object".to_string(),
            value: serde_json::json!(task_shape_decision),
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
            // System capability 为同步回调（无 spawn_blocking / thread-local runtime
            // 后台任务），drop future 即中止，无孤儿执行，故取消为空操作。
            SubWorkflowLaunch {
                child_execution_id: String::new(),
                output: Box::pin(async move {
                    let result = cognitive_router
                        .execute_system_capability(&capability_id, cap_input)
                        .await?;
                    Ok((String::new(), result))
                }),
                cancel: Box::pin(async {}),
            }
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

    tracing::info!(
        workflow_id = %COGNITIVE_ROUTER_MAIN_ID,
        l1_pre = %l1_pre.as_str(),
        variables_count = opts.variables.as_ref().map(|v| v.len()).unwrap_or(0),
        "🚦 [DIAG] run_workflow 调用前"
    );
    let workflow =
        state.work_engine.run_workflow(COGNITIVE_ROUTER_MAIN_ID, opts).await.map_err(|e| {
            tracing::error!(
                workflow_id = %COGNITIVE_ROUTER_MAIN_ID,
                error = %e,
                "🚦 [DIAG] run_workflow 失败!"
            );
            CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                .with_category(ErrorCategory::Retryable)
                .with_detail(format!("认知编排器主 DAG 执行失败: {e}"))
        })?;

    tracing::info!(has_output = workflow.output.is_some(), "🚦 [DIAG] run_workflow 成功返回");

    // 🚦 [DIAG] 打印 results keys（每个节点的 output_var → 输出）和 output keys
    tracing::info!(
        result_keys = ?workflow.results.keys().cloned().collect::<Vec<_>>(),
        output_keys = ?workflow
            .output
            .as_ref()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())),
        "🚦 [DIAG] DAG results keys & output keys"
    );

    // ── 降级路径也要构建路由观测数据 ────────────────────────────────────
    // 认知编排的降级分支（general 域 Ask、能力补齐兜底、缺失字段降级）都是合法的路由
    // 路径，前端路由观测面板应该能看到完整的三层阶段，而不是一片空白。
    //
    // 构建策略：
    // 1. L1Domain 必有 — l1_pre 在主 DAG 前就已算出（规则匹配，零 LLM 成本）
    // 2. L2Cluster / L3* — 尝试从 workflow.results 中已完成的子工作流输出提取
    //    （results 同时以 node_id 和 output_var 为 key，直接用 results["l2_result"]
    //     / results["l3_result"] 就能拿到子工作流的 EndNode 信封）
    let fallback_stage_records: Vec<RouteStageRecord> = {
        let mut records: Vec<RouteStageRecord> = Vec::new();

        // L1 域路由：必有（预路由规则匹配）
        records.push(RouteStageRecord {
            stage: axagent_harness::RouteStage::L1Domain,
            success: true,
            confidence: 0.95, // 规则匹配置信度
            elapsed_ms: 0,
            summary: format!("L1 预路由判定: {}", l1_pre.as_str()),
        });

        // L2 簇路由：如果 l2_result 在 results 里，从里面提取 category/confidence
        if let Some(l2_raw) = workflow.results.get("l2_result") {
            let l2_obj = unwrap_end_envelope(l2_raw);
            if let Some(l2_cat) = l2_obj.get("category").and_then(|v| v.as_str()) {
                let conf = l2_obj.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
                records.push(RouteStageRecord {
                    stage: axagent_harness::RouteStage::L2Cluster,
                    success: true,
                    confidence: conf,
                    elapsed_ms: 0,
                    summary: format!("L2 簇路由: {} (置信度 {:.2})", l2_cat, conf),
                });
            }
        }

        // L3 阶段：如果 l3_result 在 results 里且含 stage_records，直接透传
        if let Some(l3_raw) = workflow.results.get("l3_result") {
            let l3_obj = unwrap_end_envelope(l3_raw);
            if let Some(arr) = l3_obj.get("stage_records").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Ok(rec) = serde_json::from_value::<RouteStageRecord>(item.clone()) {
                        records.push(rec);
                    }
                }
            }
        }

        records
    };
    let fallback_stage_views: Vec<RouteStageView> =
        fallback_stage_records.iter().map(RouteStageView::from).collect();

    // 4. 解析 l3_result（主 DAG EndNode 输出）
    // 主 DAG 无产出时：
    // - 若 L1 预路由判定为 general 域 → 构造合成 l3_result 降级为 Ask 模式
    //   （通用问答无需触发能力补齐，保持认知编排的业务分支语义）
    // - 其他域 → 进入能力补齐提议通道，用户同意后补齐能力并提示重发
    let l3_value = match workflow.output {
        Some(v) => v,
        None => {
            if l1_pre == CapabilityDomain::General {
                tracing::info!(
                    input_len = input.len(),
                    "🧭 L1=general 且主 DAG 无产出，降级为 Ask 模式（通用问答）"
                );
                serde_json::json!({
                    "route_path": "fallback:general_qa",
                    "domain": "general",
                    "cluster": "",
                    "capability_id": "",
                    "confidence": 0.5,
                    "execution_mode": "ask",
                    "is_circuit_broken": false,
                    "candidates": [],
                    "raw_count": 0,
                    "is_llm_fallback": true,
                    "fallback_path": "general_domain_ask_degrade",
                    "stage_records": [],
                })
            } else {
                let proposal = build_capability_gap_proposal(None, &input);
                store_capability_gap(app, &state, &proposal).await?;
                tracing::info!("🧭 主 DAG 无产出，能力缺口已存储，降级为 Ask 模式回答用户");
                return execute_general_ask(
                    app,
                    state,
                    request,
                    &input,
                    fallback_stage_views.clone(),
                )
                .await;
            }
        },
    };
    // B0: 拆开主 DAG EndNode 的终止信封。
    // EndNode 配置 output_var 时，EndExecutor 会把提取到的 l3_result 包装为
    // `{status:"terminated", node_id:"end", output:<实际值>, source:"l3_result"}`，
    // 且该信封经 apply_node_status_update 覆写 results["l3_result"]，最终
    // workflow.output 顶层是信封而非扁平路由决策 —— 不拆包则下方 B1 必备字段
    // 校验全部判缺失，误触发能力缺口存储 + Ask 模式降级。
    // 与 SubWorkflowExecutor 在子工作流边界的拆包补偿同构（见其 executor 注释）。
    let l3_value = unwrap_end_envelope(&l3_value);
    let l3 = l3_value.as_object().cloned().unwrap_or_default();

    // B1: 验证 l3_result 必须包含关键字段
    // 字段缺失时：
    // - 若 L1 预路由判定为 general 域 → 直接走 Ask 模式执行（通用问答兜底）
    // - 其他域 → 触发能力补齐提议通道
    const REQUIRED_L3_FIELDS: &[&str] =
        &["route_path", "capability_id", "confidence", "execution_mode"];
    let missing_fields: Vec<&str> =
        REQUIRED_L3_FIELDS.iter().filter(|f| !l3.contains_key(**f)).copied().collect();
    if !missing_fields.is_empty() {
        tracing::warn!(
            missing = ?missing_fields,
            l3_value = %l3_value,
            "认知编排主 DAG 输出缺少关键字段"
        );
        if l1_pre == CapabilityDomain::General {
            tracing::info!("🧭 字段缺失但 L1=general，降级为 Ask 模式（通用问答）");
            return execute_general_ask(app, state, request, &input, fallback_stage_views.clone())
                .await;
        }
        let proposal = build_capability_gap_proposal(None, &input);
        store_capability_gap(app, &state, &proposal).await?;
        tracing::info!("🧭 字段缺失，能力缺口已存储，降级为 Ask 模式回答用户");
        return execute_general_ask(app, state, request, &input, fallback_stage_views.clone())
            .await;
    }

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

    // 熔断过滤数量：RAR 原始候选数 - 最终候选数（兜底 0 表示无过滤）
    let raw_count = l3.get("raw_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let filtered_count = raw_count.saturating_sub(candidate_details.len());

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
    let circuit_break_reason_str =
        circuit_break_reason.as_deref().unwrap_or("self-reference circuit breaker");
    if l3.get("is_circuit_broken").and_then(|v| v.as_bool()).unwrap_or(false) {
        // 熔断 + 无候选（非自指）→ 进入能力补齐提议通道（T0.5）
        // 自指熔断是系统保护机制，不应触发能力补齐；其余场景（全部候选被拦截/无可用路径）
        // 表明系统当前无对应能力，征求用户同意后自动补齐。
        // 但若 domain=general，说明是通用问答场景，应直接降级为 Ask 模式而非触发补齐。
        if candidate_details.is_empty() && circuit_break_reason_str != "self_reference" {
            if domain == "general" {
                tracing::info!(
                    reason = %circuit_break_reason_str,
                    "🧭 熔断无候选但 domain=general，降级为 Ask 模式"
                );
                return execute_general_ask(
                    app,
                    state,
                    request,
                    &input,
                    fallback_stage_views.clone(),
                )
                .await;
            }
            let proposal = build_capability_gap_proposal(None, &input);
            store_capability_gap(app, &state, &proposal).await?;
            tracing::info!(reason = %circuit_break_reason_str,
                "🧭 熔断无候选，能力缺口已存储，降级为 Ask 模式回答用户");
            return execute_general_ask(app, state, request, &input, fallback_stage_views.clone())
                .await;
        }

        let mut params = HashMap::new();
        params.insert("reason".to_string(), circuit_break_reason_str.to_string());
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

    tracing::info!(
        route_path = %route_path,
        domain = %domain,
        cluster = %cluster,
        capability_id = %capability_id,
        confidence = confidence,
        execution_mode = %mode.as_str(),
        is_llm_fallback = is_llm_fallback,
        fallback_path = ?fallback_path,
        "🚦 [DIAG] 路由决策完成"
    );

    // 路由观测阶段记录：复用 fallback_stage_records（已包含 L1Domain + L2Cluster + L3Graph）。
    // 不直接取 l3.stage_records — 那里只有 L3Graph，会漏掉 L1/L2 阶段。
    let stage_records = fallback_stage_views.clone();

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
        filtered_count,
        execution_mode: mode.as_str().to_string(),
        selected_workflow_name,
        selected_agent_profile,
        stage_records,
        total_elapsed_ms: total_start.elapsed().as_millis() as u64,
        execution: None,
        task_shape: task_shape_decision.clone(),
    };

    // ── 缓存路由决策（供多轮短路复用）──
    // 只在有效路由完成时写入：非 Clarify、非 general_ask 降级、有 capability_id
    if !response.capability_id.is_empty()
        && mode != ExecutionMode::Clarify
        && request.conversation_id.is_some()
    {
        let conv_id = request.conversation_id.clone().unwrap_or_default();
        // 查消息数写入缓存
        let current_msg_count =
            axagent_dao::repo::message::get_conversation_stats(state.harness.db(), &conv_id)
                .await
                .map(|s| s.total_messages)
                .unwrap_or(0);

        ROUTE_SHORT_CIRCUIT.insert(
            conv_id.clone(),
            LastRouteDecision {
                capability_id: response.capability_id.clone(),
                execution_mode: mode.as_str().to_string(),
                route_path: response.route_path.clone(),
                domain: response.domain.clone(),
                cluster: response.cluster.clone(),
                msg_count: current_msg_count,
                timestamp: Instant::now(),
            },
        );
        tracing::info!(
            target: "axagent.cognitive.shortcut",
            "📝 缓存路由决策 conv_id={} capability_id={} exec_mode={} route_path={} msg_count={}",
            conv_id, response.capability_id, mode.as_str(), response.route_path, current_msg_count
        );
    }

    // ── 分支执行：按执行模式复用既有执行器 ──
    // 决策标签：为该轮执行生成，Workflow 分支透传给 workflow_execute 持久化，
    // Agent 分支在此处直接写入 assistant 消息。
    let decision = decision_from_response(&response);

    // ── F3 闸：会话已加载能力优先（软档：domain 交集）──
    // 已加载能力的 domain 与路由命中能力的 domain 有交集时（相等或任一方 General），
    // 不让现成模板直发执行——用户意图大概率是"用已加载的能力组合做事"，
    // 转 agent 路径让 LLM 在已加载能力上下文中编排（注入 extra_tools / extra_skills）。
    // 无交集（如加载了股票能力却问天气）仍走模板直发，避免无关请求多耗一轮 LLM。
    let defer_to_agent = if !loaded_capability_ids.is_empty() && !response.capability_id.is_empty()
    {
        match state.capability_indexer.get_passport(&response.capability_id).await {
            Some(hit) => {
                let mut overlap = false;
                for cid in &loaded_capability_ids {
                    if state.capability_indexer.get_passport(cid).await.is_some_and(|p| {
                        p.domain == hit.domain
                            || p.domain == CapabilityDomain::General
                            || hit.domain == CapabilityDomain::General
                    }) {
                        overlap = true;
                        break;
                    }
                }
                overlap
            },
            None => false,
        }
    } else {
        false
    };
    if defer_to_agent {
        tracing::info!(
            capability_id = %response.capability_id,
            execution_mode = ?mode,
            "🧭 F3：会话已加载能力与命中模板 domain 交集，转 agent 路径（不直发模板）"
        );
        // ── 一致性修正：mode 与响应字段须反映真实分派 ──
        // 守卫只让 Workflow/Direct 落到本函数的通配分支（实际由 agent_query 执行），
        // 但 mode 本身仍是 Workflow，会连带两处不一致（与 F2 短路缓存问题同类）：
        //   ① response.execution_mode = "workflow" 却返回 Agent 执行视图
        //      → 前端展示标签撒谎；落库决策标签也记成 workflow，污染进化证据统计；
        //   ② 下方通配分支把 mode 透传给 agent_query 的 execution_mode 参数
        //      → agent 运行时被错误告知"workflow 模式"。
        // 统一改为 Delegate（委派给 agent 执行），与实际分派路径对齐。
        // 注：decision（上方 decision_from_response 生成）只在 Workflow 分支
        // 传给 workflow_execute，defer 路径不使用，无需重算。
        mode = ExecutionMode::Delegate;
        response.execution_mode = ExecutionMode::Delegate.as_str().to_string();
    }

    // ── Clarify 兜底无候选 → 能力补齐提议通道（T0.5）──
    // 主 DAG 决策为 Clarify（置信度模糊）但候选为空（RAR/图谱兜底无命中）时，
    // 不进入空候选展示，而是生成 capability_missing 提议征求用户同意；
    // 拒绝则保持原 Clarify 空候选行为（返回空候选，前端自行兜底）。
    // 但若 domain=general，说明是通用问答场景，应直接降级为 Ask 模式。
    if mode == ExecutionMode::Clarify && response.candidate_details.is_empty() {
        if response.domain == "general" {
            tracing::info!("🧭 Clarify 空候选但 domain=general，降级为 Ask 模式");
            return execute_general_ask(app, state, request, &input, fallback_stage_views.clone())
                .await;
        }
        let proposal = build_capability_gap_proposal(None, &input);
        store_capability_gap(app, &state, &proposal).await?;
        tracing::info!("🧭 Clarify 空候选，能力缺口已存储，降级为 Ask 模式回答用户");
        return execute_general_ask(app, state, request, &input, fallback_stage_views.clone())
            .await;
    }

    response.execution = Some(match mode {
        // Workflow / Direct：capability_id 即工作流模板 ID，交给 WorkEngine 执行
        // 执行失败 → 存缺口 + 降级 LLM 回答（分支 1）
        // 守卫：会话已加载能力与命中模板 domain 交集时（F3），不直发模板，
        // 落到下方通配分支转 agent 路径（LLM 在已加载能力上下文中编排）。
        ExecutionMode::Workflow | ExecutionMode::Direct if !defer_to_agent => {
            let workflow_id = response.capability_id.clone();
            match crate::commands::workflows::workflow_execute(
                app.clone(),
                state.clone(),
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
            {
                Ok(execution_id) => {
                    // 记录补齐工作流命中执行（evolution:workflow: 前缀的为补齐产物）
                    if workflow_id.starts_with("evolution:workflow:") {
                        tracing::info!(
                            capability_id = %workflow_id,
                            execution_mode = ?mode,
                            "补齐工作流已被命中执行"
                        );
                    }
                    CognitiveExecutionView::Workflow { workflow_id, execution_id }
                },
                Err(e) => {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        error = %e,
                        "🧭 工作流执行失败，存储能力缺口并降级为 LLM 回答"
                    );
                    let proposal = build_capability_gap_proposal(None, &input);
                    store_capability_gap(app, &state, &proposal).await?;
                    return execute_general_ask(
                        app,
                        state,
                        request,
                        &input,
                        fallback_stage_views.clone(),
                    )
                    .await;
                },
            }
        },
        // ParameterExtract 无主流程分支：该值仅由 JSON 快速通道（前置 2）直接 return 产出，
        // 主 match 的 `_` 通配 arm 会接住任何意外出现的该值（实际不可达）。
        // Plan：域明确但无具体工作流命中，触发 plan_generate 拆解任务
        // 执行失败 → 存缺口 + 降级 LLM 回答（分支 3）
        ExecutionMode::Plan => {
            let conversation_id = request.conversation_id.clone().ok_or_else(|| {
                CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
                    .with_category(ErrorCategory::Validation)
                    .with_param("field", "conversation_id")
            })?;
            match crate::commands::plan::plan_generate(
                state.clone(),
                app.clone(),
                crate::commands::plan::PlanGenerateRequest {
                    conversation_id: conversation_id.clone(),
                    content: input.clone(),
                },
            )
            .await
            {
                Ok(plan) => CognitiveExecutionView::Plan { conversation_id, plan_id: plan.id },
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "🧭 Plan 生成失败，存储能力缺口并降级为 LLM 回答"
                    );
                    let proposal = build_capability_gap_proposal(None, &input);
                    store_capability_gap(app, &state, &proposal).await?;
                    return execute_general_ask(
                        app,
                        state,
                        request,
                        &input,
                        fallback_stage_views.clone(),
                    )
                    .await;
                },
            }
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

            // ── P3: 策略实装（ExecutionStrategy 真正影响执行路径）──────────

            // 2a. ApprovalGate 真实阻断：通过 oneshot 通道等待用户审批决策
            use axagent_harness::ExecutionStrategy;
            if let Some(ts) = &task_shape_decision {
                if matches!(ts.recommended_strategy, ExecutionStrategy::ApprovalGate) {
                    let preview: String = input.chars().take(80).collect();
                    let approval_id =
                        format!("{}-{}", conversation_id, chrono::Utc::now().timestamp_millis());

                    tracing::warn!(
                        conversation_id = %conversation_id,
                        approval_id = %approval_id,
                        input_preview = %preview,
                        evidence = ?ts.evidence,
                        merge_score = ts.merge_score,
                        split_score = ts.split_score,
                        "🚨 ApprovalGate 命中：高危任务触发审批阻断"
                    );

                    // 创建 oneshot 通道并注册到 AppState
                    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                    {
                        let mut senders = state.task_shape_approval_senders.lock().await;
                        senders.insert(approval_id.clone(), tx);
                    }

                    // 向前端发送审批请求事件
                    let _ = app.emit(
                        "task-shape-approval-request",
                        serde_json::json!({
                            "approvalId": &approval_id,
                            "conversationId": &conversation_id,
                            "inputPreview": &preview,
                            "evidence": &ts.evidence,
                            "mergeScore": ts.merge_score,
                            "splitScore": ts.split_score
                        }),
                    );

                    // 阻塞等待用户决策（true=批准, false=拒绝, 通道关闭=拒绝）
                    let approved = rx.await.unwrap_or(false);

                    // 清理通道
                    {
                        let mut senders = state.task_shape_approval_senders.lock().await;
                        senders.remove(&approval_id);
                    }

                    if !approved {
                        tracing::info!(
                            conversation_id = %conversation_id,
                            approval_id = %approval_id,
                            "ApprovalGate 审批被拒绝，任务终止"
                        );
                        response.execution = Some(CognitiveExecutionView::Agent {
                            conversation_id: conversation_id.clone(),
                            assistant_message_id: String::new(),
                            status: Some("approval_rejected".to_string()),
                        });
                        return Ok(response);
                    }

                    tracing::info!(
                        conversation_id = %conversation_id,
                        approval_id = %approval_id,
                        "ApprovalGate 审批通过，继续执行"
                    );
                }
            }

            // 2b. DelegateSingleExpert：用户未显式指定 expert 时，用分类器建议覆盖
            //     用户显式指定优先（request.agent_profile_id），尊重用户意图
            let effective_expert_id = if request.agent_profile_id.is_none() {
                task_shape_decision
                    .as_ref()
                    .and_then(|ts| match &ts.recommended_strategy {
                        ExecutionStrategy::DelegateSingleExpert {
                            expert_id: expert_from_shape,
                        } => {
                            tracing::info!(
                                from_shape = expert_from_shape,
                                already_selected = ?dynamic_expert_id,
                                "[cognitive_query] task_shape 建议委派专家"
                            );
                            Some(expert_from_shape.clone())
                        },
                        _ => None,
                    })
                    .or(dynamic_expert_id)
            } else {
                dynamic_expert_id
            };

            // 暴露闭环：命中能力按需注入（Phase 1.5 + 遗留边界①/②）。
            // - Tool：tool_ref 单数 → extra_tools
            // - Toolchain：steps 展开为各步骤真实工具 → extra_tools（遗留②：agent 拿到组合按序编排）
            // - Skill：按名加载 → extra_skills（遗留①：需注册 handler 才能执行，不能只注 schema）
            //
            // 抽取为 resolve_exposure_injection 后，此处还合并 F3 会话已加载能力的注入：
            // 用户已通过 CapabilityLoad 叠加的能力组合，同样按 exposure/kind 注入，
            // 让 LLM 在"路由命中能力 + 已加载能力"的完整上下文中编排。
            let (route_tools, route_skills) = resolve_exposure_injection(
                &state,
                if response.capability_id.is_empty() {
                    &[]
                } else {
                    std::slice::from_ref(&response.capability_id)
                },
            )
            .await;
            let (orchestration_tools, orchestration_skills) = if !loaded_capability_ids.is_empty() {
                let (loaded_tools, loaded_skills) =
                    resolve_exposure_injection(&state, &loaded_capability_ids).await;
                merge_injection((route_tools, route_skills), (loaded_tools, loaded_skills))
            } else {
                (route_tools, route_skills)
            };

            let agent_request = AgentQueryRequest {
                conversation_id,
                input: input.clone(),
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
                expert_id: effective_expert_id,
                // P1-4: 路由精化结果随行（capability_id/名称/描述/kind），agent 直接加载定义
                agent_context: {
                    let selected = response
                        .candidate_details
                        .iter()
                        .find(|c| c.capability_id == response.capability_id);
                    Some(merge_routing_hint(
                        request.agent_context.clone(),
                        &response.capability_id,
                        selected.map(|c| c.name.as_str()).unwrap_or(""),
                        selected.map(|c| c.description.as_str()).unwrap_or(""),
                        selected.map(|c| c.kind.as_str()).unwrap_or("workflow"),
                    ))
                },
                // 透传认知编排决策模式（Ask/Act/Delegate），让 agent 运行时感知当前编排模式
                execution_mode: Some(mode.as_str().to_string()),
                // P1: 透传任务形态决策（原则三标尺），运行时按任务而非按会话覆盖权限初值
                task_shape: task_shape_decision.clone(),
                // Phase 1.5 暴露闭环：命中能力的真实工具按需注入（tool_ref → chat_tools）
                extra_tools: orchestration_tools,
                // 遗留边界①：命中 Skill 时按名加载（skill_tools + handler 注册）
                extra_skills: orchestration_skills,
            };

            tracing::info!(
                conversation_id = %agent_request.conversation_id,
                execution_mode = ?agent_request.execution_mode,
                expert_id = ?agent_request.expert_id,
                agent_profile_id = ?agent_request.agent_profile_id,
                "🚦 [DIAG] 调用 agent_query 开始（同步阻塞等 LLM 回复）"
            );

            // Agent 执行失败 → 存缺口 + 降级 LLM 回答（分支 5）
            // 先提取执行载体 ID（agent_request 随后被 move 进 agent_query，无法再借用）
            let executed_agent_profile_id = agent_request.agent_profile_id.clone();
            let agent_resp = match crate::commands::agent::agent_query(
                app.clone(),
                state.clone(),
                agent_request,
            )
            .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!(
                        capability_id = %response.capability_id,
                        error = %e,
                        "🧭 Agent 执行失败，存储能力缺口并降级为 LLM 回答"
                    );
                    let proposal = build_capability_gap_proposal(None, &input);
                    store_capability_gap(app, &state, &proposal).await?;
                    return execute_general_ask(
                        app,
                        state,
                        request,
                        &input,
                        fallback_stage_views.clone(),
                    )
                    .await;
                },
            };

            tracing::info!(
                assistant_message_id = %agent_resp.assistant_message_id,
                status = ?agent_resp.status,
                "🚦 [DIAG] agent_query 完成返回"
            );

            // Phase 1 反馈闭环：Agent 能力执行统计回写（排序器 β 历史成功率数据源）。
            // capability_id = `agent:{profile_id}`（对齐 state.rs 专家护照注册格式）；
            // profile_id 缺失时无法定位护照，跳过；失败仅告警不阻塞主响应。
            if let Some(pid) = executed_agent_profile_id.as_deref() {
                let _ = axagent_dao::repo::capability_stats::record_execution(
                    state.harness.db(),
                    &format!("agent:{pid}"),
                    true, // agent_query 返回 Ok 即视为调用成功
                    0,    // 耗时由 agent 执行细节承载，此处不重复统计
                )
                .await;
            }

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

/// 通用问答降级执行器：当三层路由无法产出有效决策时，直接以 Ask 模式委派 agent 执行。
///
/// 适用场景：
/// 1. L1 预路由判定为 general 域但三层路由无产出——这不是「能力缺失」，
///    而是「通用问答」的正确降级路径。
/// 2. L1/L2/L3 部分路由阶段完成但最终 l3_result 缺关键字段或触发熔断——
///    兜底降级 Ask 模式继续执行，避免中断用户请求。
///
/// # 参数
/// - `stage_records`: 降级路径的路由观测记录。由调用方从已执行的路由阶段
///   （L1 预路由必有 + 从 workflow.results 提取的 L2/L3）构建，保证前端
///   路由观测面板能看到完整的三层路由轨迹，即使最终走了降级分支。
async fn execute_general_ask(
    app: &tauri::AppHandle,
    state: State<'_, AppState>,
    request: &CognitiveQueryRequest,
    input: &str,
    stage_records: Vec<RouteStageView>,
) -> Result<CognitiveQueryResponse, CommandError> {
    let conversation_id = request.conversation_id.clone().ok_or_else(|| {
        CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            .with_category(ErrorCategory::Validation)
            .with_param("field", "conversation_id")
    })?;

    tracing::info!(input_len = input.len(), "🧭 通用问答降级：以 Ask 模式委派 agent 执行");

    let agent_request = AgentQueryRequest {
        conversation_id: conversation_id.clone(),
        input: input.to_string(),
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
        agent_profile_id: None,
        expert_id: None,
        agent_context: request.agent_context.clone(),
        execution_mode: Some(ExecutionMode::Ask.as_str().to_string()),
        task_shape: None,
        // 通用问答降级：无能力命中，不注入工具/技能（Ask 模式本就以问答为主）
        extra_tools: None,
        extra_skills: None,
    };

    let agent_resp = crate::commands::agent::agent_query(app.clone(), state.clone(), agent_request)
        .await
        .map_err(|e| executor_error(e, axagent_harness::error_codes::cognitive::ROUTE_FAILED))?;

    Ok(CognitiveQueryResponse {
        route_path: "fallback:general_qa".to_string(),
        domain: "general".to_string(),
        cluster: String::new(),
        capability_id: String::new(),
        confidence: 0.5,
        is_llm_fallback: true,
        circuit_broken: false,
        circuit_break_reason: None,
        fallback_path: Some("general_domain_ask_degrade".to_string()),
        candidates: Vec::new(),
        candidate_details: Vec::new(),
        filtered_count: 0,
        execution_mode: ExecutionMode::Ask.as_str().to_string(),
        selected_workflow_name: None,
        selected_agent_profile: None,
        stage_records,
        total_elapsed_ms: 0,
        execution: Some(CognitiveExecutionView::Agent {
            conversation_id: agent_resp.conversation_id,
            assistant_message_id: agent_resp.assistant_message_id,
            status: agent_resp.status,
        }),
        task_shape: None,
    })
}

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
    input: &str,
) -> Result<(), CommandError> {
    match proposal.gap_type {
        CapabilityGapType::CapabilityMissing => {
            // 1. 生成实际工作流模板并落库
            let template_id = format!("auto_generated:{}", proposal.id);
            match crate::commands::capability_gap_workflow::generate_gap_workflow_template(
                state, input, proposal,
            )
            .await
            {
                Ok(template) => {
                    // 2. 注册能力护照（L2 检索可见）+ 同步图谱（L3 路由可见）
                    crate::commands::capability::register_evolution_product(
                        state,
                        &template.id,
                        &template.name,
                        &template.description.unwrap_or_default(),
                    )
                    .await?;
                    tracing::info!(
                        template_id = %template.id,
                        "能力补齐：工作流模板已创建，护照已注册"
                    );
                },
                Err(e) => {
                    // 模板生成失败时仍尝试注册护照（降级行为）
                    tracing::error!(%e, "工作流模板生成失败，尝试仅注册能力护照");
                    crate::commands::capability::register_evolution_product(
                        state,
                        &template_id,
                        proposal.title.trim(),
                        &proposal.proposal,
                    )
                    .await?;
                },
            }
        },
        CapabilityGapType::GuardRule => {
            let category = proposal.category.unwrap_or(PromptAttackCategory::Jailbreak);
            let rule = DynamicGuardRule {
                category,
                pattern: proposal.proposal.clone(),
                reason: proposal.reason.clone(),
                created_at: chrono::Utc::now(),
            };
            state.prompt_guard.add_dynamic_rule(rule).await;
            tracing::info!(
                category = ?category,
                pattern = %proposal.proposal,
                "GuardRule: 动态防护规则已注入"
            );
        },
        CapabilityGapType::ExemptAuthorize => {
            let exemption = DynamicGuardRule {
                category: PromptAttackCategory::Jailbreak,
                pattern: proposal.proposal.clone(),
                reason: format!("有界豁免: {}", proposal.reason),
                created_at: chrono::Utc::now(),
            };
            // ExemptAuthorize 用豁免模式，不在安全拦截器中检查
            // 而是记录到豁免列表，安全拦截放行时优先匹配
            state.prompt_guard.add_dynamic_rule(exemption).await;
            tracing::info!(
                pattern = %proposal.proposal,
                "ExemptAuthorize: 有界豁免已注入"
            );
        },
        CapabilityGapType::SkillEvolution => {
            // 技能进化由进化 hook（evolution_hook.rs）在工具执行后即时处理：
            // 用户同意 → 遗传算法进化 → 版本替换落库，此处不重复执行（避免重复进化）。
            tracing::info!(
                gap_id = %proposal.id,
                "SkillEvolution: 提议已由技能侧反思 hook 处理，忽略重复应用"
            );
        },
    }
    Ok(())
}

/// 用户同意等待超时（秒）。超时视为拒绝，保持原安全行为。
const CONSENT_TIMEOUT: Duration = Duration::from_secs(180);
/// 前端同意弹窗事件名（T0.13 EvolutionConsentModal 监听）。
const EVOLUTION_CONSENT_EVENT: &str = "evolution-consent-request";

/// 征求用户同意：通过事件通道下发提议，阻塞等待前端弹窗回传。
///
/// 认知编排器（`await_user_consent` 薄封装）与 `SkillEvolutionHook`（wiring 层即时技能进化）
/// 共用本公共实现，避免重复（禁区 12）。内部复用 `agent_plan_approvals` 同款挂起审批槽模式：
/// 1. 插入 `oneshot` sender 到 `evolution_consent_senders`（proposalId → sender）
/// 2. emit `evolution-consent-request` 事件（携带 camelCase 提议）
/// 3. await receiver（180s 超时，超时视为拒绝）
/// 4. 前端弹窗由 `capability_gap_consent` 命令回传结果
///
/// 返回 `true` = 用户同意；`false` = 用户拒绝 / 超时 / 前端无监听。
pub(crate) async fn await_capability_consent(
    app: &tauri::AppHandle,
    senders: &Arc<tokio::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>>,
    proposal: &CapabilityGapProposal,
) -> Result<bool, CommandError> {
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    senders.lock().await.insert(proposal.id.clone(), tx);
    // 事件下发失败不阻断：视为拒绝（保持原安全行为），并清理挂起槽
    if let Err(e) = app.emit(EVOLUTION_CONSENT_EVENT, proposal) {
        tracing::warn!(%e, "🧭 能力补齐提议事件下发失败，视为用户拒绝");
        senders.lock().await.remove(&proposal.id);
        return Ok(false);
    }
    tracing::info!(proposal_id = %proposal.id, "🧭 能力补齐提议已下发，等待用户同意/拒绝");
    let approved = match tokio::time::timeout(CONSENT_TIMEOUT, rx).await {
        Ok(Ok(approved)) => approved,
        // sender 被 drop（前端从未回传）或超时 → 视为拒绝
        Ok(Err(_)) | Err(_) => false,
    };
    // 清理残留挂起槽（前端可能从未回传）
    senders.lock().await.remove(&proposal.id);
    tracing::info!(proposal_id = %proposal.id, approved = %approved, "🧭 能力补齐提议审批结果");
    Ok(approved)
}

/// 非阻塞存储能力缺口提议（替代 await_user_consent 的即时弹窗模式）。
///
/// 将提议存入 `pending_capability_gaps`，同时 emit 事件通知前端显示徽章提示，
/// 但**不阻塞**请求主流程。用户可在能力管理面板中手动审核处理。
pub(crate) async fn store_capability_gap(
    app: &tauri::AppHandle,
    state: &AppState,
    proposal: &CapabilityGapProposal,
) -> Result<(), CommandError> {
    let mut gaps = state.pending_capability_gaps.lock().await;
    let existed = gaps.insert(proposal.id.clone(), proposal.clone()).is_some();
    drop(gaps);
    // 事件下发失败不阻断存储 — 前端可通过 list_pending_gaps 主动拉取
    if let Err(e) = app.emit(EVOLUTION_CONSENT_EVENT, proposal) {
        tracing::warn!(%e, proposal_id = %proposal.id, "🧭 能力缺口通知事件下发失败");
    }
    if existed {
        tracing::info!(proposal_id = %proposal.id, "🧭 能力缺口已更新（已存在）");
    } else {
        tracing::info!(proposal_id = %proposal.id, "🧭 能力缺口已存储（等待用户手动处理）");
    }
    Ok(())
}

/// 列出所有待处理的能力缺口提议
#[agent_command(
    domain = cognitive,
    safety = Safe,
    call_mode = StateOnly,
    description = "列出所有待处理的能力缺口提议"
)]
#[tauri::command]
pub async fn list_pending_gaps(
    state: State<'_, AppState>,
) -> Result<Vec<CapabilityGapProposal>, CommandError> {
    let gaps = state.pending_capability_gaps.lock().await;
    Ok(gaps.values().cloned().collect())
}

/// 处理能力缺口提议（用户手动审核：同意或拒绝）
#[agent_command(
    domain = cognitive,
    safety = Caution,
    call_mode = StateInput,
    description = "处理能力缺口提议（同意则执行补齐，拒绝则移除）"
)]
#[tauri::command]
pub async fn resolve_capability_gap(
    state: State<'_, AppState>,
    proposal_id: String,
    approved: bool,
) -> Result<(), CommandError> {
    let proposal = {
        let gaps = state.pending_capability_gaps.lock().await;
        gaps.get(&proposal_id).cloned()
    };
    let proposal = proposal.ok_or_else(|| {
        CommandError::new(axagent_harness::error_codes::cognitive::NO_CANDIDATE)
            .with_detail(format!("能力缺口提议不存在: {}", proposal_id))
    })?;
    if approved {
        tracing::info!(proposal_id = %proposal_id, "🧭 用户同意能力补齐，开始执行");
        apply_capability_gap_proposal(&state, &proposal, "").await?;
    } else {
        tracing::info!(proposal_id = %proposal_id, "🧭 用户拒绝能力补齐");
    }
    let mut gaps = state.pending_capability_gaps.lock().await;
    gaps.remove(&proposal_id);
    Ok(())
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

// ── P3: ApprovalGate 审批回传命令 ──────────────────────────

/// ApprovalGate 审批请求参数（前端回传）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskShapeApprovalRequest {
    /// 审批 ID（与 `task-shape-approval-request` 事件中的 `approvalId` 对应）
    pub approval_id: String,
    /// 用户是否批准执行
    pub approved: bool,
}

/// ApprovalGate 审批回传 — 前端审批弹窗同意/拒绝后调用。
///
/// 从 `task_shape_approval_senders` 取出对应 oneshot sender，
/// 唤醒 `cognitive_query` 中阻塞等待的 ApprovalGate barrier。
#[tauri::command]
pub async fn respond_task_shape_approval(
    state: State<'_, AppState>,
    request: TaskShapeApprovalRequest,
) -> Result<(), String> {
    let mut senders = state.task_shape_approval_senders.lock().await;
    if let Some(sender) = senders.remove(&request.approval_id) {
        let _ = sender.send(request.approved);
        tracing::info!(
            approval_id = %request.approval_id,
            approved = %request.approved,
            "🚨 ApprovalGate 审批回传"
        );
        Ok(())
    } else {
        tracing::warn!(
            approval_id = %request.approval_id,
            "🚨 ApprovalGate 审批回传：无挂起的审批（可能已超时或已处理）"
        );
        Ok(()) // 不报错，避免前端报异常
    }
}

// ── 遗留边界③：任务拆解 → 逐项能力发现 ────────────────────────

/// 任务拆解 + 逐项发现的请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecomposeTaskRequest {
    /// 用户原始任务
    pub input: String,
    /// 每个子目标的能力发现候选数（缺省 5）
    #[serde(default)]
    pub top_k: Option<usize>,
}

/// 单个子目标 + 其能力发现结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubGoalDiscoveryDto {
    pub sub_task_id: String,
    pub name: String,
    pub description: String,
    /// 前置子任务 ID（依赖拓扑）
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// 该子目标的能力发现结果（primary_match + alternatives + 各阶段耗时）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery: Option<axagent_harness::CapabilityDiscoveryResult>,
}

/// 任务拆解 → 逐项能力发现（遗留边界③：子目标级逐项发现）。
///
/// 用 `RuleBasedDecomposer`（纯规则关键词匹配，无 LLM 调用，不违反 orchestrator
/// 运行时边界）把大目标拆成子目标 DAG，再对每个子目标执行完整能力发现管线
/// （`CapabilityRouter::discover`），输出「子目标 → 候选能力」映射。
///
/// 设计定位：这是交互边界扩展，不侵入 `cognitive_query` 主链路；前端可将其作为
/// 复杂任务的预分析步骤（展示子目标 + 每项候选），或由编排层逐项执行。
#[agent_command(domain = cognitive, safety = Safe, call_mode = StateInput, description = "任务拆解与逐项能力发现")]
#[tauri::command]
pub async fn cognitive_decompose_task(
    state: State<'_, AppState>,
    request: DecomposeTaskRequest,
) -> Result<Vec<SubGoalDiscoveryDto>, CommandError> {
    let input = request.input.trim().to_string();
    if input.is_empty() {
        return Err(CommandError::new(axagent_harness::error_codes::cognitive::EMPTY_INPUT)
            .with_category(ErrorCategory::Validation));
    }

    // 1. 规则拆解（RuleBasedDecomposer 不返回 Err，default 分支兜底为三段式 DAG）
    let decomposer = RuleBasedDecomposer::new();
    let plan = decomposer.decompose(&input, OrchestrationStrategy::Ordered).map_err(|e| {
        CommandError::new(axagent_harness::error_codes::cognitive::ROUTE_FAILED)
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(format!("任务拆解失败: {e}"))
    })?;

    if plan.sub_tasks.is_empty() {
        return Ok(Vec::new());
    }

    // 2. 逐项发现：每个子目标跑一次完整能力发现管线
    let top_k = request.top_k.unwrap_or(5).clamp(1, 20);
    let mut results = Vec::with_capacity(plan.sub_tasks.len());
    for st in &plan.sub_tasks {
        // 子目标检索文本 = 任务名 + 描述，聚焦该子目标意图
        let goal_text = if st.description.trim().is_empty() {
            st.name.clone()
        } else {
            format!("{} — {}", st.name, st.description)
        };
        let discovery_request = axagent_harness::CapabilityDiscoveryRequest {
            user_input: goal_text.clone(),
            query: axagent_harness::CapabilityQuery {
                user_input: goal_text,
                top_k,
                ..Default::default()
            },
            enable_completion: false,
            enable_circuit_breaker: false,
            ..Default::default()
        };
        let discovery = axagent_harness::CapabilityRouter::discover(
            state.capability_router.as_ref(),
            &discovery_request,
        )
        .await
        .ok();

        tracing::debug!(
            sub_task = %st.id,
            capability_hit = discovery.as_ref().and_then(|d| d.primary_match.as_ref().map(|m| m.passport.capability_id.as_str())).unwrap_or("none"),
            "🧩 子目标能力发现完成"
        );

        results.push(SubGoalDiscoveryDto {
            sub_task_id: st.id.clone(),
            name: st.name.clone(),
            description: st.description.clone(),
            dependencies: st.dependencies.clone(),
            discovery,
        });
    }

    Ok(results)
}

// ── P2：工具链确定性执行器（固定顺序、失败短路） ────────────────

/// 工具链执行的请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolchainRequest {
    /// Toolchain 能力护照 ID（如 `toolchain:data_pipeline`）
    pub capability_id: String,
    /// 每步工具的输入（首步使用；后续步骤透传上一步输出）
    pub input: String,
}

/// 单步执行结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainStepResult {
    /// 步骤序号（0-based）
    pub step_index: usize,
    /// 该步对应的能力 ID
    pub capability_id: String,
    /// 该步对应的真实工具名
    pub tool_name: String,
    pub success: bool,
    /// 输出预览（截断前 500 字符）
    pub output_preview: String,
}

/// 工具链确定性执行器（P2：固定顺序、失败短路）。
///
/// 按护照 `steps` 顺序调用各步骤对应的真实工具（经步骤护照 `tool_ref` →
/// `UnifiedToolRegistry::execute`，集成权限/Hook/沙箱），步骤间透传输出；
/// 任一步失败立即短路返回（携带失败步骤与错误），与"Toolchain 线性串接"语义一致。
#[agent_command(domain = cognitive, safety = Caution, call_mode = StateInput, description = "执行工具链（固定顺序、失败短路）")]
#[tauri::command]
pub async fn cognitive_execute_toolchain(
    state: State<'_, AppState>,
    request: ExecuteToolchainRequest,
) -> Result<Vec<ToolchainStepResult>, CommandError> {
    let toolchain_err = |e: String| {
        CommandError::new(axagent_harness::error_codes::cognitive::TOOLCHAIN_EXEC_FAILED)
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(e)
    };

    // 1. 定位护照并校验类型
    let passport =
        state.capability_indexer.get_passport(&request.capability_id).await.ok_or_else(|| {
            CommandError::new(axagent_harness::error_codes::capability::NOT_FOUND)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(format!("capability '{}' not found", request.capability_id))
        })?;
    if passport.kind != CapabilityKind::Toolchain {
        return Err(toolchain_err(format!(
            "能力 '{}' 不是 Toolchain 类型（kind={}）",
            request.capability_id,
            passport.kind.as_str()
        )));
    }
    if passport.steps.is_empty() {
        return Ok(Vec::new());
    }

    // 2. 顺序执行，失败短路；步骤间透传输出
    let registry = state.local_tool_registry.lock().await;
    let mut results: Vec<ToolchainStepResult> = Vec::with_capacity(passport.steps.len());
    let mut current_input = request.input.clone();

    for (idx, step) in passport.steps.iter().enumerate() {
        let step_passport = state.capability_indexer.get_passport(step).await;
        let Some(sp) = step_passport else {
            let preview = format!("步骤 {idx} 能力 '{step}' 未注册");
            results.push(ToolchainStepResult {
                step_index: idx,
                capability_id: step.clone(),
                tool_name: String::new(),
                success: false,
                output_preview: preview.clone(),
            });
            return Err(toolchain_err(preview));
        };
        let Some(tool_ref) = sp.tool_ref else {
            let preview = format!("步骤 {idx} 能力 '{step}' 无工具引用（tool_ref）");
            results.push(ToolchainStepResult {
                step_index: idx,
                capability_id: step.clone(),
                tool_name: String::new(),
                success: false,
                output_preview: preview.clone(),
            });
            return Err(toolchain_err(preview));
        };

        match registry.execute(&tool_ref.tool_name, &current_input).await {
            Ok(res) => {
                let success = !res.is_error;
                let output = res.content.clone();
                results.push(ToolchainStepResult {
                    step_index: idx,
                    capability_id: step.clone(),
                    tool_name: tool_ref.tool_name.clone(),
                    success,
                    output_preview: output.chars().take(500).collect(),
                });
                if !success {
                    // 失败短路：记录后终止，返回已执行步骤 + 失败步骤
                    return Ok(results);
                }
                current_input = output;
            },
            Err(e) => {
                let msg = format!("步骤 {idx} 工具 '{}' 执行失败: {e}", tool_ref.tool_name);
                results.push(ToolchainStepResult {
                    step_index: idx,
                    capability_id: step.clone(),
                    tool_name: tool_ref.tool_name.clone(),
                    success: false,
                    output_preview: msg.clone(),
                });
                return Err(toolchain_err(msg));
            },
        }
    }

    Ok(results)
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

    /// F2-1：执行视图 → execution_mode 反推是确定性映射。
    ///
    /// 短路命中后 `execution_mode` 以实际执行视图反推（而非缓存值），
    /// 四种视图 + execution 为 None 的极端情形都必须映射到真实分派目标。
    #[test]
    fn derive_mode_from_execution_view_maps_all_views() {
        let cases: Vec<(Option<CognitiveExecutionView>, &str)> = vec![
            (
                Some(CognitiveExecutionView::Workflow {
                    workflow_id: "wf-1".into(),
                    execution_id: "ex-1".into(),
                }),
                "workflow",
            ),
            (
                Some(CognitiveExecutionView::Plan {
                    conversation_id: "c-1".into(),
                    plan_id: "p-1".into(),
                }),
                "plan",
            ),
            (Some(CognitiveExecutionView::Clarify { candidates: vec![] }), "clarify"),
            (
                Some(CognitiveExecutionView::Agent {
                    conversation_id: "c-1".into(),
                    assistant_message_id: "m-1".into(),
                    status: None,
                }),
                "delegate",
            ),
            // 极端情形：execution 为 None（响应构造异常）也应落到 agent 执行路径
            (None, "delegate"),
        ];
        for (view, expected) in cases {
            assert_eq!(
                derive_mode_from_execution_view(&view),
                expected,
                "视图 {:?} 反推应得到 {}",
                view,
                expected
            );
        }
    }

    /// F2-2：短路缓存口径统一 — 缓存 mode=plan 而能力是 Workflow kind 时，
    /// 覆盖后响应必须报 workflow（与 execution 视图一致），不得沿袭缓存撒谎值。
    ///
    /// 复刻方案文档第七节场景：`shortcut_override { execution_mode: "plan" }`
    /// + forced 路径实际产出 Workflow 视图 → 断言最终 `execution_mode == "workflow"`。
    #[test]
    fn apply_shortcut_override_overwrites_stale_cached_mode() {
        // forced 路径（Workflow kind 能力）实际产出的响应：execution 已是 Workflow 视图，
        // 但 execution_mode 还是占位值（此处故意填 plan，模拟上一轮 confidence 分档的缓存值）
        let mut response = CognitiveQueryResponse {
            route_path: "/__forced__".into(),
            domain: "__forced__".into(),
            cluster: "__forced__".into(),
            capability_id: "wf-refund-auto".into(),
            confidence: 1.0,
            is_llm_fallback: false,
            circuit_broken: false,
            circuit_break_reason: None,
            fallback_path: None,
            candidates: vec![],
            candidate_details: vec![],
            filtered_count: 0,
            execution_mode: ExecutionMode::Plan.as_str().to_string(),
            selected_workflow_name: Some("refund-auto".into()),
            selected_agent_profile: None,
            stage_records: vec![],
            total_elapsed_ms: 0,
            execution: Some(CognitiveExecutionView::Workflow {
                workflow_id: "wf-refund-auto".into(),
                execution_id: "ex-1".into(),
            }),
            task_shape: None,
        };
        // 上一轮缓存：mode=plan（按 confidence 分档的结果，与本次 forced 分派口径不同）
        let ovr = LastRouteDecision {
            capability_id: "wf-refund-auto".into(),
            execution_mode: ExecutionMode::Plan.as_str().to_string(),
            route_path: "/trade/refund/auto".into(),
            domain: "trade".into(),
            cluster: "refund".into(),
            msg_count: 3,
            timestamp: Instant::now(),
        };

        apply_shortcut_override(&mut response, &ovr);

        // 真实路由字段来自缓存
        assert_eq!(response.route_path, "/trade/refund/auto");
        assert_eq!(response.domain, "trade");
        assert_eq!(response.cluster, "refund");
        // execution_mode 反推自实际执行视图，不沿袭缓存的 plan 撒谎值
        assert_eq!(response.execution_mode, "workflow");
        // confidence 保持 forced 路径的 1.0（不覆盖）
        assert_eq!(response.confidence, 1.0);
    }
}
