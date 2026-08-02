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
use async_trait::async_trait;
use axagent_agent::AxAgentApiClient;
use axagent_dao::repo::provider;
use axagent_harness::fleet::{DispatchEvent, FleetIntentLlm, FleetMember, FleetMemberStatus};
use axagent_harness::runtime_types::permissions::PermissionMode;
use axagent_harness::runtime_types::permissions::PermissionPolicy;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext, resolve_base_url_for_type};
use axagent_runtime::harness::RuntimeHarness;
use axagent_runtime_core::ConversationRuntimeFactoryArgs;
use axagent_runtime_core::RuntimeFeatureConfig;
use axagent_runtime_core::create_conversation_runtime;
use axagent_tools::registry::UnifiedToolRegistry;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tracing::{info, warn};

/// 已解析的默认提供商上下文（供路由与执行共用）。
struct ResolvedProvider {
    provider_id: String,
    model_id: String,
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
}

/// 从 Harness 解析「第一个启用且含可用 key 的提供商」。
async fn resolve_default_provider(harness: &RuntimeHarness) -> Result<ResolvedProvider, String> {
    let providers = provider::list_providers(harness.db()).await.unwrap_or_default();

    let prov = providers
        .into_iter()
        .find(|p| p.enabled && p.keys.iter().any(|k| k.enabled))
        .ok_or_else(|| "没有启用的模型提供商".to_string())?;

    let key =
        prov.keys.iter().find(|k| k.enabled).ok_or_else(|| "没有可用的 API key".to_string())?;
    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, harness.master_key())
        .map_err(|e| format!("解密 API key 失败: {e}"))?;

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
            &prov.proxy_config,
            &axagent_dao::repo::settings::get_settings(harness.db()).await.unwrap_or_default(),
        ),
        custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = harness
        .get_adapter_for_provider(&prov)
        .await
        .ok_or_else(|| format!("无适配器可用: {:?}", prov.provider_type))?;

    // 默认模型：取该 provider 模型列表的第一个
    let model_id = prov.models.first().map(|m| m.model_id.clone()).unwrap_or_default();

    Ok(ResolvedProvider { provider_id: prov.id, model_id, adapter, ctx })
}

/// 真实 LLM 意图分类：用默认提供商跑一次非流式 chat，返回 `{"agent_slug": "..."}` JSON 文本。
///
/// 调用失败时返回 `Err`，由上层兜底到第一个可路由成员（不阻塞用户）。
async fn route_with_harness(
    harness: &RuntimeHarness,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let resolved = resolve_default_provider(harness).await?;

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.0),
        top_p: None,
        max_tokens: Some(256),
        ..Default::default()
    };

    let resp = resolved
        .adapter
        .chat(&resolved.ctx, Arc::new(request))
        .await
        .map_err(|e| format!("路由 LLM 调用失败: {e}"))?;

    Ok(resp.content)
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
        status: FleetMemberStatus::Busy,
    };
    events.push(busy_evt.clone());
    emit(busy_evt);

    // ── 2. 解析提供商（失败则报错并复位状态）──
    let resolved = match resolve_default_provider(&app_state.harness).await {
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

    // ── 3. 获取/创建 AgentSession（conversation_id = member.agent_id）──
    let conversation_id = member.agent_id.clone();
    let session = match app_state
        .agent_session_manager
        .get_or_create_session(resolved.provider_id.clone(), conversation_id.clone())
        .await
    {
        Ok(s) => s,
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

    // ── 5. 系统提示词：成员角色 + 办公室上下文 ──
    let system_prompt = format!(
        "你是 AxAgent 办公室（Fleet）中的一名成员。\n\
         agent slug: {}\n\
         显示名: {}\n\
         角色职责: {}\n\
         当前房间: {}\n\n\
         请基于你的角色职责，尽最大努力完成用户交给你的任务。\n\
         回答使用与用户相同的语言。",
        member.agent_slug,
        member.display_name,
        if member.role.is_empty() {
            "(未设定角色)"
        } else {
            &member.role
        },
        member.room_id,
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
        status: FleetMemberStatus::Idle,
    };
    events.push(idle_evt.clone());
    emit(idle_evt);

    Ok(events)
}
