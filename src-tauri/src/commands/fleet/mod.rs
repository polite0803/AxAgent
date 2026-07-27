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
//! ### Dispatcher 智能路由
//! - `fleet_dispatch` — 群聊智能路由（LLM 决定路由到哪个 agent）
//! - `fleet_direct_message` — 直接 DM 指定 agent（绕过 LLM 路由）

use crate::AppState;
use axagent_harness::fleet::{
    DispatchChatMessage, DispatchEvent, Fleet, FleetIntentLlm, FleetMember, FleetMemberStatus,
    FleetMetadata, FleetRepository, FleetStatus, IntentDispatcher, NoopFleetIntentLlm,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// ── 舰队 CRUD ────────────────────────────────────────────────────────

/// 列出所有舰队（可选状态过滤）
#[tauri::command]
pub async fn fleet_list(
    app_state: State<'_, AppState>,
    status_filter: Option<FleetStatus>,
) -> Result<Vec<Fleet>, String> {
    app_state
        .fleet_repository
        .list_fleets(status_filter)
        .await
        .map_err(|e| format!("列出舰队失败: {e}"))
}

/// 获取舰队详情
#[tauri::command]
pub async fn fleet_get(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<Option<Fleet>, String> {
    app_state.fleet_repository.get_fleet(&fleet_id).await.map_err(|e| format!("查询舰队失败: {e}"))
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
#[tauri::command]
pub async fn fleet_create(
    app_state: State<'_, AppState>,
    input: CreateFleetInput,
) -> Result<Fleet, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let fleet = Fleet {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        scene_template_slug: input.scene_template_slug,
        status: FleetStatus::Active,
        created_at: now,
        updated_at: now,
        metadata: input.metadata,
    };
    app_state.fleet_repository.create_fleet(fleet).await.map_err(|e| format!("创建舰队失败: {e}"))
}

/// 更新舰队状态
#[tauri::command]
pub async fn fleet_update_status(
    app_state: State<'_, AppState>,
    fleet_id: String,
    status: FleetStatus,
) -> Result<(), String> {
    app_state
        .fleet_repository
        .update_fleet_status(&fleet_id, status)
        .await
        .map_err(|e| format!("更新舰队状态失败: {e}"))
}

/// 删除舰队（级联删除成员）
#[tauri::command]
pub async fn fleet_delete(app_state: State<'_, AppState>, fleet_id: String) -> Result<(), String> {
    app_state
        .fleet_repository
        .delete_fleet(&fleet_id)
        .await
        .map_err(|e| format!("删除舰队失败: {e}"))
}

// ── 成员管理 ─────────────────────────────────────────────────────────

/// 列出舰队所有成员
#[tauri::command]
pub async fn fleet_list_members(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<Vec<FleetMember>, String> {
    app_state
        .fleet_repository
        .list_members(&fleet_id)
        .await
        .map_err(|e| format!("列出成员失败: {e}"))
}

/// 添加成员的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberInput {
    /// 所属舰队 ID
    pub fleet_id: String,
    /// 关联的 AgentSession ID（由 SessionManager 创建）
    pub agent_id: String,
    /// agent slug（业务标识，用于 Dispatcher 路由）
    pub agent_slug: String,
    /// 显示名称
    pub display_name: String,
    /// 角色描述（注入到 Dispatcher prompt）
    #[serde(default)]
    pub role: String,
    /// 房间 ID（前端 Phaser 渲染位置，如 "manager" / "meeting"）
    #[serde(default = "default_room_id")]
    pub room_id: String,
}

fn default_room_id() -> String {
    "workspace".to_string()
}

/// 添加成员到舰队
#[tauri::command]
pub async fn fleet_add_member(
    app_state: State<'_, AppState>,
    input: AddMemberInput,
) -> Result<FleetMember, String> {
    let member = FleetMember {
        id: uuid::Uuid::new_v4().to_string(),
        fleet_id: input.fleet_id,
        agent_id: input.agent_id,
        agent_slug: input.agent_slug,
        display_name: input.display_name,
        role: input.role,
        room_id: input.room_id,
        status: FleetMemberStatus::Idle,
        joined_at: chrono::Utc::now().timestamp_millis(),
        today_tokens: 0,
        total_tokens: 0,
    };
    app_state.fleet_repository.add_member(member).await.map_err(|e| format!("添加成员失败: {e}"))
}

/// 获取单个成员
#[tauri::command]
pub async fn fleet_get_member(
    app_state: State<'_, AppState>,
    member_id: String,
) -> Result<Option<FleetMember>, String> {
    app_state
        .fleet_repository
        .get_member(&member_id)
        .await
        .map_err(|e| format!("查询成员失败: {e}"))
}

/// 更新成员状态
#[tauri::command]
pub async fn fleet_update_member_status(
    app_state: State<'_, AppState>,
    member_id: String,
    status: FleetMemberStatus,
) -> Result<(), String> {
    app_state
        .fleet_repository
        .update_member_status(&member_id, status)
        .await
        .map_err(|e| format!("更新成员状态失败: {e}"))
}

/// 移除成员
#[tauri::command]
pub async fn fleet_remove_member(
    app_state: State<'_, AppState>,
    member_id: String,
) -> Result<(), String> {
    app_state
        .fleet_repository
        .remove_member(&member_id)
        .await
        .map_err(|e| format!("移除成员失败: {e}"))
}

/// 重置舰队所有成员今日 token（每日定时任务调用）
#[tauri::command]
pub async fn fleet_reset_daily_tokens(
    app_state: State<'_, AppState>,
    fleet_id: String,
) -> Result<(), String> {
    app_state
        .fleet_repository
        .reset_daily_tokens(&fleet_id)
        .await
        .map_err(|e| format!("重置今日 token 失败: {e}"))
}

// ── Dispatcher 智能路由 ──────────────────────────────────────────────

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

/// 群聊智能路由 — 由 LLM 决定路由到哪个 agent
///
/// 返回 DispatchEvent 列表（前端按顺序消费：Routing → AgentMessage → Complete）
#[tauri::command]
pub async fn fleet_dispatch(
    app_state: State<'_, AppState>,
    input: DispatchInput,
) -> Result<Vec<DispatchEvent>, String> {
    // 懒加载 dispatcher（避免 AppState 持有 dispatcher 字段，每次调用都重建轻量级实例）
    // 注意：FleetIntentLlm 实例通过 AppState 获取，此处用 Noop 兜底
    // 真正的 LLM 路由需要在 wiring 层注入 FleetIntentLlm 到 AppState（P1 任务）
    let dispatcher = build_dispatcher(&app_state);
    dispatcher
        .dispatch_stream(&input.fleet_id, &input.user_message, input.history)
        .await
        .map_err(|e| format!("调度失败: {e}"))
}

/// 直接 DM 指定 agent（绕过 LLM 路由）
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

#[tauri::command]
pub async fn fleet_direct_message(
    app_state: State<'_, AppState>,
    input: DirectMessageInput,
) -> Result<Vec<DispatchEvent>, String> {
    let dispatcher = build_dispatcher(&app_state);
    dispatcher
        .direct_message_stream(
            &input.fleet_id,
            &input.agent_slug,
            &input.user_message,
            input.history,
        )
        .await
        .map_err(|e| format!("直接消息失败: {e}"))
}

/// 构建 LlmDispatcher（每次调用都新建，内部仅持有 Arc 引用，开销极低）
fn build_dispatcher(app_state: &AppState) -> axagent_agent::LlmDispatcher {
    let fleet_repo: Arc<dyn FleetRepository> = Arc::clone(&app_state.fleet_repository);
    // TODO(P1): 注入真实的 FleetIntentLlm 实现（wiring 层包装 ProviderLlmBridge）
    // 当前用 NoopFleetIntentLlm 兜底,dispatch_stream 会回退到首个可用成员
    let intent_llm: Arc<dyn FleetIntentLlm> = Arc::new(NoopFleetIntentLlm);
    axagent_agent::LlmDispatcher::new(fleet_repo, intent_llm)
}
