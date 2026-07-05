use crate::commands::agent::emit_status;
use crate::commands::agent::agent_err;
use crate::commands::agent::load_skill_tools;
use axagent_harness::types::agent::AgentContentBlock;
use crate::commands::agent::context_keys;
use crate::commands::agent::resolve_base_url_for_type;
use crate::app_state::AppState;
use crate::commands::agent::LAST_KNOWN_SETTINGS;
use std::sync::Mutex;
use axagent_harness::types::function_call::Value;
use crate::commands::agent::build_agent_system_prompt;
use crate::commands::agent::steer_queue;
use axagent_harness::types::skill::SkillExecutionContext;
use crate::commands::agent::check_and_suggest_workflow_match;
use crate::app_state::AppState as State;
use axagent_harness::types::function_call::ChatTool;
use crate::commands::agent::load_enabled_skill_contents;
use axagent_harness::types::conversation::MessageRole;
use axagent_harness::types::provider::ProviderProxyConfig;
use crate::commands::providers as provider;
use crate::commands::error::ErrorResponse;
use crate::commands::agent::pricing;
use crate::commands::conversations as conversation;
use std::collections::HashSet;
use axagent_harness::traits::tool_registry::UnifiedToolRegistry;
use crate::commands::agent::SKILL_MCP_REGISTRY;
use crate::commands::agent::message;
use crate::commands::agent::build_streaming_api_client;
use crate::commands::agent::search_provider;
use crate::commands::agent::agent_status_err;
use axagent_harness::types::mcp::McpServer;
use crate::commands::agent::execute_skill_sync;
use std::sync::Arc;
use tracing::{info, warn};
use serde_json::Value;
#[tauri::command]
#[tracing::instrument(skip(app, app_state))]
pub(super) async fn agent_query(
    app: AppHandle,
    app_state: State<'_, AppState>,
    request: AgentQueryRequest,
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
        .map_err(|e| e.to_string())?;
    let conversation_scenario = conversation.scenario.clone();
    let enabled_skill_ids = conversation.enabled_skill_ids.clone();

    // AgentProfile = AgentRole + Expert（两两组装，运行时拼接提示词）
    // 不再持久化预合并的 system_prompt，修改 Expert/Role 后自动生效
    let mut role_system_prompt: Option<String> = None;
    let mut expert_system_prompt: Option<String> = None;
    let mut effective_agent_role: Option<axagent_runtime::agent_roles::AgentRole> = None;
    let mut profile_recommended_tools: Vec<String> = Vec::new();
    let mut profile_disallowed_tools: Vec<String> = Vec::new();

    if let Some(ref profile_id) = request.agent_profile_id {
        if let Ok(profile) =
            axagent_core::repo::agent_profile::get_agent_profile(app_state.harness.db(), profile_id)
                .await
        {
            // Layer 1: AgentRole system_prompt（岗位）
            if let Some(ref role_name) = profile.agent_role {
                if let Some(resolved) = axagent_runtime::agent_roles::AgentRole::resolve(
                    app_state.harness.db(),
                    role_name,
                )
                .await
                {
                    effective_agent_role =
                        axagent_runtime::agent_roles::AgentRole::from_str_opt(&resolved.name);
                    if !resolved.system_prompt.is_empty() {
                        role_system_prompt = Some(resolved.system_prompt);
                    }
                }
            }

            // Layer 2: Expert domain knowledge（技能）
            if let Some(ref expert_id) = profile.expert_id {
                if let Ok(Some(expert)) =
                    axagent_core::entity::agency_experts::Entity::find_by_id(expert_id)
                        .one(app_state.harness.db())
                        .await
                        .map_err(|e| e.to_string())
                {
                    if !expert.system_prompt.is_empty() {
                        expert_system_prompt = Some(expert.system_prompt);
                    }
                    // 合并 Expert 的推荐工具
                    if let Some(ref tools_json) = expert.recommended_tools {
                        if let Ok(tools) = serde_json::from_str::<Vec<String>>(tools_json) {
                            profile_recommended_tools.extend(tools);
                        }
                    }
                }
            }

            // 合并 Profile 自身推荐/禁用工具
            profile_recommended_tools.extend(profile.recommended_tools);
            profile_disallowed_tools = profile.disallowed_tools;
        }
    }

    // 提示词拼接：Role → Expert（两部分动态拼接，不在 DB 中预缓存）
    let mut prompt_parts: Vec<&str> = Vec::new();
    if let Some(ref s) = role_system_prompt {
        if !s.is_empty() {
            prompt_parts.push(s.as_str());
        }
    }
    if let Some(ref s) = expert_system_prompt {
        if !s.is_empty() {
            prompt_parts.push(s.as_str());
        }
    }
    let effective_system_prompt: Option<String> = if prompt_parts.is_empty() {
        None
    } else {
        Some(prompt_parts.join("\n\n"))
    };

    // AgentProfile 未产生有效提示词时，降级到请求中携带的 system_prompt
    let effective_system_prompt = effective_system_prompt.or_else(|| request.system_prompt.clone());

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
        AsyncRunningAgentGuard {
            conversation_id: conversation_id.clone(),
            running_agents: app_state.running_agents.clone(),
            cancel_tokens: app_state.agent_cancel_tokens.clone(),
            paused_set: app_state.agent_paused.clone(),
        }
    });

    // Set workflow_status to "running" for workflow-type sessions
    if conversation.session_type == "workflow" {
        let _ = axagent_core::repo::conversation::update_conversation(
            app_state.harness.db(),
            &conversation_id,
            axagent_harness::types::UpdateConversationInput {
                workflow_status: Some(Some("running".to_string())),
                ..Default::default()
            },
        )
        .await;
    }

    info!("[agent_query] Got provider: {}", request.provider_id);

    // Get provider
    let prov = provider::get_provider(app_state.harness.db(), &request.provider_id)
        .await
        .map_err(|e| e.to_string())?;
    info!("[agent_query] Got provider keys count: {}", prov.keys.len());

    // Get active key
    let key = prov
        .keys
        .iter()
        .find(|k| k.enabled)
        .ok_or_else(|| "No active API key for provider".to_string())?;
    info!("[agent_query] Found active key");

    // Decrypt key
    let api_key =
        axagent_core::crypto::decrypt_key(&key.key_encrypted, app_state.harness.master_key())
            .map_err(|e| e.to_string())?;
    info!("[agent_query] Decrypted API key");

    // H2: Get settings from database with last-known-good fallback
    let settings = axagent_core::repo::settings::get_settings(app_state.harness.db())
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to load settings from DB, attempting cached fallback: {}", e);
            let cache = LAST_KNOWN_SETTINGS.get_or_init(|| Mutex::new(None));
            if let Ok(guard) = cache.lock() {
                if let Some(ref cached) = *guard {
                    warn!("Using cached last-known-good settings as fallback");
                    return cached.clone();
                }
            }
            warn!("No cached settings available, using empty defaults — pricing/budget may be affected");
            Default::default()
        });
    // Update cache on successful read
    if let Ok(mut guard) = LAST_KNOWN_SETTINGS
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *guard = Some(settings.clone());
    }

    // Create provider context
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: ProviderProxyConfig::resolve(&prov.proxy_config, &settings),
        custom_headers: prov
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // Get model info for param overrides
    let resolved_model = axagent_core::repo::provider::get_model(
        app_state.harness.db(),
        &request.provider_id,
        &request.model_id,
    )
    .await
    .ok();
    let model_param_overrides = resolved_model
        .as_ref()
        .and_then(|m| m.param_overrides.clone());
    let use_max_completion_tokens = model_param_overrides
        .as_ref()
        .and_then(|p| p.use_max_completion_tokens);
    let thinking_param_style = model_param_overrides
        .as_ref()
        .and_then(|p| p.thinking_param_style.clone());
    let request_delay_ms = model_param_overrides
        .as_ref()
        .and_then(|p| p.request_delay_ms);

    // Resolve effective model parameters: request options → model overrides → defaults
    let effective_temperature = request
        .options
        .as_ref()
        .and_then(|o| o.temperature)
        .or_else(|| {
            model_param_overrides
                .as_ref()
                .and_then(|p| p.temperature.map(|v| v as f64))
        });
    let effective_top_p = request.options.as_ref().and_then(|o| o.top_p).or_else(|| {
        model_param_overrides
            .as_ref()
            .and_then(|p| p.top_p.map(|v| v as f64))
    });
    let effective_max_tokens = request
        .options
        .as_ref()
        .and_then(|o| o.max_tokens)
        .or_else(|| model_param_overrides.as_ref().and_then(|p| p.max_tokens));

    // Create provider adapter instance
    let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
        axagent_harness::types::ProviderType::OpenAI => {
            Arc::new(axagent_providers::openai::OpenAIAdapter::new())
        },
        axagent_harness::types::ProviderType::OpenAIResponses => {
            Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new())
        },
        axagent_harness::types::ProviderType::Anthropic => {
            Arc::new(axagent_providers::anthropic::AnthropicAdapter::new())
        },
        axagent_harness::types::ProviderType::Gemini => {
            Arc::new(axagent_providers::gemini::GeminiAdapter::new())
        },
        axagent_harness::types::ProviderType::OpenClaw => {
            Arc::new(axagent_providers::openclaw::OpenClawAdapter::new())
        },
        axagent_harness::types::ProviderType::Hermes => {
            Arc::new(axagent_providers::hermes::HermesAdapter::new())
        },
        axagent_harness::types::ProviderType::Ollama => {
            Arc::new(axagent_providers::ollama::OllamaAdapter::new())
        },
    };

    // Load MCP tools for enabled servers (same logic as Q&A mode)
    let mcp_ids = request.enabled_mcp_server_ids.clone().unwrap_or_else(|| {
        warn!("enabled_mcp_server_ids is None, using empty default");
        Vec::new()
    });
    let mut tool_registry = UnifiedToolRegistry::new();
    let mut chat_tools: Vec<ChatTool> = Vec::new();

    // Load enabled state for the unified tool registry
    tool_registry
        .load_enabled_state(app_state.harness.db())
        .await;

    // Build all_server_ids from remote MCP servers only (no builtin)
    let all_server_ids: Vec<String> = mcp_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

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
                let server = match axagent_core::repo::mcp_server::get_mcp_server(&db, &sid).await {
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
                    axagent_core::repo::mcp_server::list_tools_for_server(&db, &sid).await
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
                Some(ServerTools {
                    server,
                    chat_tools,
                    tool_descriptors,
                })
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

    // ── 注入 axagent-tools 统一工具到 chat_tools ──
    let disabled_set: HashSet<String> = request
        .options
        .as_ref()
        .and_then(|o| o.disabled_tools.as_ref())
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();
    let unified_chat_tools: Vec<ChatTool> = tool_registry
        .get_chat_tools()
        .into_iter()
        .filter(|t| !disabled_set.contains(&t.function.name))
        .collect();
    // 同步注册表的屏蔽列表
    if !disabled_set.is_empty() {
        tool_registry = tool_registry.with_blocked_tools(disabled_set.into_iter().collect());
    }
    info!(
        "[agent] UnifiedToolRegistry provides {} tools to LLM ({} disabled)",
        unified_chat_tools.len(),
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
    let skill_contents = load_enabled_skill_contents(
        &app_state,
        conversation_scenario.as_deref(),
        &enabled_skill_ids,
    )
    .await;

    // Convert enabled skills to ChatTool definitions for Agent to call
    let (skill_tools, skill_map) =
        load_skill_tools(&app_state, conversation_scenario.as_deref(), &enabled_skill_ids).await;
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
        search_provider::get_search_provider(app_state.harness.db(), sp_id)
            .await
            .ok()
    } else {
        search_provider::list_search_providers(app_state.harness.db())
            .await
            .ok()
            .and_then(|providers| providers.into_iter().find(|p| p.enabled))
    };
    if let Some(ref sp) = search_provider_used {
        let api_key = axagent_core::entity::search_providers::Entity::find_by_id(&sp.id)
            .one(app_state.harness.db())
            .await
            .ok()
            .flatten()
            .and_then(|e| e.api_key_ref)
            .and_then(|enc| {
                axagent_core::crypto::decrypt_key(&enc, app_state.harness.master_key()).ok()
            })
            .unwrap_or_else(|| {
                warn!("search provider API key decryption failed, using empty default");
                String::new()
            });
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

    // Register skill tool handlers in tool_registry for execution
    // This is done AFTER tool_registry is fully configured to ensure MCP tools are available
    // The skill handlers will use a global registry for MCP tool execution
    if skill_tools_count > 0 {
        let _ = SKILL_MCP_REGISTRY.set(std::sync::Arc::new(tool_registry.clone()));
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
                std::sync::Arc::new(move |input: &str| {
                    execute_skill_sync(&skill_id, &skill_name, &skill_content, input, &ctx)
                        .map_err(axagent_harness::ToolError::new)
                }),
            );
        }
        info!("[agent] Added {} skill tools to chat_tools", skill_tools_count);
        info!("[agent] Registered {} skill tool handlers", skill_map.len());
    }

    // Create API client with tool definitions, model ID and parameters
    // Also attach a streaming callback to emit text/thinking deltas in real-time
    let api_client = build_streaming_api_client(
        adapter,
        ctx,
        chat_tools.clone(),
        &request.model_id,
        effective_temperature,
        effective_top_p,
        effective_max_tokens,
        request.thinking_budget,
        use_max_completion_tokens,
        thinking_param_style,
        request_delay_ms,
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
            .map_err(|e| e.to_string())?
        }
    } else {
        Vec::new()
    };

    // Build data: URLs for image attachments so the LLM can see them
    let image_urls: Vec<String> = persisted_attachments
        .iter()
        .filter(|a| a.file_type.starts_with("image/"))
        .filter_map(|a| {
            let file_store = axagent_core::file_store::FileStore::new();
            if a.file_path.is_empty() {
                // Use inline data if available
                a.data
                    .as_ref()
                    .map(|d| format!("data:{};base64,{}", a.file_type, d))
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
    .map_err(|e| e.to_string())?;

    // Increment the persisted message count
    axagent_core::repo::conversation::increment_message_count(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Use the long-lived SessionManager from AppState (persists sessions across queries)
    let session_manager = &app_state.agent_session_manager;
    // Ensure app_handle is set (idempotent if already set)
    session_manager.set_app_handle(app.clone()).await;
    session_manager
        .set_default_workspace_dir(settings.default_workspace_dir.clone())
        .await;
    info!(
        "[agent_query] Using AppState SessionManager, has_app_handle: {}",
        session_manager.has_app_handle().await
    );

    // Get or create session (reuse existing session to preserve conversation history)
    let session = session_manager
        .get_or_create_session(prov.id.clone(), conversation_id.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Apply agent role if specified — sets role on session and filters tools
    // Apply agent role: prefer agent_profile.agent_role > request.role > auto-estimate
    let mut resolved_role = if let Some(role) = effective_agent_role {
        info!("[agent_query] Using role from agent_profile: {}", role);
        Some(role)
    } else if let Some(role) = request
        .role
        .as_deref()
        .and_then(axagent_runtime::agent_roles::AgentRole::from_str_opt)
    {
        info!("[agent_query] Using role from request: {}", role);
        Some(role)
    } else {
        None
    };

    // Filter chat_tools by role's allowed tools, plus profile recommended/disallowed
    if let Some(role) = resolved_role {
        let allowed_tools: Vec<&str> = role.default_tools();
        let mut allowed_set: HashSet<&str> = allowed_tools.iter().copied().collect();
        for t in &profile_recommended_tools {
            allowed_set.insert(t.as_str());
        }
        for t in &profile_disallowed_tools {
            allowed_set.remove(t.as_str());
        }
        chat_tools.retain(|t| allowed_set.contains(t.function.name.as_str()));
        info!(
            "[agent_query] Role '{}' filtered tools: {} remaining (profile: +{}/-{})",
            role,
            chat_tools.len(),
            profile_recommended_tools.len(),
            profile_disallowed_tools.len(),
        );
    }

    // Smart decision: if no explicit role was set, estimate task complexity
    // and auto-assign a role for high-complexity multi-step tasks.
    resolved_role = if resolved_role.is_none() {
        let complexity = axagent_trajectory::estimate_complexity_public(&request.input);
        info!("[agent_query] Auto-estimated task complexity: {:?}", complexity);
        match complexity {
            axagent_trajectory::Complexity::High => {
                // High complexity tasks benefit from the Coordinator role
                // which is designed for task decomposition and orchestration
                let auto_role = axagent_runtime::agent_roles::AgentRole::Coordinator;
                info!("[agent_query] Auto-assigning role '{}' for high-complexity task", auto_role);
                Some(auto_role)
            },
            axagent_trajectory::Complexity::Medium => {
                // Medium complexity: use Developer role for implementation tasks
                let auto_role = axagent_runtime::agent_roles::AgentRole::Developer;
                info!(
                    "[agent_query] Auto-assigning role '{}' for medium-complexity task",
                    auto_role
                );
                Some(auto_role)
            },
            axagent_trajectory::Complexity::Low => {
                // Low complexity: no role filtering needed, use all tools
                None
            },
        }
    } else {
        resolved_role
    };

    // RAG retrieval: search enabled knowledge bases and memory namespaces
    let kb_ids = request
        .enabled_knowledge_base_ids
        .clone()
        .unwrap_or_default();
    // Auto-inherit memory namespace IDs from conversation settings if not explicitly provided
    let mem_ids = if request.enabled_memory_namespace_ids.is_some() {
        request
            .enabled_memory_namespace_ids
            .clone()
            .unwrap_or_else(|| {
                warn!("enabled_memory_namespace_ids is None (explicitly set), using empty default");
                Vec::new()
            })
    } else {
        // Fallback: load enabled memory namespaces from the conversation's settings
        match axagent_core::repo::conversation::get_conversation(
            app_state.harness.db(),
            &conversation_id,
        )
        .await
        {
            Ok(conv) => conv.enabled_memory_namespace_ids,
            Err(_) => Vec::new(),
        }
    };
    let wiki_ids = request.enabled_wiki_ids.clone().unwrap_or_else(|| {
        warn!("enabled_wiki_ids is None, using empty default");
        Vec::new()
    });
    let rag_result = crate::indexing::collect_rag_context(
        app_state.harness.db(),
        app_state.harness.master_key(),
        &app_state.vector_store,
        &kb_ids,
        &mem_ids,
        &wiki_ids,
        &request.input,
        5,
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
    let db_session = axagent_core::repo::agent_session::get_agent_session_by_conversation_id(
        app_state.harness.db(),
        &conversation_id,
    )
    .await
    .ok()
    .flatten();
    let workspace_root_for_prompt = db_session.as_ref().and_then(|s| s.cwd.clone());

    // 将 workspace cwd 注入工具注册表，确保工具执行时使用正确的工作目录
    if let Some(ref cwd) = workspace_root_for_prompt {
        if !cwd.is_empty() {
            tool_registry = tool_registry.with_working_dir(cwd.as_str());
            info!("[agent] Tool registry working_dir set to: {}", cwd);
        }
    }

    let app_language = axagent_core::repo::settings::get_settings(app_state.harness.db())
        .await
        .ok()
        .map(|s| s.language);

    let system_prompt = build_agent_system_prompt(
        effective_system_prompt.as_deref(),
        rag_context_parts.as_deref(),
        &skill_contents,
        resolved_role,
        working_memory_text.as_deref(),
        if nudge_ref.is_empty() {
            None
        } else {
            Some(&nudge_ref)
        },
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
            let mut q = steer_queue().lock().await;
            if q.is_empty() {
                None
            } else {
                let instructions = std::mem::take(&mut *q);
                drop(q);
                let formatted: String = instructions
                    .iter()
                    .enumerate()
                    .map(|(i, inst)| format!("- [steer-{}] {}", i, inst.1.join(", ")))
                    .collect::<Vec<_>>()
                    .join("\n");
                info!("[agent_query] Injecting {} steer instruction(s)", instructions.len());
                Some(formatted)
            }
        },
    );

    // Attach image URLs to the API client for multimodal support
    let api_client = api_client.with_image_urls(image_urls);

    // Resolve permission mode from the agent session DB record (db_session fetched above)
    let permission_mode_str = db_session
        .as_ref()
        .map(|s| s.permission_mode.clone())
        .unwrap_or_else(|| "default".to_string());
    let runtime_permission_mode = match permission_mode_str.as_str() {
        "full_access" => axagent_runtime::PermissionMode::Allow,
        "accept_edits" => axagent_runtime::PermissionMode::WorkspaceWrite,
        "default" => axagent_runtime::PermissionMode::Prompt,
        _ => axagent_runtime::PermissionMode::Prompt,
    };
    info!(
        "[agent_query] Permission mode: {} -> {:?}",
        permission_mode_str, runtime_permission_mode
    );

    // Get always-allowed tools for this conversation
    let always_allowed = app_state
        .agent_always_allowed
        .lock()
        .await
        .get(&conversation_id)
        .cloned()
        .unwrap_or_default();

    // Get workspace root from agent session for permission boundary checks
    let workspace_root = db_session
        .as_ref()
        .and_then(|s| s.cwd.clone())
        .unwrap_or_else(|| {
            warn!("workspace_root (cwd) missing from session, using empty default");
            String::new()
        });

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
    let estimated_input_tokens =
        axagent_core::token_counter::estimate_tokens(&request.input) as u64;
    if let Err(budget_err) = pricing::check_token_budget(estimated_input_tokens) {
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
    app_state
        .agent_cancel_tokens
        .insert(conversation_id.clone(), cancel_token.clone());

    // Drain steer queue and inject instructions into the prompt
    let augmented_input = {
        let mut queue = steer_queue().lock().await;
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

    let result: Result<
        (axagent_runtime::TurnSummary, axagent_runtime::Session),
        axagent_runtime::RuntimeError,
    > = session_manager
        .run_turn_with_tools(
            &session_id,
            augmented_input,
            api_client,
            tool_registry,
            system_prompt,
            conversation_id.clone(),
            runtime_permission_mode,
            app_state.agent_prompters.clone(),
            Some(cancel_token),
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

    // Remove the prompter from AppState now that the turn is complete
    {
        let mut prompters = app_state.agent_prompters.lock().await;
        prompters.remove(&conversation_id);
    }

    // Clean up paused state in case the agent was paused but the turn
    // completed (e.g. via cancel while paused).
    {
        let mut paused = app_state.agent_paused.lock().await;
        paused.remove(&conversation_id);
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
            let parts_json = {
                let all_blocks: Vec<serde_json::Value> = summary
                    .assistant_messages
                    .iter()
                    .flat_map(|msg| &msg.blocks)
                    .map(|block| match block {
                        axagent_runtime::ContentBlock::Text { text } => {
                            serde_json::json!({ "type": "text", "text": text })
                        }
                        axagent_runtime::ContentBlock::ToolUse { id, name, input } => {
                            serde_json::json!({ "type": "tool_use", "id": id, "name": name, "input": input })
                        }
                        axagent_runtime::ContentBlock::ToolResult { tool_use_id, tool_name, output, is_error } => {
                            serde_json::json!({ "type": "tool_result", "tool_use_id": tool_use_id, "tool_name": tool_name, "output": output, "is_error": is_error })
                        }
                    })
                    .collect();
                if all_blocks.is_empty() {
                    None
                } else {
                    serde_json::to_string(&all_blocks).inspect_err(|e| tracing::error!(%e, "serde_json 序列化失败")).ok()
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
            )
            .await
            .map_err(|e| e.to_string())?;

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
            let cost_usd = pricing::estimate_cost_usd(
                &request.model_id,
                summary.usage.input_tokens as u64,
                summary.usage.output_tokens as u64,
                resolved_model.as_ref().and_then(|m| m.input_price_per_mtok),
                resolved_model
                    .as_ref()
                    .and_then(|m| m.output_price_per_mtok),
            );
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
                let _ = axagent_core::repo::conversation::update_conversation(
                    app_state.harness.db(),
                    &conversation_id,
                    axagent_harness::types::UpdateConversationInput {
                        workflow_status: Some(Some("completed".to_string())),
                        ..Default::default()
                    },
                )
                .await;
            }

            // Semantic workflow matching for conversation-type sessions:
            // After the first agent response, check if user input matches any preset template
            if conversation.session_type == "conversation" {
                let _ = check_and_suggest_workflow_match(
                    app_state.harness.db(),
                    &app,
                    &conversation_id,
                    &request.input,
                )
                .await;
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
                    let mut tool_results_vec: Vec<axagent_trajectory::ToolResult> = Vec::new();

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
                                tool_results_vec.push(axagent_trajectory::ToolResult {
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
                let trajectory = axagent_trajectory::Trajectory::new(
                    conversation_id.clone(),
                    "default_user".to_string(),
                    trajectory_input[..trajectory_input.len().min(100)].to_string(),
                    trajectory_input[..trajectory_input.len().min(200)].to_string(),
                    outcome,
                    (now.timestamp_millis() - start_time.timestamp_millis()).max(0) as u64,
                    steps,
                );

                // P6: Inject known patterns into trajectory for reward computation
                let mut trajectory = trajectory;
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
                        let rewards = rl.compute_rewards(&mut traj_for_rl);
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
                        let verbosity = style
                            .verbosity
                            .unwrap_or(axagent_trajectory::Verbosity::Unchanged);
                        let tech = style
                            .technical_level
                            .unwrap_or(axagent_trajectory::TechnicalLevel::Unchanged);
                        let fmt = style
                            .format
                            .unwrap_or(axagent_trajectory::ContentFormat::Unchanged);
                        profile.update_style(verbosity, tech, fmt);
                    }
                }
            }

            Ok(AgentQueryResponse {
                conversation_id,
                assistant_message_id: assistant_message.id,
            })
        },
        Err(e) => {
            let error_msg = e.to_string();

            // Set workflow_status to "failed" for workflow-type sessions
            if conversation.session_type == "workflow" {
                let _ = axagent_core::repo::conversation::update_conversation(
                    app_state.harness.db(),
                    &conversation_id,
                    axagent_harness::types::UpdateConversationInput {
                        workflow_status: Some(Some("failed".to_string())),
                        ..Default::default()
                    },
                )
                .await;
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

            Err(error_msg)
        },
    }
}
