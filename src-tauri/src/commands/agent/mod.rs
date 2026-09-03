// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::agent as agent_err;
use crate::commands::error_code::agent_input as agent_input_err;
use crate::commands::error_code::agent_status as agent_status_err;
use crate::commands::error_code::steer as steer_err;
use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_agent::{
    AxAgentApiClient, DefaultToTReasoningProvider, FallbackProviderAdapter, TreeOfThoughtsEngine,
};
use axagent_agent_macro::agent_command;
use axagent_dao::repo::{conversation, message, provider, search_provider};
use axagent_harness::runtime_types::permissions::PermissionPolicy;
use axagent_harness::types::{
    Attachment, ChatContent, ChatMessage, ChatRequest, ChatTool, ChatToolFunction, McpServer,
    MessageRole,
};
use axagent_harness::{
    ProviderAdapter, ProviderRequestContext, ToolDomain, resolve_base_url_for_type,
};
use axagent_runtime_core::ConversationRuntimeFactoryArgs;
use axagent_runtime_core::create_conversation_runtime;
use axagent_runtime_core::execution_progress::AgentExecutionProgressSnapshot;
use axagent_storage::cloud_workspace::CloudWorkspace;
use axagent_storage::workspace_uri::WorkspaceUri;
use axagent_tools::context_keys;
use axagent_tools::registry::{
    DISCLOSURE_TOOLS, McpServerConfig, SCREEN_PERCEPTION_TOOL, UnifiedToolRegistry,
    is_disclosure_immune,
};
use base64::Engine;
use dashmap::DashMap;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use tracing::info;

use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

mod payloads;
pub use payloads::*;

pub mod pricing;
#[allow(unused_imports)]
pub use pricing::init_pricing_config;
// 供本模块内其他函数调用（pricing.rs 中为 pub(super)，对本模块可见）
#[allow(unused_imports)]
use pricing::{check_token_budget, estimate_cost_usd};

pub mod skill_execution;
use skill_execution::{
    SkillExecutionContext, build_agent_system_prompt, execute_skill_sync,
    load_enabled_skill_contents, load_skill_tools,
};

/// 渐进式披露 L0 索引层：把能力护照渲染成轻量目录注入系统提示
mod capability_index;
pub mod command_bridge;
use command_bridge::{
    CommandCache, CommandRegistry, build_chat_tools as build_tauri_command_chat_tools,
    build_command_handlers, build_command_index_string, preload_command_cache,
    resolve_command_domains,
};

/// AskUser 桥接器的具体实现，由 wiring 层注入到 UnifiedToolRegistry。
/// 当 LLM 调用 AskUserQuestionTool 时，通过此桥接器：
/// 1. emit `agent-ask-user` 事件到前端
/// 2. 阻塞等待用户通过 `agent_respond_ask` 回复
#[derive(Debug, Clone)]
struct AppAskUserBridge {
    app_handle: AppHandle,
    ask_senders: Arc<
        tokio::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
    >,
    conversation_id: String,
    assistant_message_id: String,
}

impl axagent_harness::AskUserBridge for AppAskUserBridge {
    fn ask_user_blocking(
        &self,
        ask_id: String,
        questions_json: serde_json::Value,
        _conversation_id: &str,
    ) -> Result<String, String> {
        let questions = questions_json["questions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|q| {
                        let question = q["question"].as_str().unwrap_or("");
                        let multi = q["multiSelect"].as_bool().unwrap_or(false);
                        let options: Vec<String> = q["options"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|o| o["label"].as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        serde_json::json!({
                            "question": question,
                            "multiSelect": multi,
                            "options": options,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // 提取第一个问题的文本作为主问题
        let question_text =
            questions.first().and_then(|q| q["question"].as_str()).unwrap_or("").to_string();
        let options: Vec<String> = questions
            .first()
            .and_then(|q| q["options"].as_array())
            .map(|a| a.iter().filter_map(|o| o.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let event_payload = serde_json::json!({
            "conversationId": self.conversation_id,
            "assistantMessageId": self.assistant_message_id,
            "askId": ask_id,
            "question": question_text,
            "options": options,
        });

        let _ = self.app_handle.emit("agent-ask-user", &event_payload);

        // 创建 oneshot channel 并阻塞等待用户回复
        let (tx, rx) = tokio::sync::oneshot::channel();

        // 需要同步插入 sender（在 async 上下文中使用 block_in_place + block_on）
        let ask_senders = self.ask_senders.clone();
        let ask_id_clone = ask_id.clone();
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                let mut senders = ask_senders.lock().await;
                senders.insert(ask_id_clone, tx);
            });
        });

        // 阻塞等待用户回复，5 分钟超时
        let ask_senders_cleanup = self.ask_senders.clone();
        let ask_id_cleanup = ask_id.clone();
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
                    Ok(Ok(answer)) => Ok(answer),
                    Ok(Err(_)) => {
                        // sender 被丢弃，清理 ask_senders
                        let mut senders = ask_senders_cleanup.lock().await;
                        senders.remove(&ask_id_cleanup);
                        Err(ErrorResponse::err(agent_input_err::CHANNEL_CLOSED))
                    },
                    Err(_) => {
                        // 超时，清理 ask_senders
                        let mut senders = ask_senders_cleanup.lock().await;
                        senders.remove(&ask_id_cleanup);
                        Err(ErrorResponse::err(agent_input_err::WAIT_REPLY_TIMEOUT))
                    },
                }
            })
        })
    }
}

/// Async RAII guard that removes a conversation ID from AppState::running_agents on drop.
/// Ensures cleanup even if the spawned task panics.
struct AsyncRunningAgentGuard {
    conversation_id: String,
    running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>>,
    paused_set: Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
    pause_states: Arc<DashMap<String, Arc<axagent_runtime_core::PauseState>>>,
}

