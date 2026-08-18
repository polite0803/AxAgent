// SPDX-License-Identifier: AGPL-3.0-only

//! Fleet 真实执行器 — 将路由决策落地为真实的 Agent 回合执行。
//!
//! ## 背景
//!
//! 此前 `fleet_dispatch` / `fleet_direct_message` 只产生 `Routing` 事件就返回，
//! agent 从未真正执行（演示假功能）。本模块补齐完整链路：
//!
//! ```text
//! 路由决策（真实 LLM 意图分类 / 直接指定）
//!   → 成员状态 Busy + AgentStatus 事件
//!   → 解析默认提供商 + ProviderRequestContext + ProviderAdapter
//!   → SessionManager::get_or_create_session（conversation_id = member.agent_id）
//!   → 构建 ApiClient + 最小工具注册表
//!   → create_conversation_runtime + run_turn_with_tools（真实 LLM 推理）
//!   → 提取 assistant 文本 → AgentMessage 事件
//!   → 提取 usage → TokenUsage 事件 + 累加成员 token
//!   → 成员状态 Idle + AgentStatus 事件 + Complete
//! ```
//!
//! ## 设计取舍
//!
//! - **不加载 MCP 服务器**：MCP 工具加载链很重（并发拉取描述/凭证），办公室场景
//!   默认不启用；内置工具（load_enabled_state）照常加载。
//! - **不注入 AskUser 桥接**：办公室回合不阻塞等待用户确认，工具权限走
//!   `PermissionMode::Prompt`，超出默认权限的工具调用会失败并记录（不会卡死）。
//! - **成员不绑定 provider**：使用「第一个启用且含可用 key 的提供商」+ 其默认模型。
//!   后续如需按成员差异化，可在 `FleetMember.role` 中约定 `provider:<id>` 前缀。

use crate::AppState;
use crate::commands::error::ErrorCategory;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::fleet as fleet_err;
use crate::commands::memory::resolve_default_provider;
use async_trait::async_trait;
use axagent_agent::AxAgentApiClient;
use axagent_dao::repo::agent_profile;
use axagent_dao::repo::agent_role;
use axagent_entities::agency_experts;
use axagent_harness::fleet::{DispatchEvent, FleetIntentLlm, FleetMember, FleetMemberStatus};
use axagent_harness::runtime_types::permissions::PermissionMode;
use axagent_harness::runtime_types::permissions::PermissionPolicy;
use axagent_runtime::harness::RuntimeHarness;
use axagent_runtime_core::ConversationRuntimeFactoryArgs;
use axagent_runtime_core::RuntimeFeatureConfig;
use axagent_runtime_core::create_conversation_runtime;
use axagent_tools::registry::UnifiedToolRegistry;
use sea_orm::EntityTrait;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tracing::{info, warn};

/// 真实 LLM 意图分类：用默认提供商跑一次非流式 chat，返回 `{"agent_slug": "..."}` JSON 文本。
///
/// 调用失败时返回 `Err`，由上层兜底到第一个可路由成员（不阻塞用户）。
async fn route_with_harness(
    harness: &RuntimeHarness,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    axagent_runtime::llm_helpers::chat_with_default_provider(
        harness,
        system_prompt,
        user_prompt,
        256,
    )
    .await
}

/// `FleetIntentLlm` 的真实实现（wiring 层注入）：
/// 持有 `RuntimeHarness`（Clone），用默认提供商做意图分类。
///
/// 由 `init/state.rs` 注入到 `AppState.fleet_intent_llm`，
/// 供 `fleet_dispatch` 等命令消费；也供 `axagent_agent::LlmDispatcher` 构造使用。
pub struct ProviderFleetIntentLlm {
    harness: RuntimeHarness,
}

impl ProviderFleetIntentLlm {
    pub fn new(harness: RuntimeHarness) -> Self {
        Self { harness }
    }
}

#[async_trait]
impl FleetIntentLlm for ProviderFleetIntentLlm {
    async fn route(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        route_with_harness(&self.harness, system_prompt, user_prompt).await
    }
}

