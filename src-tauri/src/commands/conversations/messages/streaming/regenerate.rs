use super::spawn_stream_task;
#[tracing::instrument(skip(app, state))]
pub async fn regenerate_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: RegenerateMessageParams,
) -> Result<(), String> {
    let RegenerateMessageParams {
        conversation_id,
        user_message_id,
        options,
    } = params;
    tracing::info!("[regenerate_message] Called for conversation={} user_message_id={:?}", conversation_id, user_message_id);
    let SendMessageOptions {
        enabled_mcp_server_ids,
        thinking_budget,
        enabled_knowledge_base_ids,
        enabled_memory_namespace_ids,
        enabled_wiki_ids,
    } = options;
    tracing::info!("[regenerate_message] Step 1: loading messages from DB");
    // 1. Get all active messages for the conversation
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    tracing::info!("[regenerate_message] Step 2: messages loaded, count={}", messages.len());

    // Find target user message: use provided ID or fall back to last user message
    let last_user_msg = if let Some(ref uid) = user_message_id {
        messages
            .iter()
            .find(|m| m.id == *uid && m.role == MessageRole::User)
            .ok_or_else(|| format!("User message {} not found", uid))?
            .clone()
    } else {
        messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .ok_or("No user message found to regenerate from")?
            .clone()
    };
    tracing::info!("[regenerate_message] Step 3: user_msg_id={}", last_user_msg.id);

    // 2. Count existing AI reply versions for this user message
    let existing_versions = axagent_dao::repo::message::list_message_versions(
        state.harness.db(),
        &conversation_id,
        &last_user_msg.id,
    )
    .await
    .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let new_version_index = existing_versions.len() as i32;
    tracing::info!("[regenerate_message] Step 4: existing versions={}", existing_versions.len());

    // Preserve original created_at from first version to maintain message position
    let original_created_at = existing_versions.first().map(|v| v.created_at);

    // Find the currently active version's model to regenerate with the same model
    let active_version = existing_versions.iter().find(|v| v.is_active);
    let active_model_id = active_version.and_then(|v| v.model_id.clone());
    let active_provider_id = active_version.and_then(|v| v.provider_id.clone());

    // 3. Deactivate all existing AI reply versions for this user message
    tracing::info!("[regenerate_message] Step 5: deactivating old versions ({} existing)", existing_versions.len());
    use axagent_entities::messages as msg_entity;
    use sea_orm::sea_query::Expr;
    msg_entity::Entity::update_many()
        .filter(msg_entity::Column::ConversationId.eq(&conversation_id))
        .filter(msg_entity::Column::ParentMessageId.eq(&last_user_msg.id))
        .col_expr(msg_entity::Column::IsActive, Expr::value(0))
        .exec(state.harness.db())
        .await
        .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    tracing::info!("[regenerate_message] Step 6: old versions deactivated");

    // 4. Get conversation details
    tracing::info!("[regenerate_message] Step 7: loading conversation");
    let mut conversation =
        axagent_dao::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    tracing::info!("[regenerate_message] Step 8: conversation loaded, provider={} model={}", conversation.provider_id, conversation.model_id);

    // Override conversation model_id/provider_id so spawn_stream_task uses the correct model
    if let Some(ref mid) = active_model_id {
        conversation.model_id = mid.clone();
    }
    if let Some(ref pid) = active_provider_id {
        conversation.provider_id = pid.clone();
    }

    // 5. Get provider config + decrypt key
    let provider =
        axagent_dao::repo::provider::get_provider(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let key_row =
        axagent_dao::repo::provider::get_active_key(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let decrypted_key =
        axagent_crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;

    // 6. Rebuild chat messages (active messages only — old inactive versions excluded)
    let remaining_messages =
        axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let file_store = axagent_storage::file_store::FileStore::new();

    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Resolve effective system prompt: conversation → category → global default
    let effective_system_prompt = resolve_system_prompt(state.harness.db(), &conversation).await;

    if let Some(ref sys) = effective_system_prompt {
        chat_messages.push(ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(sys.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    }

    // RAG retrieval for regeneration: resolve from context_sources when explicit IDs are not provided
    let memory_tag = {
        let (kb_ids, mem_ids, wiki_ids) = resolve_rag_ids(
            state.harness.db(),
            &conversation_id,
            enabled_knowledge_base_ids,
            enabled_memory_namespace_ids,
            enabled_wiki_ids,
        )
        .await;
        let mut rag_result = crate::indexing::collect_rag_context(
            state.harness.db(),
            state.harness.master_key(),
            &state.vector_store,
            &kb_ids,
            &mem_ids,
            &wiki_ids,
            &last_user_msg.content,
            5,
        )
        .await;

        let tag = build_memory_retrieval_tag(&rag_result.source_results);

        // Always emit so frontend can replace the searching indicator
        let _ = app.emit(
            "rag-context-retrieved",
            RagContextRetrievedEvent {
                conversation_id: conversation_id.clone(),
                sources: rag_result.source_results,
            },
        );

        let wm_content_2: String;
        {
            let ms = state.memory_service.read().await;
            wm_content_2 = ms.format_for_prompt().await;
        }

        if !rag_result.context_parts.is_empty() {
            dedup_rag_against_working_memory(&wm_content_2, &mut rag_result.context_parts);
            let rag_budget = crate::context_manager::token_budget::RETRIEVED_MEMORIES;
            let rag_items = apply_rag_token_budget(&rag_result.context_parts, rag_budget);
            if let Some(msg) = build_rag_chat_message(&rag_items) {
                chat_messages.push(msg);
            }
        }
        if let Some(msg) = build_working_memory_chat_message(&wm_content_2) {
            chat_messages.push(msg);
        }
        tag
    };

    // Find the target user message position, then search for context-clear/compressed BEFORE it
    let target_pos = remaining_messages
        .iter()
        .position(|m| m.id == last_user_msg.id);
    let search_range = match target_pos {
        Some(pos) => &remaining_messages[..pos],
        None => &remaining_messages[..],
    };
    let clear_idx = search_range.iter().rposition(|m| {
        m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
    });
    let effective_messages = match clear_idx {
        Some(idx) => &remaining_messages[idx + 1..],
        None => &remaining_messages[..],
    };

    for m in effective_messages {
        if m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
        {
            continue;
        }
        // Skip error messages — they should not be sent as context
        if m.status == "error" {
            continue;
        }
        // Include messages up to and including the last user message
        chat_messages.push(chat_message_from_message(&file_store, m).map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?);
        // Stop after the user message we're regenerating from
        if m.id == last_user_msg.id {
            break;
        }
    }

    // 7. Spawn streaming with new version
    let assistant_message_id = axagent_kit::utils::gen_id();

    let global_settings = axagent_dao::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // Load MCP tools for enabled servers
    let mcp_ids: Vec<String> = enabled_mcp_server_ids
        .unwrap_or_default()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    // Check if any search provider is configured — auto-include web_search
    let has_search_provider =
        axagent_dao::repo::search_provider::list_search_providers(state.harness.db())
            .await
            .map(|providers| providers.iter().any(|p| p.enabled))
            .unwrap_or(false);
    // 从数据库加载全局禁用工具列表（与 agent 模式 load_enabled_state 一致）
    // TODO: group_enabled 过滤需要 tool_registry.load_enabled_state(db)，
    // streaming 流程中未创建 tool_registry，暂不实现组级别过滤。
    let disabled_tools_set: std::collections::HashSet<String> =
        axagent_harness::repositories::settings_repository()
            .get_setting("disabled_tools")
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
            .map(|v| v.into_iter().collect())
            .unwrap_or_default();
    let tools: Option<Vec<ChatTool>> = if mcp_ids.is_empty() && !has_search_provider {
        None
    } else {
        let mut all_tools = Vec::new();
        if has_search_provider {
            all_tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: "web_search".to_string(),
                    description: Some(
                        "MUST use this to search the internet for current, real-time, or recent information. Call this function whenever the user asks about: today's news, current events, latest developments, stock prices, weather, sports scores, or any topic that requires up-to-date information beyond your knowledge cutoff. The search returns relevant web results. Do NOT tell users you cannot access real-time data — use this tool instead.".to_string()
                    ),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": { "query": { "type": "string", "description": "The search query" } },
                        "required": ["query"]
                    })),
                },
            });
        }
        // Auto-include builtin local tools — mirrors UnifiedToolRegistry register_all()
        // Tool names MUST match the `fn name()` return value of each tool implementation
        let builtin_local_tools: &[(&str, &str)] = &[
            ("Skill", "加载预注册的 Skill。skill: Skill名称, args: 可选参数。"),
            ("DiscoverSkills", "搜索已安装的 Skill。query: 名称/描述关键词。"),
            ("FileRead", "读取文件。file_path: 路径, offset: 起始行, limit: 行数。"),
            ("FileWrite", "创建/覆盖文件。file_path: 路径, content: 内容。"),
            (
                "FileEdit",
                "精确编辑文件。file_path: 路径, old_string: 旧文本, new_string: 新文本。",
            ),
            ("Glob", "glob 搜索文件。pattern: glob模式。"),
            ("Grep", "正则搜索文件内容。pattern: 正则表达式。"),
            ("Bash", "执行 shell 命令。command: 命令, description: 说明。"),
            ("WebFetch", "获取 URL 内容。url: 目标URL。"),
            ("WebSearch", "搜索互联网。query: 搜索词。"),
            ("TaskCreate", "创建后台任务。subject: 标题, description: 描述。"),
            ("TaskList", "列出所有任务。"),
            ("TaskUpdate", "更新任务状态。taskId: ID, status: 新状态。"),
            ("TodoWrite", "管理待办事项。"),
            ("Agent", "启动子Agent处理复杂任务。"),
            ("EnterPlanMode", "进入计划模式。"),
            ("ListDirectory", "列出目录。path: 路径。"),
            ("DeleteFile", "删除文件。file_path: 路径。"),
        ];
        for (name, desc) in builtin_local_tools {
            // 过滤被禁用的内置工具
            if disabled_tools_set.contains(*name) {
                continue;
            }
            all_tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: (*name).to_owned(),
                    description: Some((*desc).to_owned()),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {},
                    })),
                },
            });
        }
        for server_id in &mcp_ids {
            if let Ok(descriptors) =
                axagent_dao::repo::mcp_server::list_tools_for_server(state.harness.db(), server_id)
                    .await
            {
                for td in descriptors {
                    // 过滤被禁用的 MCP 工具
                    if disabled_tools_set.contains(&td.name) {
                        continue;
                    }
                    let parameters: Option<serde_json::Value> = td
                        .input_schema_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok());
                    all_tools.push(ChatTool {
                        r#type: "function".to_string(),
                        function: ChatToolFunction {
                            name: td.name,
                            description: td.description,
                            parameters,
                        },
                    });
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        all_tools.retain(|t| seen.insert(t.function.name.clone()));
        if all_tools.is_empty() {
            None
        } else {
            Some(all_tools)
        }
    };

    let regen_model_overrides = axagent_dao::repo::provider::get_model(
        state.harness.db(),
        &conversation.provider_id,
        &conversation.model_id,
    )
    .await
    .ok()
    .and_then(|m| m.param_overrides);
    let use_max_completion_tokens = regen_model_overrides
        .as_ref()
        .and_then(|p| p.use_max_completion_tokens);
    let force_max_tokens = regen_model_overrides
        .as_ref()
        .and_then(|p| p.force_max_tokens);
    let no_system_role = regen_model_overrides
        .as_ref()
        .and_then(|p| p.no_system_role)
        .unwrap_or(false);
    let thinking_param_style = regen_model_overrides
        .as_ref()
        .and_then(|p| p.thinking_param_style.clone());
    let regen_request_delay_ms = regen_model_overrides
        .as_ref()
        .and_then(|p| p.request_delay_ms);

    // Convert system messages to user messages if model doesn't support system role
    if no_system_role {
        for msg in &mut chat_messages {
            if msg.role == "system" {
                msg.role = "user".to_string();
            }
        }
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    if state.stream_cancel_flags.contains_key(&conversation_id) {
        // Stale-flag recovery: if a cancel flag exists but no stream is actually
        // running, wait briefly and remove it so the new request can proceed.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if state.stream_cancel_flags.contains_key(&conversation_id) {
            tracing::warn!(
                "[regenerate_message] Removing stale cancel flag for {}",
                conversation_id
            );
            state.stream_cancel_flags.remove(&conversation_id);
        }
    }
    state
        .stream_cancel_flags
        .insert(conversation_id.clone(), cancel_flag.clone());
    tracing::info!("[regenerate_message] Step final: spawning stream task");
    spawn_stream_task(
        app,
        state.harness.db().clone(),
        state.harness.clone(),
        StreamTaskParams {
            conversation_id,
            assistant_message_id,
            conversation,
            provider,
            ctx,
            chat_messages,
            is_first_message: false,
            user_content: last_user_msg.content,
            parent_message_id: last_user_msg.id,
            version_index: new_version_index,
            tools,
            thinking_budget,
            mcp_server_ids: mcp_ids,
            override_created_at: original_created_at,
            use_max_completion_tokens,
            force_max_tokens,
            thinking_param_style,
            request_delay_ms: regen_request_delay_ms,
            settings: global_settings,
            cancel_flag,
            cancel_flags: state.stream_cancel_flags.clone(),
            content_prefix: memory_tag,
            create_inactive: false,
            skip_placeholder_create: false,
        },
    );
    tracing::info!("[regenerate_message] Stream task spawned, returning Ok");
    Ok(())
}