impl Drop for AsyncRunningAgentGuard {
    fn drop(&mut self) {
        let running_agents = self.running_agents.clone();
        let cancel_tokens = self.cancel_tokens.clone();
        let paused_set = self.paused_set.clone();
        let pause_states = self.pause_states.clone();
        let conversation_id = self.conversation_id.clone();
        if let Ok(_handle) = tokio::runtime::Handle::try_current() {
            tokio::spawn(catch_unwind_logged("agent.cleanup", async move {
                let mut agents = running_agents.write().await;
                agents.remove(&conversation_id);
                cancel_tokens.remove(&conversation_id);
                let mut paused = paused_set.lock().await;
                paused.remove(&conversation_id);
                pause_states.remove(&conversation_id);
            }));
        } else {
            let mut agents = running_agents.blocking_write();
            agents.remove(&conversation_id);
            cancel_tokens.remove(&conversation_id);
            let mut paused = paused_set.blocking_lock();
            paused.remove(&conversation_id);
            pause_states.remove(&conversation_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Payload types for Tauri events

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Execute an agent query
/// 发射 agent 状态事件，让前端实时展示后端进度
fn emit_status(
    app: &AppHandle,
    conversation_id: &str,
    phase: &str,
    message: &str,
    code: Option<&str>,
) {
    let _ = app.emit(
        "agent-status",
        AgentStatusPayload {
            conversation_id: conversation_id.to_string(),
            phase: phase.to_string(),
            message: message.to_string(),
            code: code.map(String::from),
        },
    );
}

/// 将字符串形式的 domain 名解析为 ToolDomain 枚举值（兼容历史旧值 core/invest/opc）
fn parse_domain_str(s: &str) -> Option<ToolDomain> {
    s.parse().ok()
}

/// AgentProfile 解析后的工具上下文。
///
/// 由 `resolve_profile_tool_context` 统一产出，供 `agent_query` 和
/// `get_tool_count` 共享，保证筛选语义一致（禁区 12：禁止重复定义）。
#[derive(Default)]
pub(crate) struct ProfileToolContext {
    /// 结构化 Agent 标识：AgentProfile 名称，供轨迹记录时标记执行来源
    pub agent_name: Option<String>,
    /// 角色 + 专家合并的工具域（不含默认兜底的 General）
    pub active_domains: HashSet<ToolDomain>,
    /// 岗位（AgentRole）的 system_prompt
    pub role_system_prompt: Option<String>,
    /// 技能（Expert）的 system_prompt
    pub expert_system_prompt: Option<String>,
    /// Profile 自身的推荐工具（白名单字符串）
    pub recommended_tools: Vec<String>,
    /// Profile 自身的禁用工具（黑名单字符串）
    pub disallowed_tools: Vec<String>,
}

/// 解析 AgentProfile 的工具上下文（角色 + 专家 + Profile 自身）。
///
/// 与 `agent_query` 中的三源合并逻辑保持一致：
/// - Layer 1: `profile.agent_role` → AgentRole 的 `active_domains` + system_prompt
/// - Layer 2: `profile.expert_id` → Expert 的 `active_domains` + system_prompt
/// - Layer 3: Profile 自身的 `recommended_tools` / `disallowed_tools`
///
/// active_domains 语义（修复缺陷 5）：
///   - Role 定义权限上界（岗位允许激活的域）
///   - Expert 只能在 Role 允许的域内激活（取交集）
///   - 若 Role 不存在或 active_domains 为空 → Expert 的 domains 全部生效（向后兼容）
///
/// 返回 `None` 表示 profile 不存在或查询失败（调用方应回退到默认路径）。
/// 所有失败路径均通过 `tracing::warn!` 记录（修复缺陷 4：静默吞错）。
///
/// `override_expert_id`：认知编排层传入的动态专家（角色护照命中共执行载体未组合专家时，
/// 通过 RAR 检索补全）。优先于 profile 自带 expert_id，用于"角色 + 专家"运行时组合。
pub(crate) async fn resolve_profile_tool_context(
    app_state: &AppState,
    profile_id: &str,
    override_expert_id: Option<&str>,
) -> Option<ProfileToolContext> {
    let profile = match axagent_dao::repo::agent_profile::get_agent_profile(
        app_state.harness.db(),
        profile_id,
    )
    .await
    {
        Ok(p) => p,
        Err(axagent_harness::AxAgentError::NotFound(_)) => {
            tracing::warn!(
                profile_id = %profile_id,
                "AgentProfile 不存在（profile_id 无效或已删除），降级到默认路径"
            );
            return None;
        },
        Err(e) => {
            tracing::warn!(
                profile_id = %profile_id,
                error = %e,
                "AgentProfile 查询失败（DB 错误），降级到默认路径"
            );
            return None;
        },
    };

    let mut ctx = ProfileToolContext::default();

    // Layer 1: AgentRole system_prompt（岗位）+ active_domains
    let mut role_domains: HashSet<ToolDomain> = HashSet::new();
    let mut has_role_domains = false;
    if let Some(ref role_name) = profile.agent_role {
        match axagent_runtime::agent_roles::resolve(role_name).await {
            Some(resolved) => {
                if !resolved.system_prompt.is_empty() {
                    ctx.role_system_prompt = Some(resolved.system_prompt);
                }
                for d in &resolved.active_domains {
                    if let Some(td) = parse_domain_str(d) {
                        role_domains.insert(td);
                        has_role_domains = true;
                    }
                }
            },
            None => {
                tracing::warn!(
                    profile_id = %profile_id,
                    agent_role = %role_name,
                    "AgentRole 解析失败（role_name 在 DB 和文件注册表中均未找到），role 提示词不会生效"
                );
            },
        }
    }

    // Layer 2: Expert domain knowledge（技能）+ active_domains
    // 动态专家覆盖优先（认知编排层 RAR 补全），否则回退 profile 自带 expert_id。
    let resolved_expert_id = override_expert_id
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .or(profile.expert_id.clone());
    if let Some(ref expert_id) = resolved_expert_id {
        match axagent_entities::agency_experts::Entity::find_by_id(expert_id)
            .one(app_state.harness.db())
            .await
        {
            Ok(Some(expert)) => {
                if !expert.system_prompt.is_empty() {
                    ctx.expert_system_prompt = Some(expert.system_prompt);
                }
                if let Some(ref domains_json) = expert.active_domains {
                    if let Ok(domains) = serde_json::from_str::<Vec<String>>(domains_json) {
                        // 修复缺陷 5：active_domains 取交集（Role 定义上界）
                        let expert_domains: HashSet<ToolDomain> =
                            domains.iter().filter_map(|d| parse_domain_str(d)).collect();
                        if has_role_domains {
                            // Role 存在且有 active_domains → 取交集
                            let intersection: HashSet<ToolDomain> =
                                expert_domains.intersection(&role_domains).cloned().collect();
                            let dropped: Vec<_> =
                                expert_domains.difference(&role_domains).collect();
                            if !dropped.is_empty() {
                                tracing::info!(
                                    profile_id = %profile_id,
                                    expert_id = %expert_id,
                                    role_domains = ?role_domains,
                                    dropped_expert_domains = ?dropped,
                                    "Expert 的部分 active_domains 不在 Role 允许范围内，已按交集语义裁剪"
                                );
                            }
                            ctx.active_domains = intersection;
                        } else {
                            // Role 不存在或无 active_domains → Expert 全部生效（向后兼容）
                            ctx.active_domains = expert_domains;
                        }
                    }
                }
            },
            Ok(None) => {
                tracing::warn!(
                    profile_id = %profile_id,
                    expert_id = %expert_id,
                    "Expert 不存在（expert_id 无效或已删除），expert 提示词不会生效"
                );
            },
            Err(e) => {
                tracing::warn!(
                    profile_id = %profile_id,
                    expert_id = %expert_id,
                    error = %e,
                    "Expert 查询失败（DB 错误），expert 提示词不会生效"
                );
            },
        }
    }

    // 若 Role 有 active_domains 但 Expert 未覆盖或无 Expert，
    // 直接使用 Role 的 domains（保证 Role 自身定义的域生效）
    if ctx.active_domains.is_empty() && has_role_domains {
        ctx.active_domains = role_domains.clone();
    }

    // Layer 3: Profile 自身推荐/禁用工具
    ctx.recommended_tools = profile.recommended_tools;
    ctx.disallowed_tools = profile.disallowed_tools;

    // 结构化 Agent 标识：轨迹记录时据此标记执行来源
    ctx.agent_name = Some(profile.name.clone());

    Some(ctx)
}

/// 构建带 streaming 事件回调的 `AxAgentApiClient`。
///
/// `agent_query` 内的 if/else 分支（`AxAgentApiClient::new` vs `with_tools`）的 60 行
/// 字节级重复（9 个 `.with_*` + 50 行事件回调）收敛到此处。
/// 调用方只需关心：是否传入 tools / 几个数值参数 / 三个 streaming 捕获变量。
#[allow(clippy::too_many_arguments)]
fn build_streaming_api_client(
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    chat_tools: Vec<ChatTool>,
    dynamic_tools: axagent_harness::DynamicToolSet,
    model_id: &str,
    effective_temperature: Option<f64>,
    effective_top_p: Option<f64>,
    effective_max_tokens: Option<u32>,
    thinking_budget: Option<u32>,
    use_max_completion_tokens: Option<bool>,
    thinking_param_style: Option<String>,
    request_delay_ms: Option<u64>,
    llm_cache: Option<Arc<dyn axagent_harness::cache_interceptor::HarnessCache>>,
    stream_conv_id: String,
    stream_msg_id: String,
    stream_app: AppHandle,
) -> AxAgentApiClient {
    use axagent_runtime::AssistantEvent;

    // 1. 选构造路径
    let mut client = if chat_tools.is_empty() {
        AxAgentApiClient::new(adapter, ctx)
    } else {
        AxAgentApiClient::with_tools(adapter, ctx, chat_tools)
    };

    // 1.5 绑定运行时动态工具集：CapabilityLoad 在循环内激活的工具经此下发
    client = client.with_dynamic_tools(dynamic_tools);

    // 2. 通用参数链（9 项）
    client = client
        .with_model(model_id)
        .with_temperature(effective_temperature)
        .with_top_p(effective_top_p)
        .with_max_tokens(effective_max_tokens)
        .with_thinking_budget(thinking_budget)
        .with_use_max_completion_tokens(use_max_completion_tokens)
        .with_thinking_param_style(thinking_param_style)
        .with_request_delay_ms(request_delay_ms);

    // 2.5 语义缓存拦截器（可选）：注入后主聊天路径经 execute_llm_stream 走中心化入口，
    // 相同请求命中缓存即合成流短路，省去真实 LLM 调用。缓存开关关闭时 llm_cache 为 None，
    // llm_config 保持全 None，等价于直通 provider（零额外开销）。
    if let Some(cache) = llm_cache {
        client = client.with_llm_config(axagent_harness::LlmCallConfig {
            cache: Some(cache),
            cache_ttl_secs: 3600,
            ..Default::default()
        });
    }

    // 3. streaming 事件回调（4 类事件 + 兜底）
    client.with_on_event(Box::new(move |event: &AssistantEvent| match event {
        AssistantEvent::TextDelta(text) => {
            let _ = stream_app.emit(
                "agent-stream-text",
                AgentStreamTextPayload {
                    conversation_id: stream_conv_id.clone(),
                    assistant_message_id: stream_msg_id.clone(),
                    text: text.clone(),
                },
            );
        },
        AssistantEvent::ThinkingDelta(thinking) => {
            let _ = stream_app.emit(
                "agent-stream-thinking",
                AgentStreamThinkingPayload {
                    conversation_id: stream_conv_id.clone(),
                    assistant_message_id: stream_msg_id.clone(),
                    thinking: thinking.clone(),
                },
            );
        },
        AssistantEvent::ToolUse { id, name, input } => {
            let _ = stream_app.emit(
                "agent-tool-use",
                AgentToolUsePayload {
                    conversation_id: stream_conv_id.clone(),
                    assistant_message_id: stream_msg_id.clone(),
                    tool_use_id: id.clone(),
                    tool_name: name.clone(),
                    input: serde_json::from_str(input).unwrap_or(serde_json::Value::Null),
                    execution_id: None,
                },
            );
        },
        AssistantEvent::PromptCache(evt) => {
            let _ = stream_app.emit(
                "prompt-cache-event",
                PromptCachePayload {
                    conversation_id: stream_conv_id.clone(),
                    assistant_message_id: stream_msg_id.clone(),
                    unexpected: evt.unexpected,
                    reason: evt.reason.clone(),
                    cache_read_input_tokens: evt.current_cache_read_input_tokens,
                    token_drop: evt.token_drop,
                },
            );
        },
        _ => {},
    }))
}

/// 应用 AgentProfile 的工具策略：先追加 extra，再统一剔除 blocked。
///
/// **顺序不可颠倒 —— blocked 必须是最后一道闸。** 上游 `extra_schemas` 由
/// `UnifiedToolRegistry::get_chat_tools_by_names` 取得，该方法只过滤 registry 层的
/// `disable()`，**不看 profile 的 `disallowed_tools`**。若先 retain 再注入，认知编排
/// 按需注入的 extra_tools（取自能力护照的 `tool_ref`）会把刚被 profile 禁用的工具
/// 重新注回，等于绕过禁用策略——F4 把该路径扩展到 Clarify 二次执行后，绕过面更大。
///
/// `extra_schemas` 中与已有工具同名的项会被丢弃（保留先出现者），避免 LLM 侧
/// "Tool names must be unique" 报错。
///
/// **最后一个例外**：`DISCLOSURE_TOOLS`（能力/技能发现闭环的元工具）对 profile 黑名单
/// 免疫——它们被禁用会让编排器「发现不了任何能力」且极难归因到具体 profile 配置。
/// 判定统一走 `is_disclosure_immune`，勿在此处另写一份名单（与 `get_tool_count` 共享）。
///
/// 纯函数：不碰注册表、不 await，便于单测锁定顺序语义（见本文件末尾 `tests` 模块）。
fn apply_tool_policy(
    mut chat_tools: Vec<ChatTool>,
    extra_schemas: Vec<ChatTool>,
    blocked_names: &HashSet<String>,
) -> Vec<ChatTool> {
    let mut existing_names: HashSet<String> =
        chat_tools.iter().map(|t| t.function.name.clone()).collect();
    for t in extra_schemas {
        if existing_names.insert(t.function.name.clone()) {
            chat_tools.push(t);
        }
    }
    chat_tools.retain(|t| {
        !blocked_names.contains(&t.function.name) || is_disclosure_immune(&t.function.name)
    });
    chat_tools
}

#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "执行智能体查询")]
#[tauri::command]
pub async fn agent_query(
    app: AppHandle,
    app_state: State<'_, AppState>,
    mut request: AgentQueryRequest,
) -> Result<AgentQueryResponse, String> {
    let conversation_id = request.conversation_id.clone();
    info!("[agent_query] Starting for conversation: {}", conversation_id);
    emit_status(
        &app,
        &conversation_id,
        "init",
        "正在初始化...",
        Some(agent_status_err::INITIALIZING),
    );

    let conversation = conversation::get_conversation(app_state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let conversation_scenario = conversation.scenario.clone();
    let enabled_skill_ids = conversation.enabled_skill_ids.clone();

    // AgentProfile = AgentRole + Expert（两两组装，运行时拼接提示词）
    // 不再持久化预合并的 system_prompt，修改 Expert/Role 后自动生效。
    // 解析逻辑统一收敛到 `resolve_profile_tool_context`，供 `agent_query` 和
    // `get_tool_count` 共享，避免筛选语义漂移（禁区 12）。
    let (
        role_system_prompt,
        expert_system_prompt,
        profile_recommended_tools,
        profile_disallowed_tools,
        profile_active_domains,
        profile_agent_name,
    ) = if let Some(ref profile_id) = request.agent_profile_id {
        match resolve_profile_tool_context(&app_state, profile_id, request.expert_id.as_deref())
            .await
        {
            Some(ctx) => (
                ctx.role_system_prompt,
                ctx.expert_system_prompt,
                ctx.recommended_tools,
                ctx.disallowed_tools,
                ctx.active_domains,
                ctx.agent_name,
            ),
            None => Default::default(),
        }
    } else {
        Default::default()
    };

    // 提示词合并：persona / role / expert / request.system_prompt 分层组装。
    //
    // 语义（与 build_agent_system_prompt 配合）：
    //   - persona_prompt：系统级身份注入，作为独立 slot，不与 user-custom 混淆
    //   - role_system_prompt + expert_system_prompt：profile 提示词，作为 <agent-profile> 段
    //   - request.system_prompt：用户/调用方临时覆盖，作为 <user-custom-prompt> 段
    //
    // 不再用 prompt_parts.join("\n\n") 简单拼接，避免：
    //   1. persona 被错误包装为 user-custom（缺陷 2）
    //   2. 主聊天与工作流两套合并语义割裂（缺陷 1）
    //   3. role 裸字符串占据 primacy 位置（缺陷 3，由 build_agent_system_prompt 配合修复）
    let persona_prompt = axagent_agent::personality::PersonalityManager::get_active()
        .ok()
        .flatten()
        .map(|p| p.system_prompt_injection());

    // Profile 提示词合并：role + expert（运行时拼接，不在 DB 中预缓存）
    let profile_prompt: Option<String> = {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(ref s) = role_system_prompt {
            if !s.is_empty() {
                parts.push(s.as_str());
            }
        }
        if let Some(ref s) = expert_system_prompt {
            if !s.is_empty() {
                parts.push(s.as_str());
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    };

    // Profile 未产生有效提示词时，降级到 request.system_prompt（用户/调用方临时覆盖）
    let user_custom_prompt = if profile_prompt.is_none() {
        if request.system_prompt.is_some() {
            tracing::warn!(
                conversation_id = %conversation_id,
                "AgentProfile 未产生有效提示词（role/expert 均为空），降级到 request.system_prompt"
            );
        }
        request.system_prompt.clone()
    } else {
        None
    };

    // Pre-generate a placeholder assistant message ID for streaming events.
    // The actual DB message is created after the turn completes, at which point
    // we emit an "agent-message-id" event so the frontend can remap the
    // placeholder to the real ID. This ensures streaming events always carry
    // a non-empty assistantMessageId that the frontend can use for correlation.
    let streaming_message_id = format!("stream_{}", uuid::Uuid::new_v4());

    // Check if agent is already running for this conversation.
    // Insert into running_agents and create the RAII guard atomically
    // (within the same lock scope) to prevent a race where another
    // agent_query could slip in between the insert and guard creation.
    let mut _guard = Some({
        let mut running = app_state.running_agents.write().await;
        if running.contains(&conversation_id) {
            return Err(ErrorResponse::new(agent_err::RUNNING).into());
        }
        running.insert(conversation_id.clone());

        // 安全网：超时自动清理 running_agents，防止 panic 导致 guard 未正确 drop
        {
            let cid = conversation_id.clone();
            let running = app_state.running_agents.clone();
            let cancel = app_state.agent_cancel_tokens.clone();
            let paused = app_state.agent_paused.clone();
            let pause_states = app_state.agent_pause_states.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
                // 10 分钟后如果还在 running_agents 中，说明 guard 未能正常 drop（panic 等）
                let mut agents = running.write().await;
                if agents.remove(&cid) {
                    tracing::warn!(
                        "agent_query: timeout cleanup removed stale running_agents entry for {}",
                        cid
                    );
                    cancel.remove(&cid);
                    let mut p = paused.lock().await;
                    p.remove(&cid);
                    pause_states.remove(&cid);
                }
            });
        }

        AsyncRunningAgentGuard {
            conversation_id: conversation_id.clone(),
            running_agents: app_state.running_agents.clone(),
            cancel_tokens: app_state.agent_cancel_tokens.clone(),
            paused_set: app_state.agent_paused.clone(),
            pause_states: app_state.agent_pause_states.clone(),
        }
    });

    // Set workflow_status to "running" for workflow-type sessions
    if conversation.session_type == "workflow" {
        if let Err(e) = axagent_dao::repo::conversation::update_conversation(
            app_state.harness.db(),
            &conversation_id,
            axagent_harness::types::UpdateConversationInput {
                workflow_status: Some(Some("running".to_string())),
                ..Default::default()
            },
        )
        .await
        {
            tracing::warn!("工作流会话状态更新为 running 失败 id={}: {}", conversation_id, e);
        }
    }

    // Get settings from database（提前加载，供 Smart Router 门控 + 后续 proxy 解析复用）
    let settings =
        axagent_dao::repo::settings::get_settings(app_state.harness.db()).await.unwrap_or_default();

    // Smart Router：按任务复杂度自动改写 provider/model。
    //
    // 在 `get_provider` 之前改写 `request.provider_id` / `request.model_id`，
    // 后续凭据（get_active_key/decrypt）与 adapter 解析链会自动用新 provider 重跑，
    // 天然完成"切换 provider 时凭据/adapter 重解析"，无需额外处理。
    // 关闭开关或映射表为空时完全不介入，保持用户手选的 provider/model。
    if settings.smart_router_enabled {
        use axagent_harness::route_bridge::ModelTierResolver;
        let resolver = crate::smart_router::AppModelTierResolver::new(
            settings.smart_router_tier_mappings.clone(),
        );
        if !resolver.is_empty() {
            let decision = app_state.cost_aware_router.route(&request.input);
            let tier = decision.tier.as_str();
            if let Some(mapping) = resolver.resolve(tier).await {
                info!(
                    "[agent_query] SmartRouter: tier='{}' ({}) → provider='{}' model='{}'",
                    tier, decision.reason, mapping.provider_id, mapping.model_id
                );
                if !mapping.provider_id.is_empty() {
                    request.provider_id = mapping.provider_id;
                }
                if !mapping.model_id.is_empty() {
                    request.model_id = mapping.model_id;
                }
            } else {
                info!("[agent_query] SmartRouter: tier='{}' 无映射，保持原 provider/model", tier);
            }
        }
    }

    info!("[agent_query] Got provider: {}", request.provider_id);

    // Get provider
    let prov = provider::get_provider(app_state.harness.db(), &request.provider_id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;
    info!("[agent_query] Got provider keys count: {}", prov.keys.len());

    // model_id 占位符解析：认知编排降级路径 / 前端未选模型时 model_id 为 "default" 或空，
    // 若原样作为 ChatRequest.model 发送，网关报 400 "Missing required field: model"，
    // 流式回复中断 → 前端消息停在 partial（"主动停止"）。
    // 解析顺序：settings.default_model_id（须属于该 provider）→ 该 provider 模型列表第一个。
    if request.model_id.is_empty() || request.model_id == "default" {
        let resolved = prov
            .models
            .iter()
            .find(|m| Some(m.model_id.as_str()) == settings.default_model_id.as_deref())
            .or_else(|| prov.models.first())
            .map(|m| m.model_id.clone());
        match resolved {
            Some(real_model) => {
                info!(
                    "[agent_query] model_id 占位 '{}' → 解析为 '{}' (provider {})",
                    request.model_id, real_model, prov.id
                );
                request.model_id = real_model;
            },
            None => {
                return Err(String::from(
                    crate::commands::error::ErrorResponse::from_error_with_code(
                        axagent_harness::error_codes::provider::MODEL_NOT_FOUND,
                        format!(
                            "provider {} 无可用模型（model_id 占位 '{}' 无法解析）",
                            prov.id, request.model_id
                        ),
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    )
                    .with_param("provider_id", &prov.id)
                    .with_param("model_id", &request.model_id),
                ));
            },
        }
    }

    // Get active key (使用 DAO 层的 round-robin 轮询逻辑)
    let key = provider::get_active_key(app_state.harness.db(), &request.provider_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    info!("[agent_query] Found active key (rotation_index={})", key.rotation_index);

    // Decrypt key
    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, app_state.harness.master_key())
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    info!("[agent_query] Decrypted API key");

    // Create provider context
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
            &prov.proxy_config,
            &settings,
        ),
        custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // Get model info for param overrides
    let resolved_model = axagent_dao::repo::provider::get_model(
        app_state.harness.db(),
        &request.provider_id,
        &request.model_id,
    )
    .await
    .ok();
    let model_param_overrides = resolved_model.as_ref().and_then(|m| m.param_overrides.clone());
    let use_max_completion_tokens =
        model_param_overrides.as_ref().and_then(|p| p.use_max_completion_tokens);
    let thinking_param_style =
        model_param_overrides.as_ref().and_then(|p| p.thinking_param_style.clone());
    let request_delay_ms = model_param_overrides.as_ref().and_then(|p| p.request_delay_ms);

    // 当模型不在 DB 中时，从模型名推断关键参数，确保 o-series 等模型正确使用 max_completion_tokens
    let use_max_completion_tokens = use_max_completion_tokens.or_else(|| {
        let model_lower = request.model_id.to_lowercase();
        if model_lower.contains("o1-") || model_lower.contains("o3-") || model_lower.contains("o4-")
        {
            tracing::info!(
                "[agent_query] Model '{}' not in DB, inferring use_max_completion_tokens=true",
                request.model_id
            );
            Some(true)
        } else {
            None
        }
    });

    let thinking_param_style = thinking_param_style.or_else(|| {
        let model_lower = request.model_id.to_lowercase();
        // 检测可能的 thinking 模型（如 DeepSeek R1 系列）
        if model_lower.contains("deepseek-r1") || model_lower.contains("deepseek-reasoner") {
            Some("deepseek".to_string())
        } else if model_lower.contains("claude") && model_lower.contains("3.5") {
            Some("anthropic".to_string())
        } else {
            None
        }
    });

    // Resolve effective model parameters: request options → model overrides → defaults
    let effective_temperature =
        request.options.as_ref().and_then(|o| o.temperature).or_else(|| {
            model_param_overrides.as_ref().and_then(|p| p.temperature.map(|v| v as f64))
        });
    let effective_top_p = request
        .options
        .as_ref()
        .and_then(|o| o.top_p)
        .or_else(|| model_param_overrides.as_ref().and_then(|p| p.top_p.map(|v| v as f64)));
    let effective_max_tokens = request
        .options
        .as_ref()
        .and_then(|o| o.max_tokens)
        .or_else(|| model_param_overrides.as_ref().and_then(|p| p.max_tokens));

    // Create provider adapter instance via RuntimeHarness（支持 tool_adaptation 包裹）
    let adapter: Arc<dyn ProviderAdapter> =
        app_state.harness.get_adapter_for_provider(&prov).await.ok_or_else(|| {
            format!("No adapter available for provider type {:?}", prov.provider_type)
        })?;

    // 构建 fallback 适配器：加载其他已启用的提供商作为备用
    let adapter = {
        let all_providers =
            provider::list_providers(app_state.harness.db()).await.unwrap_or_default();
        let mut fallback_adapters: Vec<Arc<dyn ProviderAdapter>> = Vec::new();
        let mut fallback_contexts: Vec<ProviderRequestContext> = Vec::new();
        for fb_prov in &all_providers {
            if fb_prov.id == prov.id || !fb_prov.enabled {
                continue;
            }
            if let Some(fb_key) = fb_prov.keys.iter().find(|k| k.enabled) {
                if let Ok(fb_api_key) = axagent_crypto::decrypt_key(
                    &fb_key.key_encrypted,
                    app_state.harness.master_key(),
                ) {
                    let fb_base_url =
                        resolve_base_url_for_type(&fb_prov.api_host, &fb_prov.provider_type);
                    let fb_ctx = ProviderRequestContext {
                        api_key: fb_api_key,
                        key_id: fb_key.id.clone(),
                        provider_id: fb_prov.id.clone(),
                        base_url: Some(fb_base_url),
                        api_path: fb_prov.api_path.clone(),
                        proxy_config:
                            axagent_harness::types::provider_model::resolve_provider_proxy(
                                &fb_prov.proxy_config,
                                &settings,
                            ),
                        custom_headers: fb_prov
                            .custom_headers
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok()),
                        api_mode: None,
                        conversation: None,
                        previous_response_id: None,
                        store_response: None,
                    };
                    if let Some(fb_adapter) =
                        app_state.harness.get_adapter_for_provider(fb_prov).await
                    {
                        fallback_adapters.push(fb_adapter);
                        fallback_contexts.push(fb_ctx);
                        tracing::info!(
                            "[agent_query] Registered fallback provider: {} ({:?})",
                            fb_prov.id,
                            fb_prov.provider_type
                        );
                    }
                }
            }
        }
        if fallback_adapters.is_empty() {
            adapter
        } else {
            tracing::info!(
                "[agent_query] FallbackAdapter created with {} fallback(s)",
                fallback_adapters.len()
            );
            Arc::new(FallbackProviderAdapter::new(adapter, fallback_adapters, fallback_contexts))
        }
    };

    // ── Agent 作用域标识（能力加载状态的多 Agent 隔离维度）──
    // 画像 + 专家共同决定「这是哪个 Agent」：同一会话里不同执行载体加载的能力
    // 必须分开记账，否则子 Agent 加载的技能会串进主 Agent 的上下文。
    // 二者皆空时回落为 harness 的 DEFAULT_AGENT_ID（单 Agent 场景）。
    let agent_scope_id: String = match (
        request.agent_profile_id.as_deref().filter(|s| !s.trim().is_empty()),
        request.expert_id.as_deref().filter(|s| !s.trim().is_empty()),
    ) {
        (Some(profile), Some(expert)) => format!("{profile}/{expert}"),
        (Some(profile), None) => profile.to_string(),
        (None, Some(expert)) => expert.to_string(),
        (None, None) => axagent_harness::DEFAULT_AGENT_ID.to_string(),
    };

    // ── DIAG 快照：认知编排 vs 直连的全链路诊断 ──
    let di_execution_mode = request.execution_mode.clone();
    let di_mcp_count = request.enabled_mcp_server_ids.as_ref().map(|v| v.len()).unwrap_or(0);
    let di_profile_id = request.agent_profile_id.clone();
    let di_disabled_tools_count = request
        .options
        .as_ref()
        .and_then(|o| o.disabled_tools.as_ref())
        .map(|v| v.len())
        .unwrap_or(0);
    tracing::info!(
        "[agent_query] 🧩 DIAG SNAPSHOT: execution_mode={:?} mcp_ids={} profile_id={:?} dis_tools={}",
        di_execution_mode,
        di_mcp_count,
        di_profile_id,
        di_disabled_tools_count,
    );

    // Load MCP tools for enabled servers (same logic as Q&A mode)
    // 认知编排执行阶段（execution_mode=Some）跳过 MCP 加载 — 与 built-in/skill/tauri 逻辑一致
    let mcp_ids: Vec<String> = if request.execution_mode.is_some() {
        info!("[agent] execution_mode={:?} — 跳过 MCP server 工具加载", request.execution_mode);
        Vec::new()
    } else {
        request.enabled_mcp_server_ids.clone().unwrap_or_default()
    };
    let mut tool_registry = UnifiedToolRegistry::new();
    let mut chat_tools: Vec<ChatTool> = Vec::new();

    // Load enabled state for the unified tool registry
    tool_registry.load_enabled_state(app_state.harness.db()).await;

    // Build all_server_ids from remote MCP servers only (no builtin)
    let all_server_ids: Vec<String> =
        mcp_ids.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

    info!("[agent] all_server_ids (remote MCP only): {:?}", all_server_ids);

    // Phase 1: 并发加载所有 MCP 服务器配置和工具描述
    let db = app_state.harness.db();
    struct ServerTools {
        server: McpServer,
        chat_tools: Vec<ChatTool>,
        tool_descriptors: Vec<(String, Option<String>, Option<Value>)>, // (name, description, params)
    }

    let load_futures: Vec<_> = all_server_ids
        .iter()
        .map(|server_id| {
            let db = db.clone();
            let app_handle = app.clone();
            let conv_id = conversation_id.clone();
            let sid = server_id.clone();
            async move {
                let server = match axagent_dao::repo::mcp_server::get_mcp_server(&db, &sid).await {
                    Ok(s) => s,
                    Err(e) => {
                        info!("[agent] Failed to load MCP server '{}': {}", sid, e);
                        let _ = app_handle.emit(
                            "agent-mcp-load-failed",
                            serde_json::json!({
                                "conversationId": conv_id,
                                "serverId": sid,
                                "error": e.to_string(),
                            }),
                        );
                        return None;
                    },
                };

                let mut chat_tools = Vec::new();
                let mut tool_descriptors = Vec::new();
                if let Ok(descriptors) =
                    axagent_dao::repo::mcp_server::list_tools_for_server(&db, &sid).await
                {
                    for td in descriptors {
                        let parameters: Option<Value> = td
                            .input_schema_json
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok());
                        chat_tools.push(ChatTool {
                            r#type: "function".to_string(),
                            function: ChatToolFunction {
                                name: td.name.clone(),
                                description: td.description.clone(),
                                parameters: parameters.clone(),
                            },
                        });
                        tool_descriptors.push((td.name, td.description, parameters));
                    }
                }
                Some(ServerTools { server, chat_tools, tool_descriptors })
            }
        })
        .collect();

    let server_tools_list = futures::future::join_all(load_futures).await;

    // Phase 2: 合并结果到 chat_tools 和 tool_registry（纯内存操作）
    for st in server_tools_list.into_iter().flatten() {
        for chat_tool in st.chat_tools {
            chat_tools.push(chat_tool);
        }
        for (i, (name, desc, params)) in st.tool_descriptors.into_iter().enumerate() {
            let _ = i;
            tool_registry = tool_registry.register_mcp_tool(
                st.server.id.clone(),
                st.server.name.clone(),
                name,
                desc,
                params,
                McpServerConfig {
                    server_id: st.server.id.clone(),
                    server_name: st.server.name.clone(),
                    transport: st.server.transport.clone(),
                    command: st.server.command.clone(),
                    args_json: st.server.args_json.clone(),
                    env_json: st.server.env_json.clone(),
                    endpoint: st.server.endpoint.clone(),
                    execute_timeout_secs: st.server.execute_timeout_secs,
                    connection_pool_size: None,
                    retry_attempts: None,
                    retry_delay_ms: None,
                },
            );
        }
    }

    // ── 注入 axagent-tools 统一工具到 chat_tools（按功能域过滤）─
    let disabled_set: HashSet<String> = request
        .options
        .as_ref()
        .and_then(|o| o.disabled_tools.as_ref())
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();

    // 解析活跃功能域（三源合并：前端显式 > 角色/专家组合 > 默认）
    let active_domains: std::collections::HashSet<ToolDomain> = if let Some(ref domains) =
        request.options.as_ref().and_then(|o| o.active_domains.as_ref()).filter(|v| !v.is_empty())
    {
        // ① 前端显式传入
        domains.iter().filter_map(|s| parse_domain_str(s)).collect()
    } else if !profile_active_domains.is_empty() {
        // ② 角色/专家组合（role.active_domains ∪ expert.active_domains）
        // 确保 General 始终存在（历史 Core 已并入 General）
        let mut d = profile_active_domains;
        d.insert(ToolDomain::General);
        d
    } else {
        // ③ 默认（自由对话无任何关联）
        let mut d = std::collections::HashSet::new();
        d.insert(ToolDomain::General);
        d
    };

    // 认知编排决策后的执行（execution_mode=Ask/Act/Delegate/Plan）：
    // 跳过全量 built-in tools 收集 — tool 调用应由能力发现路径按需注入
    // 只有直连 agent（execution_mode=None）才全量塞工具
    let unified_chat_tools: Vec<ChatTool> = if request.execution_mode.is_some() {
        info!(
            "[agent] execution_mode={:?} — 认知编排执行阶段仅放行渐进式披露工具 ({} domains)",
            request.execution_mode,
            active_domains.len()
        );
        // 按名字点名放行，不能按域放行：披露工具挂在 General 域下，
        // 而该域含 Bash / FileWrite / FileEdit / DeleteFile 等写操作工具。
        tool_registry
            .get_chat_tools_by_names(DISCLOSURE_TOOLS.iter().copied())
            .into_iter()
            .filter(|t| !disabled_set.contains(&t.function.name))
            .collect()
    } else {
        tool_registry
            .get_chat_tools_for_domains(&active_domains, None)
            .into_iter()
            .filter(|t| !disabled_set.contains(&t.function.name))
            .collect()
    };
    // 同步注册表的屏蔽列表
    if !disabled_set.is_empty() {
        tool_registry = tool_registry.with_blocked_tools(disabled_set.into_iter().collect());
    }
    let domain_names: Vec<String> = active_domains.iter().map(|d| d.as_str().to_string()).collect();
    info!(
        "[agent] UnifiedToolRegistry provides {} tools to LLM (domains: [{}], {} disabled)",
        unified_chat_tools.len(),
        domain_names.join(", "),
        request
            .options
            .as_ref()
            .and_then(|o| o.disabled_tools.as_ref())
            .map(|v| v.len())
            .unwrap_or(0)
    );
    // 去重：local_tools 已经包含统一工具，避免 DeepSeek 等 API 报 Tool names must be unique
    let existing_names: std::collections::HashSet<String> =
        chat_tools.iter().map(|t| t.function.name.clone()).collect();
    for t in unified_chat_tools {
        if !existing_names.contains(&t.function.name) {
            chat_tools.push(t);
        }
    }

    // Load enabled skills content for system prompt injection
    // 认知编排执行阶段（execution_mode=Some）跳过 — skills 注入 system prompt 会撑爆 context（曾导致 5MB+）
    // 认知编排的能力发现走路由层，不走 skills 注入
    let skill_contents: Vec<(String, String)> = if request.execution_mode.is_some() {
        info!(
            "[agent] execution_mode={:?} — 认知编排执行阶段跳过 skill_contents 加载 (system prompt 注入)",
            request.execution_mode
        );
        Vec::new()
    } else {
        load_enabled_skill_contents(
            &app_state,
            conversation_scenario.as_deref(),
            &enabled_skill_ids,
        )
        .await
    };

    // Convert enabled skills to ChatTool definitions for Agent to call
    // 被动模式（execution_mode=None）：按会话启用技能加载（全量）
    // 主动模式（execution_mode=Some）：仅按认知编排命中的技能（extra_skills）按需加载，
    // 解决"主动模式技能不可用"的遗留边界①——技能需注册 handler 才能执行，
    // 不能仅注入 schema（否则 LLM 调用 skill_xxx 会 404）。
    let extra_skills: Vec<String> = request.extra_skills.clone().unwrap_or_default();
    let (skill_tools, skill_map) = if request.execution_mode.is_some() {
        if extra_skills.is_empty() {
            (Vec::new(), Default::default())
        } else {
            // load_skill_tools 的 enabled_skill_ids 语义即"指定技能名集合"，
            // 主动模式传技能名列表即可按名精确加载（scenario=None 不做场景过滤）
            load_skill_tools(&app_state, None, &extra_skills).await
        }
    } else {
        load_skill_tools(&app_state, conversation_scenario.as_deref(), &enabled_skill_ids).await
    };
    let skill_tools_count = skill_tools.len();
    if !skill_tools.is_empty() {
        let existing_names: std::collections::HashSet<String> =
            chat_tools.iter().map(|t| t.function.name.clone()).collect();
        for t in skill_tools {
            if !existing_names.contains(&t.function.name) {
                chat_tools.push(t);
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    chat_tools.retain(|t| seen.insert(t.function.name.clone()));

    info!(
        "[agent] chat_tools registered: {}, tool_registry MCP tools: {:?}",
        chat_tools.len(),
        tool_registry.list_tools()
    );

    // Configure tool execution recorder and context
    let mut tool_registry = tool_registry
        .with_recorder_from_db(app_state.harness.db())
        .with_execution_context(conversation_id.clone(), None);

    // ── 加载搜索提供商配置，注入到 tool_extra ──
    // 优先使用请求中指定的 search_provider_id，否则取第一个已启用的提供商
    let search_provider_used = if let Some(ref sp_id) = request.search_provider_id {
        search_provider::get_search_provider(app_state.harness.db(), sp_id).await.ok()
    } else {
        search_provider::list_search_providers(app_state.harness.db())
            .await
            .ok()
            .and_then(|providers| providers.into_iter().find(|p| p.enabled))
    };
    if let Some(ref sp) = search_provider_used {
        let api_key = axagent_entities::search_providers::Entity::find_by_id(&sp.id)
            .one(app_state.harness.db())
            .await
            .ok()
            .flatten()
            .and_then(|e| e.api_key_ref)
            .and_then(|enc| {
                match axagent_crypto::decrypt_key(&enc, app_state.harness.master_key()) {
                    Ok(key) => Some(key),
                    Err(e) => {
                        tracing::warn!("[agent] 搜索 API key 解密失败: {}", e);
                        None
                    },
                }
            })
            .unwrap_or_default();
        tool_registry = tool_registry
            .with_tool_extra(context_keys::SEARCH_PROVIDER_TYPE, &sp.provider_type)
            .with_tool_extra(context_keys::SEARCH_MAX_RESULTS, sp.result_limit.to_string())
            .with_tool_extra(context_keys::SEARCH_TIMEOUT_MS, sp.timeout_ms.to_string());
        if let Some(ref endpoint) = sp.endpoint {
            tool_registry =
                tool_registry.with_tool_extra(context_keys::SEARCH_ENDPOINT, endpoint.as_str());
        }
        if !api_key.is_empty() {
            tool_registry = tool_registry.with_tool_extra(context_keys::SEARCH_API_KEY, &api_key);
        }
        if let Some(ref region) = sp.region {
            tool_registry =
                tool_registry.with_tool_extra(context_keys::SEARCH_REGION, region.as_str());
        }
        if let Some(safe_search) = sp.safe_search {
            tool_registry = tool_registry.with_tool_extra(
                context_keys::SEARCH_SAFE_SEARCH,
                if safe_search { "1" } else { "0" },
            );
        }
        info!("[agent] Search provider configured: type={}, id={}", sp.provider_type, sp.id);
    } else {
        info!("[agent] No search provider configured — WebSearch will fall back to DDG");
    }

    // ── 注入 vault_kb_id 到 tool_extra，修复 Obsidian 集成链路断裂 ──
    // obsidian_* 工具通过 ToolContext.extra["vault_kb_id"] 取 KB ID，再从 VaultRegistry 取 vault。
    // 此前缺少注入，导致所有 obsidian_* 工具调用时报 NotBound 错误。
    // 优先从 request.enabled_knowledge_base_ids 筛选 ConnectedVault KB；
    // 若未指定，回退到第一个启用的 ConnectedVault KB（仅当全局唯一时自动绑定）。
    match axagent_dao::repo::knowledge::list_knowledge_bases(app_state.harness.db()).await {
        Ok(all_kbs) => {
            let vault_kbs: Vec<_> = all_kbs
                .iter()
                .filter(|kb| {
                    kb.enabled
                        && matches!(kb.kind, axagent_harness::KbKind::ConnectedVault)
                        && kb.vault_path.is_some()
                })
                .collect();

            if !vault_kbs.is_empty() {
                let enabled_ids = request.enabled_knowledge_base_ids.as_deref().unwrap_or(&[]);
                let target = if !enabled_ids.is_empty() {
                    // 从显式启用的 KB 列表中筛选 ConnectedVault
                    vault_kbs.iter().find(|kb| enabled_ids.iter().any(|id| id == &kb.id)).copied()
                } else if vault_kbs.len() == 1 {
                    // 未指定启用列表：仅当全局唯一时自动绑定
                    vault_kbs.first().copied()
                } else {
                    None
                };

                if let Some(kb) = target {
                    tool_registry = tool_registry.with_tool_extra("vault_kb_id", kb.id.as_str());
                    if let Some(vault_path) = kb.vault_path.as_ref() {
                        info!(
                            "[agent] Obsidian vault bound: kb_id={} name={} vault={}",
                            kb.id, kb.name, vault_path
                        );
                    }
                } else if vault_kbs.len() > 1 {
                    info!(
                        "[agent] 发现 {} 个 ConnectedVault KB 但未在 enabled_knowledge_base_ids 中指定，obsidian_* 工具暂不绑定",
                        vault_kbs.len()
                    );
                }
            }
        },
        Err(e) => {
            tracing::warn!("[agent] 查询 ConnectedVault KB 失败，obsidian_* 工具将不绑定: {}", e);
        },
    }

    // Register skill tool handlers in tool_registry for execution.
    // skill handler 以 "content" 模式返回结果供 LLM 处理，MCP 工具调用由 LLM 层直接走 tool_registry。
    if skill_tools_count > 0 {
        let skill_ctx = SkillExecutionContext::new(
            app.clone(),
            &app_state,
            adapter.clone(),
            ctx.key_id.clone(),
            ctx.api_key.clone(),
            conversation_id.clone(),
            streaming_message_id.clone(),
        );
        for (tool_name, skill) in &skill_map {
            let skill_name = skill.name.clone();
            let skill_id = skill.id.clone();
            let skill_content = skill.content.clone();
            let ctx = skill_ctx.clone();
            tool_registry.register_skill_tool(
                tool_name.clone(),
                Box::new(move |input: &str| {
                    execute_skill_sync(&skill_id, &skill_name, &skill_content, input, &ctx)
                        .map_err(axagent_harness::ToolError::new)
                }),
            );
        }
        info!("[agent] Added {} skill tools to chat_tools", skill_tools_count);
        info!("[agent] Registered {} skill tool handlers", skill_map.len());
    }

    // ── Tauri 命令桥接器：将现有 Tauri 命令注册为 Agent 可调用的工具 ──
    // 认知编排执行阶段（execution_mode=Some）同样跳过 — tool 调用应由能力发现路径注入
    if request.execution_mode.is_none() {
        let tauri_tools = build_tauri_command_chat_tools();
        let tauri_tool_count = tauri_tools.len();
        chat_tools.extend(tauri_tools);

        let handlers = build_command_handlers(app_state.harness.db().clone(), app.clone());
        let handler_count = handlers.len();
        for (tool_name, handler) in handlers {
            tool_registry.register_skill_tool(tool_name, handler);
        }
        info!("[agent] Added {} Tauri command tools to chat_tools", tauri_tool_count);
        info!("[agent] Registered {} Tauri command handlers", handler_count);
    } else {
        info!("[agent] execution_mode={:?} — 跳过 Tauri 命令桥接工具", request.execution_mode);
    }

    // Create API client with tool definitions, model ID and parameters
    // Also attach a streaming callback to emit text/thinking deltas in real-time
    // 语义缓存注入：仅当运行时开关开启时，把共享的 SemanticCache 作为 HarnessCache
    // 拦截器注入主聊天路径。命中相同请求即短路省一次 LLM 调用；关闭则完全直通。
    let llm_cache: Option<Arc<dyn axagent_harness::cache_interceptor::HarnessCache>> = {
        let cache_state = app_state.semantic_cache.lock().await;
        if cache_state.enabled {
            let arc: Arc<dyn axagent_harness::cache_interceptor::HarnessCache> =
                cache_state.cache.clone();
            Some(arc)
        } else {
            None
        }
    };

    // Clone adapter & ctx for ToT pre-processing (build_streaming_api_client consumes originals)
    let tot_adapter = adapter.clone();
    let tot_ctx = ctx.clone();

    // ── 运行时动态工具集（能力按需加载闭环 P0-4）──
    // 三方共享同一份：tool_registry 透传给 CapabilityLoad 写入、
    // api_client 每次请求前合并下发、runtime 每轮取快照放进 ApiRequest。
    // 每会话一份，会话结束随 Arc 释放，不存在跨会话串扰。
    let dynamic_tools = axagent_harness::DynamicToolSet::new();

    // ── 工具微调：extra_tools / blocked_tools ──
    // 来源：AgentProfile.recommended_tools（额外追加）+ disallowed_tools（排除）
    //      + 认知编排按需注入的 extra_tools（Phase 1.5 暴露闭环：
    //        主动模式 execution_mode=Some 下，命中能力的真实工具定义凭此注入，
    //        解决此前"主动模式工具列表为空、发现的能力执行不了"的执行断链）
    //
    // ⚠️ 位置强约束：必须位于 `build_streaming_api_client` **之前**。
    // 该函数按值接收 `chat_tools.clone()` 作为快照，此后对 `chat_tools` 的任何
    // 增删都不会反映到下发 LLM 的工具列表里——本块曾放在持久化附件之后（约 1556 行），
    // 导致整块策略（profile 禁用 / 推荐 + 编排注入）静默失效，且因用的是
    // `push`/`retain` 而非赋值，`unused_assignments` 也报不出来。改成本函数返回新
    // Vec 的写法后，编译器立刻暴露了该问题。挪动此处前请先看这条注释。
    let orchestration_tools: Vec<String> = request.extra_tools.clone().unwrap_or_default();
    // 屏幕感知门控（可见性侧）：关闭时不把 ComputerUse 的 schema 下发给 LLM。
    // 仅靠后段 `tool_registry.tools.disable(..)` 的执行期拦截是不够的 —— LLM 仍会在工具列表
    // 里看到它并尝试调用，每次都撞上 disabled 而失败，白耗一轮 tool call。
    // 执行期拦截保留，两侧不可互相替代：可见性挡的是「看到」，执行期挡的是「调成」。
    let screen_perception_off = !settings.screen_perception_enabled;
    if !profile_recommended_tools.is_empty()
        || !profile_disallowed_tools.is_empty()
        || !orchestration_tools.is_empty()
        || screen_perception_off
    {
        let mut extra_names: HashSet<String> = profile_recommended_tools.into_iter().collect();
        extra_names.extend(orchestration_tools);
        let mut blocked_names: HashSet<String> = profile_disallowed_tools.into_iter().collect();
        if screen_perception_off {
            blocked_names.insert(SCREEN_PERCEPTION_TOOL.to_string());
        }
        // extra: 按名字从注册表取完整 schema（复用 registry 的统一实现）
        let extra_schemas =
            tool_registry.get_chat_tools_by_names(extra_names.iter().map(String::as_str));
        // 顺序语义（先追加后剔除）封装在 `apply_tool_policy` 内，勿在此处拆开改写。
        chat_tools = apply_tool_policy(chat_tools, extra_schemas, &blocked_names);
    }

    let api_client = build_streaming_api_client(
        adapter,
        ctx,
        chat_tools.clone(),
        dynamic_tools.clone(),
        &request.model_id,
        effective_temperature,
        effective_top_p,
        effective_max_tokens,
        request.thinking_budget,
        use_max_completion_tokens,
        thinking_param_style,
        request_delay_ms,
        llm_cache,
        conversation_id.clone(),
        streaming_message_id.clone(),
        app.clone(),
    );

    // Persist attachments (images, files) to disk and DB
    let persisted_attachments: Vec<Attachment> = if let Some(ref attachments) = request.attachments
    {
        if attachments.is_empty() {
            Vec::new()
        } else {
            crate::commands::conversations::persist_attachments(
                &app_state,
                &conversation_id,
                attachments,
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
        }
    } else {
        Vec::new()
    };

    // Build data: URLs for image attachments so the LLM can see them
    let image_urls: Vec<String> = persisted_attachments
        .iter()
        .filter(|a| a.file_type.starts_with("image/"))
        .filter_map(|a| {
            let file_store = axagent_storage::file_store::FileStore::new();
            if a.file_path.is_empty() {
                // Use inline data if available
                a.data.as_ref().map(|d| format!("data:{};base64,{}", a.file_type, d))
            } else {
                // Read from storage and encode
                file_store.read_file(&a.file_path).ok().map(|data| {
                    format!(
                        "data:{};base64,{}",
                        a.file_type,
                        base64::engine::general_purpose::STANDARD.encode(data)
                    )
                })
            }
        })
        .collect();

    // Persist user message to DB (with attachments)
    let _user_message = message::create_message(
        app_state.harness.db(),
        &conversation_id,
        MessageRole::User,
        &request.input,
        &persisted_attachments,
        None,
        0,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // Increment the persisted message count
    axagent_dao::repo::conversation::increment_message_count(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // Use the long-lived SessionManager from AppState (persists sessions across queries)
    let session_manager = &app_state.agent_session_manager;
    // Ensure app_handle is set (idempotent if already set)
    session_manager.set_app_handle(app.clone()).await;
    session_manager.set_default_workspace_dir(settings.default_workspace_dir.clone()).await;
    info!(
        "[agent_query] Using AppState SessionManager, has_app_handle: {}",
        session_manager.has_app_handle().await
    );

    // Get or create session (reuse existing session to preserve conversation history)
    let session = session_manager
        .get_or_create_session(prov.id.clone(), conversation_id.clone())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // P0-3 跨进程上下文重建：进程重启 / Session 被 LRU 驱逐后内存会话为空，
    // 从 DB 消息历史回灌（含已完成 turn 的 tool_use / tool_result 观察块）。
    // 仅对全新空会话生效（seed 内部幂等）；尾部 user 行由 seed 截断，
    // 避免本 turn 的输入（上方 L1533 已落库）被计入两次。
    {
        const SEED_HISTORY_MAX_MESSAGES: usize = 40;
        let history_rows =
            axagent_dao::repo::message::list_messages(app_state.harness.db(), &conversation_id)
                .await
                .unwrap_or_default();
        let mut history: Vec<axagent_harness::ConversationMessage> = history_rows
            .iter()
            .flat_map(|m| {
                axagent_harness::conversation_model::history_to_conversation_messages(
                    m.role,
                    &m.content,
                    m.parts.as_deref(),
                )
            })
            .collect();
        let total = history.len();
        if total > SEED_HISTORY_MAX_MESSAGES {
            history.drain(..total - SEED_HISTORY_MAX_MESSAGES);
            // 截断不能留下孤立的 tool 观察行（配对的 tool_use 已在前段被丢弃）
            while matches!(
                history.first().map(|m| m.role),
                Some(axagent_harness::conversation_model::MessageRole::Tool)
            ) {
                history.remove(0);
            }
        }
        let seeded = session_manager.seed_session_history(&conversation_id, history).await;
        if seeded {
            info!(
                "[agent_query] Restored cross-process session context for conversation: {}",
                conversation_id
            );
        }
    }

    // RAG retrieval: search enabled knowledge bases and memory namespaces
    let kb_ids = request.enabled_knowledge_base_ids.clone().unwrap_or_default();
    // Auto-inherit memory namespace IDs from conversation settings if not explicitly provided
    let mem_ids = if request.enabled_memory_namespace_ids.is_some() {
        request.enabled_memory_namespace_ids.clone().unwrap_or_default()
    } else {
        // Fallback: load enabled memory namespaces from the conversation's settings
        match axagent_dao::repo::conversation::get_conversation(
            app_state.harness.db(),
            &conversation_id,
        )
        .await
        {
            Ok(conv) => conv.enabled_memory_namespace_ids,
            Err(_) => Vec::new(),
        }
    };
    let wiki_ids = request.enabled_wiki_ids.clone().unwrap_or_default();
    let rag_result = crate::indexing::collect_rag_context(
        app_state.harness.db(),
        app_state.harness.master_key(),
        &app_state.vector_store,
        &kb_ids,
        &mem_ids,
        &wiki_ids,
        &request.input,
        5,
        &app_state.credential_manager,
    )
    .await;

    // Emit RAG results to frontend
    let _ = app.emit(
        "rag-context-retrieved",
        axagent_harness::types::RagContextRetrievedEvent {
            conversation_id: conversation_id.clone(),
            sources: rag_result.source_results,
        },
    );

    // Build system prompt with custom persona, RAG context, tool awareness, skill contents, and working memory
    let rag_context_parts = if rag_result.context_parts.is_empty() {
        None
    } else {
        Some(rag_result.context_parts)
    };
    // Format working memory from MemoryService
    let working_memory_text = {
        let ms = app_state.memory_service.read().await;
        let wm = ms.format_for_prompt().await;
        if wm.is_empty() { None } else { Some(wm) }
    };

    // Generate nudge messages from NudgeService (skill creation reminders, memory save suggestions, etc.)
    let nudge_messages: Vec<String> = {
        let mut ns = app_state.nudge_service.lock().await;
        let pending = ns.get_pending_nudges(&conversation_id);
        let messages: Vec<String> = pending
            .iter()
            .map(|n| {
                let action_suffix = match &n.suggested_action {
                    Some(a) => format!(" Suggested action: {}", a),
                    None => String::new(),
                };
                format!(
                    "- [{}] {} ({}).{}",
                    match n.urgency {
                        axagent_trajectory::Urgency::High => "HIGH",
                        axagent_trajectory::Urgency::Medium => "MED",
                        axagent_trajectory::Urgency::Low => "LOW",
                    },
                    n.reason,
                    n.entity_name,
                    action_suffix
                )
            })
            .collect();

        // Mark nudges as presented since they'll be injected into the prompt
        let nudge_ids: Vec<String> = pending.iter().map(|n| n.id.clone()).collect();
        for id in nudge_ids {
            ns.mark_nudge_presented(&id);
        }

        messages
    };
    let nudge_ref: Vec<String> = if nudge_messages.is_empty() {
        Vec::new()
    } else {
        nudge_messages.clone()
    };

    // P3: Generate insight messages from LearningInsightSystem for prompt injection
    let insight_messages: Vec<String> = {
        let is = app_state.insight_system.read().await;
        let insights = is.get_insights();
        insights
            .iter()
            .take(5)
            .map(|i| {
                let action_suffix = match &i.suggested_action {
                    Some(a) => format!(" Suggested: {}", a),
                    None => String::new(),
                };
                format!(
                    "- [{}] {} (confidence: {:.0}%).{}",
                    match i.category {
                        axagent_trajectory::InsightCategory::Pattern => "PATTERN",
                        axagent_trajectory::InsightCategory::Preference => "PREF",
                        axagent_trajectory::InsightCategory::Improvement => "IMPROVE",
                        axagent_trajectory::InsightCategory::Warning => "WARN",
                    },
                    i.title,
                    i.confidence * 100.0,
                    action_suffix
                )
            })
            .collect()
    };

    // P5: Generate pattern messages from PatternLearner for prompt injection
    let pattern_messages: Vec<String> = {
        let pl = app_state.pattern_learner.read().await;
        let high_value = pl.get_high_value_patterns(0.5);
        let all_patterns = pl.get_patterns_by_type(axagent_trajectory::PatternType::ToolSequence);
        let failure_patterns: Vec<_> = all_patterns
            .iter()
            .filter(|p| p.success_rate < 0.4 && p.frequency >= 2)
            .take(3)
            .collect();
        let mut msgs = Vec::new();
        // High-value success patterns
        for p in high_value.iter().take(5) {
            msgs.push(format!(
                "- [SUCCESS] {} ({:.0}% success, {} uses): {}",
                p.name,
                p.success_rate * 100.0,
                p.frequency,
                p.description
            ));
        }
        // Failure patterns to avoid
        for p in &failure_patterns {
            msgs.push(format!(
                "- [AVOID] {} ({:.0}% success, {} uses): {}",
                p.name,
                p.success_rate * 100.0,
                p.frequency,
                p.description
            ));
        }
        msgs
    };

    // P8: Format user profile and adaptation hint for system prompt injection
    let user_profile_text = {
        let profile = app_state.user_profile.read().await;
        let text = profile.format_for_prompt();
        if text.is_empty() { None } else { Some(text) }
    };
    let adaptation_hint_text = {
        let mut rl = app_state.realtime_learning.lock().await;
        let adaptation = rl.compute_adaptation();
        let mut hint = String::new();
        if let Some(ref style) = adaptation.response_style {
            let mut parts = Vec::new();
            if let Some(ref v) = style.verbosity {
                match v {
                    axagent_trajectory::Verbosity::Shorter => {
                        parts.push("Use shorter, more concise responses")
                    },
                    axagent_trajectory::Verbosity::Longer => {
                        parts.push("Provide more detailed explanations")
                    },
                    _ => {},
                }
            }
            if let Some(ref t) = style.technical_level {
                match t {
                    axagent_trajectory::TechnicalLevel::Simpler => {
                        parts.push("Use simpler language and concepts")
                    },
                    axagent_trajectory::TechnicalLevel::MoreDetailed => {
                        parts.push("Use more technical depth")
                    },
                    _ => {},
                }
            }
            if let Some(ref f) = style.format {
                match f {
                    axagent_trajectory::ContentFormat::List => {
                        parts.push("Prefer list/bullet format")
                    },
                    axagent_trajectory::ContentFormat::Paragraph => {
                        parts.push("Prefer paragraph format")
                    },
                    axagent_trajectory::ContentFormat::Code => {
                        parts.push("Prefer code-first responses")
                    },
                    _ => {},
                }
            }
            if !parts.is_empty() {
                hint = format!("Based on recent interactions: {}.", parts.join("; "));
            }
        }
        if let Some(ref adjustments) = adaptation.content_adjustments {
            if !adjustments.is_empty() {
                if !hint.is_empty() {
                    hint.push(' ');
                }
                hint.push_str(&format!("Additional adjustments: {}", adjustments.join("; ")));
            }
        }
        if hint.is_empty() { None } else { Some(hint) }
    };

    // Retrieve workspace root from agent session DB record before building system prompt
    let db_session = axagent_dao::repo::agent_session::get_agent_session_by_conversation_id(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .ok()
    .flatten();
    let workspace_root_for_prompt = db_session.as_ref().and_then(|s| s.cwd.clone());
    info!(
        "[agent_query] workspace_root_for_prompt from DB: {:?} (session exists: {}, will inject into system_prompt: {})",
        workspace_root_for_prompt,
        db_session.is_some(),
        workspace_root_for_prompt.as_ref().is_some_and(|c| !c.is_empty()),
    );

    // 将 workspace cwd 注入工具注册表，确保工具执行时使用正确的工作目录
    if let Some(ref cwd) = workspace_root_for_prompt {
        if !cwd.is_empty() {
            tool_registry = tool_registry.with_working_dir(cwd.as_str());
            info!("[agent_query] Tool registry working_dir set to: {}", cwd);
        }
    }

    let app_language = axagent_dao::repo::settings::get_settings(app_state.harness.db())
        .await
        .ok()
        .map(|s| s.language);

    let system_prompt = build_agent_system_prompt(
        persona_prompt.as_deref(),
        profile_prompt.as_deref(),
        user_custom_prompt.as_deref(),
        rag_context_parts.as_deref(),
        &skill_contents,
        working_memory_text.as_deref(),
        // nudge_messages 通过 runtime.set_nudge_lines 在每次 LLM 调用前动态注入，此处传 None 避免重复
        None,
        if insight_messages.is_empty() {
            None
        } else {
            Some(&insight_messages)
        },
        if pattern_messages.is_empty() {
            None
        } else {
            Some(&pattern_messages)
        },
        user_profile_text.as_deref(),
        adaptation_hint_text.as_deref(),
        workspace_root_for_prompt.as_deref(),
        app_language.as_deref(),
        {
            let mut q = app_state.steer_queue.lock().await;
            let instructions = q.remove(&conversation_id).unwrap_or_default();
            drop(q);
            if instructions.is_empty() {
                None
            } else {
                let formatted: String = instructions
                    .iter()
                    .enumerate()
                    .map(|(i, inst)| format!("- [steer-{}] {}", i, inst))
                    .collect::<Vec<_>>()
                    .join("\n");
                info!(
                    "[agent_query] Injecting {} steer instruction(s) for conversationId={}",
                    instructions.len(),
                    conversation_id
                );
                Some(formatted)
            }
        },
        request.agent_context.as_ref(),
    );

    // ── 注入 Tauri 命令索引到系统提示 ──
    // 使用配置化的领域映射解析可见命令域
    let command_domains: Vec<axagent_harness::CapabilityDomain> = {
        // 将 ToolDomain 集合转换为字符串集合
        let tool_domain_set: std::collections::HashSet<String> =
            active_domains.iter().map(|d| d.as_str().to_string()).collect();
        resolve_command_domains(&tool_domain_set)
    };

    // 使用 CommandRegistry 和 CommandCache 构建索引
    // 首次调用预加载缓存
    let (_preloaded_index, hit_rate) = preload_command_cache(&command_domains);
    let command_registry = CommandRegistry::default();
    let mut command_cache = CommandCache::default();
    let command_index = command_cache.get(&command_domains, &command_registry);

    // 验证 build_command_index_string 函数
    let _verified_index = build_command_index_string(&command_domains);

    let (hits, misses, cache_size) = command_cache.stats();
    info!(
        "[agent] Command index injected: {} domains, {} chars (cache: {} hits, {} misses, {} entries, preload_hit_rate: {:.1}%)",
        command_domains.len(),
        command_index.len(),
        hits,
        misses,
        cache_size,
        hit_rate * 100.0
    );

    // 将命令索引追加到系统提示中
    // 认知编排执行阶段跳过 — 已跳过 Tauri 命令桥接 tools（见 1305 行），命令索引无意义且撑爆 context
    let mut system_prompt = system_prompt;
    if request.execution_mode.is_none() {
        system_prompt
            .push(format!("<tauri-command-index>\n{}\n</tauri-command-index>", command_index));
    } else {
        info!(
            "[agent] execution_mode={:?} — 跳过 tauri-command-index 注入 (认知编排模式下无 Tauri tools)",
            request.execution_mode
        );
    }

    // ── 注入能力目录（渐进式披露 L0 索引层）──
    // 主动（认知编排执行）与被动模式口径一致：都只注入「所有可发现能力」的轻量摘要目录，
    // 完整定义（入参 schema / SOP / 前置条件）由 LLM 自行调 CapabilityView 按需展开。
    // 改造前主动模式此处完全空白 —— LLM 既看不到目录也无从选择，能力信息全由路由器代劳。
    let capability_passports = app_state.capability_indexer.list_passports().await;
    let capability_index = capability_index::build_capability_index_string(
        &capability_passports,
        capability_index::CAPABILITY_INDEX_TOKEN_BUDGET,
    );
    info!(
        "[agent] capability-index injected: {} passports, {} tokens (execution_mode: {:?})",
        capability_passports.len(),
        axagent_harness::util_fns::estimate_tokens(&capability_index),
        request.execution_mode
    );
    system_prompt.push(format!(
        "<capability-index data-axagent=\"1\">\n{capability_index}\n</capability-index>"
    ));

    // ── 注入认知编排模式（可选）──
    // 当 agent 由认知编排器（Ask/Act/Delegate）触发时，透传当前编排模式使 agent 运行时
    // 感知高层决策上下文；直连 agent（非认知编排）时缺省为 None，不注入此段。
    if let Some(ref mode) = request.execution_mode {
        if !mode.is_empty() {
            system_prompt.push(format!(
                "<cognitive-mode>\nYour current execution mode is: {mode}\n</cognitive-mode>"
            ));
        }
    }

    // Attach image URLs to the API client for multimodal support
    let api_client = api_client.with_image_urls(image_urls);

    // Resolve permission mode from the agent session DB record (db_session fetched above)
    let permission_mode_str = db_session
        .as_ref()
        .map(|s| s.permission_mode.clone())
        .unwrap_or_else(|| "default".to_string());
    let mut runtime_permission_mode = match permission_mode_str.as_str() {
        "full_access" => axagent_runtime::PermissionMode::Allow,
        "accept_edits" => axagent_runtime::PermissionMode::WorkspaceWrite,
        "default" => axagent_runtime::PermissionMode::Prompt,
        _ => axagent_runtime::PermissionMode::Prompt,
    };
    info!(
        "[agent_query] Permission mode: {} -> {:?}",
        permission_mode_str, runtime_permission_mode
    );

    // P2: 按任务形态决策的「安全隔离需求」覆盖会话级权限模式（只降级不升级）。
    // 当 UNITY_P0_TASK_SHAPE flag 关闭或分类失败时 task_shape 为 None，本块无操作。
    // 严格级别：ReadOnly > Prompt > WorkspaceWrite > DangerFullAccess > Allow
    // 设计原则：用户最严格配置不可被覆盖（Prompt 不降级为更宽松），只可能降级。
    if let Some(ts) = &request.task_shape {
        use axagent_harness::task_shape::resolve_effective_permission;
        let effective =
            resolve_effective_permission(runtime_permission_mode, Some(ts.isolation_need));
        if effective != runtime_permission_mode {
            info!(
                "[agent_query] Permission overridden by task_shape: {:?} -> {:?} (isolation={:?})",
                runtime_permission_mode, effective, ts.isolation_need
            );
            runtime_permission_mode = effective;
        }
    }

    // Get always-allowed tools for this conversation
    let always_allowed = app_state
        .agent_always_allowed
        .lock()
        .await
        .get(&conversation_id)
        .cloned()
        .unwrap_or_default();

    // Get workspace root from agent session for permission boundary checks
    let workspace_root = db_session.as_ref().and_then(|s| s.cwd.clone()).unwrap_or_default();

    // Create ChannelPermissionPrompter for interactive permission approval
    let prompter = axagent_agent::ChannelPermissionPrompter::new(
        app.clone(),
        conversation_id.clone(),
        always_allowed,
        workspace_root,
    );

    // Register the prompter in AppState so agent_approve can find it
    {
        let mut prompters = app_state.agent_prompters.lock().await;
        prompters.insert(conversation_id.clone(), prompter.clone());
    }

    // Check token budget before expensive LLM call
    let estimated_input_tokens = axagent_kit::token_counter::estimate_tokens(&request.input) as u64;
    if let Err(budget_err) = check_token_budget(estimated_input_tokens) {
        tracing::warn!("[agent_query] Token budget check failed: {}", budget_err);
        // Emit error to frontend
        let _ = app.emit(
            "agent-error",
            AgentErrorPayload {
                conversation_id: conversation_id.clone(),
                assistant_message_id: None,
                message: budget_err.clone(),
            },
        );
        return Err(budget_err);
    }

    // Run turn via SessionManager (handles pre-compaction, runtime creation,
    // post-compaction, and session persistence)
    let session_id = session.session().session_id.clone();
    info!("[agent_query] About to run_turn_with_tools for session: {}", session_id);
    emit_status(
        &app,
        &conversation_id,
        "running",
        "正在调用模型...",
        Some(agent_status_err::CALLING_MODEL),
    );

    // Create and register a cancel token for this agent run
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    app_state.agent_cancel_tokens.insert(conversation_id.clone(), cancel_token.clone());

    // Drain steer queue and inject instructions into the prompt
    let mut augmented_input = {
        let mut queue = app_state.steer_queue.lock().await;
        if let Some(instructions) = queue.remove(&conversation_id) {
            if instructions.is_empty() {
                request.input.clone()
            } else {
                info!(
                    "[agent_query] Injecting {} steer instruction(s) for conversationId={}",
                    instructions.len(),
                    conversation_id
                );
                emit_status(
                    &app,
                    &conversation_id,
                    "steer_applied",
                    &format!("已应用 {} 条引导指令", instructions.len()),
                    Some(agent_status_err::STEER_APPLIED),
                );
                format!(
                    "{}\n[系统提示：用户发送了以下引导指令，请在后续操作中遵循这些指引]\n{}",
                    request.input,
                    instructions
                        .iter()
                        .enumerate()
                        .map(|(i, instr)| format!("{}. {}", i + 1, instr))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        } else {
            request.input.clone()
        }
    };

    // P4: Save input for trajectory recording (request.input is moved below)
    let trajectory_input = request.input.clone();

    // 注入 AskUserBridge 到工具注册表，使 AskUserQuestionTool 能够阻塞等待用户回复
    tool_registry.ask_user_bridge = Some(Arc::new(AppAskUserBridge {
        app_handle: app.clone(),
        ask_senders: app_state.agent_ask_senders.clone(),
        conversation_id: conversation_id.clone(),
        assistant_message_id: streaming_message_id.clone(),
    }));

    // 屏幕感知（computer_use 工具）受 settings.screen_perception_enabled 门控。
    // 关闭时禁用该工具，用户无法触发桌面控制 / 截图能力。
    // 注意：UnifiedToolRegistry::clone 不复制 disabled 集合，故必须在最终传入
    // runtime 之前（create_conversation_runtime 之前）禁用，避免被后续 clone 清空。
    //
    // ⚠️ 本处是**执行期**拦截，管不到「已下发给 LLM 的工具列表」——那个列表在
    // `build_streaming_api_client` 处就已按值快照（约 1458 行），早于此处。故可见性侧的
    // 过滤放在前段工具策略块（并入 blocked_names），两处共用 `SCREEN_PERCEPTION_TOOL`。
    // 只保留本处会导致「LLM 看得到却调不动」，只保留前段则挡不住手工构造的调用。
    if !settings.screen_perception_enabled {
        tool_registry.tools.disable(SCREEN_PERCEPTION_TOOL);
    }

    // Tree of Thoughts 多路径推理预处理：仅当用户开启时生效。
    // ToT 对用户输入生成多条推理路径 → 评估 → 剪枝 → 选最佳路径，
    // 将精选分析注入 augmented_input 的 <tot_analysis> 块，供 LLM 在回答时参考。
    // 开销：~4-6 次额外 LLM 调用（branching=2, depth=2, threshold=0.3）。
    if settings.tot_enabled {
        let tot_provider: Arc<dyn axagent_agent::LlmReasoningProvider> =
            Arc::new(DefaultToTReasoningProvider::from_provider_adapter(
                tot_adapter,
                tot_ctx,
                request.model_id.clone(),
            ));
        let mut tot_engine = TreeOfThoughtsEngine::new(2, 2, 0.3);
        match tot_engine.solve(&augmented_input, &tot_provider).await {
            Ok(tot_analysis) if !tot_analysis.is_empty() => {
                tracing::info!(
                    "[agent_query] ToT analysis complete: {} chars injected",
                    tot_analysis.len()
                );
                augmented_input = format!(
                    "<tot_analysis>\n{}\n</tot_analysis>\n\n{}",
                    tot_analysis, augmented_input,
                );
            },
            Ok(_) => tracing::debug!("[agent_query] ToT returned empty analysis — skipping"),
            Err(e) => tracing::warn!("[agent_query] ToT solve failed: {} — falling through", e),
        }
    }

    // Build runtime via factory, then delegate to session_manager.
    // This keeps axagent-runtime-core out of the agent crate's dependencies.
    // RuntimeFeatureConfig 承载 error_recovery / thought_chain 等开关，
    // 由 AppSettings 驱动（幽灵开关真正生效的接入口）。
    let runtime_feature_config = axagent_runtime_core::RuntimeFeatureConfig::default()
        .with_error_recovery(settings.error_recovery_enabled)
        .with_thought_chain(settings.thought_chain_enabled);

    // P0-3：暂停桥接。创建共享 PauseState 并注册到 AppState，
    // agent_pause/agent_resume 命令通过它挂起/唤醒 runtime 循环（wait_while_paused）。
    let pause_state = Arc::new(axagent_runtime_core::PauseState::new());
    app_state.agent_pause_states.insert(conversation_id.clone(), pause_state.clone());

    // ── 技能侧反思钩子（自我进化通道二：能力偏弱进化改进）注入 ──
    // 实现 harness `SkillEvolutionHook` 契约：工具执行完成（`skill_` 前缀）后即时
    // 固化成败证据、贝叶斯判定；命中则 spawn 走用户同意通道生成技能进化提议，同意后执行。
    // 与周期扫描（start_skill_evolution）互补，使即时弱技能能被及时发现并进化。
    let skill_evolution_hook: Arc<dyn axagent_harness::SkillEvolutionHook> =
        Arc::new(crate::commands::evolution_hook::SkillEvolutionHookImpl {
            app: app.clone(),
            trajectory_storage: app_state.trajectory_storage.clone(),
            skill_evolution_engine: app_state.skill_evolution_engine.clone(),
            evolution_consent_senders: app_state.evolution_consent_senders.clone(),
            constitution: app_state.constitution.clone(),
        });

    // 会话状态闭环：tool_registry 拿到动态工具集后透传给 ToolContext，
    // CapabilityLoad 才能把工具定义写进去；agent_id 同理，用于状态按 Agent 分键。
    let tool_registry = tool_registry
        .with_dynamic_tools(dynamic_tools.clone())
        .with_agent_id(agent_scope_id.clone());

    // 上下文注入器：每轮 LLM 调用前读会话状态，把已加载能力的完整定义注入系统提示。
    // 这是「写入（CapabilityLoad）→ 读取（本注入器）」的读取侧，两者经 SessionState 解耦。
    let capability_indexer_trait: Arc<dyn axagent_harness::CapabilityIndexer> =
        app_state.capability_indexer.clone();
    let loaded_capability_contributor =
        axagent_agent::context_contributors::LoadedCapabilityContributor::new(
            app_state.session_state_store.clone(),
            capability_indexer_trait,
        );

    let mut runtime = create_conversation_runtime(
        ConversationRuntimeFactoryArgs::new(
            session.session().clone(),
            Box::new(api_client),
            Box::new(tool_registry),
            PermissionPolicy::new(runtime_permission_mode).with_permission_rules_from_lists(
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            system_prompt,
            runtime_feature_config,
        )
        .with_hook_chain_option(crate::commands::multi_agent::get_global_hook_chain())
        .with_pause_state(pause_state)
        .with_skill_evolution_hook(skill_evolution_hook)
        .with_dynamic_tools(dynamic_tools)
        .with_conversation_id(conversation_id.clone())
        .with_agent_id(agent_scope_id.clone())
        .with_context_contributor(Box::new(loaded_capability_contributor)),
    );

    // 将 nudge 注入到运行时级 system_prompt（通过 <memory_context> 块在每次 LLM 调用前注入）
    // 此路径替代了静态 build_agent_system_prompt 中的 <nudge-suggestions> 注入，避免重复
    // 受 settings.proactive_nudge_enabled 门控（此前该幽灵开关从未被后端读取）
    if settings.proactive_nudge_enabled && !nudge_ref.is_empty() {
        runtime.set_nudge_lines(nudge_ref);
    }

    let result: Result<
        (axagent_runtime::TurnSummary, axagent_runtime::Session),
        axagent_runtime::RuntimeError,
    > = session_manager
        .run_turn_with_tools(
            &session_id,
            augmented_input,
            runtime,
            conversation_id.clone(),
            Some(cancel_token),
            app_state.agent_prompters.clone(),
        )
        .await;
    info!("[agent_query] run_turn_with_tools completed");

    // Clean up cancel token
    app_state.agent_cancel_tokens.remove(&conversation_id);

    // Eagerly and synchronously remove from running_agents to close the
    // race window where a second agent_query could slip in before the
    // RAII guard's tokio::spawn runs.  Consume the guard via Option::take()
    // so its Drop doesn't double-remove.
    {
        let mut running = app_state.running_agents.write().await;
        running.remove(&conversation_id);
    }
    _guard.take();

    // Persist the updated always-allowed set back to AppState
    {
        let updated_always = prompter.get_always_allowed();
        let mut always_map = app_state.agent_always_allowed.lock().await;
        always_map.insert(conversation_id.clone(), updated_always);
    }

    // Remove the prompter from AppState now that the turn is complete.
    // Clear any pending permission requests first to avoid leaking blocked
    // threads that would otherwise wait for the 5-minute timeout.
    {
        prompter.clear_pending();
        let mut prompters = app_state.agent_prompters.lock().await;
        prompters.remove(&conversation_id);
    }

    // Clean up paused state in case the agent was paused but the turn
    // completed (e.g. via cancel while paused).
    {
        let mut paused = app_state.agent_paused.lock().await;
        paused.remove(&conversation_id);
        app_state.agent_pause_states.remove(&conversation_id);
    }

    match result {
        Ok((summary, _updated_session)) => {
            // Extract text from all assistant message blocks
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

            // Serialize structured content blocks as parts JSON
            // 统一经 `types::ContentBlock`（tagged，camelCase 字段）序列化，
            // 保证 DB parts 可被 `history_to_conversation_messages` 原样解析回。
            // assistant(tool_use) 与 tool(tool_result) 观察都落库——
            // P0-3 跨进程上下文重建依赖观察数据。
            let parts_json = {
                let all_blocks: Vec<serde_json::Value> = summary
                    .assistant_messages
                    .iter()
                    .chain(summary.tool_results.iter())
                    .flat_map(|msg| &msg.blocks)
                    .map(|block| {
                        serde_json::to_value(axagent_harness::types::ContentBlock::from(
                            block.clone(),
                        ))
                        .unwrap_or(serde_json::Value::Null)
                    })
                    .filter(|value| !value.is_null())
                    .collect();
                if all_blocks.is_empty() {
                    None
                } else {
                    serde_json::to_string(&all_blocks).ok()
                }
            };

            // Create assistant message in DB
            let assistant_message = message::create_message_with_parts(
                app_state.harness.db(),
                &conversation_id,
                MessageRole::Assistant,
                &text,
                &[],
                None,
                0,
                parts_json.as_deref(),
                None,
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

            // Update token usage stats on the assistant message
            if let Err(e) = message::update_message_usage(
                app_state.harness.db(),
                &assistant_message.id,
                Some(summary.usage.input_tokens as i64),
                Some(summary.usage.output_tokens as i64),
                Some(summary.usage.cache_creation_input_tokens as i64),
                Some(summary.usage.cache_read_input_tokens as i64),
            )
            .await
            {
                tracing::warn!("Failed to update message usage: {}", e);
            }

            // Persist thinking content to the message record
            if !summary.thinking.is_empty() {
                if let Err(e) = message::update_message_thinking(
                    app_state.harness.db(),
                    &assistant_message.id,
                    Some(&summary.thinking),
                )
                .await
                {
                    tracing::warn!("Failed to update message thinking: {}", e);
                }
            }

            // Emit agent-message-id event so the frontend can remap the
            // streaming placeholder ID to the real DB message ID.
            let _ = app.emit(
                "agent-message-id",
                serde_json::json!({
                    "conversationId": conversation_id,
                    "streamingMessageId": streaming_message_id,
                    "assistantMessageId": assistant_message.id,
                }),
            );

            // Emit agent-done event
            let cost_usd = estimate_cost_usd(
                &request.model_id,
                summary.usage.input_tokens as u64,
                summary.usage.output_tokens as u64,
                resolved_model.as_ref().and_then(|m| m.input_price_per_mtok),
                resolved_model.as_ref().and_then(|m| m.output_price_per_mtok),
            );

            // Persist cost to agent_sessions table for dashboard display.
            // run_turn_with_tools saves tokens but hardcodes cost_delta=0.0;
            // we persist the real cost here now that we have pricing info.
            if let (Some(axagent_session_id), Some(real_cost)) =
                (session.axagent_session_id(), cost_usd)
            {
                if let Err(e) = axagent_dao::repo::agent_session::update_agent_session_after_query(
                    app_state.harness.db(),
                    axagent_session_id,
                    "idle",
                    None,
                    0,         // tokens already saved by session_manager
                    real_cost, // real cost delta
                )
                .await
                {
                    tracing::warn!("Agent session 成本持久化失败 id={}: {}", axagent_session_id, e);
                }
            }

            let blocks: Vec<AgentContentBlock> = summary
                .assistant_messages
                .iter()
                .flat_map(|msg| &msg.blocks)
                .map(|block| match block {
                    axagent_runtime::ContentBlock::Text { text } => AgentContentBlock {
                        block_type: "text".to_string(),
                        text: Some(text.clone()),
                        id: None,
                        name: None,
                        input: None,
                        tool_use_id: None,
                        tool_name: None,
                        output: None,
                        is_error: None,
                    },
                    axagent_runtime::ContentBlock::ToolUse { id, name, input } => {
                        AgentContentBlock {
                            block_type: "tool_use".to_string(),
                            id: Some(id.clone()),
                            name: Some(name.clone()),
                            input: Some(input.clone()),
                            text: None,
                            tool_use_id: None,
                            tool_name: None,
                            output: None,
                            is_error: None,
                        }
                    },
                    axagent_runtime::ContentBlock::ToolResult {
                        tool_use_id,
                        tool_name,
                        output,
                        is_error,
                    } => AgentContentBlock {
                        block_type: "tool_result".to_string(),
                        tool_use_id: Some(tool_use_id.clone()),
                        tool_name: Some(tool_name.clone()),
                        output: Some(output.clone()),
                        is_error: Some(*is_error),
                        text: None,
                        id: None,
                        name: None,
                        input: None,
                    },
                })
                .collect();
            let blocks_opt = if blocks.is_empty() {
                None
            } else {
                Some(blocks)
            };

            let payload = AgentDonePayload {
                conversation_id: conversation_id.clone(),
                assistant_message_id: assistant_message.id.clone(),
                text,
                thinking: if summary.thinking.is_empty() {
                    None
                } else {
                    Some(summary.thinking)
                },
                usage: Some(AgentUsagePayload {
                    input_tokens: summary.usage.input_tokens as u64,
                    output_tokens: summary.usage.output_tokens as u64,
                }),
                num_turns: Some(summary.iterations as u32),
                cost_usd,
                blocks: blocks_opt,
            };
            let _ = app.emit("agent-done", &payload);

            // Set workflow_status to "completed" for workflow-type sessions
            if conversation.session_type == "workflow" {
                if let Err(e) = axagent_dao::repo::conversation::update_conversation(
                    app_state.harness.db(),
                    &conversation_id,
                    axagent_harness::types::UpdateConversationInput {
                        workflow_status: Some(Some("completed".to_string())),
                        ..Default::default()
                    },
                )
                .await
                {
                    tracing::warn!(
                        "工作流会话状态更新为 completed 失败 id={}: {}",
                        conversation_id,
                        e
                    );
                }
            }

            // P4: Record trajectory for closed-loop learning
            // Build a Trajectory from the turn summary and save to TrajectoryStorage.
            // This is the critical data pipeline that feeds ClosedLoopService.tick().
            {
                let storage = &app_state.trajectory_storage;
                let now = chrono::Utc::now();
                let start_time =
                    now - chrono::Duration::milliseconds(summary.usage.output_tokens as i64 * 10);

                // Build trajectory steps from the turn
                let mut steps = Vec::new();

                // User message step
                steps.push(axagent_trajectory::TrajectoryStep {
                    timestamp_ms: start_time.timestamp_millis() as u64,
                    role: axagent_trajectory::MessageRole::User,
                    content: trajectory_input.clone(),
                    reasoning: None,
                    tool_calls: None,
                    tool_results: None,
                });

                // Assistant message step(s)
                for msg in &summary.assistant_messages {
                    let mut content_parts = Vec::new();
                    let mut tool_calls_vec: Vec<axagent_trajectory::ToolCall> = Vec::new();
                    let mut tool_results_vec: Vec<axagent_trajectory::TrajectoryToolResult> =
                        Vec::new();

                    for block in &msg.blocks {
                        match block {
                            axagent_runtime::ContentBlock::Text { text: t } => {
                                content_parts.push(t.clone());
                            },
                            axagent_runtime::ContentBlock::ToolUse { id, name, input } => {
                                tool_calls_vec.push(axagent_trajectory::ToolCall {
                                    id: id.clone(),
                                    name: name.clone(),
                                    arguments: input.to_string(),
                                });
                            },
                            axagent_runtime::ContentBlock::ToolResult {
                                tool_use_id,
                                tool_name,
                                output: result_content,
                                is_error,
                            } => {
                                tool_results_vec.push(axagent_trajectory::TrajectoryToolResult {
                                    tool_use_id: tool_use_id.clone(),
                                    tool_name: tool_name.clone(),
                                    output: result_content.clone(),
                                    is_error: *is_error,
                                });
                            },
                        }
                    }

                    steps.push(axagent_trajectory::TrajectoryStep {
                        timestamp_ms: now.timestamp_millis() as u64,
                        role: axagent_trajectory::MessageRole::Assistant,
                        content: content_parts.join("\n"),
                        reasoning: None,
                        tool_calls: if tool_calls_vec.is_empty() {
                            None
                        } else {
                            Some(tool_calls_vec)
                        },
                        tool_results: if tool_results_vec.is_empty() {
                            None
                        } else {
                            Some(tool_results_vec)
                        },
                    });
                }

                // P4-Mirror: MemoryFlush tool → 同步到 MemoryService（FTS5 可搜索）
                for msg in &summary.assistant_messages {
                    for block in &msg.blocks {
                        if let axagent_runtime::ContentBlock::ToolResult {
                            tool_name,
                            output: result_content,
                            is_error,
                            ..
                        } = block
                        {
                            if !is_error && tool_name == "MemoryFlush" && !result_content.is_empty()
                            {
                                let mem = app_state.memory_service.write().await;
                                let result = mem.add_memory("agent", result_content).await;
                                if result.success {
                                    tracing::debug!(
                                        "[P4-Mirror] Mirrored MemoryFlush result to MemoryService"
                                    );
                                } else {
                                    tracing::warn!(
                                        "MemoryFlush 同步到 MemoryService 失败: {}",
                                        result.message
                                    );
                                }
                            }
                        }
                    }
                }

                // Determine outcome based on tool results
                let has_errors = steps.iter().any(|s| {
                    s.tool_results
                        .as_ref()
                        .is_some_and(|results| results.iter().any(|r| r.is_error))
                });
                let outcome = if has_errors {
                    axagent_trajectory::TrajectoryOutcome::Partial
                } else {
                    axagent_trajectory::TrajectoryOutcome::Success
                };

                // Build and save trajectory
                let mut trajectory = axagent_trajectory::Trajectory::new(
                    conversation_id.clone(),
                    "default_user".to_string(),
                    trajectory_input[..trajectory_input.len().min(100)].to_string(),
                    trajectory_input[..trajectory_input.len().min(200)].to_string(),
                    outcome,
                    (now.timestamp_millis() - start_time.timestamp_millis()).max(0) as u64,
                    steps,
                );
                // 结构化 agent 标识：记录该轨迹由哪个 Agent 执行（AgentProfile 名称）
                if let Some(name) = &profile_agent_name {
                    trajectory.agent_name = Some(name.clone());
                }

                // ★ P2-3: Scorecard 真实评分流程 — 先计算质量分
                axagent_harness::trajectory_scorer::TrajectoryScorer::apply(&mut trajectory);
                // 此时 trajectory.quality 和 trajectory.value_score 已由 scorer 计算完毕

                // P6: Inject known patterns into trajectory for reward computation
                {
                    let pl = app_state.pattern_learner.read().await;
                    let high_value = pl.get_high_value_patterns(0.3);
                    for p in &high_value {
                        trajectory.patterns.push(p.id.clone());
                    }
                }

                if let Err(e) = storage.save_trajectory(&trajectory).await {
                    tracing::warn!("[P4] Failed to save trajectory: {}", e);
                } else {
                    tracing::debug!(
                        "[P4] Saved trajectory {} with {} steps, outcome={:?}",
                        &trajectory.id[..trajectory.id.len().min(12)],
                        trajectory.steps.len(),
                        outcome
                    );

                    // ★ P2-3: Scorecard 三档门禁评估（真实接入评分流程）
                    {
                        let quality = trajectory.quality.clone();
                        let estimated_safety = 0.85; // 安全分可后续对接注入检测结果
                        let gate_result =
                            axagent_harness::trajectory_scorer::TrajectoryScorer::evaluate_gate(
                                quality,
                                estimated_safety,
                                axagent_harness::trajectory_scorer::GateLevel::Soft,
                            );
                        tracing::info!(
                            "[Scorecard] Gate {:?}: passed={}, quality={:.3}, safety={:.3}, detail={}",
                            gate_result.level,
                            gate_result.passed,
                            gate_result.quality_score,
                            gate_result.safety_score,
                            gate_result.detail
                        );

                        // ★ P2-4: 创建防篡改证据链 (SHA-256 哈希)
                        let evidence_summary = format!(
                            "质量分={:.3}, 安全分={:.3}, 效率分={:.3}, 通过={}",
                            gate_result.quality_score,
                            gate_result.safety_score,
                            gate_result.efficiency_score,
                            gate_result.passed
                        );
                        let evidence = axagent_harness::workflow_types::ReviewEvidenceIdentity::new(
                            "scorecard_gate",
                            &trajectory.id,
                            "conversation",
                            evidence_summary,
                        );
                        tracing::info!(
                            "[Evidence] Created evidence {} with SHA-256 hash={}...",
                            evidence.id,
                            &evidence.evidence_hash[..16]
                        );
                    }

                    // P5: Real-time pattern learning — learn from this trajectory immediately
                    {
                        let mut pl = app_state.pattern_learner.write().await;
                        let new_patterns = pl.learn_from_trajectory(&trajectory);
                        if !new_patterns.is_empty() {
                            tracing::debug!(
                                "[P5] Learned {} patterns from trajectory",
                                new_patterns.len()
                            );
                            // Persist newly discovered patterns
                            for pattern in &new_patterns {
                                if let Err(e) = storage.save_pattern(pattern).await {
                                    tracing::warn!("[P5] Failed to persist pattern: {}", e);
                                }
                            }
                        }
                    }

                    // P6: Real-time RL reward computation for this trajectory
                    {
                        let rl = app_state.rl_engine.read().await;
                        let mut traj_for_rl = trajectory.clone();
                        let rewards = rl.compute_rewards(&mut traj_for_rl).await;
                        if !rewards.is_empty() {
                            let total_reward: f64 = rewards.iter().map(|r| r.value).sum();
                            tracing::debug!(
                                "[P6] Computed {} rewards for trajectory, total={:.3}",
                                rewards.len(),
                                total_reward
                            );
                            // Update value_score based on reward
                            let mut updated = trajectory.clone();
                            updated.rewards = rewards;
                            updated.value_score = (updated.value_score + total_reward) / 2.0;
                            if let Err(e) = storage.save_trajectory(&updated).await {
                                tracing::warn!("Failed to save trajectory: {}", e);
                            }
                        }
                    }

                    // P4-Skill: Analyze trajectory and propose new skills if applicable
                    {
                        let mut proposal_service = app_state.skill_proposal_service.write().await;
                        if let Some(proposal) = proposal_service.analyze_and_propose(&trajectory) {
                            tracing::info!(
                                "[P4-Skill] Proposed new skill '{}' from trajectory {} (confidence={:.2})",
                                proposal.suggested_name,
                                &trajectory.id[..8],
                                proposal.confidence
                            );
                            let mut is = app_state.insight_system.write().await;
                            is.add_insight(axagent_trajectory::LearningInsight {
                                id: format!(
                                    "skill_proposal_{}",
                                    chrono::Utc::now().timestamp_millis()
                                ),
                                category: axagent_trajectory::InsightCategory::Improvement,
                                title: format!("New skill suggested: {}", proposal.suggested_name),
                                description: format!(
                                    "Task: {}. Confidence: {:.0}%",
                                    proposal.task_description,
                                    proposal.confidence * 100.0
                                ),
                                confidence: proposal.confidence,
                                evidence: vec![],
                                suggested_action: Some(format!(
                                    "Create skill '{}' to automate this workflow in the future",
                                    proposal.suggested_name
                                )),
                                created_at: chrono::Utc::now().timestamp_millis(),
                            });

                            // 推送技能提案事件到前端，触发通知面板
                            let _ = app.emit("skill-proposal", &proposal);
                        }
                    }
                }

                // P4: Auto-record feedback signal based on outcome
                {
                    let mut rl = app_state.realtime_learning.lock().await;
                    let (fb_type, fb_content) = match outcome {
                        axagent_trajectory::TrajectoryOutcome::Success => (
                            axagent_trajectory::FeedbackType::Success,
                            "Turn completed successfully".to_string(),
                        ),
                        axagent_trajectory::TrajectoryOutcome::Partial => (
                            axagent_trajectory::FeedbackType::Partial,
                            "Turn completed with some errors".to_string(),
                        ),
                        axagent_trajectory::TrajectoryOutcome::Failure => {
                            (axagent_trajectory::FeedbackType::Failure, "Turn failed".to_string())
                        },
                        axagent_trajectory::TrajectoryOutcome::Abandoned => (
                            axagent_trajectory::FeedbackType::Partial,
                            "Turn was abandoned".to_string(),
                        ),
                    };
                    rl.record_feedback(axagent_trajectory::FeedbackSignal {
                        feedback_type: fb_type,
                        source: axagent_trajectory::FeedbackSource::System,
                        content: fb_content,
                        timestamp: now.timestamp_millis(),
                        context: None,
                    });

                    // P8: Compute adaptation and update user profile
                    let adaptation = rl.compute_adaptation();
                    if let Some(ref style) = adaptation.response_style {
                        let mut profile = app_state.user_profile.write().await;
                        let verbosity =
                            style.verbosity.unwrap_or(axagent_trajectory::Verbosity::Unchanged);
                        let tech = style
                            .technical_level
                            .unwrap_or(axagent_trajectory::TechnicalLevel::Unchanged);
                        let fmt =
                            style.format.unwrap_or(axagent_trajectory::ContentFormat::Unchanged);
                        profile.update_style(verbosity, tech, fmt);
                    }
                }
            }

            // P0 修复：触发 Webhook AgentEnd 事件
            if let Some(ref emitter) = app_state.webhook_event_emitter {
                emitter.emit_agent_end(&conversation_id, "completed").await;
            }

            Ok(AgentQueryResponse {
                conversation_id,
                assistant_message_id: assistant_message.id,
                status: None,
            })
        },
        Err(e) => {
            let error_msg = e.to_string();

            // Set workflow_status to "failed" for workflow-type sessions
            if conversation.session_type == "workflow" {
                if let Err(e) = axagent_dao::repo::conversation::update_conversation(
                    app_state.harness.db(),
                    &conversation_id,
                    axagent_harness::types::UpdateConversationInput {
                        workflow_status: Some(Some("failed".to_string())),
                        ..Default::default()
                    },
                )
                .await
                {
                    tracing::warn!(
                        "工作流会话状态更新为 failed 失败 id={}: {}",
                        conversation_id,
                        e
                    );
                }
            }

            // Emit agent-error event
            let _ = app.emit(
                "agent-error",
                AgentErrorPayload {
                    conversation_id: conversation_id.clone(),
                    assistant_message_id: None,
                    message: error_msg.clone(),
                },
            );

            // P0 修复：触发 Webhook AgentError 事件
            if let Some(ref emitter) = app_state.webhook_event_emitter {
                emitter.emit_agent_error(&conversation_id, &error_msg).await;
            }

            Err(error_msg)
        },
    }
}

/// Approve or reject a pending plan (P0-2 plan confirmation gate)
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "审批或拒绝待确认的计划")]
#[tauri::command]
pub async fn agent_approve_plan(
    app_state: State<'_, AppState>,
    request: AgentApprovePlanRequest,
) -> Result<(), String> {
    info!(
        "[agent_approve_plan] conversationId={}, decision={}",
        request.conversation_id, request.decision
    );
    let approved = request.decision == "approve";
    let mut approvals = app_state.agent_plan_approvals.lock().await;
    if let Some(sender) = approvals.remove(&request.conversation_id) {
        let _ = sender.send(approved);
    } else {
        info!(
            "[agent_approve_plan] No pending plan approval for conversationId={}",
            request.conversation_id
        );
    }
    Ok(())
}

/// Approve a permission request
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "审批或拒绝工具权限请求")]
#[tauri::command]
pub async fn agent_approve(
    app_state: State<'_, AppState>,
    request: AgentApproveRequest,
) -> Result<AgentApproveResponse, String> {
    info!(
        "[agent_approve] conversationId={}, toolUseId={}, decision={}",
        request.conversation_id, request.tool_use_id, request.decision
    );

    // Convert the frontend decision string to a PermissionPromptDecision
    let decision = match request.decision.as_str() {
        "allow_once" => axagent_runtime::PermissionPromptDecision::Allow,
        "allow_always" => axagent_runtime::PermissionPromptDecision::Allow,
        "deny" => axagent_runtime::PermissionPromptDecision::Deny {
            reason: "User denied permission".to_string(),
        },
        other => axagent_runtime::PermissionPromptDecision::Deny {
            reason: format!("Unknown decision: {}", other),
        },
    };

    // Find the ChannelPermissionPrompter for this conversation and deliver the decision
    let prompters = app_state.agent_prompters.lock().await;
    if let Some(prompter) = prompters.get(&request.conversation_id) {
        let delivered = prompter.deliver_decision(&request.tool_use_id, decision);
        if !delivered {
            info!(
                "[agent_approve] No pending sender for toolUseId={}, may have already been resolved",
                request.tool_use_id
            );
        }
    } else {
        info!("[agent_approve] No active prompter for conversationId={}", request.conversation_id);
    }
    drop(prompters);

    // If "allow_always", add the tool to the always-allowed set for this conversation
    // Lock ordering: agent_prompters → agent_always_allowed (never hold both simultaneously)
    if request.decision == "allow_always" {
        // Use the tool_name (sent by frontend) as the key for always_allowed,
        // because ChannelPermissionPrompter::decide() checks by tool_name.
        // Fall back to tool_use_id if tool_name is not provided (backward compat).
        let always_key = request.tool_name.as_deref().unwrap_or(&request.tool_use_id);

        // Update the prompter's always_allowed set first (lock ordering: prompters before always_allowed)
        {
            let prompters = app_state.agent_prompters.lock().await;
            if let Some(prompter) = prompters.get(&request.conversation_id) {
                prompter.add_always_allowed(always_key);
            }
        }

        // Then update the global always_allowed map (prompters lock already dropped)
        {
            let mut always = app_state.agent_always_allowed.lock().await;
            let entry = always.entry(request.conversation_id.clone()).or_default();
            entry.insert(always_key.to_string());
        }
    }

    Ok(())
}

/// Respond to an ask request
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "响应用户提问请求")]
#[tauri::command]
pub async fn agent_respond_ask(
    app_state: State<'_, AppState>,
    request: AgentRespondAskRequest,
) -> Result<(), String> {
    info!("[agent_respond_ask] askId={}, answer length={}", request.ask_id, request.answer.len());

    // Deliver the answer through the oneshot channel
    let mut senders = app_state.agent_ask_senders.lock().await;
    if let Some(sender) = senders.remove(&request.ask_id) {
        let _ = sender.send(request.answer);
        Ok(())
    } else {
        // No pending sender found — this can happen if the ask timed out
        info!(
            "[agent_respond_ask] No pending sender for askId={}, may have already been resolved",
            request.ask_id
        );
        Ok(())
    }
}

/// Shared internal agent cancellation logic.
/// Used by both `agent_cancel` command and `delete_conversation` cleanup.
/// Performs the common cleanup steps: cancel token, prompter, paused, AskUser senders, event emit.
/// Callers that need to additionally clean `always_allowed` or `running_agents`
/// should do so after calling this function.
pub(crate) async fn cancel_agent_internal(
    app: &tauri::AppHandle,
    app_state: &AppState,
    conversation_id: &str,
    reason: &str,
) {
    // Trigger the cancel token to abort the run_turn loop.
    // Only set the flag — do NOT remove the token here.
    // The token will be cleaned up by agent_query after run_turn_with_tools
    // completes, which avoids a race where the agent loop hasn't checked
    // the flag yet but the token (and its Arc) is already gone.
    {
        let tokens = &app_state.agent_cancel_tokens;
        if let Some(token) = tokens.get(conversation_id) {
            token.store(true, std::sync::atomic::Ordering::Release);
            info!(
                "[cancel_agent_internal] Set cancel token for conversationId={}",
                conversation_id
            );
        }
    }

    // Clean up the permission prompter for this conversation.
    // Call clear_pending() first to unblock any waiting rx.recv() calls,
    // then remove from the map.
    {
        let mut prompters = app_state.agent_prompters.lock().await;
        if let Some(prompter) = prompters.get(conversation_id) {
            prompter.clear_pending();
        }
        prompters.remove(conversation_id);
    }

    // Clean up paused state — if the agent was paused when cancelled,
    // the paused entry would otherwise remain indefinitely.
    {
        let mut paused = app_state.agent_paused.lock().await;
        paused.remove(conversation_id);
    }

    // Clean up AskUser senders — if the agent was waiting for an AskUser
    // response when cancelled, the oneshot sender would leak.
    {
        let mut ask_senders = app_state.agent_ask_senders.lock().await;
        ask_senders.retain(|k, _| !k.starts_with(conversation_id));
    }

    // Emit cancellation event so frontend can clean up
    let _ = app.emit(
        "agent-cancelled",
        serde_json::json!({
            "conversationId": conversation_id,
            "reason": reason,
        }),
    );
}

/// Cancel an agent task
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "取消正在执行的智能体任务")]
#[tauri::command]
pub async fn agent_cancel(
    app: AppHandle,
    app_state: State<'_, AppState>,
    request: AgentCancelRequest,
) -> Result<AgentCancelResponse, String> {
    // Note: We intentionally do NOT remove from running_agents here.
    // The AsyncRunningAgentGuard (RAII) in agent_query is the sole owner of
    // that entry and will remove it on Drop. Removing it here would
    // create a double-remove race and break the RAII invariant.
    // The cancel token (set above) is what actually stops the agent loop;
    // running_agents is only a concurrency guard for agent_query entry.

    cancel_agent_internal(&app, app_state.inner(), &request.conversation_id, "User cancelled")
        .await;

    Ok(())
}

/// Check if an agent is currently running for a conversation.
/// Used by the frontend after page refresh to detect orphaned agent runs.
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "查询智能体是否正在运行")]
#[tauri::command]
pub async fn agent_is_running(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<bool, String> {
    let running = app_state.running_agents.read().await;
    Ok(running.contains(&conversation_id))
}

/// Pause a running agent. The agent loop checks the paused set before each iteration;
/// when paused it sleeps until resumed or cancelled.
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "暂停正在运行的智能体")]
#[tauri::command]
pub async fn agent_pause(
    app: AppHandle,
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Verify the agent is actually running and insert into paused set atomically.
    // Hold the running_agents read lock while inserting to close the TOCTOU window
    // where the agent could complete between the check and the insert.
    {
        let running = app_state.running_agents.read().await;
        if !running.contains(&conversation_id) {
            return Err(ErrorResponse::new(agent_err::NOT_RUNNING)
                .with_detail(format!("No running agent for conversation {}", conversation_id))
                .into());
        }
        let mut paused = app_state.agent_paused.lock().await;
        paused.insert(conversation_id.clone());
    }

    // P0-3：桥接 runtime 层 PauseState，唤醒/挂起实际执行循环
    if let Some(ps) = app_state.agent_pause_states.get(&conversation_id) {
        ps.pause();
    }

    info!("[agent_pause] Paused agent for conversationId={}", conversation_id);

    let _ = app.emit(
        "agent-paused",
        serde_json::json!({
            "conversationId": conversation_id,
        }),
    );

    Ok(())
}

/// Resume a paused agent.
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "恢复已暂停的智能体")]
#[tauri::command]
pub async fn agent_resume(
    app: AppHandle,
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Verify the agent is actually paused
    {
        let paused = app_state.agent_paused.lock().await;
        if !paused.contains(&conversation_id) {
            return Err(ErrorResponse::new(agent_err::NOT_PAUSED)
                .with_detail(format!("Agent for conversation {} is not paused", conversation_id))
                .into());
        }
    }

    {
        let mut paused = app_state.agent_paused.lock().await;
        paused.remove(&conversation_id);
    }

    // P0-3：唤醒 runtime 层 PauseState，解除 wait_while_paused 阻塞
    if let Some(ps) = app_state.agent_pause_states.get(&conversation_id) {
        ps.resume();
    }

    info!("[agent_resume] Resumed agent for conversationId={}", conversation_id);

    let _ = app.emit(
        "agent-resumed",
        serde_json::json!({
            "conversationId": conversation_id,
        }),
    );

    Ok(())
}

/// Check if an agent is paused. An agent is only considered paused if it is
/// both in the paused set AND still running (to filter out stale entries).
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "查询智能体是否处于暂停状态")]
#[tauri::command]
pub async fn agent_is_paused(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<bool, String> {
    let paused = app_state.agent_paused.lock().await;
    if !paused.contains(&conversation_id) {
        return Ok(false);
    }
    drop(paused);
    // 双重检查：agent 必须仍在运行中
    let running = app_state.running_agents.read().await;
    Ok(running.contains(&conversation_id))
}

/// Runtime statistics for a running agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeStats {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub running: bool,
    pub paused: bool,
    #[serde(rename = "activeSessions")]
    pub active_sessions: usize,
    #[serde(rename = "pendingPermissions")]
    pub pending_permissions: usize,
    #[serde(rename = "pendingAskUser")]
    pub pending_ask_user: usize,
    #[serde(rename = "activeToolCalls")]
    pub active_tool_calls: usize,
    /// Real-time execution progress for frontend panels
    /// (AgentStatsPanel, ExecutionTimeline, etc.)
    #[serde(rename = "executionProgress")]
    pub execution_progress: Option<AgentExecutionProgressSnapshot>,
}

