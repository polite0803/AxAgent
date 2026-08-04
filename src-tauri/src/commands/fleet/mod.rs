// SPDX-License-Identifier: AGPL-3.0-only

//! Fleet（多办公室 AI 团队）命令模块。
//!
//! ## 命令清单
//!
//! ### 舰队 CRUD
//! - `fleet_list` — 列出所有舰队（可选状态过滤）
//! - `fleet_get` — 获取舰队详情
//! - `fleet_create` — 创建舰队
//! - `fleet_update_status` — 更新舰队状态（active/paused/stopped）
//! - `fleet_delete` — 删除舰队（级联删除成员）
//!
//! ### 成员管理
//! - `fleet_list_members` — 列出舰队所有成员
//! - `fleet_add_member` — 添加成员
//! - `fleet_get_member` — 获取单个成员
//! - `fleet_update_member_status` — 更新成员状态
//! - `fleet_remove_member` — 移除成员
//! - `fleet_reset_daily_tokens` — 重置舰队所有成员今日 token
//!
//! ### Dispatcher 智能路由（真实执行）
//! - `fleet_dispatch` — 群聊智能路由：真实 LLM 意图分类 → 路由到成员 → 真实 Agent 回合执行
//!   （通过 `Channel<DispatchEvent>` 流式回传事件）
//! - `fleet_direct_message` — 直接 DM 指定 agent（绕过 LLM 路由，仍真实执行）
//!
//! ## 错误处理
//!
//! 所有命令返回 `Result<T, ErrorResponse>`，错误码见 `error_code::fleet`，
//! 前端按 `error.${code}` 走 i18n 翻译（`@/lib/errorI18n.ts`）。

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::fleet as fleet_err;
use agent_macro::agent_command;
use axagent_harness::fleet::{
    DispatchChatMessage, DispatchEvent, Fleet, FleetMember, FleetMemberStatus, FleetMetadata,
    FleetStatus,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::warn;

pub mod executor;
use executor::execute_fleet_turn;

// ── 舰队 CRUD ────────────────────────────────────────────────────────

/// 列出所有舰队（可选状态过滤）
#[agent_command(domain = fleet, safety = Safe, call_mode = StateInput, description = "列出所有舰队")]
#[tauri::command]
pub async fn fleet_list(
    app_state: State<'_, AppState>,
    status_filter: Option<FleetStatus>,
) -> Result<Vec<Fleet>, ErrorResponse> {
    app_state
        .fleet_repository
        .list_fleets(status_filter)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 获取舰队详情
#[agent_command(domain = fleet, safety = Safe, call_mode = StateInput, description = "获取舰队详情")]
#[tauri::command]
pub async fn fleet_get(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<Option<Fleet>, ErrorResponse> {
    app_state
        .fleet_repository
        .get_fleet(&fleet_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 创建舰队的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFleetInput {
    /// 显示名称
    pub name: String,
    /// 场景模板 slug（可选）
    pub scene_template_slug: Option<String>,
    /// 业务元数据
    #[serde(default)]
    pub metadata: FleetMetadata,
}

/// 创建舰队
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "创建舰队")]
#[tauri::command]
pub async fn fleet_create(
    app_state: State<'_, AppState>,
    input: CreateFleetInput,
) -> Result<Fleet, ErrorResponse> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(
            ErrorResponse::new(fleet_err::NAME_REQUIRED).with_category(ErrorCategory::Validation)
        );
    }
    let now = chrono::Utc::now().timestamp_millis();
    let fleet = Fleet {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        scene_template_slug: input.scene_template_slug,
        status: FleetStatus::Active,
        created_at: now,
        updated_at: now,
        metadata: input.metadata,
    };
    app_state
        .fleet_repository
        .create_fleet(fleet)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 更新舰队状态
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "更新舰队状态")]
#[tauri::command]
pub async fn fleet_update_status(
    app_state: State<'_, AppState>,
    fleet_id: String,
    status: FleetStatus,
) -> Result<(), ErrorResponse> {
    app_state
        .fleet_repository
        .update_fleet_status(&fleet_id, status)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 删除舰队（级联删除成员）
#[agent_command(domain = fleet, safety = Dangerous, call_mode = StateInput, description = "删除舰队")]
#[tauri::command]
pub async fn fleet_delete(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<(), ErrorResponse> {
    app_state
        .fleet_repository
        .delete_fleet(&fleet_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

// ── 成员管理 ─────────────────────────────────────────────────────────

/// 列出舰队所有成员
#[agent_command(domain = fleet, safety = Safe, call_mode = StateInput, description = "列出舰队所有成员")]
#[tauri::command]
pub async fn fleet_list_members(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<Vec<FleetMember>, ErrorResponse> {
    app_state
        .fleet_repository
        .list_members(&fleet_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 添加成员的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberInput {
    /// 所属舰队 ID
    pub fleet_id: String,
    /// 关联的 AgentSession ID（会话键 = conversation_id，由 get_or_create_session 懒创建）
    pub agent_id: String,
    /// agent slug（业务标识，用于 Dispatcher 路由）
    pub agent_slug: String,
    /// 显示名称
    pub display_name: String,
    /// 角色描述（注入到 Dispatcher prompt；与 agent_profile_id 二选一，均可）
    #[serde(default)]
    pub role: String,
    /// 关联的 AgentProfile ID（AgentProfile = 角色 + 专家组合，定义成员智能体身份）
    #[serde(default)]
    pub agent_profile_id: Option<String>,
    /// 房间 ID（前端 Phaser 渲染位置，如 "manager" / "meeting"）
    #[serde(default = "default_room_id")]
    pub room_id: String,
}

fn default_room_id() -> String {
    "workspace".to_string()
}

/// 添加成员到舰队
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "添加成员到舰队")]
#[tauri::command]
pub async fn fleet_add_member(
    app_state: State<'_, AppState>,
    input: AddMemberInput,
) -> Result<FleetMember, ErrorResponse> {
    if input.agent_slug.trim().is_empty() {
        return Err(ErrorResponse::new(fleet_err::NAME_REQUIRED)
            .with_category(ErrorCategory::Validation)
            .with_detail("agent_slug 不能为空".to_string()));
    }
    // 同舰队内 slug 唯一性校验：slug 是 Dispatcher 路由与前端事件回写的键，
    // 重复会导致 DM 错配 / 事件状态回写到错误成员（精灵动画失真）。
    let slug = input.agent_slug.trim().to_string();
    let existing = app_state
        .fleet_repository
        .list_members(&input.fleet_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))?;
    if existing.iter().any(|m| m.agent_slug == slug) {
        return Err(ErrorResponse::new(fleet_err::SLUG_EXISTS)
            .with_category(ErrorCategory::Validation)
            .with_param("slug", slug.clone()));
    }
    let member = FleetMember {
        id: uuid::Uuid::new_v4().to_string(),
        fleet_id: input.fleet_id,
        agent_id: input.agent_id,
        agent_slug: slug,
        display_name: input.display_name,
        role: input.role,
        agent_profile_id: input.agent_profile_id,
        room_id: input.room_id,
        status: FleetMemberStatus::Idle,
        joined_at: chrono::Utc::now().timestamp_millis(),
        today_tokens: 0,
        total_tokens: 0,
    };
    app_state
        .fleet_repository
        .add_member(member)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 获取单个成员
#[agent_command(domain = fleet, safety = Safe, call_mode = StateInput, description = "获取单个成员详情")]
#[tauri::command]
pub async fn fleet_get_member(
    app_state: State<'_, AppState>,
    member_id: String,
) -> Result<Option<FleetMember>, ErrorResponse> {
    app_state
        .fleet_repository
        .get_member(&member_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 更新成员状态
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "更新成员状态")]
#[tauri::command]
pub async fn fleet_update_member_status(
    app_state: State<'_, AppState>,
    member_id: String,
    status: FleetMemberStatus,
) -> Result<(), ErrorResponse> {
    app_state
        .fleet_repository
        .update_member_status(&member_id, status)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 移除成员
#[agent_command(domain = fleet, safety = Dangerous, call_mode = StateInput, description = "移除成员")]
#[tauri::command]
pub async fn fleet_remove_member(
    app_state: State<'_, AppState>,
    member_id: String,
) -> Result<(), ErrorResponse> {
    app_state
        .fleet_repository
        .remove_member(&member_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 重置舰队所有成员今日 token（每日定时任务调用）
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "重置舰队所有成员今日token")]
#[tauri::command]
pub async fn fleet_reset_daily_tokens(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<(), ErrorResponse> {
    app_state
        .fleet_repository
        .reset_daily_tokens(&fleet_id)
        .await
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

// ── Dispatcher 智能路由（真实执行）───────────────────────────────────

/// 群聊智能路由的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchInput {
    /// 舰队 ID
    pub fleet_id: String,
    /// 用户消息
    pub user_message: String,
    /// 历史消息（可选，最早的在前面）
    #[serde(default)]
    pub history: Vec<DispatchChatMessage>,
}

/// 群聊智能路由 — 真实 LLM 意图分类 → 路由到成员 → 真实 Agent 回合执行。
///
/// 事件通过 `Channel<DispatchEvent>` 流式回传：
/// `Routing → AgentStatus(busy) → [Process/AgentMessage/TokenUsage]* → AgentStatus(idle) → Complete`。
#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "群聊智能路由分派执行")]
#[tauri::command]
pub async fn fleet_dispatch(
    app_state: State<'_, AppState>,
    input: DispatchInput,
    on_event: tauri::ipc::Channel<DispatchEvent>,
) -> Result<(), ErrorResponse> {
    // 1. 加载成员并过滤可路由成员（Idle / Busy）
    let members = app_state.fleet_repository.list_members(&input.fleet_id).await.map_err(|e| {
        ErrorResponse::from_error_with_code(fleet_err::NOT_FOUND, e, ErrorCategory::General)
    })?;
    if members.is_empty() {
        return Err(
            ErrorResponse::new(fleet_err::NO_MEMBERS).with_category(ErrorCategory::Validation)
        );
    }
    let routable: Vec<FleetMember> = members
        .into_iter()
        .filter(|m| matches!(m.status, FleetMemberStatus::Idle | FleetMemberStatus::Busy))
        .collect();
    if routable.is_empty() {
        return Err(ErrorResponse::new(fleet_err::ALL_MEMBERS_UNAVAILABLE));
    }

    // 2. 真实 LLM 意图路由（wiring 层注入的 FleetIntentLlm）；失败/未命中时兜底到第一个可路由成员
    let system_prompt = build_fleet_system_prompt(&routable);
    let user_prompt = build_fleet_user_prompt(&input.user_message, &input.history);
    let mut fell_back = false;
    let target_slug = match app_state.fleet_intent_llm.route(&system_prompt, &user_prompt).await {
        Ok(resp) => {
            match parse_route_response(&resp).and_then(|slug| resolve_target_slug(&routable, &slug))
            {
                Some(slug) => slug,
                None => {
                    fell_back = true;
                    warn!("[fleet] LLM 路由未命中任何成员，兜底到首个成员");
                    routable[0].agent_slug.clone()
                },
            }
        },
        Err(e) => {
            fell_back = true;
            warn!("[fleet] LLM 路由失败，兜底到首个成员: {e}");
            routable[0].agent_slug.clone()
        },
    };

    let target = routable
        .into_iter()
        .find(|m| m.agent_slug == target_slug)
        .expect("target_slug 来自 routable，必然存在");

    // 兜底时明确告知实际路由到的成员（避免静默错配，前端据此提示用户）
    if fell_back {
        let notice = DispatchEvent::Process {
            agent_slug: target.agent_slug.clone(),
            agent_id: target.agent_id.clone(),
            status: format!("意图路由未命中，本次任务转派给成员「{}」", target.display_name),
        };
        send_event(&on_event, notice);
    }

    // 3. 路由决策事件
    send_event(
        &on_event,
        DispatchEvent::Routing {
            agent_slug: target.agent_slug.clone(),
            agent_id: target.agent_id.clone(),
            room_id: target.room_id.clone(),
            task_summary: input.user_message.clone(),
        },
    );

    // 4. 真实执行成员回合（事件流式转发；错误事件已由 executor 推送，仍需 Complete 收尾）
    let emit = |evt: DispatchEvent| {
        let _ = on_event.send(evt);
    };
    let result = execute_fleet_turn(&app_state, &target, &input.user_message, &emit).await;

    // 5. 流结束
    send_event(&on_event, DispatchEvent::Complete);

    result.map(|_| ())
}

/// 直接 DM 指定 agent（绕过 LLM 路由，仍真实执行）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectMessageInput {
    /// 舰队 ID
    pub fleet_id: String,
    /// 目标 agent slug
    pub agent_slug: String,
    /// 用户消息
    pub user_message: String,
    /// 历史消息
    #[serde(default)]
    pub history: Vec<DispatchChatMessage>,
}

#[agent_command(domain = fleet, safety = Caution, call_mode = StateInput, description = "直接发送消息给指定agent")]
#[tauri::command]
pub async fn fleet_direct_message(
    app_state: State<'_, AppState>,
    input: DirectMessageInput,
    on_event: tauri::ipc::Channel<DispatchEvent>,
) -> Result<(), ErrorResponse> {
    // 1. 定位目标成员
    let members = app_state.fleet_repository.list_members(&input.fleet_id).await.map_err(|e| {
        ErrorResponse::from_error_with_code(fleet_err::NOT_FOUND, e, ErrorCategory::General)
    })?;
    let target =
        members.into_iter().find(|m| m.agent_slug == input.agent_slug).ok_or_else(|| {
            ErrorResponse::new(fleet_err::TARGET_NOT_IN_FLEET)
                .with_category(ErrorCategory::Validation)
                .with_param("slug", input.agent_slug.clone())
        })?;

    // 2. 路由决策事件
    send_event(
        &on_event,
        DispatchEvent::Routing {
            agent_slug: target.agent_slug.clone(),
            agent_id: target.agent_id.clone(),
            room_id: target.room_id.clone(),
            task_summary: input.user_message.clone(),
        },
    );

    // 3. 真实执行成员回合（错误事件已由 executor 推送，仍需 Complete 收尾）
    let emit = |evt: DispatchEvent| {
        let _ = on_event.send(evt);
    };
    let result = execute_fleet_turn(&app_state, &target, &input.user_message, &emit).await;

    // 4. 流结束
    send_event(&on_event, DispatchEvent::Complete);

    result.map(|_| ())
}

// ── 内部工具 ─────────────────────────────────────────────────────────

/// 发送事件到 Channel（忽略前端已销毁的静默失败）
fn send_event(channel: &tauri::ipc::Channel<DispatchEvent>, event: DispatchEvent) {
    if let Err(e) = channel.send(event) {
        warn!("[fleet] 事件推送失败（前端可能已关闭）: {e}");
    }
}

/// 构造路由系统提示词（成员列表 + 路由规则）
fn build_fleet_system_prompt(members: &[FleetMember]) -> String {
    let member_list: Vec<String> = members
        .iter()
        .map(|m| {
            format!(
                "- slug: \"{}\", 角色: \"{}\", 房间: \"{}\", 状态: {:?}",
                m.agent_slug, m.role, m.room_id, m.status
            )
        })
        .collect();

    format!(
        "你是一个智能调度员,负责将用户消息路由到最合适的 AI agent。\n\n\
         ## 可用成员\n{}\n\n\
         ## 路由规则\n\
         1. 仔细分析用户消息的意图\n\
         2. 根据成员的角色描述选择最合适的一个\n\
         3. 仅返回 JSON,不要任何额外文本\n\n\
         ## 返回格式\n\
         {{\"agent_slug\": \"<成员 slug>\", \"reason\": \"<选择原因,简短>\"}}",
        member_list.join("\n")
    )
}

/// 构造路由用户提示词（用户消息 + 历史）
fn build_fleet_user_prompt(user_message: &str, history: &[DispatchChatMessage]) -> String {
    if history.is_empty() {
        return format!("用户消息:\n{user_message}");
    }

    let history_text: Vec<String> = history
        .iter()
        .filter(|h| h.role == "user" || h.role == "assistant")
        .map(|h| {
            let speaker = h.agent_slug.as_deref().unwrap_or("user");
            format!("[{speaker}]: {}", h.content)
        })
        .collect();

    format!("历史对话:\n{}\n\n用户消息:\n{user_message}", history_text.join("\n"))
}

/// 解析 LLM 返回的 JSON，提取 agent_slug（兼容 markdown 包裹）
fn parse_route_response(response: &str) -> Option<String> {
    let trimmed = response.trim();
    let json_str = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(|s| s.trim())
        .unwrap_or(trimmed);

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let slug = parsed.get("agent_slug")?.as_str()?.to_string();
    if slug.is_empty() { None } else { Some(slug) }
}

/// 将 LLM 返回的目标标识解析到实际成员 slug：精确 → 归一化 → 子串/显示名匹配。
///
/// LLM 自由文本可能返回带引号/大小写变体/显示名，直接 `==` 比对经常落空，
/// 此处逐级容错后再兜底，减少"静默路由到第一个成员"的错配概率。
fn resolve_target_slug(routable: &[FleetMember], raw_slug: &str) -> Option<String> {
    // 1. 精确匹配
    if let Some(m) = routable.iter().find(|m| m.agent_slug == raw_slug) {
        return Some(m.agent_slug.clone());
    }
    // 2. 归一化匹配（去空白/引号，大小写不敏感）
    let norm = |s: &str| s.trim().trim_matches('"').trim_matches('\'').to_lowercase();
    let normalized = norm(raw_slug);
    if let Some(m) = routable.iter().find(|m| norm(&m.agent_slug) == normalized) {
        return Some(m.agent_slug.clone());
    }
    // 3. 子串 / 显示名匹配（LLM 可能返回 display_name 或别名）
    routable
        .iter()
        .find(|m| {
            norm(&m.agent_slug).contains(&normalized)
                || normalized.contains(&norm(&m.agent_slug))
                || norm(&m.display_name) == normalized
        })
        .map(|m| m.agent_slug.clone())
}