/// 解析成员关联的 AgentProfile，返回 (展示名, 组合系统提示词)。
///
/// AgentProfile = 角色（agent_role → `agent_roles.system_prompt`）+
/// 专家（expert_id → `agency_experts.system_prompt`）组合而成。解析失败或
/// 成员未关联 profile 时返回 `None`，由调用方回退自由文本 `role`。
async fn resolve_member_profile(
    harness: &RuntimeHarness,
    member: &FleetMember,
) -> Option<(String, String)> {
    let profile_id = member.agent_profile_id.as_deref()?;
    let profile = agent_profile::get_agent_profile(harness.db(), profile_id).await.ok()?;

    let mut parts: Vec<String> = Vec::new();
    // 专家提示词：专家系统提示词描述「我是谁 / 专业领域」
    if let Some(expert_id) = profile.expert_id.as_deref() {
        if let Ok(Some(exp)) = agency_experts::Entity::find_by_id(expert_id).one(harness.db()).await
        {
            let sp = exp.system_prompt.trim().to_string();
            if !sp.is_empty() {
                parts.push(sp);
            }
        }
    }
    // 角色提示词：角色系统提示词描述「在团队中扮演什么角色 / 职责」
    if let Some(role_id) = profile.agent_role.as_deref() {
        if let Ok(Some(role)) = agent_role::get_agent_role(harness.db(), role_id).await {
            let sp = role.system_prompt.trim().to_string();
            if !sp.is_empty() {
                parts.push(sp);
            }
        }
    }
    // 兜底：描述 / 名称
    if parts.is_empty() {
        if let Some(desc) = profile.description.as_deref() {
            let d = desc.trim().to_string();
            if !d.is_empty() {
                parts.push(d);
            }
        }
    }
    if parts.is_empty() {
        parts.push(profile.name.clone());
    }

    Some((profile.name, parts.join("\n\n")))
}