/// Get runtime statistics for an agent conversation.
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "获取智能体运行时统计信息")]
#[tauri::command]
pub async fn agent_runtime_stats(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<AgentRuntimeStats, String> {
    let running = {
        let r = app_state.running_agents.read().await;
        r.contains(&conversation_id)
            // Agent 模式下的 regenerate 走 spawn_stream_task（stream_cancel_flags），
            // 不走 agent_query（running_agents），所以也要检查 stream 层是否存活
            || app_state.stream_cancel_flags.contains_key(&conversation_id)
    };
    let paused = {
        let p = app_state.agent_paused.lock().await;
        p.contains(&conversation_id)
    };
    let active_sessions = app_state.agent_session_manager.session_count().await;
    let pending_permissions = {
        let prompters = app_state.agent_prompters.lock().await;
        prompters.get(&conversation_id).map(|p| p.pending_count()).unwrap_or(0)
    };
    let pending_ask_user = {
        let ask = app_state.agent_ask_senders.lock().await;
        ask.keys().filter(|k| k.starts_with(&conversation_id)).count()
    };
    let active_tool_calls = {
        // 使用实际挂起的权限请求数作为活跃工具调用计数。
        // 当工具等待审批时，pending_permissions 反映实际并发数。
        pending_permissions
    };

    // Read real-time execution progress from the SessionManager.
    let execution_progress =
        app_state.agent_session_manager.get_progress(&conversation_id).await.map(|p| p.snapshot());

    Ok(AgentRuntimeStats {
        conversation_id,
        running,
        paused,
        active_sessions,
        pending_permissions,
        pending_ask_user,
        active_tool_calls,
        execution_progress,
    })
}

/// Model routing configuration for multi-model collaboration.
/// Defines which model handles which type of task in the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingConfig {
    /// Primary model for general decision-making and response generation.
    #[serde(rename = "primaryModelId")]
    pub primary_model_id: String,
    /// Optional model for code review tasks (tool results containing code).
    #[serde(rename = "codeReviewModelId")]
    pub code_review_model_id: Option<String>,
    /// Optional model for summarization/compaction tasks.
    #[serde(rename = "summarizationModelId")]
    pub summarization_model_id: Option<String>,
    /// Optional model for translation tasks.
    #[serde(rename = "translationModelId")]
    pub translation_model_id: Option<String>,
    /// Routing rules: map of pattern → model_id.
    /// Pattern matches against tool_name or content keywords.
    #[serde(rename = "routingRules")]
    pub routing_rules: Option<std::collections::HashMap<String, String>>,
}

