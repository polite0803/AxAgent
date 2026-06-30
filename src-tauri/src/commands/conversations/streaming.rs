// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::AppState;
#[cfg(test)]
use crate::app_state::SemanticCacheState;
#[cfg(test)]
use crate::commands::proactive::ProactiveService;
use axagent_agent::clean_output;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_providers::ProviderRequestContext;
#[cfg(test)]
use axagent_runtime_core::prompt_cache::PromptCache;
use dashmap::DashMap;
use sea_orm::*;
#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::{Emitter, State};

fn spawn_stream_task(
    app: tauri::AppHandle,
    db: sea_orm::DatabaseConnection,
    harness: axagent_runtime::harness::RuntimeHarness,
    params: StreamTaskParams,
) {
    let StreamTaskParams {
        conversation_id,
        assistant_message_id,
        conversation,
        provider,
        ctx,
        chat_messages,
        is_first_message,
        user_content,
        parent_message_id,
        version_index,
        tools,
        thinking_budget,
        mcp_server_ids,
        override_created_at,
        use_max_completion_tokens,
        force_max_tokens,
        thinking_param_style,
        request_delay_ms,
        settings,
        cancel_flag,
        cancel_flags,
        content_prefix,
        create_inactive,
        skip_placeholder_create,
    } = params;
    let model_id = conversation.model_id.clone();
    let harness = harness.clone();

    tokio::spawn(async move {
        // 确保 panic 后 cancel_flag 一定被清理
        struct CancelGuard {
            flags: Arc<DashMap<String, Arc<AtomicBool>>>,
            key: String,
        }
        impl Drop for CancelGuard {
            fn drop(&mut self) {
                self.flags.remove(&self.key);
            }
        }
        let _cancel_guard = CancelGuard {
            flags: cancel_flags.clone(),
            key: conversation_id.clone(),
        };

        let future = std::panic::AssertUnwindSafe(async {
            // --- 原始 stream task 主体 ---
            let registry_key = provider.provider_type.registry_key();
            let adapter = match harness.provider_registry().get(registry_key) {
                Some(a) => a,
                None => {
                    let _ = app.emit(
                        "chat-stream-error",
                        ChatStreamErrorEvent {
                            conversation_id: conversation_id.clone(),
                            message_id: assistant_message_id.clone(),
                            error: format!("Unsupported provider type: {}", registry_key),
                        },
                    );
                    return;
                },
            };

            const MAX_TOOL_ITERATIONS: usize = 10;
            const MAX_CONSECUTIVE_TOOL_ERRORS: usize = 3;
            let mut chat_messages = chat_messages;
            let mut iteration = 0;
            let mut total_content = String::new();
            let mut total_usage: Option<TokenUsage> = None;
            let mut final_tool_calls_json: Option<String> = None;
            let mut had_stream_error = false;
            let mut last_stream_error: Option<String> = None;
            let mut final_tokens_per_second: Option<f64> = None;
            let mut final_first_token_latency_ms: Option<i64> = None;
            let mut consecutive_tool_errors: usize = 0;

            // Early create: persist a placeholder message so it survives crash/refresh
            // Skip if the caller already created the placeholder before spawning.
            if !skip_placeholder_create {
                if let Err(e) =
                    (axagent_core::entity::messages::ActiveModel {
                        id: Set(assistant_message_id.clone()),
                        conversation_id: Set(conversation_id.clone()),
                        role: Set("assistant".to_string()),
                        content: Set(String::new()),
                        provider_id: Set(Some(provider.id.clone())),
                        model_id: Set(Some(model_id.clone())),
                        token_count: Set(None),
                        prompt_tokens: Set(None),
                        completion_tokens: Set(None),
                        attachments: Set("[]".to_string()),
                        thinking: Set(None),
                        created_at: Set(
                            override_created_at.unwrap_or_else(axagent_core::utils::now_ts)
                        ),
                        branch_id: Set(None),
                        parent_message_id: Set(Some(parent_message_id.clone())),
                        version_index: Set(version_index),
                        is_active: Set(if create_inactive { 0 } else { 1 }),
                        tool_calls_json: Set(None),
                        tool_call_id: Set(None),
                        status: Set("partial".to_string()),
                        tokens_per_second: Set(None),
                        first_token_latency_ms: Set(None),
                        parts: Set(None),
                        cache_creation_tokens: Set(None),
                        cache_read_tokens: Set(None),
                    })
                    .insert(&db)
                    .await
                {
                    tracing::error!("Failed to create placeholder assistant message: {}", e);
                }
            }

            'tool_loop: loop {
                iteration += 1;
                if iteration > MAX_TOOL_ITERATIONS {
                    tracing::warn!(
                        "Tool call loop exceeded max iterations ({})",
                        MAX_TOOL_ITERATIONS
                    );
                    break;
                }

                // Check cancellation before starting a new iteration
                if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                    tracing::info!(
                        "[spawn_stream_task] Cancelled by user before iteration {}",
                        iteration
                    );
                    break;
                }

                // Apply request delay to avoid rate limits (per-model configuration)
                if let Some(delay_ms) = request_delay_ms {
                    if delay_ms > 0 && iteration > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }

                let request = ChatRequest {
                    model: model_id.clone(),
                    messages: chat_messages.clone(),
                    stream: true,
                    temperature: conversation.temperature.map(|v| v as f64),
                    top_p: conversation.top_p.map(|v| v as f64),
                    max_tokens: if force_max_tokens == Some(true) {
                        conversation.max_tokens.or(Some(4096))
                    } else {
                        conversation.max_tokens
                    },
                    tools: tools.clone(),
                    thinking_budget,
                    use_max_completion_tokens,
                    thinking_param_style: thinking_param_style.clone(),
                    api_mode: None,
                    instructions: None,
                    conversation: None,
                    previous_response_id: None,
                    store: None,
                };

                let mut stream = adapter.chat_stream(&ctx, request, None);
                let suppress_thinking = thinking_budget == Some(0);
                let (content, usage, tool_calls, stream_error, iter_tps, iter_ttft) =
                    consume_stream(
                        &app,
                        &mut stream,
                        StreamConsumptionParams {
                            conversation_id: &conversation_id,
                            message_id: &assistant_message_id,
                            model_id: &model_id,
                            provider_id: &provider.id,
                            cancel_flag: &cancel_flag,
                            suppress_thinking,
                        },
                    )
                    .await;

                total_content.push_str(&content);
                if usage.is_some() {
                    total_usage = usage;
                }
                // Keep first iteration's TTFT, last iteration's TPS
                if final_first_token_latency_ms.is_none() {
                    final_first_token_latency_ms = iter_ttft;
                }
                if iter_tps.is_some() {
                    final_tokens_per_second = iter_tps;
                }

                // If stream errored, save what we have and break
                if stream_error.is_some() {
                    last_stream_error = stream_error;
                    had_stream_error = true;
                    break;
                }

                // If no tool calls, we're done
                let tool_calls = match tool_calls {
                    Some(tc) if !tc.is_empty() => tc,
                    _ => {
                        // Final iteration has no tool calls — clear any stale value so the
                        // stored message won't carry orphaned tool_calls_json (which would
                        // break context for subsequent requests since the matching tool
                        // response messages are stored as is_active=0 and excluded from
                        // list_messages).
                        final_tool_calls_json = None;
                        break;
                    },
                };

                // Save the tool_calls JSON for the final message
                let tc_json = serde_json::to_string(&tool_calls).ok();
                final_tool_calls_json = tc_json.clone();

                // Add assistant message with tool_calls to chat history for next round
                // Strip <think> tags from the assistant content sent to the provider
                let stripped_content = strip_think_tags(&content);
                chat_messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: ChatContent::Text(stripped_content),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    thinking: None,
                });

                // Persist the intermediate assistant message with tool_calls
                // Returns the generated ID so tool results can reference it as parent
                let intermediate_msg_id =
                    axagent_core::repo::message::create_assistant_tool_call_message(
                        &db,
                        &conversation_id,
                        &content,
                        tc_json.as_deref(),
                        &provider.id,
                        &model_id,
                        &parent_message_id,
                    )
                    .await
                    .unwrap_or_else(|_| axagent_core::utils::gen_id());

                // Execute each tool call
                for tc in &tool_calls {
                    // Look up server name for events
                    let server_name = if tc.function.name == "web_search" {
                        "Web Search".to_string()
                    } else {
                        match axagent_core::repo::mcp_server::find_server_for_tool(
                            &db,
                            &tc.function.name,
                            &mcp_server_ids,
                        )
                        .await
                        {
                            Ok(Some((srv, _))) => srv.name.clone(),
                            _ => "unknown".to_string(),
                        }
                    };

                    // Emit :::mcp opener as stream chunk — frontend shows loading state
                    let metadata = serde_json::json!({
                        "name": server_name,
                        "tool": tc.function.name,
                        "id": tc.id,
                        "arguments": tc.function.arguments,
                    });
                    let mcp_opener = format!("\n\n:::mcp {}\n", metadata);
                    total_content.push_str(&mcp_opener);
                    let _ = app.emit(
                        "chat-stream-chunk",
                        ChatStreamEvent {
                            conversation_id: conversation_id.clone(),
                            message_id: assistant_message_id.clone(),
                            model_id: Some(model_id.clone()),
                            provider_id: Some(provider.id.clone()),
                            chunk: ChatStreamChunk {
                                content: Some(mcp_opener.clone()),
                                thinking: None,
                                done: false,
                                is_final: None,
                                usage: None,
                                tool_calls: None,
                            },
                        },
                    );

                    // Create execution record
                    let server_id_for_exec =
                        match axagent_core::repo::mcp_server::find_server_for_tool(
                            &db,
                            &tc.function.name,
                            &mcp_server_ids,
                        )
                        .await
                        {
                            Ok(Some((srv, _))) => srv.id.clone(),
                            _ => String::new(),
                        };
                    let exec = axagent_core::repo::tool_execution::create_tool_execution(
                        &db,
                        &conversation_id,
                        Some(&assistant_message_id),
                        &server_id_for_exec,
                        &tc.function.name,
                        Some(&tc.function.arguments),
                        None,
                    )
                    .await;

                    // Execute the tool
                    let start = std::time::Instant::now();
                    let (result_content, is_error) =
                        execute_tool_call(&db, tc, &mcp_server_ids, harness.master_key()).await;
                    let _duration_ms = start.elapsed().as_millis() as i64;

                    if is_error {
                        consecutive_tool_errors += 1;
                        if consecutive_tool_errors >= MAX_CONSECUTIVE_TOOL_ERRORS {
                            tracing::warn!(
                                "[spawn_stream_task] {} consecutive tool errors, stopping tool loop",
                                consecutive_tool_errors
                            );
                            break 'tool_loop;
                        }
                    } else {
                        consecutive_tool_errors = 0;
                    }

                    // Update execution record
                    if let Ok(ref exec) = exec {
                        let _ = axagent_core::repo::tool_execution::update_tool_execution_status(
                            &db,
                            &exec.id,
                            if is_error { "failed" } else { "success" },
                            Some(&result_content),
                            if is_error {
                                Some(&result_content)
                            } else {
                                None
                            },
                        )
                        .await;
                    }

                    // Emit :::mcp result + closer as stream chunk — frontend shows completed state
                    let mcp_closer = format!("{}\n:::\n\n", result_content);
                    total_content.push_str(&mcp_closer);
                    let _ = app.emit(
                        "chat-stream-chunk",
                        ChatStreamEvent {
                            conversation_id: conversation_id.clone(),
                            message_id: assistant_message_id.clone(),
                            model_id: Some(model_id.clone()),
                            provider_id: Some(provider.id.clone()),
                            chunk: ChatStreamChunk {
                                content: Some(mcp_closer.clone()),
                                thinking: None,
                                done: false,
                                is_final: None,
                                usage: None,
                                tool_calls: None,
                            },
                        },
                    );

                    // Persist tool result message to DB (parent is the intermediate assistant message)
                    let _ = axagent_core::repo::message::create_tool_result_message(
                        &db,
                        &conversation_id,
                        &tc.id,
                        &result_content,
                        &intermediate_msg_id,
                    )
                    .await;

                    // Add tool result to in-memory chat messages for next provider call
                    chat_messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: ChatContent::Text(result_content.to_string()),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                        thinking: None,
                    });
                }
                // Continue loop — will call provider again with tool results
            }

            // After loop: update the placeholder message with final content and status
            let was_cancelled = cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
            let final_status = if had_stream_error {
                "error"
            } else if was_cancelled {
                "partial"
            } else {
                "complete"
            };

            if had_stream_error {
                let err = last_stream_error.as_deref().unwrap_or("Unknown error");
                let base_url = ctx.base_url.as_deref().unwrap_or("(not set)");
                let api_path_display = ctx.api_path.as_deref().unwrap_or("(default)");
                let error_diag = format!(
                    "\n\n---\n⚠️ Stream Error: {}\nBase URL: {}\nAPI Path: {}\nModel: {}\nProvider: {} ({:?})",
                    err,
                    base_url,
                    api_path_display,
                    model_id,
                    provider.name,
                    provider.provider_type,
                );
                if total_content.is_empty() {
                    total_content = format!(
                        "{}\n\nBase URL: {}\nAPI Path: {}\nModel: {}\nProvider: {} ({:?})",
                        err,
                        base_url,
                        api_path_display,
                        model_id,
                        provider.name,
                        provider.provider_type,
                    );
                } else {
                    total_content.push_str(&error_diag);
                }
            }
            let token_count = total_usage.as_ref().map(|u| u.completion_tokens);
            let prompt_tokens = total_usage.as_ref().map(|u| u.prompt_tokens);
            let completion_tokens = total_usage.as_ref().map(|u| u.completion_tokens);
            // Prepend memory retrieval tag (if any) so it persists in DB
            let cleaned_total = clean_output(&total_content);
            let saved_content = if content_prefix.is_empty() {
                cleaned_total
            } else {
                format!("{}{}", content_prefix, cleaned_total)
            };
            if let Err(e) = axagent_core::entity::messages::Entity::update(
                axagent_core::entity::messages::ActiveModel {
                    id: Set(assistant_message_id.clone()),
                    content: Set(saved_content),
                    token_count: Set(token_count.map(|v| v as i64)),
                    prompt_tokens: Set(prompt_tokens.map(|v| v as i64)),
                    completion_tokens: Set(completion_tokens.map(|v| v as i64)),
                    thinking: Set(None), // thinking is now embedded in content as <think> tags
                    tool_calls_json: Set(final_tool_calls_json),
                    status: Set(final_status.to_string()),
                    tokens_per_second: Set(final_tokens_per_second),
                    first_token_latency_ms: Set(final_first_token_latency_ms),
                    ..Default::default()
                },
            )
            .exec(&db)
            .await
            {
                tracing::error!("Failed to update assistant message: {}", e);
            }

            // Increment message count for the assistant message
            if let Err(e) =
                axagent_core::repo::conversation::increment_message_count(&db, &conversation_id)
                    .await
            {
                tracing::error!("Failed to increment message count: {}", e);
            }

            // Auto-title: if this is the first user message, set conversation title
            if is_first_message {
                // Set truncated title immediately for instant feedback
                let fallback_title = if user_content.chars().count() > 30 {
                    format!("{}...", user_content.chars().take(30).collect::<String>())
                } else {
                    user_content.clone()
                };

                if let Err(e) = axagent_core::repo::conversation::update_conversation_title(
                    &db,
                    &conversation_id,
                    &fallback_title,
                )
                .await
                {
                    tracing::error!("Failed to auto-update title: {}", e);
                } else {
                    let _ = app.emit(
                        "conversation-title-updated",
                        ConversationTitleUpdatedEvent {
                            conversation_id: conversation_id.clone(),
                            title: fallback_title,
                        },
                    );
                }

                // Notify frontend that title generation is starting
                let _ = app.emit(
                    "conversation-title-generating",
                    ConversationTitleGeneratingEvent {
                        conversation_id: conversation_id.clone(),
                        generating: true,
                        error: None,
                    },
                );

                // Try AI-powered title generation — first message, so full
                // conversation is user message + current assistant response
                let auto_messages = vec![
                    (MessageRole::User, user_content.clone()),
                    (MessageRole::Assistant, total_content.clone()),
                ];
                let ai_title = generate_ai_title(
                    &harness,
                    &auto_messages,
                    TitleFallbackModel {
                        provider: &provider,
                        ctx: &ctx,
                        model_id: &model_id,
                    },
                    &settings,
                )
                .await;

                match ai_title {
                    Ok(title) => {
                        if let Err(e) = axagent_core::repo::conversation::update_conversation_title(
                            &db,
                            &conversation_id,
                            &title,
                        )
                        .await
                        {
                            tracing::error!("Failed to update AI-generated title: {}", e);
                            let _ = app.emit(
                                "conversation-title-generating",
                                ConversationTitleGeneratingEvent {
                                    conversation_id: conversation_id.clone(),
                                    generating: false,
                                    error: Some(format!("Failed to save title: {}", e)),
                                },
                            );
                        } else {
                            let _ = app.emit(
                                "conversation-title-updated",
                                ConversationTitleUpdatedEvent {
                                    conversation_id: conversation_id.clone(),
                                    title,
                                },
                            );
                            let _ = app.emit(
                                "conversation-title-generating",
                                ConversationTitleGeneratingEvent {
                                    conversation_id: conversation_id.clone(),
                                    generating: false,
                                    error: None,
                                },
                            );
                        }
                    },
                    Err(err) => {
                        tracing::warn!("Auto title generation failed: {}", err);
                        let _ = app.emit(
                            "conversation-title-generating",
                            ConversationTitleGeneratingEvent {
                                conversation_id: conversation_id.clone(),
                                generating: false,
                                error: Some(err),
                            },
                        );
                    },
                }
            }
        });

        // panic 保护：捕获 unwind 并 emit 错误事件
        let result = future.catch_unwind().await;
        match result {
            Ok(()) => {},
            Err(panic_err) => {
                let msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                    (*s).to_owned()
                } else {
                    "Unknown panic in stream task".to_string()
                };
                tracing::error!("[spawn_stream_task] PANIC: {}", msg);
                let _ = app.emit(
                    "chat-stream-error",
                    ChatStreamErrorEvent {
                        conversation_id: conversation_id.clone(),
                        message_id: assistant_message_id.clone(),
                        error: format!("Internal error: {}", msg),
                    },
                );
            },
        }

        // 发送 chat-stream-done 事件（前端释放 loading 状态）
        let _ = app.emit(
            "chat-stream-done",
            serde_json::json!({
                "conversationId": conversation_id,
                "messageId": assistant_message_id,
                "modelId": model_id,
                "providerId": provider.id,
            }),
        );

        // CancelGuard 在 drop 时自动清理 cancel_flag
    });
}

pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: SendMessageParams,
) -> Result<Message, String> {
    let SendMessageParams {
        conversation_id,
        content,
        attachments,
        options,
    } = params;
    let SendMessageOptions {
        enabled_mcp_server_ids,
        thinking_budget,
        enabled_knowledge_base_ids,
        enabled_memory_namespace_ids,
        enabled_wiki_ids,
    } = options;
    let persisted_attachments = persist_attachments(&state, &conversation_id, &attachments)
        .await
        .map_err(|e| e.to_string())?;

    // 1. Save user message to DB
    let user_message = axagent_core::repo::message::create_message(
        state.harness.db(),
        &conversation_id,
        MessageRole::User,
        &content,
        &persisted_attachments,
        None,
        0,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Increment the persisted message count
    axagent_core::repo::conversation::increment_message_count(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Get conversation details (provider_id, model_id)
    let conversation =
        axagent_core::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;

    // Check if this is the first message (message_count was 0 before we incremented)
    let is_first_message = conversation.message_count <= 1;

    // 3. Get provider config + decrypt key
    let provider =
        axagent_core::repo::provider::get_provider(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let key_row =
        axagent_core::repo::provider::get_active_key(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    // Get model info for param overrides and token budget
    let resolved_model = axagent_core::repo::provider::get_model(
        state.harness.db(),
        &conversation.provider_id,
        &conversation.model_id,
    )
    .await
    .ok();
    let model_param_overrides = resolved_model
        .as_ref()
        .and_then(|m| m.param_overrides.clone());
    let no_system_role = model_param_overrides
        .as_ref()
        .and_then(|p| p.no_system_role)
        .unwrap_or(false);
    let use_max_completion_tokens = model_param_overrides
        .as_ref()
        .and_then(|p| p.use_max_completion_tokens);
    let force_max_tokens = model_param_overrides
        .as_ref()
        .and_then(|p| p.force_max_tokens);
    let thinking_param_style = model_param_overrides
        .as_ref()
        .and_then(|p| p.thinking_param_style.clone());
    let request_delay_ms = model_param_overrides
        .as_ref()
        .and_then(|p| p.request_delay_ms);

    // 4. Build ChatRequest from conversation messages
    let db_messages =
        axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;
    let file_store = axagent_core::file_store::FileStore::new();

    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Load agent profile and inject role description if set
    if let Some(ref pid) = conversation.agent_profile_id {
        if let Ok(Some(profile)) =
            axagent_core::entity::agent_profiles::Entity::find_by_id(pid.as_str())
                .one(state.harness.db())
                .await
        {
            if profile.agent_role.is_some()
                || !profile.description.as_deref().unwrap_or("").is_empty()
            {
                let role_msg = format!(
                    "You are acting as: {}. Role: {}. {}",
                    profile.name,
                    profile.agent_role.as_deref().unwrap_or("general"),
                    profile.description.as_deref().unwrap_or("")
                );
                chat_messages.push(ChatMessage {
                    role: if no_system_role {
                        "user".to_string()
                    } else {
                        "system".to_string()
                    },
                    content: ChatContent::Text(role_msg),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                });
            }
        }
    }

    // Resolve effective system prompt: conversation → category → global default
    let effective_system_prompt = resolve_system_prompt(state.harness.db(), &conversation).await;

    // Prepend system prompt if present
    if let Some(ref sys) = effective_system_prompt {
        tracing::info!(
            "[send_message] model={} effective_system_prompt='{}'",
            &conversation.model_id,
            &sys[..sys.len().min(80)]
        );
        chat_messages.push(ChatMessage {
            role: if no_system_role {
                "user".to_string()
            } else {
                "system".to_string()
            },
            content: ChatContent::Text(sys.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    } else {
        tracing::info!("[send_message] model={} NO system prompt", &conversation.model_id);
    }

    // Inject current date + search hint
    {
        let now = chrono::Local::now();
        // i18n-exempt: LLM context injection (current date + web search hint) — model interaction data, not UI
        let date_msg = format!(
            "Current date: {}. IMPORTANT: You have access to a `web_search` function that can retrieve real-time information from the internet. You MUST use it whenever the user asks about current events, recent news, today's topics, latest updates, or any information that may have changed since your training cutoff. Do NOT claim you cannot access real-time data — call web_search instead.",
            now.format("%Y-%m-%d")
        );
        chat_messages.push(ChatMessage {
            role: if no_system_role {
                "user".to_string()
            } else {
                "system".to_string()
            },
            content: ChatContent::Text(date_msg),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    }

    // Inject output language directive from app settings
    if let Ok(settings) = axagent_core::repo::settings::get_settings(state.harness.db()).await {
        if !settings.language.is_empty() {
            let already_present = chat_messages.iter().any(|m| match &m.content {
                ChatContent::Text(t) => axagent_core::utils::has_output_language_directive(t),
                _ => false,
            });
            if !already_present {
                let directive =
                    axagent_core::utils::build_output_language_directive(&settings.language);
                chat_messages.push(ChatMessage {
                    role: if no_system_role {
                        "user".to_string()
                    } else {
                        "system".to_string()
                    },
                    content: ChatContent::Text(directive),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                });
            }
        }
    }

    // RAG retrieval: resolve from context_sources when explicit IDs are not provided
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
        &content,
        5,
    )
    .await;

    // Build memory retrieval tag for persistence before moving source_results
    let memory_tag = build_memory_retrieval_tag(&rag_result.source_results);

    // Always emit RAG results to frontend so it can replace the searching indicator
    let _ = app.emit(
        "rag-context-retrieved",
        RagContextRetrievedEvent {
            conversation_id: conversation_id.clone(),
            sources: rag_result.source_results.clone(),
        },
    );

    // Record retrieval hits for analytics
    {
        let hits: Vec<(String, String, String, f64, String)> = rag_result
            .source_results
            .iter()
            .flat_map(|src| {
                src.items.iter().map(|item| {
                    (
                        src.container_id.clone(),
                        item.document_id.clone(),
                        item.id.clone(),
                        item.score as f64,
                        item.content.chars().take(200).collect(),
                    )
                })
            })
            .collect();
        if !hits.is_empty() {
            let _ = axagent_core::repo::retrieval_hit::record_hits(
                state.harness.db(),
                &conversation_id,
                &user_message.id,
                &hits,
            )
            .await;
        }
    }

    let wm_content: String;
    {
        let ms = state.memory_service.read().await;
        wm_content = ms.format_for_prompt();
    }

    if !rag_result.context_parts.is_empty() {
        dedup_rag_against_working_memory(&wm_content, &mut rag_result.context_parts);
        let rag_budget = crate::context_manager::token_budget::RETRIEVED_MEMORIES;
        let rag_items = apply_rag_token_budget(&rag_result.context_parts, rag_budget);
        if let Some(msg) = build_rag_chat_message(&rag_items) {
            chat_messages.push(msg);
        }
    }

    if let Some(msg) = build_working_memory_chat_message(&wm_content) {
        chat_messages.push(msg);
    }

    // Find last context-clear or context-compressed marker to truncate history
    let marker_idx = db_messages.iter().rposition(|m| {
        m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
    });
    let effective_messages = match marker_idx {
        Some(idx) => &db_messages[idx + 1..],
        None => &db_messages[..],
    };

    let mut history_messages: Vec<ChatMessage> = Vec::new();
    for m in effective_messages {
        if m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
        {
            continue;
        }
        if m.role == MessageRole::Tool {
            continue;
        }
        if m.role == MessageRole::Assistant && m.tool_calls_json.is_some() {
            continue;
        }
        // Skip error messages — they should not be sent as context
        if m.status == "error" {
            continue;
        }
        history_messages
            .push(chat_message_from_message(&file_store, m).map_err(|e| e.to_string())?);
    }

    // Resolve proxy config early (needed for both summary generation and main request)
    let global_settings = axagent_core::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    // Get model info for token budget and param overrides
    // Get model context window for token budget (resolved_model fetched earlier)
    let model_context_window = resolved_model.as_ref().and_then(|m| m.max_tokens);

    // Load existing summary for this conversation
    let existing_summary =
        axagent_core::repo::conversation::get_summary(state.harness.db(), &conversation_id)
            .await
            .ok()
            .flatten();

    // Auto-compression: if enabled and tokens exceed threshold, compress now
    if conversation.context_compression
        && !history_messages.is_empty()
        && crate::context_manager::should_auto_compress(
            &chat_messages,
            &history_messages,
            model_context_window,
        )
    {
        // Perform synchronous compression before sending
        if let Ok(summary_text) = compress::do_compress(
            state.harness.db(),
            CompressContext {
                conversation_id: &conversation_id,
                history_messages: &history_messages,
                existing_summary: existing_summary.as_ref().map(|s| s.summary_text.as_str()),
                settings: &global_settings,
                master_key: state.harness.master_key(),
            },
            CompressProviderInfo {
                provider: &provider,
                decrypted_key: &decrypted_key,
                key_id: &key_row.id,
                proxy_config: &resolved_proxy,
                model_id: &conversation.model_id,
                use_max_completion_tokens,
            },
            &state.harness,
        )
        .await
        {
            // Insert compression marker
            let _ = axagent_core::repo::message::create_message(
                state.harness.db(),
                &conversation_id,
                MessageRole::System,
                crate::context_manager::COMPRESSION_MARKER,
                &[],
                None,
                0,
            )
            .await;

            // Emit marker to frontend
            let _ =
                app.emit(&format!("conversation:compressed:{}", conversation_id), &summary_text);

            // After compression, history is now empty (marker splits it)
            // Context = system + summary + current user message only
            chat_messages = crate::context_manager::build_context(
                &chat_messages,
                &[],
                Some(&summary_text),
                model_context_window,
            );
        } else {
            // Compression failed — fall back to sliding window
            chat_messages = crate::context_manager::build_context(
                &chat_messages,
                &history_messages,
                existing_summary.as_ref().map(|s| s.summary_text.as_str()),
                model_context_window,
            );
        }
    } else {
        // No auto-compression: use existing summary (if any) + sliding window
        chat_messages = crate::context_manager::build_context(
            &chat_messages,
            &history_messages,
            existing_summary.as_ref().map(|s| s.summary_text.as_str()),
            model_context_window,
        );
    }

    // 5. Generate assistant message ID upfront
    let assistant_message_id = axagent_core::utils::gen_id();

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

    // 6. Load MCP tools for enabled servers
    let mcp_ids: Vec<String> = enabled_mcp_server_ids
        .unwrap_or_default()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    // Check if any search provider is configured — auto-include web_search if so
    let has_search_provider =
        axagent_core::repo::search_provider::list_search_providers(state.harness.db())
            .await
            .map(|providers| {
                let enabled = providers.iter().any(|p| p.enabled);
                tracing::info!(
                    "[send_message] search providers: total={}, enabled={}",
                    providers.len(),
                    enabled
                );
                enabled
            })
            .unwrap_or(false);
    let tools: Option<Vec<ChatTool>> = if mcp_ids.is_empty() && !has_search_provider {
        None
    } else {
        let mut all_tools = Vec::new();
        // Auto-include web_search if any search provider is configured
        if has_search_provider {
            tracing::info!("[send_message] injecting web_search tool into tools list");
            all_tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: "web_search".to_string(),
                    description: Some(
                        "MUST use this to search the internet for current, real-time, or recent information. Call this function whenever the user asks about: today's news, current events, latest developments, stock prices, weather, sports scores, or any topic that requires up-to-date information beyond your knowledge cutoff. The search returns relevant web results. Do NOT tell users you cannot access real-time data — use this tool instead.".to_string()
                    ),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            }
                        },
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
                axagent_core::repo::mcp_server::list_tools_for_server(state.harness.db(), server_id)
                    .await
            {
                for td in descriptors {
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

    // 7. Spawn streaming in background
    // Convert all remaining system messages to user messages if model doesn't support system role
    if no_system_role {
        for msg in &mut chat_messages {
            if msg.role == "system" {
                msg.role = "user".to_string();
            }
        }
    }

    let user_msg_id = user_message.id.clone();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    if state.stream_cancel_flags.contains_key(&conversation_id) {
        return Err("已有正在进行的请求，请等待完成后再发送".to_string());
    }
    state
        .stream_cancel_flags
        .insert(conversation_id.clone(), cancel_flag.clone());
    spawn_stream_task(
        app,
        state.harness.db().clone(),
        state.harness.clone(),
        StreamTaskParams {
            conversation_id: conversation_id.clone(),
            assistant_message_id,
            conversation,
            provider,
            ctx,
            chat_messages,
            is_first_message,
            user_content: content,
            parent_message_id: user_msg_id,
            version_index: 0,
            tools,
            thinking_budget,
            mcp_server_ids: mcp_ids,
            override_created_at: Some(user_message.created_at + 1),
            use_max_completion_tokens,
            force_max_tokens,
            thinking_param_style,
            request_delay_ms,
            settings: global_settings,
            cancel_flag,
            cancel_flags: state.stream_cancel_flags.clone(),
            content_prefix: memory_tag,
            create_inactive: false,
            skip_placeholder_create: false,
        },
    );

    // Return the user message immediately
    Ok(user_message)
}

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
    let SendMessageOptions {
        enabled_mcp_server_ids,
        thinking_budget,
        enabled_knowledge_base_ids,
        enabled_memory_namespace_ids,
        enabled_wiki_ids,
    } = options;
    // 1. Get all active messages for the conversation
    let messages = axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

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

    // 2. Count existing AI reply versions for this user message
    let existing_versions = axagent_core::repo::message::list_message_versions(
        state.harness.db(),
        &conversation_id,
        &last_user_msg.id,
    )
    .await
    .map_err(|e| e.to_string())?;
    let new_version_index = existing_versions.len() as i32;

    // Preserve original created_at from first version to maintain message position
    let original_created_at = existing_versions.first().map(|v| v.created_at);

    // Find the currently active version's model to regenerate with the same model
    let active_version = existing_versions.iter().find(|v| v.is_active);
    let active_model_id = active_version.and_then(|v| v.model_id.clone());
    let active_provider_id = active_version.and_then(|v| v.provider_id.clone());

    // 3. Deactivate all existing AI reply versions for this user message
    use axagent_core::entity::messages as msg_entity;
    use sea_orm::sea_query::Expr;
    msg_entity::Entity::update_many()
        .filter(msg_entity::Column::ConversationId.eq(&conversation_id))
        .filter(msg_entity::Column::ParentMessageId.eq(&last_user_msg.id))
        .col_expr(msg_entity::Column::IsActive, Expr::value(0))
        .exec(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;

    // 4. Get conversation details
    let mut conversation =
        axagent_core::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;

    // Override conversation model_id/provider_id so spawn_stream_task uses the correct model
    if let Some(ref mid) = active_model_id {
        conversation.model_id = mid.clone();
    }
    if let Some(ref pid) = active_provider_id {
        conversation.provider_id = pid.clone();
    }

    // 5. Get provider config + decrypt key
    let provider =
        axagent_core::repo::provider::get_provider(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let key_row =
        axagent_core::repo::provider::get_active_key(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    // 6. Rebuild chat messages (active messages only — old inactive versions excluded)
    let remaining_messages =
        axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;
    let file_store = axagent_core::file_store::FileStore::new();

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
            wm_content_2 = ms.format_for_prompt();
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
        chat_messages.push(chat_message_from_message(&file_store, m).map_err(|e| e.to_string())?);
        // Stop after the user message we're regenerating from
        if m.id == last_user_msg.id {
            break;
        }
    }

    // 7. Spawn streaming with new version
    let assistant_message_id = axagent_core::utils::gen_id();

    let global_settings = axagent_core::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

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
        axagent_core::repo::search_provider::list_search_providers(state.harness.db())
            .await
            .map(|providers| providers.iter().any(|p| p.enabled))
            .unwrap_or(false);
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
                axagent_core::repo::mcp_server::list_tools_for_server(state.harness.db(), server_id)
                    .await
            {
                for td in descriptors {
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

    let regen_model_overrides = axagent_core::repo::provider::get_model(
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
        return Err("已有正在进行的请求，请等待完成后再发送".to_string());
    }
    state
        .stream_cancel_flags
        .insert(conversation_id.clone(), cancel_flag.clone());
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

    Ok(())
}

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
    let messages = axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    let user_msg = messages
        .iter()
        .find(|m| m.id == user_message_id && m.role == MessageRole::User)
        .ok_or_else(|| format!("User message {} not found", user_message_id))?
        .clone();

    // Count existing versions and preserve original created_at
    let existing_versions = axagent_core::repo::message::list_message_versions(
        state.harness.db(),
        &conversation_id,
        &user_msg.id,
    )
    .await
    .map_err(|e| e.to_string())?;
    let new_version_index = existing_versions.len() as i32;
    let original_created_at = existing_versions.first().map(|v| v.created_at);

    let companion = is_companion.unwrap_or(false);

    // Deactivate all existing versions (skip for companion models in multi-model mode)
    use axagent_core::entity::messages as msg_entity;
    use sea_orm::sea_query::Expr;
    if !companion {
        msg_entity::Entity::update_many()
            .filter(msg_entity::Column::ConversationId.eq(&conversation_id))
            .filter(msg_entity::Column::ParentMessageId.eq(&user_msg.id))
            .col_expr(msg_entity::Column::IsActive, Expr::value(0))
            .exec(state.harness.db())
            .await
            .map_err(|e| e.to_string())?;
    }

    // Get conversation, but override model_id and provider_id to target values
    let mut conversation =
        axagent_core::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;
    conversation.model_id = target_model_id;
    conversation.provider_id = target_provider_id.clone();

    // Use target provider instead of conversation's default
    let provider =
        axagent_core::repo::provider::get_provider(state.harness.db(), &target_provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let key_row =
        axagent_core::repo::provider::get_active_key(state.harness.db(), &target_provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    // Build context messages (same logic as regenerate_message)
    let remaining_messages =
        axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;
    let file_store = axagent_core::file_store::FileStore::new();
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
            wm_content_3 = ms.format_for_prompt();
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
        chat_messages.push(chat_message_from_message(&file_store, m).map_err(|e| e.to_string())?);
        if m.id == user_msg.id {
            break;
        }
    }

    let assistant_message_id = axagent_core::utils::gen_id();
    let global_settings = axagent_core::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

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
        axagent_core::repo::search_provider::list_search_providers(state.harness.db())
            .await
            .map(|providers| providers.iter().any(|p| p.enabled))
            .unwrap_or(false);
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
                axagent_core::repo::mcp_server::list_tools_for_server(state.harness.db(), server_id)
                    .await
            {
                for td in descriptors {
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

    let rwm_overrides = axagent_core::repo::provider::get_model(
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
        if let Err(e) = (axagent_core::entity::messages::ActiveModel {
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
            created_at: Set(original_created_at.unwrap_or_else(axagent_core::utils::now_ts)),
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
