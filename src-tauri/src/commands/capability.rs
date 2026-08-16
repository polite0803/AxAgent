// SPDX-License-Identifier: AGPL-3.0-only
//! 能力发现 Tauri 命令集
//!
//! 暴露能力注册、发现、索引管理等命令给前端调用。
//! 所有命令从 AppState 获取已注入的路由器和索引器实例。

use crate::AppState;
use crate::commands::error::{CommandError, ErrorCategory, ErrorResponse};
use agent_macro::agent_command;
use axagent_dao::repo::agent_profile as agent_profile_repo;
use axagent_dao::repo::agent_role as agent_role_repo;
use axagent_entities::{agency_experts, agent_roles};
use axagent_harness::trajectory_types::TrajectoryOutcome;
use axagent_harness::{
    CapabilityDiscoveryRequest, CapabilityDiscoveryResult, CapabilityDomain, CapabilityIndexer,
    CapabilityKind, CapabilityLevel, CapabilityPassportDto, CapabilityQuery, DiscoveryWeights,
    FilterContext, Reflection, SessionBudget, Visibility,
};
use axagent_trajectory::{
    ComputationGraph, ComputationNode, NodeType, TextGradConfig, TextGradEngine,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 运行时能力注册表检视 DTO（P3：外部插件注册的可查询闭环）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRegistrationDetailDto {
    pub id: String,
    pub version: String,
    pub contract: String,
    pub description: String,
    pub origin: String,
    pub plugin_id: Option<String>,
}

// ── DTO 类型 ──────────────────────────────────────

/// 注册能力护照的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPassportRequest {
    pub passport: CapabilityPassportDto,
}

/// 能力发现的请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverRequest {
    pub user_input: String,
    #[serde(default)]
    pub filter_context: Option<FilterContext>,
    #[serde(default)]
    pub query: Option<CapabilityQuery>,
    #[serde(default)]
    pub weights: Option<DiscoveryWeights>,
    #[serde(default)]
    pub budget: Option<SessionBudget>,
    #[serde(default = "default_true")]
    pub enable_completion: bool,
    #[serde(default = "default_false")]
    pub enable_circuit_breaker: bool,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

// ── Tauri 命令 ────────────────────────────────────

/// 注册一个能力护照到索引
#[agent_command(domain = capability, safety = Caution, call_mode = StateInput, description = "注册能力护照")]
#[tauri::command]
pub async fn capability_register_passport(
    state: State<'_, AppState>,
    request: RegisterPassportRequest,
) -> Result<axagent_harness::IndexResult, CommandError> {
    state.capability_indexer.index_passport(&request.passport).await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::REGISTER_FAILED,
            e,
            ErrorCategory::Unrecoverable,
        )
    })
}

/// 批量注册能力护照
#[agent_command(domain = capability, safety = Caution, call_mode = StateInput, description = "批量注册能力护照")]
#[tauri::command]
pub async fn capability_register_batch(
    state: State<'_, AppState>,
    passports: Vec<CapabilityPassportDto>,
) -> Result<Vec<axagent_harness::IndexResult>, CommandError> {
    Ok(state.capability_indexer.index_batch(&passports).await)
}

/// 执行能力发现管线
#[agent_command(domain = capability, safety = Safe, call_mode = StateInput, description = "执行能力发现管线")]
#[tauri::command]
pub async fn capability_discover(
    state: State<'_, AppState>,
    request: DiscoverRequest,
) -> Result<CapabilityDiscoveryResult, CommandError> {
    let mut query = request.query.unwrap_or_default();
    // 确保 query.user_input 使用用户实际输入
    if query.user_input.is_empty() {
        query.user_input = request.user_input.clone();
    }
    let filter_context = request.filter_context.unwrap_or_default();
    let weights = request.weights.unwrap_or_default();
    let budget = request.budget.unwrap_or_default();

    let discovery_request = CapabilityDiscoveryRequest {
        user_input: request.user_input,
        filter_context,
        query,
        weights,
        budget,
        enable_completion: request.enable_completion,
        enable_circuit_breaker: request.enable_circuit_breaker,
        enable_rar: false,
        rar_top_k: 5,
    };

    axagent_harness::CapabilityRouter::discover(
        state.capability_router.as_ref(),
        &discovery_request,
    )
    .await
    .map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::DISCOVER_FAILED,
            e,
            ErrorCategory::Retryable,
        )
    })
}