/// 真实执行一个成员回合，产出事件流。
///
/// - `member`: 目标成员（路由已确定）
/// - `user_message`: 用户消息（含历史摘要）
/// - `emit`: 事件回调（命令层转发到 Channel）
/// - 返回所有事件（不含路由决策与 Complete，由调用方补充）
pub async fn execute_fleet_turn(
    app_state: &AppState,
    member: &FleetMember,
    user_message: &str,
    emit: &(dyn Fn(DispatchEvent) + Sync),
) -> Result<Vec<DispatchEvent>, ErrorResponse> {
    let mut events: Vec<DispatchEvent> = Vec::new();

    // ── 1. 状态 → Busy ──
    let _ =
        app_state.fleet_repository.update_member_status(&member.id, FleetMemberStatus::Busy).await;
    let busy_evt = DispatchEvent::AgentStatus {
        agent_slug: member.agent_slug.clone(),
        agent_id: member.agent_id.clone(),
        status: FleetMemberStatus::Busy,
    };
    events.push(busy_evt.clone());
    emit(busy_evt);

    // ── 2. 解析提供商（失败则报错并复位状态）──
    let resolved = match resolve_default_provider(app_state).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("解析模型提供商失败: {e}");
            let err_evt = DispatchEvent::Error { message: msg.clone() };
            events.push(err_evt.clone());
            emit(err_evt);
            let _ = app_state
                .fleet_repository
                .update_member_status(&member.id, FleetMemberStatus::Error)
                .await;
            let idle_evt = DispatchEvent::AgentStatus {
                agent_slug: member.agent_slug.clone(),
                agent_id: member.agent_id.clone(),
                status: FleetMemberStatus::Error,
            };
            events.push(idle_evt.clone());
            emit(idle_evt);
            return Err(ErrorResponse::from_error_with_code(
                fleet_err::NO_PROVIDER,
                msg,
                ErrorCategory::Retryable,
            ));
        },
    };

    // ── 2.5 解析成员 AgentProfile（角色+专家组合），定义智能体身份 ──
    let resolved_agent_profile = resolve_member_profile(&app_state.harness, member).await;
    let role_label = resolved_agent_profile
        .as_ref()
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| member.role.clone());

    // ── 3. 获取/创建 AgentSession（conversation_id = member.agent_id）──
    let conversation_id = member.agent_id.clone();
    let session = match app_state
        .agent_session_manager
        .get_or_create_session(resolved.provider_id.clone(), conversation_id.clone())
        .await
    {
        Ok(s) => s.with_role(role_label),
        Err(e) => {
            let err_evt =
                DispatchEvent::Error { message: format!("创建 Agent 会话失败: {e}") };
            events.push(err_evt.clone());
            emit(err_evt);
            return Err(ErrorResponse::from_error_with_code(
                fleet_err::EXECUTION_FAILED,
                format!("创建 Agent 会话失败: {e}"),
                ErrorCategory::Retryable,
            ));
        },
    };
    let session_id = session.session().session_id.clone();

    // ── 4. 构建 ApiClient + 最小工具注册表 ──
    let api_client = AxAgentApiClient::new(resolved.adapter.clone(), resolved.ctx.clone())
        .with_model(resolved.model_id.clone());

    let mut tool_registry = UnifiedToolRegistry::new();
    tool_registry.load_enabled_state(app_state.harness.db()).await;

    // ── 5. 系统提示词：AgentProfile（角色+专家组合）优先，回退角色文本 ──
    let base_prompt = match &resolved_agent_profile {
        Some((_, prompt)) => prompt.clone(),
        None => {
            if member.role.is_empty() {
                "(未设定角色)".to_string()
            } else {
                member.role.clone()
            }
        },
    };
    let system_prompt = format!(
        "你是 AxAgent 办公室（Fleet）中的一名成员。\n\
         agent slug: {}\n\
         显示名: {}\n\
         角色职责: {}\n\
         当前房间: {}\n\n\
         请基于你的角色与职责，尽最大努力完成用户交给你的任务。\n\
         回答使用与用户相同的语言。",
        member.agent_slug, member.display_name, base_prompt, member.room_id,
    );

    // ── 6. 构建 runtime 并执行回合 ──
    let runtime = create_conversation_runtime(ConversationRuntimeFactoryArgs::new(
        session.session().clone(),
        Box::new(api_client),
        Box::new(tool_registry),
        PermissionPolicy::new(PermissionMode::Prompt),
        vec![system_prompt],
        RuntimeFeatureConfig::default(),
    ));

    let cancel_token = Arc::new(AtomicBool::new(false));
    let result = app_state
        .agent_session_manager
        .run_turn_with_tools(
            &session_id,
            user_message.to_string(),
            runtime,
            conversation_id.clone(),
            Some(cancel_token),
            app_state.agent_prompters.clone(),
        )
        .await;

    match result {
        Ok((summary, _updated_session)) => {
            // 提取 assistant 文本
            let mut text = String::new();
            for msg in &summary.assistant_messages {
                for block in &msg.blocks {
                    if let axagent_runtime::ContentBlock::Text { text: block_text } = block {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(block_text);
                    }
                }
            }
            if !text.trim().is_empty() {
                let msg_evt = DispatchEvent::AgentMessage {
                    agent_slug: member.agent_slug.clone(),
                    agent_id: member.agent_id.clone(),
                    content: text.trim().to_string(),
                };
                events.push(msg_evt.clone());
                emit(msg_evt);
            }

            // Token 用量上报 + 累加
            let input_tokens = summary.usage.input_tokens;
            let output_tokens = summary.usage.output_tokens;
            let total = input_tokens + output_tokens;
            if total > 0 {
                let _ =
                    app_state.fleet_repository.add_member_tokens(&member.id, total as u64).await;
                let usage_evt = DispatchEvent::TokenUsage {
                    agent_slug: member.agent_slug.clone(),
                    agent_id: member.agent_id.clone(),
                    input_tokens: input_tokens as u64,
                    output_tokens: output_tokens as u64,
                };
                events.push(usage_evt.clone());
                emit(usage_evt);
            }

            info!(
                "[fleet] member {} turn completed: {} chars, {} tokens",
                member.agent_slug,
                text.len(),
                total
            );
        },
        Err(e) => {
            let msg = format!("成员 {} 执行失败: {e}", member.agent_slug);
            warn!("{msg}");
            let err_evt = DispatchEvent::Error { message: msg.clone() };
            events.push(err_evt.clone());
            emit(err_evt);
            let _ = app_state
                .fleet_repository
                .update_member_status(&member.id, FleetMemberStatus::Error)
                .await;
            let err_status = DispatchEvent::AgentStatus {
                agent_slug: member.agent_slug.clone(),
                agent_id: member.agent_id.clone(),
                status: FleetMemberStatus::Error,
            };
            events.push(err_status.clone());
            emit(err_status);
            return Err(ErrorResponse::from_error_with_code(
                fleet_err::EXECUTION_FAILED,
                msg,
                ErrorCategory::Retryable,
            ));
        },
    }

    // ── 7. 状态 → Idle ──
    let _ =
        app_state.fleet_repository.update_member_status(&member.id, FleetMemberStatus::Idle).await;
    let idle_evt = DispatchEvent::AgentStatus {
        agent_slug: member.agent_slug.clone(),
        agent_id: member.agent_id.clone(),
        status: FleetMemberStatus::Idle,
    };
    events.push(idle_evt.clone());
    emit(idle_evt);

    Ok(events)
}