/// Resolve which model to use for a given task context.
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "根据任务上下文解析应使用的模型")]
#[tauri::command]
pub async fn agent_resolve_model(
    routing_config: ModelRoutingConfig,
    task_type: String,
    tool_name: Option<String>,
    content_hint: Option<String>,
) -> Result<String, String> {
    // Check routing rules first (highest priority)
    if let Some(rules) = &routing_config.routing_rules {
        // Match by task_type
        if let Some(model_id) = rules.get(&task_type) {
            return Ok(model_id.clone());
        }
        // Match by tool_name
        if let Some(tool) = &tool_name {
            if let Some(model_id) = rules.get(tool) {
                return Ok(model_id.clone());
            }
            // Match by tool_name prefix patterns
            for (pattern, model_id) in rules {
                if tool.starts_with(pattern) || tool.contains(pattern) {
                    return Ok(model_id.clone());
                }
            }
        }
        // Match by content keywords
        if let Some(content) = &content_hint {
            let content_lower = content.to_lowercase();
            for (pattern, model_id) in rules {
                if content_lower.contains(&pattern.to_lowercase()) {
                    return Ok(model_id.clone());
                }
            }
        }
    }

    // Built-in task type routing
    match task_type.as_str() {
        "code_review" | "code_review_result" => Ok(routing_config
            .code_review_model_id
            .unwrap_or_else(|| routing_config.primary_model_id.clone())),
        "summarize" | "compact" | "summary" => Ok(routing_config
            .summarization_model_id
            .unwrap_or_else(|| routing_config.primary_model_id.clone())),
        "translate" | "translation" => Ok(routing_config
            .translation_model_id
            .unwrap_or_else(|| routing_config.primary_model_id.clone())),
        _ => Ok(routing_config.primary_model_id.clone()),
    }
}