/// 列出已注册的能力
#[agent_command(domain = capability, safety = Safe, call_mode = StateOnly, description = "列出已注册的能力")]
#[tauri::command]
pub async fn capability_list_passports(
    state: State<'_, AppState>,
) -> Result<Vec<CapabilityPassportDto>, CommandError> {
    let ids = state.capability_indexer.list_capability_ids().await;
    let mut passports = Vec::new();
    for id in ids {
        if let Some(passport) = state.capability_indexer.get_passport(&id).await {
            passports.push(passport);
        }
    }
    Ok(passports)
}

/// 删除一个能力
#[agent_command(domain = capability, safety = Dangerous, call_mode = StateInput, description = "删除能力护照")]
#[tauri::command]
pub async fn capability_remove_passport(
    state: State<'_, AppState>,
    capability_id: String,
) -> Result<(), CommandError> {
    state.capability_indexer.remove_index(&capability_id).await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::NOT_FOUND,
            e,
            ErrorCategory::Unrecoverable,
        )
    })
}

/// 获取索引统计信息
#[agent_command(domain = capability, safety = Safe, call_mode = StateOnly, description = "获取能力索引统计信息")]
#[tauri::command]
pub async fn capability_get_stats(
    state: State<'_, AppState>,
) -> Result<axagent_harness::CapabilityIndexStats, CommandError> {
    state.capability_indexer.get_stats().await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::STATS_FAILED,
            e,
            ErrorCategory::Retryable,
        )
    })
}

/// 列出运行时能力注册表（P3：内置与外部插件平权的可查询检视闭环）。
///
/// 返回全部已注册能力及其来源；外部插件注册的能力额外标注来源插件 ID。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateOnly, description = "列出运行时能力注册表")]
#[tauri::command]
pub async fn capability_registry_dump()
-> Result<Vec<CapabilityRegistrationDetailDto>, ErrorResponse> {
    Ok(axagent_harness::get_capability_registry()
        .list_with_details()
        .into_iter()
        .map(|d| CapabilityRegistrationDetailDto {
            id: d.definition.id,
            version: d.definition.version,
            contract: d.definition.contract,
            description: d.definition.description,
            origin: d.origin.as_str().to_string(),
            plugin_id: d.plugin_id,
        })
        .collect())
}

// ── 能力进化：按 kind 分发到 skill / workflow 进化引擎 ─────────────

/// 能力进化请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolveCapabilityRequest {
    /// 能力护照 ID（如 `workflow:{template_id}` / `skill:{name}`）
    pub capability_id: String,
    /// 工作流进化反思上下文（可选；缺省走启发式变异）
    #[serde(default)]
    pub reflections: Vec<Reflection>,
}

/// 能力进化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolveCapabilityResult {
    pub capability_id: String,
    /// 进化是否产生有效改进
    pub improved: bool,
    /// 进化前等级
    pub old_level: CapabilityLevel,
    /// 进化后等级
    pub new_level: CapabilityLevel,
    /// 进化引擎返回的原始结果摘要（技能改进 / 工作流变异详情）
    pub detail: serde_json::Value,
}