#[tracing::instrument(skip(app, state))]
pub async fn regenerate_with_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: RegenerateWithModelParams,
) -> Result<(), String> {
    let RegenerateWithModelParams {
        conversation_id,
        user_message_id,
        target_provider_id,
        target_model_id,
        options,
        is_companion,
    } = params;
    let SendMessageOptions {
        enabled_mcp_server_ids,
        thinking_budget,
        enabled_knowledge_base_ids,
        enabled_memory_namespace_ids,
        enabled_wiki_ids,
    } = options;
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;

    let user_msg = messages
        .iter()
        .find(|m| m.id == user_message_id && m.role == MessageRole::User)
        .ok_or_else(|| format!("User message {} not found", user_message_id))?
        .clone();

    // Count existing versions and preserve original created_at
    let existing_versions = axagent_dao::repo::message::list_message_versions(
        state.harness.db(),
        &conversation_id,
        &user_msg.id,
    )
    .await
    .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let new_version_index = existing_versions.len() as i32;
    let original_created_at = existing_versions.first().map(|v| v.created_at);

    let companion = is_companion.unwrap_or(false);

    // Deactivate all existing versions (skip for companion models in multi-model mode)
    use axagent_entities::messages as msg_entity;
    use sea_orm::sea_query::Expr;
    if !companion {
        msg_entity::Entity::update_many()
            .filter(msg_entity::Column::ConversationId.eq(&conversation_id))
            .filter(msg_entity::Column::ParentMessageId.eq(&user_msg.id))
            .col_expr(msg_entity::Column::IsActive, Expr::value(0))
            .exec(state.harness.db())
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    }

    // Get conversation, but override model_id and provider_id to target values
    let mut conversation =
        axagent_dao::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    conversation.model_id = target_model_id;
    conversation.provider_id = target_provider_id.clone();

    // Use target provider instead of conversation's default
    let provider =
        axagent_dao::repo::provider::get_provider(state.harness.db(), &target_provider_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let key_row =
        axagent_dao::repo::provider::get_active_key(state.harness.db(), &target_provider_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let decrypted_key =
        axagent_crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;

    // Build context messages (same logic as regenerate_message)
    let remaining_messages =
        axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?;
    let file_store = axagent_storage::file_store::FileStore::new();
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Resolve effective system prompt: conversation → category → global default
    let effective_system_prompt = resolve_system_prompt(state.harness.db(), &conversation).await;

    if let Some(ref sys) = effective_system_prompt {
        tracing::info!(
            "[regenerate_with_model] model={} provider={} effective_system_prompt='{}'",
            &conversation.model_id,
            &conversation.provider_id,
            &sys[..sys.len().min(80)]
        );
        chat_messages.push(ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(sys.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    } else {
        tracing::info!(
            "[regenerate_with_model] model={} provider={} NO system prompt",
            &conversation.model_id,
            &conversation.provider_id
        );
    }

    // RAG retrieval: resolve from context_sources when explicit IDs are not provided
    let memory_tag = {
        let (kb_ids, mem_ids, wiki_ids) = resolve_rag_ids(
            state.harness.db(),
            &conversation_id,
            enabled_knowledge_base_ids,
            enabled_memory_namespace_ids,
            enabled_wiki_ids,
        )
        .await;
        let mut rag_result = crate::indexing::collect_rag_context(
            state.harness.db(),
            state.harness.master_key(),
            &state.vector_store,
            &kb_ids,
            &mem_ids,
            &wiki_ids,
            &user_msg.content,
            5,
        )
        .await;

        let tag = build_memory_retrieval_tag(&rag_result.source_results);

        // Always emit so frontend can replace the searching indicator
        let _ = app.emit(
            "rag-context-retrieved",
            RagContextRetrievedEvent {
                conversation_id: conversation_id.clone(),
                sources: rag_result.source_results,
            },
        );

        let wm_content_3: String;
        {
            let ms = state.memory_service.read().await;
            wm_content_3 = ms.format_for_prompt().await;
        }

        if !rag_result.context_parts.is_empty() {
            dedup_rag_against_working_memory(&wm_content_3, &mut rag_result.context_parts);
            let rag_budget = crate::context_manager::token_budget::RETRIEVED_MEMORIES;
            let rag_items = apply_rag_token_budget(&rag_result.context_parts, rag_budget);
            if let Some(msg) = build_rag_chat_message(&rag_items) {
                chat_messages.push(msg);
            }
        }
        if let Some(msg) = build_working_memory_chat_message(&wm_content_3) {
            chat_messages.push(msg);
        }
        tag
    };

    // Context building with context-clear/compressed handling
    let target_pos = remaining_messages.iter().position(|m| m.id == user_msg.id);
    let search_range = match target_pos {
        Some(pos) => &remaining_messages[..pos],
        None => &remaining_messages[..],
    };
    let clear_idx = search_range.iter().rposition(|m| {
        m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
    });
    let effective_messages = match clear_idx {
        Some(idx) => &remaining_messages[idx + 1..],
        None => &remaining_messages[..],
    };
    for m in effective_messages {
        if m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
        {
            continue;
        }
        // Skip error messages — they should not be sent as context
        if m.status == "error" {
            continue;
        }
        chat_messages.push(chat_message_from_message(&file_store, m).map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))?);
        if m.id == user_msg.id {
            break;
        }
    }

    let assistant_message_id = axagent_kit::utils::gen_id();
    let global_settings = axagent_dao::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let mcp_ids: Vec<String> = enabled_mcp_server_ids
        .unwrap_or_default()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let has_search_provider =
        axagent_dao::repo::search_provider::list_search_providers(state.harness.db())
            .await
            .map(|providers| providers.iter().any(|p| p.enabled))
            .unwrap_or(false);
    // 从数据库加载全局禁用工具列表（与 agent 模式 load_enabled_state 一致）
    // TODO: group_enabled 过滤需要 tool_registry.load_enabled_state(db)，
    // streaming 流程中未创建 tool_registry，暂不实现组级别过滤。
    let disabled_tools_set: std::collections::HashSet<String> =
        axagent_harness::repositories::settings_repository()
            .get_setting("disabled_tools")
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
            .map(|v| v.into_iter().collect())
            .unwrap_or_default();
    let tools: Option<Vec<ChatTool>> = if mcp_ids.is_empty() && !has_search_provider {
        None
    } else {
        let mut all_tools = Vec::new();
        if has_search_provider {
            all_tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: "web_search".to_string(),
                    description: Some(
                        "MUST use this to search the internet for current, real-time, or recent information. Call this function whenever the user asks about: today's news, current events, latest developments, stock prices, weather, sports scores, or any topic that requires up-to-date information beyond your knowledge cutoff. The search returns relevant web results. Do NOT tell users you cannot access real-time data — use this tool instead.".to_string()
                    ),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": { "query": { "type": "string", "description": "The search query" } },
                        "required": ["query"]
                    })),
                },
            });
        }
        // Auto-include builtin local tools — mirrors UnifiedToolRegistry register_all()
        // Tool names MUST match the `fn name()` return value of each tool implementation
        let builtin_local_tools: &[(&str, &str)] = &[
            ("Skill", "加载预注册的 Skill。skill: Skill名称, args: 可选参数。"),
            ("DiscoverSkills", "搜索已安装的 Skill。query: 名称/描述关键词。"),
            ("FileRead", "读取文件。file_path: 路径, offset: 起始行, limit: 行数。"),
            ("FileWrite", "创建/覆盖文件。file_path: 路径, content: 内容。"),
            (
                "FileEdit",
                "精确编辑文件。file_path: 路径, old_string: 旧文本, new_string: 新文本。",
            ),
            ("Glob", "glob 搜索文件。pattern: glob模式。"),
            ("Grep", "正则搜索文件内容。pattern: 正则表达式。"),
            ("Bash", "执行 shell 命令。command: 命令, description: 说明。"),
            ("WebFetch", "获取 URL 内容。url: 目标URL。"),
            ("WebSearch", "搜索互联网。query: 搜索词。"),
            ("TaskCreate", "创建后台任务。subject: 标题, description: 描述。"),
            ("TaskList", "列出所有任务。"),
            ("TaskUpdate", "更新任务状态。taskId: ID, status: 新状态。"),
            ("TodoWrite", "管理待办事项。"),
            ("Agent", "启动子Agent处理复杂任务。"),
            ("EnterPlanMode", "进入计划模式。"),
            ("ListDirectory", "列出目录。path: 路径。"),
            ("DeleteFile", "删除文件。file_path: 路径。"),
        ];
        for (name, desc) in builtin_local_tools {
            // 过滤被禁用的内置工具
            if disabled_tools_set.contains(*name) {
                continue;
            }
            all_tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: (*name).to_owned(),
                    description: Some((*desc).to_owned()),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {},
                    })),
                },
            });
        }
        for server_id in &mcp_ids {
            if let Ok(descriptors) =
                axagent_dao::repo::mcp_server::list_tools_for_server(state.harness.db(), server_id)
                    .await
            {
                for td in descriptors {
                    // 过滤被禁用的 MCP 工具
                    if disabled_tools_set.contains(&td.name) {
                        continue;
                    }
                    let parameters: Option<serde_json::Value> = td
                        .input_schema_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok());
                    all_tools.push(ChatTool {
                        r#type: "function".to_string(),
                        function: ChatToolFunction {
                            name: td.name,
                            description: td.description,
                            parameters,
                        },
                    });
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        all_tools.retain(|t| seen.insert(t.function.name.clone()));
        if all_tools.is_empty() {
            None
        } else {
            Some(all_tools)
        }
    };

    let rwm_overrides = axagent_dao::repo::provider::get_model(
        state.harness.db(),
        &conversation.provider_id,
        &conversation.model_id,
    )
    .await
    .ok()
    .and_then(|m| m.param_overrides);
    let use_max_completion_tokens = rwm_overrides
        .as_ref()
        .and_then(|p| p.use_max_completion_tokens);
    let force_max_tokens = rwm_overrides.as_ref().and_then(|p| p.force_max_tokens);
    let no_system_role = rwm_overrides
        .as_ref()
        .and_then(|p| p.no_system_role)
        .unwrap_or(false);
    let thinking_param_style = rwm_overrides
        .as_ref()
        .and_then(|p| p.thinking_param_style.clone());
    let rwm_request_delay_ms = rwm_overrides.as_ref().and_then(|p| p.request_delay_ms);

    if no_system_role {
        for msg in &mut chat_messages {
            if msg.role == "system" {
                msg.role = "user".to_string();
            }
        }
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    state
        .stream_cancel_flags
        .insert(conversation_id.clone(), cancel_flag.clone());

    // Pre-create the placeholder message BEFORE spawning the stream task so that
    // the frontend can immediately discover it via listMessageVersions and enable
    // model switching in ModelTags without waiting for the first stream chunk.
    {
        use sea_orm::ActiveValue::Set;
        if let Err(e) = (axagent_entities::messages::ActiveModel {
            id: Set(assistant_message_id.clone()),
            conversation_id: Set(conversation_id.clone()),
            role: Set("assistant".to_string()),
            content: Set(String::new()),
            provider_id: Set(Some(provider.id.clone())),
            model_id: Set(Some(conversation.model_id.clone())),
            token_count: Set(None),
            prompt_tokens: Set(None),
            completion_tokens: Set(None),
            attachments: Set("[]".to_string()),
            thinking: Set(None),
            created_at: Set(original_created_at.unwrap_or_else(axagent_kit::utils::now_ts)),
            branch_id: Set(None),
            parent_message_id: Set(Some(user_msg.id.clone())),
            version_index: Set(new_version_index),
            is_active: Set(if companion { 0 } else { 1 }),
            tool_calls_json: Set(None),
            tool_call_id: Set(None),
            status: Set("partial".to_string()),
            tokens_per_second: Set(None),
            first_token_latency_ms: Set(None),
            parts: Set(None),
            cache_creation_tokens: Set(None),
            cache_read_tokens: Set(None),
        })
        .insert(state.harness.db())
        .await
        {
            tracing::error!("Failed to pre-create placeholder message: {}", e);
        }
    }

    tracing::info!(
        "[regenerate_with_model] spawning stream: model={} total_messages={} has_system_prompt={}",
        &conversation.model_id,
        chat_messages.len(),
        chat_messages
            .first()
            .map(|m| m.role == "system")
            .unwrap_or(false)
    );
    spawn_stream_task(
        app,
        state.harness.db().clone(),
        state.harness.clone(),
        StreamTaskParams {
            conversation_id,
            assistant_message_id,
            conversation,
            provider,
            ctx,
            chat_messages,
            is_first_message: false,
            user_content: user_msg.content,
            parent_message_id: user_msg.id,
            version_index: new_version_index,
            tools,
            thinking_budget,
            mcp_server_ids: mcp_ids,
            override_created_at: original_created_at,
            use_max_completion_tokens,
            force_max_tokens,
            thinking_param_style,
            request_delay_ms: rwm_request_delay_ms,
            settings: global_settings,
            cancel_flag,
            cancel_flags: state.stream_cancel_flags.clone(),
            content_prefix: memory_tag,
            create_inactive: companion,
            skip_placeholder_create: true,
        },
    );
    Ok(())
}