/// Update agent session
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "更新智能体会话配置")]
#[tauri::command]
pub async fn agent_update_session(
    app_state: State<'_, AppState>,
    request: AgentUpdateSessionRequest,
) -> Result<AgentUpdateSessionResponse, String> {
    // Get or create agent session
    let session = axagent_dao::repo::agent_session::upsert_agent_session(
        app_state.harness.db(),
        &request.conversation_id,
        request.cwd.as_deref(),
        request.permission_mode.as_deref(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(AgentUpdateSessionResponse {
        conversation_id: request.conversation_id,
        name: request.name,
        metadata: request.metadata,
        cwd: session.cwd,
        permission_mode: Some(session.permission_mode),
    })
}

/// Get agent session
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "获取智能体会话信息")]
#[tauri::command]
pub async fn agent_get_session(
    app_state: State<'_, AppState>,
    request: AgentGetSessionRequest,
) -> Result<AgentGetSessionResponse, String> {
    // Get agent session from database
    let session = axagent_dao::repo::agent_session::get_agent_session_by_conversation_id(
        app_state.harness.db(),
        &request.conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if let Some(session) = session {
        Ok(AgentGetSessionResponse {
            conversation_id: request.conversation_id,
            name: None,
            metadata: None,
            created_at: session.created_at,
            last_active_at: session.updated_at,
        })
    } else {
        // Create a new session if none exists
        let new_session = axagent_dao::repo::agent_session::upsert_agent_session(
            app_state.harness.db(),
            &request.conversation_id,
            None,
            Some("default"),
        )
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

        let created_at = new_session.created_at;
        let last_active_at = new_session.updated_at;

        Ok(AgentGetSessionResponse {
            conversation_id: request.conversation_id,
            name: None,
            metadata: None,
            created_at,
            last_active_at,
        })
    }
}

/// Ensure workspace directory
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "确保工作区目录存在并同步云空间")]
#[tauri::command]
pub async fn agent_ensure_workspace(
    app_state: State<'_, AppState>,
    _request: AgentEnsureWorkspaceRequest,
) -> Result<AgentEnsureWorkspaceResponse, String> {
    // Get the workspace_uri from app settings
    // Use the request's workspace_uri first, then fall back to DB settings.
    // Avoid rt.block_on() — it can deadlock the tokio runtime on the current thread.
    let workspace_uri_str = if let Some(ref uri) = _request.workspace_uri {
        Some(uri.clone())
    } else {
        axagent_dao::repo::settings::get_settings(app_state.harness.db())
            .await
            .ok()
            .and_then(|s| s.workspace_uri)
    };

    if let Some(uri_str) = workspace_uri_str {
        let workspace_uri = WorkspaceUri::parse(&uri_str).map_err(|e| {
            ErrorResponse::new(agent_err::INTERNAL)
                .with_detail(format!("Invalid workspace URI: {}", e))
        })?;

        if workspace_uri.is_cloud() {
            // Cloud workspace: sync to local cache
            let backend = app_state
                .sync_engine
                .as_ref()
                .ok_or("Cloud sync engine not available")?
                .backend
                .clone();

            let cache_base = dirs::cache_dir()
                .or_else(dirs::home_dir)
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".axagent")
                .join("cloud-cache");

            let device_id = std::env::var("HOSTNAME")
                .ok()
                .or_else(|| std::env::var("COMPUTERNAME").ok())
                .unwrap_or_else(|| "unknown-device".to_string());

            let mut cloud_workspace =
                CloudWorkspace::new(workspace_uri, backend, cache_base, device_id);

            // Perform sync to ensure files are available locally
            let sync_result = cloud_workspace.sync().await.map_err(|e| {
                ErrorResponse::new(agent_err::INTERNAL)
                    .with_detail(format!("Failed to sync cloud workspace: {}", e))
            })?;

            info!(
                "Cloud workspace synced: downloaded={}, uploaded={}, conflicts={}",
                sync_result.downloaded, sync_result.uploaded, sync_result.pending_conflicts,
            );

            let workspace_path = cloud_workspace
                .cache_dir()
                .to_str()
                .ok_or_else(|| "Cache path contains invalid UTF-8".to_string())?
                .to_string();

            return Ok(AgentEnsureWorkspaceResponse { workspace_path });
        }

        // Local workspace: use the path directly
        let local_path = workspace_uri
            .local_path()
            .ok_or_else(|| "Local workspace URI has invalid path".to_string())?;

        if !local_path.exists() {
            std::fs::create_dir_all(&local_path).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        }

        let workspace_path = local_path
            .to_str()
            .ok_or_else(|| {
                format!("Workspace path contains invalid UTF-8: {}", local_path.display())
            })?
            .to_string();

        return Ok(AgentEnsureWorkspaceResponse { workspace_path });
    }

    // No workspace URI configured: create default
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory".to_string())?;
    let desktop_dir = home_dir.join("Desktop");
    let workspace_dir = desktop_dir.join("AxAgent_Workspace");

    if !workspace_dir.exists() {
        std::fs::create_dir_all(&workspace_dir).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    let workspace_path = workspace_dir
        .to_str()
        .ok_or_else(|| {
            format!("Workspace path contains invalid UTF-8: {}", workspace_dir.display())
        })?
        .to_string();

    Ok(AgentEnsureWorkspaceResponse { workspace_path })
}