/// 一键进化能力以提升等级。
///
/// 按能力类型分发到对应进化引擎（技能 → 技能进化引擎，工作流 → 工作流进化器），
/// 进化成功后把护照等级提升一级（L5 封顶）。适用于能力发现面板中低等级（L1/L2）
/// 能力的「进化提升」入口。
#[agent_command(domain = capability, safety = Caution, call_mode = StateInput, description = "进化能力以提升等级")]
#[tauri::command]
pub async fn capability_evolve(
    state: State<'_, AppState>,
    request: EvolveCapabilityRequest,
) -> Result<EvolveCapabilityResult, CommandError> {
    let evolve_err = |e: String| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::EVOLVE_FAILED,
            e,
            ErrorCategory::Unrecoverable,
        )
    };

    // 1. 定位护照并记录进化前等级
    let passport =
        state.capability_indexer.get_passport(&request.capability_id).await.ok_or_else(|| {
            CommandError::new(crate::commands::error_code::capability::NOT_FOUND)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(format!("capability '{}' not found", request.capability_id))
        })?;
    let old_level = passport.level;

    // 2. 按能力类型分发进化
    let (improved, detail) = match passport.kind {
        CapabilityKind::Skill => {
            let skill_id =
                request.capability_id.strip_prefix("skill:").unwrap_or(&request.capability_id);
            evolve_skill(&state, skill_id).await.map_err(evolve_err)?
        },
        CapabilityKind::Workflow => {
            let template_id =
                request.capability_id.strip_prefix("workflow:").unwrap_or(&request.capability_id);
            let modification = state
                .workflow_evolver
                .run(template_id, &request.reflections)
                .await
                .map_err(evolve_err)?;
            let improved = !modification.changes.is_empty() && modification.fitness_delta > 0.0;
            let detail = serde_json::json!({
                "generation": modification.generation,
                "fitness_delta": modification.fitness_delta,
                "changes": modification.changes.len(),
                "reasoning": modification.reasoning,
            });
            (improved, detail)
        },
        CapabilityKind::Agent => {
            // 角色 / 专家：用 TextGrad 优化系统提示词并写回载体
            evolve_agent(&state, &request.capability_id).await.map_err(evolve_err)?
        },
        other => {
            return Err(CommandError::new(crate::commands::error_code::capability::EVOLVE_FAILED)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(format!("capability kind '{}' 暂不支持一键进化", other.as_str())));
        },
    };

    // 3. 进化成功后提升一级（L5 封顶）
    let new_level = old_level.promote();
    state
        .capability_indexer
        .update_level(&request.capability_id, new_level)
        .await
        .map_err(evolve_err)?;

    tracing::info!(
        capability_id = %request.capability_id,
        ?old_level,
        ?new_level,
        improved,
        "🧬 能力进化完成：等级已提升"
    );

    Ok(EvolveCapabilityResult {
        capability_id: request.capability_id,
        improved,
        old_level,
        new_level,
        detail,
    })
}