/// Backup and clear SDK context
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "备份并清除会话的SDK上下文")]
#[tauri::command]
pub async fn agent_backup_and_clear_sdk_context(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    axagent_dao::repo::agent_session::backup_and_clear_sdk_context_by_conversation_id(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Restore SDK context from backup
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "从备份恢复会话的SDK上下文")]
#[tauri::command]
pub async fn agent_restore_sdk_context_from_backup(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    axagent_dao::repo::agent_session::restore_sdk_context_from_backup_by_conversation_id(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 前端 SteerInput 推送方向指令。已迁移到 AppState.steer_queue，保留此模块级辅助函数
/// 仅作为非命令上下文的 fallback（当前无用途）。

#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "向智能体推送方向指令")]
#[tauri::command]
pub async fn agent_steer(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    instruction: String,
) -> Result<(), String> {
    if instruction.len() > 10_000 {
        return Err(ErrorResponse::err_with_detail(
            steer_err::INSTRUCTION_TOO_LONG,
            "instruction too long (max 10KB)",
        ));
    }
    tracing::debug!(
        "[agent_steer] instruction queued for conversationId={} ({} bytes)",
        conversation_id,
        instruction.len()
    );
    state.steer_queue.lock().await.entry(conversation_id).or_default().push(instruction);
    Ok(())
}

/// 轻量级一次性文本补全请求：不写入会话历史、不触发 agent 引擎/工具循环。
/// 供"AI 生成配置"等纯文本生成场景使用（前端 AgentGeneratorModal 等）。
#[derive(Debug, Deserialize)]
pub struct SimpleChatCompletionRequest {
    pub conversation_id: String,
    pub messages: Vec<SimpleChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    /// 显式指定 provider（可选）；缺省按 会话记录 > 第一个启用 provider 回退
    #[serde(default)]
    pub provider_id: Option<String>,
    /// 显式指定 model（可选）；缺省按 会话记录 > 第一个启用 model 回退
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SimpleChatMessage {
    pub role: String,
    pub content: String,
}

/// 解析一次性补全的 provider/model：
/// 1. 请求显式指定 → 2. 会话记录 → 3. 第一个启用的 provider + 第一个启用的 model
async fn resolve_simple_completion_target(
    app_state: &AppState,
    input: &SimpleChatCompletionRequest,
) -> Result<(String, String), String> {
    if let (Some(p), Some(m)) = (&input.provider_id, &input.model_id) {
        return Ok((p.clone(), m.clone()));
    }
    if let Ok(conv) =
        conversation::get_conversation(app_state.harness.db(), &input.conversation_id).await
    {
        if !conv.provider_id.is_empty() && !conv.model_id.is_empty() {
            return Ok((conv.provider_id, conv.model_id));
        }
    }
    let providers = provider::list_providers(app_state.harness.db()).await.unwrap_or_default();
    for p in providers {
        if !p.enabled {
            continue;
        }
        let models = provider::list_models_for_provider(app_state.harness.db(), &p.id)
            .await
            .unwrap_or_default();
        if let Some(m) = models.into_iter().find(|m| m.enabled) {
            return Ok((p.id, m.model_id));
        }
    }
    Err(ErrorResponse::err(agent_input_err::NO_PROVIDER))
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "轻量级一次性文本补全请求")]
#[tauri::command]
pub async fn simple_chat_completion(
    app_state: State<'_, AppState>,
    input: SimpleChatCompletionRequest,
) -> Result<String, String> {
    let (provider_id, model_id) = resolve_simple_completion_target(&app_state, &input).await?;

    let prov = provider::get_provider(app_state.harness.db(), &provider_id).await.map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let key =
        provider::get_active_key(app_state.harness.db(), &provider_id).await.map_err(|e| {
            String::from(ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, app_state.harness.master_key())
        .map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let settings =
        axagent_dao::repo::settings::get_settings(app_state.harness.db()).await.unwrap_or_default();
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
            &prov.proxy_config,
            &settings,
        ),
        custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter: Arc<dyn ProviderAdapter> = app_state
        .harness
        .get_adapter_for_provider(&prov)
        .await
        .ok_or_else(|| "没有可用的 provider 适配器".to_string())?;

    let messages = input
        .messages
        .iter()
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: ChatContent::Text(m.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        })
        .collect::<Vec<_>>();

    let request = ChatRequest {
        model: model_id,
        messages,
        stream: false,
        temperature: input.temperature,
        top_p: None,
        max_tokens: input.max_tokens,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
        response_format: None,
    };

    let response = adapter.chat(&ctx, Arc::new(request)).await.map_err(|e| {
        String::from(ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(response.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个最小可用的 ChatTool（只需名字参与策略运算）
    fn tool(name: &str) -> ChatTool {
        ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: name.to_string(),
                description: None,
                parameters: None,
            },
        }
    }

    fn names(tools: &[ChatTool]) -> Vec<String> {
        tools.iter().map(|t| t.function.name.clone()).collect()
    }

    fn blocked(set: &[&str]) -> HashSet<String> {
        set.iter().map(|s| s.to_string()).collect()
    }

    /// R3 回归：工具同时出现在 extra 与 blocked 时，**必须被移除**。
    /// 这是「先 retain 后注入」旧顺序下的安全绕过路径——注入会复活被禁工具。
    #[test]
    fn apply_tool_policy_blocks_tool_injected_by_extra() {
        let out = apply_tool_policy(
            vec![tool("read_file")],
            vec![tool("browser_use"), tool("shell_exec")],
            &blocked(&["shell_exec"]),
        );
        assert_eq!(names(&out), vec!["read_file", "browser_use"]);
    }

    /// blocked 必须能剔除列表里**原本就有**的工具，而不只是 extra 注入的那些。
    #[test]
    fn apply_tool_policy_blocks_preexisting_tool() {
        let out = apply_tool_policy(
            vec![tool("read_file"), tool("shell_exec"), tool("write_file")],
            Vec::new(),
            &blocked(&["shell_exec"]),
        );
        assert_eq!(names(&out), vec!["read_file", "write_file"]);
    }

    /// 多个禁用项一次性剔除，且保留项的相对顺序不变。
    #[test]
    fn apply_tool_policy_blocks_multiple_and_preserves_order() {
        let out = apply_tool_policy(
            vec![tool("a"), tool("b"), tool("c"), tool("d")],
            vec![tool("e")],
            &blocked(&["b", "d"]),
        );
        assert_eq!(names(&out), vec!["a", "c", "e"]);
    }

    /// extra 里与已有工具同名的项应被丢弃，不能产生重名（LLM 侧会报
    /// "Tool names must be unique"）。
    #[test]
    fn apply_tool_policy_extra_does_not_duplicate_existing() {
        let out = apply_tool_policy(
            vec![tool("read_file")],
            vec![tool("read_file"), tool("browser_use")],
            &blocked(&[]),
        );
        assert_eq!(names(&out), vec!["read_file", "browser_use"]);
    }

    /// extra 内部自身重名时同样去重（保留先出现者）。
    #[test]
    fn apply_tool_policy_dedups_within_extra() {
        let out = apply_tool_policy(
            Vec::new(),
            vec![tool("dup"), tool("dup"), tool("other")],
            &blocked(&[]),
        );
        assert_eq!(names(&out), vec!["dup", "other"]);
    }

    /// 无 extra 无 blocked 时是恒等变换（调用点靠外层 if 跳过整块，此处锁定
    /// 函数本身在空策略下的行为，便于将来去掉外层 if 时仍然安全）。
    #[test]
    fn apply_tool_policy_empty_policy_is_identity() {
        let out = apply_tool_policy(
            vec![tool("read_file"), tool("write_file")],
            Vec::new(),
            &blocked(&[]),
        );
        assert_eq!(names(&out), vec!["read_file", "write_file"]);
    }

    /// 空输入 + 只禁用：结果必须为空，不能 panic。
    #[test]
    fn apply_tool_policy_handles_empty_input() {
        let out = apply_tool_policy(Vec::new(), vec![tool("x")], &blocked(&["x"]));
        assert!(names(&out).is_empty());
    }

    /// 披露工具对 profile 黑名单免疫：被禁用也必须保留，否则编排器「发现不了任何能力」。
    /// 豁免名单统一取自 `is_disclosure_immune`，此处逐个锁定 7 个元工具。
    #[test]
    fn apply_tool_policy_disclosure_tools_immune_to_blocklist() {
        let all: Vec<ChatTool> = DISCLOSURE_TOOLS.iter().copied().map(tool).collect();
        let expected: Vec<String> = DISCLOSURE_TOOLS.iter().copied().map(String::from).collect();
        let blocked_all = blocked(&DISCLOSURE_TOOLS);

        // 既有的（非 extra 注入）披露工具不被移除，且顺序不变
        assert_eq!(names(&apply_tool_policy(all.clone(), Vec::new(), &blocked_all)), expected);
        // extra 注入的披露工具同样免疫（豁免不能只覆盖既有那份）
        assert_eq!(names(&apply_tool_policy(Vec::new(), all, &blocked_all)), expected);
    }

    /// 屏幕感知工具**不享受**披露工具豁免：被并入 blocked 时必须移除，且不能通过
    /// recommended 追加复活。
    ///
    /// 与上一条 `apply_tool_policy_disclosure_tools_immune_to_blocklist` 互为对照——
    /// 两者走同一个 `blocked_names`，结果相反。这两个测试一起锁定了「哪些名字免疫、
    /// 哪些不免疫」的边界。
    #[test]
    fn apply_tool_policy_screen_perception_tool_is_blockable() {
        let out = apply_tool_policy(
            vec![tool(SCREEN_PERCEPTION_TOOL), tool("read_file")],
            // 即使通过 recommended/extra 追加也不能复活
            vec![tool(SCREEN_PERCEPTION_TOOL)],
            &blocked(&[SCREEN_PERCEPTION_TOOL]),
        );
        assert_eq!(names(&out), vec!["read_file"]);
    }

    /// 免疫范围不得外溢：同一个 blocked 里，非披露工具照常被移除，只有披露工具留下。
    #[test]
    fn apply_tool_policy_immunity_does_not_leak_to_other_tools() {
        let out = apply_tool_policy(
            vec![tool("CapabilityView"), tool("shell_exec"), tool("read_file")],
            vec![tool("CapabilityLoad")],
            &blocked(&["CapabilityView", "CapabilityLoad", "shell_exec"]),
        );
        // shell_exec 被移除；两个披露工具（一个既有、一个 extra 注入）都留着
        assert_eq!(names(&out), vec!["CapabilityView", "read_file", "CapabilityLoad"]);
    }
}