/// 执行技能进化（复用 trajectory 技能进化引擎），成功时把改进内容写回技能库。
async fn evolve_skill(
    state: &AppState,
    skill_id: &str,
) -> Result<(bool, serde_json::Value), String> {
    let skill = state
        .trajectory_storage
        .get_skill(skill_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Skill '{skill_id}' not found"))?;

    let trajectories =
        state.trajectory_storage.get_trajectories(Some(30)).await.map_err(|e| e.to_string())?;
    let test_refs: Vec<_> = trajectories.iter().collect();

    let mut engine = state.skill_evolution_engine.lock().await;
    let result = engine.run(&skill, &test_refs).await;
    match result {
        Some(modification) => {
            let improved = modification.validation_result.as_ref().is_some_and(|v| v.success);
            if improved {
                let mut updated = skill.clone();
                updated.content = modification.new_content.clone();
                updated.quality_score = modification.confidence;
                if let Err(e) = state.trajectory_storage.save_skill(&updated).await {
                    tracing::warn!("[capability_evolve] 保存进化技能失败: {}", e);
                }
            }
            Ok((
                improved,
                serde_json::json!({
                    "reason": modification.reason,
                    "confidence": modification.confidence,
                    "quality_delta": modification.validation_result.as_ref().map(|v| v.quality_delta),
                }),
            ))
        },
        None => Ok((
            false,
            serde_json::json!({
                "reason": "Evolution did not produce a result",
                "confidence": 0.0,
            }),
        )),
    }
}

/// 执行 Agent 能力（角色/专家）进化。
///
/// Agent 能力的可进化载体是系统提示词：
/// - 角色护照（`agent_role:{id}`）→ 直接优化 `AgentRole.system_prompt` 并写回；
/// - 专家护照（`agent:{id}`）→ 解析执行载体（优先 `expert_id` 关联的机构专家，
///   其次 `agent_role` 关联的角色），优化其系统提示词并写回。
async fn evolve_agent(
    state: &AppState,
    capability_id: &str,
) -> Result<(bool, serde_json::Value), String> {
    if let Some(role_id) = capability_id.strip_prefix("agent_role:") {
        return evolve_agent_role(state, role_id).await;
    }
    if let Some(profile_id) = capability_id.strip_prefix("agent:") {
        return evolve_agent_profile(state, profile_id).await;
    }
    Err(format!("无法识别 Agent 能力载体: {capability_id}"))
}

/// 角色进化：用 TextGrad 优化角色系统提示词，写回 `agent_roles` 表。
async fn evolve_agent_role(
    state: &AppState,
    role_id: &str,
) -> Result<(bool, serde_json::Value), String> {
    let db = state.harness.db();
    let role = agent_role_repo::get_agent_role(db, role_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("AgentRole '{role_id}' not found"))?;

    let feedback = build_agent_feedback(state, &role.name).await;
    let llm_provider = build_agent_llm_bridge(state).await;
    let (new_prompt, improved) =
        optimize_prompt_with_text_grad(&role.system_prompt, &feedback, llm_provider).await?;
    if !improved {
        return Ok((
            false,
            serde_json::json!({
                "kind": "agent_role",
                "roleId": role_id,
                "reason": "TextGrad 未产生有效改进",
            }),
        ));
    }

    // 局部写回 system_prompt，保留其余字段（避免重置 sort_order / created_at）
    let row = agent_roles::Entity::find_by_id(role_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("AgentRole '{role_id}' not found"))?;
    let mut am: agent_roles::ActiveModel = row.into();
    am.system_prompt = Set(new_prompt.clone());
    am.updated_at = Set(axagent_harness::util_fns::now_ts());
    am.update(db).await.map_err(|e| e.to_string())?;

    Ok((
        true,
        serde_json::json!({
            "kind": "agent_role",
            "roleId": role_id,
            "promptDelta": prompt_delta(&role.system_prompt, &new_prompt),
        }),
    ))
}

/// 专家进化：定位专家执行载体（机构专家 / 角色）的系统提示词，优化后写回对应表。
async fn evolve_agent_profile(
    state: &AppState,
    profile_id: &str,
) -> Result<(bool, serde_json::Value), String> {
    let db = state.harness.db();
    let profile =
        agent_profile_repo::get_agent_profile(db, profile_id).await.map_err(|e| e.to_string())?;

    // 载体一：关联的机构专家（expert_id → agency_experts.system_prompt）
    if let Some(expert_id) = profile.expert_id.as_deref() {
        let expert = agency_experts::Entity::find_by_id(expert_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("AgencyExpert '{expert_id}' not found"))?;

        let feedback = build_agent_feedback(state, &profile.name).await;
        let old_prompt = expert.system_prompt.clone();
        let llm_provider = build_agent_llm_bridge(state).await;
        let (new_prompt, improved) =
            optimize_prompt_with_text_grad(&old_prompt, &feedback, llm_provider).await?;
        if !improved {
            return Ok((
                false,
                serde_json::json!({
                    "kind": "agent_profile",
                    "profileId": profile_id,
                    "carrier": "agency_expert",
                    "expertId": expert_id,
                    "reason": "TextGrad 未产生有效改进",
                }),
            ));
        }

        let mut am: agency_experts::ActiveModel = expert.into();
        am.system_prompt = Set(new_prompt.clone());
        am.update(db).await.map_err(|e| e.to_string())?;

        return Ok((
            true,
            serde_json::json!({
                "kind": "agent_profile",
                "profileId": profile_id,
                "carrier": "agency_expert",
                "expertId": expert_id,
                "promptDelta": prompt_delta(&old_prompt, &new_prompt),
            }),
        ));
    }

    // 载体二：关联的角色（agent_role → agent_roles.system_prompt）
    if let Some(role_id) = profile.agent_role.as_deref() {
        let (improved, mut detail) = evolve_agent_role(state, role_id).await?;
        detail["profileId"] = serde_json::Value::String(profile_id.to_string());
        detail["carrier"] = serde_json::Value::String("agent_role".to_string());
        return Ok((improved, detail));
    }

    Ok((
        false,
        serde_json::json!({
            "kind": "agent_profile",
            "profileId": profile_id,
            "reason": "该专家未绑定可进化的系统提示词载体（expert_id / agent_role）",
        }),
    ))
}

/// 尝试从 DB 构建 LLM bridge，对齐技能/工作流进化策略：
/// 存在启用的 provider → 注入 LLM 语义梯度；否则返回 None，回退本地启发式。
async fn build_agent_llm_bridge(state: &AppState) -> Option<axagent_agent::ProviderLlmBridge> {
    axagent_runtime::llm_bridge::build_llm_bridge_from_db_with(
        state.harness.master_key(),
        state.harness.provider_registry(),
        None,
        None,
    )
    .await
}

/// 用独立的 TextGrad 引擎对系统提示词做单轮梯度优化。
///
/// 使用独立实例，避免污染 AppState 中共享的全局 `text_grad_engine`
/// （其计算图节点会随 `run_text_grad_optimize` 调用累积）。
/// 传入可用的 LLM bridge 时注入语义梯度；否则用内置启发式 provider（纯本地）。
/// 返回 (新提示词, 是否产生有效修改)。
async fn optimize_prompt_with_text_grad(
    current: &str,
    feedback: &str,
    llm_provider: Option<axagent_agent::ProviderLlmBridge>,
) -> Result<(String, bool), String> {
    let mut graph = ComputationGraph::new();
    graph.add_node(
        ComputationNode::new(NodeType::Prompt, current.to_string()).with_id("system_prompt"),
    );
    let mut engine = TextGradEngine::new(graph, TextGradConfig::default());
    if let Some(bridge) = llm_provider {
        engine.set_provider(bridge);
    }

    engine.backward_text_grad(feedback).await.map_err(|e| format!("提示词梯度计算失败: {e}"))?;

    let modifications = engine.apply_gradients();

    let new_prompt = engine
        .graph()
        .get_node("system_prompt")
        .map(|n| n.content.clone())
        .unwrap_or_else(|| current.to_string());

    let improved = !modifications.is_empty() && new_prompt != current;
    Ok((new_prompt, improved))
}

/// 从轨迹库构建 Agent 能力进化的反馈文本。
///
/// 先按 Agent 名称过滤轨迹（topic / summary 文本匹配，与技能进化同款策略），
/// 确保反馈只反映该 Agent 的执行情况；若无匹配则回退全局最近轨迹，避免
/// 证据不足导致进化空转。统计执行结果（成功/失败/部分完成）并提取失败案例，
/// 作为 TextGrad 的反向传播信号（feedback）。
async fn build_agent_feedback(state: &AppState, agent_name: &str) -> String {
    let trajectories =
        state.trajectory_storage.get_trajectories(Some(200)).await.unwrap_or_default();

    // 按 Agent 名称过滤：优先精确匹配结构化 `agent_name` 字段（记录时即带标识），
    // 无结构化的旧轨迹再回退 topic / summary 文本匹配，双通道聚合同一 Agent 证据。
    let name = agent_name.to_lowercase();
    let filtered: Vec<&_> = trajectories
        .iter()
        .filter(|t| {
            t.agent_name.as_deref().map(|n| n.to_lowercase().contains(&name)).unwrap_or(false)
                || t.topic.to_lowercase().contains(&name)
                || t.summary.to_lowercase().contains(&name)
        })
        .collect();
    let (evidence, fallback) = if filtered.is_empty() {
        (trajectories.iter().collect::<Vec<_>>(), true)
    } else {
        (filtered, false)
    };

    let total = evidence.len();

    let success = evidence.iter().filter(|t| t.outcome == TrajectoryOutcome::Success).count();
    let failure = evidence.iter().filter(|t| t.outcome == TrajectoryOutcome::Failure).count();
    let partial = evidence.iter().filter(|t| t.outcome == TrajectoryOutcome::Partial).count();

    let mut feedback = String::new();
    feedback.push_str(&format!("Execution feedback for agent capability '{}'.\n", agent_name));
    if fallback {
        feedback.push_str(
            "No direct traces matched this agent name; using recent global trajectories as fallback.\n",
        );
    }
    feedback.push_str(&format!(
        "Tracked executions: total {}, success {}, failure {}, partial {}.\n",
        total, success, failure, partial
    ));
    if total > 0 {
        let rate = success as f64 / total as f64;
        feedback.push_str(&format!("Overall success rate: {:.2}.\n", rate));
    }

    for t in evidence.iter().filter(|t| t.outcome == TrajectoryOutcome::Failure).take(5) {
        feedback.push_str(&format!("Failed task: {} | summary: {}\n", t.topic, t.summary));
    }

    if failure == 0 && partial == 0 {
        feedback.push_str(
            "No significant errors observed. Focus on improving efficiency, clarity, and robustness.\n",
        );
    }

    feedback
}

/// 生成提示词变更摘要（新旧各前 120 字符），用于进化结果详情展示。
fn prompt_delta(old: &str, new: &str) -> String {
    let old_preview: String = old.chars().take(120).collect();
    let new_preview: String = new.chars().take(120).collect();
    format!("{old_preview} → {new_preview}")
}

// ── 自我进化：产物护照/图谱注册（T0.11）──────────────────────────

/// 注册进化/补齐产物为能力护照并同步进工作流图谱（T0.11）。
///
/// 通道一（能力补齐）与通道二（能力偏弱进化改进）的产物统一在此注册：
/// 1. 生成 `evolution:workflow:{product_id}` 能力护照（`auto_evolved` 标签
///    + `/evolution/` 前缀），注册进能力索引（L2 混合检索可见）；
/// 2. 同步进工作流图谱（L3 `system_workflow_graph_router` 可见），
///    使下一轮用户输入的路由决策可命中该产物。
///
/// 调用方负责先征求用户显式同意（铁律），再调用本函数完成注册。
/// 失败时返回带 `GAP_PROPOSAL_PENDING` 错误码的 `CommandError`（前端按码翻译）。
pub(crate) async fn register_evolution_product(
    state: &AppState,
    product_id: &str,
    display_name: &str,
    description: &str,
) -> Result<(), CommandError> {
    // 1. 生成能力护照（L2 混合检索可见）
    let capability_id = format!("evolution:workflow:{product_id}");
    let route_tag = format!("route:/evolution/auto_generated/workflow/{product_id}");
    let passport = CapabilityPassportDto {
        capability_id: capability_id.clone(),
        name: display_name.to_string(),
        description: description.to_string(),
        kind: CapabilityKind::Workflow,
        domain: CapabilityDomain::General,
        sub_category: "auto_generated".to_string(),
        visibility: Visibility::Public,
        tags: vec!["auto_evolved".to_string(), route_tag],
        ..Default::default()
    };
    state.capability_indexer.index_passport(&passport).await.map_err(|e| {
        CommandError::new(axagent_harness::error_codes::cognitive::GAP_PROPOSAL_PENDING)
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(format!("能力护照注册失败: {e}"))
    })?;
    // 2. 同步进工作流图谱（L3 图谱路由可见）
    state
        .cognitive_router
        .sync_evolved_workflow("general", "auto_generated", product_id, display_name)
        .await;
    tracing::info!(
        capability_id = %capability_id,
        "🗺️ 进化产物已注册护照并同步进工作流图谱"
    );
    Ok(())
}
