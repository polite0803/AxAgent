use crate::commands::spawn_guard::catch_unwind_logged;
use axagent_harness::types::conversation::ChatStreamChunk;
use axagent_harness::types::conversation::MessageRole;
use axagent_harness::types::function_call::ToolCall;
use axagent_harness::types::provider::DisabledThinkingStripState;
use axagent_harness::types::provider::ProviderConfig;
use axagent_harness::types::provider::ProviderProxyConfig;
use axagent_harness::types::provider::ProviderRequestContext;
use axagent_harness::types::settings::AppSettings;
use axagent_harness::types::settings::TitleFallbackModel;
use axagent_harness::types::settings_chat::ChatContent;
use axagent_harness::types::streaming::StreamConsumptionParams;
use axagent_runtime_core::Conversation;
use axagent_runtime_core::ConversationSearchResult;
use axagent_runtime_core::TokenUsage;
use axagent_runtime_core::UpdateConversationInput;
use crate::app_state::AppState as State;
use crate::app_state::AppState;
use crate::commands::agent::resolve_base_url_for_type;
use crate::commands::conversations::extract_reasoning_from_text;
use crate::commands::conversations::sync_context_sources;
use super::messages::{get_thinking_block_end, get_thinking_block_start, strip_disabled_thinking_content, strip_disabled_thinking_delta, strip_think_tags};
use crate::commands::error::ErrorResponse;

#[tauri::command]
pub async fn list_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>, String> {
    axagent_dao::repo::conversation::list_conversations(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, AppState>,
    title: String,
    model_id: String,
    provider_id: String,
    system_prompt: Option<String>,
) -> Result<Conversation, String> {
    axagent_dao::repo::conversation::create_conversation(
        state.harness.db(),
        &title,
        &model_id,
        &provider_id,
        system_prompt.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_conversation(
    state: State<'_, AppState>,
    id: String,
    input: UpdateConversationInput,
) -> Result<Conversation, String> {
    let needs_sync = input.enabled_knowledge_base_ids.is_some()
        || input.enabled_memory_namespace_ids.is_some()
        || input.enabled_wiki_ids.is_some();

    let updated =
        axagent_dao::repo::conversation::update_conversation(state.harness.db(), &id, input)
            .await
            .map_err(|e| e.to_string())?;

    if needs_sync {
        if let Err(e) = sync_context_sources(state.harness.db(), &id, &updated).await {
            tracing::warn!("Failed to sync context_sources for conversation {}: {}", id, e);
        }
    }

    Ok(updated)
}

#[tauri::command]
pub async fn delete_conversation(state: State<'_, AppState>, id: String) -> Result<(), String> {
    delete_conversation_with_attachments(state.harness.db(), &id).await
}

#[tauri::command]
pub async fn batch_delete_conversations(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    let db = state.harness.db().clone();
    let tasks: Vec<_> = ids
        .iter()
        .map(|id| {
            let db = db.clone();
            let id = id.clone();
            tokio::spawn(async move {
                let file_store = axagent_storage::file_store::FileStore::new();
                delete_conversation_with_attachments_using(&db, &file_store, &id).await
            })
        })
        .collect();
    let results = futures::future::join_all(tasks).await;
    let mut deleted = 0usize;
    for result in results {
        match result {
            Ok(Ok(())) => deleted += 1,
            Ok(Err(e)) => tracing::warn!("批量删除对话失败: {}", e),
            Err(e) => tracing::warn!("批量删除任务 panic: {}", e),
        }
    }
    Ok(deleted)
}

#[tauri::command]
pub async fn branch_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    until_message_id: String,
    as_child: bool,
    title: Option<String>,
) -> Result<Conversation, String> {
    axagent_dao::repo::conversation::branch_conversation(
        state.harness.db(),
        &conversation_id,
        &until_message_id,
        as_child,
        title.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

async fn delete_conversation_with_attachments(
    db: &sea_orm::DatabaseConnection,
    conversation_id: &str,
) -> Result<(), String> {
    let file_store = axagent_storage::file_store::FileStore::new();
    delete_conversation_with_attachments_using(db, &file_store, conversation_id).await
}

pub(super) async fn delete_conversation_with_attachments_using(
    db: &sea_orm::DatabaseConnection,
    file_store: &axagent_storage::file_store::FileStore,
    conversation_id: &str,
) -> Result<(), String> {
    let files =
        axagent_dao::repo::stored_file::list_stored_files_by_conversation(db, conversation_id)
            .await
            .map_err(|e| e.to_string())?;
    for file in files {
        super::file_cleanup::delete_attachment_reference(db, file_store, &file.id).await?;
    }

    // 清理关联数据（无 FK 约束，需手动删除避免孤行）
    if let Err(e) = axagent_dao::repo::conversation::delete_summary(db, conversation_id).await {
        tracing::warn!("Failed to delete conversation summary: {}", e);
    }
    if let Err(e) = axagent_entities::agent_sessions::Entity::delete_many()
        .filter(axagent_entities::agent_sessions::Column::ConversationId.eq(conversation_id))
        .exec(db)
        .await
    {
        tracing::warn!("Failed to delete agent sessions: {}", e);
    }

    axagent_dao::repo::conversation::delete_conversation(db, conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_conversations(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ConversationSearchResult>, String> {
    axagent_dao::repo::conversation::search_conversations(state.harness.db(), &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_pin_conversation(
    state: State<'_, AppState>,
    id: String,
) -> Result<Conversation, String> {
    axagent_dao::repo::conversation::toggle_pin(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_archive_conversation(
    state: State<'_, AppState>,
    id: String,
) -> Result<Conversation, String> {
    axagent_dao::repo::conversation::toggle_archive(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn archive_conversation_to_knowledge_base(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    knowledge_base_id: String,
) -> Result<Conversation, String> {
    let (updated_conv, doc) = axagent_dao::repo::conversation::archive_to_knowledge_base(
        state.harness.db(),
        &id,
        &knowledge_base_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Trigger async indexing for the newly created document
    let kb =
        axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &knowledge_base_id)
            .await
            .map_err(|e| e.to_string())?;

    if kb.embedding_provider.is_some() {
        let container = axagent_search::rag::KnowledgeContainer::from_knowledge_base(&kb);
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let vector_store = state.vector_store.clone();
        let doc_id = doc.id.clone();
        let src_path = doc.source_path.clone();
        let mime = doc.mime_type.clone();
        let semaphore = state.indexing_semaphore.clone();

        tokio::spawn(catch_unwind_logged("conversations.manage_archive_index", async move {
            let _permit = semaphore.acquire().await;
            let result = crate::indexing::index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                &doc_id,
                "",
                Some(&src_path),
                Some(&mime),
            )
            .await;

            if let Err(e) = &result {
                let err_msg = e.to_string();
                tracing::error!(
                    "Indexing failed for archived conversation doc {}: {}",
                    doc_id,
                    err_msg
                );
                let _ = axagent_dao::repo::knowledge::update_document_status_with_error(
                    &db,
                    &doc_id,
                    "failed",
                    Some(&err_msg),
                )
                .await;
            }

            let _ = app.emit(
                "knowledge-document-indexed",
                serde_json::json!({
                    "documentId": doc_id,
                    "success": result.is_ok(),
                    "error": result.err().map(|e| e.to_string()),
                }),
            );
        }));
    }

    Ok(updated_conv)
}

#[tauri::command]
pub async fn list_archived_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<Conversation>, String> {
    axagent_dao::repo::conversation::list_archived_conversations(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

/// 工作流型会话归档：将执行结果写回原始工作流模板
#[tauri::command]
pub async fn archive_workflow_session(
    state: State<'_, AppState>,
    conversation_id: String,
    feedback: Option<String>,
) -> Result<Conversation, String> {
    use axagent_entities::{conversations, workflow_template};
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = state.harness.db();

    let conv = conversations::Entity::find_by_id(&conversation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Conversation {} not found", conversation_id))?;

    use crate::commands::error::ErrorResponse;
    use crate::commands::error_code::conversation as conv_err;

    if conv.session_type != "workflow" {
        return Err(ErrorResponse::err_with_detail(
            conv_err::NOT_WORKFLOW,
            "此会话不是工作流类型，请使用普通归档",
        ));
    }

    if conv.is_archived != 0 {
        return Err(ErrorResponse::new(conv_err::ALREADY_ARCHIVED)
            .with_detail(format!("会话 {} 已经归档，请勿重复操作", conversation_id))
            .to_string());
    }

    // 如果有绑定的工作流模板，将执行数据写回模板
    if let Some(ref template_id) = conv.workflow_template_id {
        if workflow_template::Entity::find_by_id(template_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            let messages = axagent_dao::repo::message::list_messages(db, &conversation_id)
                .await
                .map_err(|e| e.to_string())?;

            let execution = axagent_entities::workflow_executions::ActiveModel {
                id: Set(uuid::Uuid::new_v4().to_string()),
                workflow_id: Set(template_id.clone()),
                status: Set(conv
                    .workflow_status
                    .clone()
                    .unwrap_or_else(|| "completed".to_string())),
                input_params: Set(None),
                output_result: Set(feedback.clone()),
                node_executions: Set(Some(
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "message_count": messages.len(),
                    })
                    .to_string(),
                )),
                total_time_ms: Set(None),
                created_at: Set(axagent_kit::utils::now_ts()),
                updated_at: Set(axagent_kit::utils::now_ts()),
            };
            execution.insert(db).await.map_err(|e| e.to_string())?;
        }
    }

    // 标记会话为已归档
    let now = chrono::Utc::now().timestamp_millis();
    let mut am: conversations::ActiveModel = conv.into();
    am.is_archived = Set(1);
    am.updated_at = Set(now);
    let updated = am.update(db).await.map_err(|e| e.to_string())?;

    let conv = axagent_dao::repo::conversation::conversation_from_entity(updated);
    Ok(conv)
}

pub(crate) async fn consume_stream(
    app: &tauri::AppHandle,
    stream: &mut std::pin::Pin<
        Box<dyn futures::Stream<Item = std::result::Result<ChatStreamChunk, String>> + Send>,
    >,
    params: StreamConsumptionParams<'_>,
) -> (
    String,
    Option<TokenUsage>,
    Option<Vec<ToolCall>>,
    Option<String>,
    Option<f64>,
    Option<i64>,
) {
    let StreamConsumptionParams {
        conversation_id,
        message_id,
        model_id,
        provider_id,
        cancel_flag,
        suppress_thinking,
    } = params;
    tracing::info!("[consume_stream] Starting for conversation={} message={}", conversation_id, message_id);
    use futures::StreamExt;
    let mut full_content = String::new();
    let mut final_usage: Option<TokenUsage> = None;
    let mut final_tool_calls: Option<Vec<ToolCall>> = None;
    let mut stream_error: Option<String> = None;

    let stream_start = std::time::Instant::now();
    let mut first_token_time: Option<std::time::Instant> = None;

    // Track <think> block state for merging thinking into content
    let mut in_thinking_block = false;
    let mut thinking_block_start: Option<std::time::Instant> = None;
    let mut thinking_durations: Vec<u64> = Vec::new();
    let mut disabled_thinking_strip_state = DisabledThinkingStripState::default();

    // Track inline <think> blocks inside content deltas (DeepSeek v4 style).
    // These models stream thinking tokens inline in `delta.content` rather than
    // through a separate `reasoning_content` field.  A single <think> block may
    // span multiple chunks, so we accumulate across deltas.
    let mut inline_think_buf: Option<String> = None;

    let mut chunk_count = 0u64;
    while let Some(result) = stream.next().await {
        chunk_count += 1;
        if chunk_count == 1 {
            tracing::info!("[consume_stream] First chunk received for conversation={}", conversation_id);
        } else if chunk_count % 10 == 0 {
            tracing::info!("[consume_stream] {} chunks received for conversation={}", chunk_count, conversation_id);
        }
        // Check for cancellation
        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("[consume_stream] Cancelled by user");
            break;
        }
        match result {
            Ok(chunk) => {
                let is_done = chunk.done;
                let content_delta = chunk.content.as_deref().map(|content| {
                    if suppress_thinking {
                        strip_disabled_thinking_delta(content, &mut disabled_thinking_strip_state)
                    } else {
                        content.to_string()
                    }
                });
                let thinking_delta = if suppress_thinking {
                    None
                } else {
                    chunk.thinking.clone()
                };

                // Build the emitted chunk with thinking merged into content
                let mut emit_content = String::new();
                let mut emit_thinking_signal: Option<String> = None;

                // Handle thinking chunks → merge into content with <think> tags
                // Uses <think data-aq> to distinguish our injected blocks from
                // upstream <think> tags (e.g. DeepSeek returns <think> in content)
                if let Some(ref t) = thinking_delta {
                    if !t.is_empty() {
                        if first_token_time.is_none() {
                            first_token_time = Some(std::time::Instant::now());
                        }
                        if !in_thinking_block {
                            // Ensure blank line before <think> so markdown parser treats it as a separate block
                            if !full_content.is_empty() {
                                emit_content.push_str("\n\n");
                            }
                            emit_content.push_str(&get_thinking_block_start());
                            in_thinking_block = true;
                            thinking_block_start = Some(std::time::Instant::now());
                        }
                        emit_content.push_str(t);
                        emit_thinking_signal = Some(String::new()); // signal: thinking active
                    }
                }

                // Handle content chunks → extract inline <think> blocks (DeepSeek v4 style)
                //
                // DeepSeek v4 may stream thinking tokens inline in `delta.content`
                // as `<think>...reasoning...</think>` (not in a separate
                // `reasoning_content` field).  We extract these blocks here and
                // route them through the thinking pipeline so they get the proper
                // `<think data-axagent="1">` wrapping instead of appearing as raw
                // text in the UI.
                if let Some(ref c) = content_delta {
                    if !c.is_empty() {
                        let extracted_thinking: Option<String>;
                        let visible_text: String;

                        if let Some(buf) = &mut inline_think_buf {
                            // Cross-delta accumulation: we saw <think> earlier,
                            // waiting for </think> to complete the block.
                            if let Some(close_pos) = c.find("</think>") {
                                buf.push_str(&c[..close_pos]);
                                let complete = std::mem::take(buf);
                                extracted_thinking = Some(complete);
                                inline_think_buf = None;
                                visible_text = c[close_pos + "</think>".len()..].to_string();
                            } else {
                                buf.push_str(c);
                                extracted_thinking = None;
                                visible_text = String::new();
                            }
                        } else {
                            // Check for complete <think>...</think> in this delta
                            let (vis, think) = extract_reasoning_from_text(c);
                            if think.is_some() {
                                visible_text = vis;
                                extracted_thinking = think;
                            } else if let Some(start) = c.find("<think") {
                                // <think> without </think> → might be a cross-delta
                                // fragment.  Buffer everything after the opening tag.
                                let after_open = &c[start..];
                                // Skip injected / closing tags we already know
                                if !after_open.starts_with("</think>")
                                    && !after_open.starts_with("<think data-axagent")
                                    && !after_open.starts_with("<think totalMs")
                                {
                                    if let Some(gt_pos) = after_open.find('>') {
                                        inline_think_buf =
                                            Some(after_open[gt_pos + 1..].to_string());
                                    }
                                    // Only emit content *before* the opening tag as visible;
                                    // the portion after <think>…</think> is captured in the buffer.
                                    visible_text = c[..start].to_string();
                                } else {
                                    visible_text = vis;
                                }
                                extracted_thinking = None;
                            } else {
                                visible_text = vis;
                                extracted_thinking = None;
                            }
                        }

                        // ── Feed extracted thinking through the pipeline ──
                        if let Some(ref think_text) = extracted_thinking {
                            if !think_text.trim().is_empty() {
                                if first_token_time.is_none() {
                                    first_token_time = Some(std::time::Instant::now());
                                }
                                if !in_thinking_block {
                                    if !full_content.is_empty() {
                                        emit_content.push_str("\n\n");
                                    }
                                    emit_content.push_str(&get_thinking_block_start());
                                    in_thinking_block = true;
                                    thinking_block_start = Some(std::time::Instant::now());
                                }
                                emit_content.push_str(think_text.trim());
                                emit_thinking_signal = Some(String::new());
                            }
                        }

                        // ── Emit visible text part ──
                        if !visible_text.is_empty() {
                            if first_token_time.is_none() {
                                first_token_time = Some(std::time::Instant::now());
                            }
                            if in_thinking_block {
                                let total_ms = thinking_block_start
                                    .map(|s| s.elapsed().as_millis() as u64)
                                    .unwrap_or(0);
                                thinking_durations.push(total_ms);
                                emit_content.push_str("\n</think>\n\n");
                                in_thinking_block = false;
                                thinking_block_start = None;
                            }
                            emit_content.push_str(&visible_text);
                        }
                    }
                }

                // On done: close any still-open <think> block
                if is_done && in_thinking_block {
                    let total_ms = thinking_block_start
                        .map(|s| s.elapsed().as_millis() as u64)
                        .unwrap_or(0);
                    thinking_durations.push(total_ms);
                    emit_content.push_str(&get_thinking_block_end());
                    in_thinking_block = false;
                    thinking_block_start = None;
                }

                full_content.push_str(&emit_content);

                if chunk.usage.is_some() {
                    final_usage.clone_from(&chunk.usage);
                }
                if chunk.tool_calls.is_some() {
                    final_tool_calls.clone_from(&chunk.tool_calls);
                }

                // Detect empty response
                if is_done
                    && full_content.is_empty()
                    && final_tool_calls.as_ref().is_none_or(|tc| tc.is_empty())
                {
                    use crate::commands::error_code::stream as stream_err;
                    let err_msg = ErrorResponse::new(stream_err::EMPTY_RESPONSE)
                        .with_detail("Provider returned empty response. This may indicate the model could not generate content for the given input, the request was filtered by content policy, or the connection was interrupted before any data was received. Try rephrasing your message or try again.".to_string());
                    let _ = app.emit(
                        "chat-stream-error",
                        ChatStreamErrorEvent {
                            conversation_id: conversation_id.to_string(),
                            message_id: message_id.to_string(),
                            error: err_msg.code.clone(),
                        },
                    );
                    tracing::warn!("[consume_stream] Empty response from provider");
                    stream_error = Some(err_msg.code);
                    break;
                }

                let mut emitted_chunk = ChatStreamChunk {
                    content: if emit_content.is_empty() {
                        None
                    } else {
                        Some(emit_content)
                    },
                    thinking: emit_thinking_signal,
                    done: is_done,
                    is_final: None,
                    usage: chunk.usage.clone(),
                    tool_calls: chunk.tool_calls.clone(),
                };
                if emitted_chunk.done && emitted_chunk.is_final.is_none() {
                    emitted_chunk.is_final = Some(
                        emitted_chunk
                            .tool_calls
                            .as_ref()
                            .is_none_or(|tool_calls| tool_calls.is_empty()),
                    );
                }

                let _ = app.emit(
                    "chat-stream-chunk",
                    ChatStreamEvent {
                        conversation_id: conversation_id.to_string(),
                        message_id: message_id.to_string(),
                        model_id: Some(model_id.to_string()),
                        provider_id: Some(provider_id.to_string()),
                        chunk: emitted_chunk,
                    },
                );

                if is_done {
                    break;
                }
            },
            Err(e) => {
                let err_msg = format!("{}", e);
                let _ = app.emit(
                    "chat-stream-error",
                    ChatStreamErrorEvent {
                        conversation_id: conversation_id.to_string(),
                        message_id: message_id.to_string(),
                        error: err_msg.clone(),
                    },
                );
                tracing::error!("Stream error: {}", e);
                stream_error = Some(err_msg);
                break;
            },
        }
    }

    // Close any dangling <think> block (e.g. stream cancelled mid-thinking)
    if in_thinking_block {
        let total_ms = thinking_block_start
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);
        thinking_durations.push(total_ms);
        full_content.push_str(&get_thinking_block_end());
    }

    // Flush any content buffered in cross-delta inline <think> accumulation.
    // If the stream ended before </think>, the partial thinking text still
    // belongs in the final output (won't be properly wrapped as <think>, but
    // no content is lost).
    if let Some(buf) = inline_think_buf.take() {
        full_content.push_str(&buf);
    }

    if suppress_thinking
        && !disabled_thinking_strip_state.in_think_block
        && !disabled_thinking_strip_state.trailing_fragment.is_empty()
        && !"<think".starts_with(&disabled_thinking_strip_state.trailing_fragment)
    {
        full_content.push_str(&disabled_thinking_strip_state.trailing_fragment);
    }

    // Post-process: replace each <think data-aq> with <think totalMs="N">
    full_content = fixup_think_tags(&full_content, &thinking_durations);
    full_content = close_unmatched_think_tags(&full_content);
    if suppress_thinking {
        full_content = strip_disabled_thinking_content(&full_content);
    }

    // Compute timing metrics
    let first_token_latency_ms = first_token_time.map(|t| (t - stream_start).as_millis() as i64);
    let tokens_per_second = match (final_usage.as_ref(), first_token_time) {
        (Some(usage), Some(ft)) if usage.output_tokens > 0 => {
            let gen_duration =
                stream_start.elapsed().as_secs_f64() - (ft - stream_start).as_secs_f64();
            if gen_duration > 0.0 {
                Some(usage.output_tokens as f64 / gen_duration)
            } else {
                None
            }
        },
        _ => None,
    };

    (
        full_content,
        final_usage,
        final_tool_calls,
        stream_error,
        tokens_per_second,
        first_token_latency_ms,
    )
}

/// Replace each `<think data-axagent="1">` marker with `<think totalMs="N">` using
/// the collected duration values. Upstream `<think>` tags (without `data-axagent`)
/// are left unchanged.
pub(crate) fn fixup_think_tags(content: &str, durations: &[u64]) -> String {
    const MARKER: &str = "<think data-axagent=\"1\">";
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;
    let mut dur_iter = durations.iter();
    while let Some(pos) = remaining.find(MARKER) {
        result.push_str(&remaining[..pos]);
        if let Some(ms) = dur_iter.next() {
            result.push_str(&format!("<think totalMs=\"{}\">", ms));
        } else {
            result.push_str("<think>");
        }
        remaining = &remaining[pos + MARKER.len()..];
    }
    result.push_str(remaining);
    result
}

/// Normalize malformed `<think` opening tags and close unmatched ones.
///
/// # Normalization
///
/// - `<think` without a proper `>` (e.g. `<think\n` from chunk-boundary
///   fragmentation) → `<think>`.
/// - `<think` whose first `>` belongs to `<`think>` or a later tag (e.g.
///   `<think\nreasoning\n</think>`) → `<think>` placed before the fragment.
///
/// # Closing
///
/// Counts every `<think[,>]` (injected `totalMs` style OR raw inline style)
/// and every `</think>`.  Appends missing `</think>\n\n` at the end so the
/// markdown parser never sees a dangling opening tag.
pub(crate) fn close_unmatched_think_tags(content: &str) -> String {
    // ── Step 1: normalize malformed opening tags ──────────────────────────
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;
    let mut open_count = 0usize;

    // We walk through the content looking for <think (opening tag) or </think> (closing tag).
    // </think> is passed through unchanged; <think is inspected and fixed up.
    loop {
        let Some(pos) = remaining.find("<think") else {
            result.push_str(remaining);
            break;
        };

        result.push_str(&remaining[..pos]);
        let tag_section = &remaining[pos..];

        // ── < / think >  (closing tag) — pass through ──────────────────────
        if let Some(stripped) = tag_section.strip_prefix("</think>") {
            result.push_str("</think>");
            remaining = stripped;
            continue;
        }

        open_count += 1;

        // ── <think … >  (opening tag) — check for a proper `>` ────────────
        // The closing `>` of the opening tag must appear *before* `</think>`
        // (if a </think> exists at all).  Otherwise the tag is malformed /
        // fragmented, and we insert `>` right after `<think`.
        let search_bound = tag_section.find("</think>").unwrap_or(tag_section.len());

        if let Some(gt_pos) = tag_section[..search_bound].find('>') {
            // Properly formed opening tag — preserve as-is.
            result.push_str(&tag_section[..=gt_pos]);
            remaining = &tag_section[gt_pos + 1..];
        } else {
            // Malformed: no `>` before `</think>` (or no `</think>` at all).
            // Insert `>` to close the tag.
            result.push_str("<think>");
            remaining = &tag_section["<think".len()..];
        }
    }

    // ── Step 2: close unmatched <think> tags ──────────────────────────────
    let close_count = result.matches("</think>").count();
    if close_count < open_count {
        for _ in 0..(open_count - close_count) {
            result.push_str("</think>\n\n");
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn strip_think_tags_removes_unclosed_block() {
        assert_eq!(strip_think_tags("Hello\n<think>secret"), "Hello\n");
    }

    #[test]
    pub(crate) fn close_unmatched_think_tags_appends_closure() {
        assert_eq!(
            close_unmatched_think_tags("prefix<think>body"),
            "prefix<think>body</think>\n\n"
        );
    }

    #[test]
    pub(crate) fn close_unmatched_think_tags_balances_injected_and_inline() {
        // Injected <think totalMs="123"> is always paired, raw <think> is unclosed
        let input = "<think totalMs=\"123\">\nthinking\n</think>\nvisible<think>deepseek";
        let out = close_unmatched_think_tags(input);
        assert_eq!(
            out,
            "<think totalMs=\"123\">\nthinking\n</think>\nvisible<think>deepseek</think>\n\n"
        );
    }

    #[test]
    pub(crate) fn close_unmatched_think_tags_fixes_malformed_opening() {
        // Newline between <think and >  (chunk-boundary fragmentation)
        let input = "<think\nreasoning\n</think>";
        let out = close_unmatched_think_tags(input);
        assert_eq!(out, "<think>\nreasoning\n</think>");
    }

    #[test]
    pub(crate) fn close_unmatched_think_tags_handles_pure_inline_think() {
        // DeepSeek-style <think> inside content, no injected tags
        let input = "Hello\n<think>secret\nstuff</think>\nworld";
        assert_eq!(close_unmatched_think_tags(input), input);
    }

    #[test]
    pub(crate) fn close_unmatched_think_tags_handles_think_without_close_in_content() {
        // <think without closing > AND without </think>
        let input = "visible\n<think\nreasoning without close";
        let out = close_unmatched_think_tags(input);
        assert_eq!(out, "visible\n<think>\nreasoning without close</think>\n\n");
    }

    #[test]
    pub(crate) fn strip_disabled_thinking_delta_handles_fragmented_tags() {
        let mut state = DisabledThinkingStripState::default();
        assert_eq!(strip_disabled_thinking_delta("Hello <thi", &mut state), "Hello ");
        assert_eq!(strip_disabled_thinking_delta("nk>secret</think> world", &mut state), " world");
    }
}

pub(crate) async fn execute_tool_call(
    db: &sea_orm::DatabaseConnection,
    tool_call: &ToolCall,
    mcp_server_ids: &[String],
    master_key: &[u8; 32],
) -> (String, bool) {
    // Handle builtin web_search — unified via core search engine
    if tool_call.function.name == "web_search" {
        tracing::info!("[web_search] LLM called");
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or(serde_json::Value::Null);
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if query.is_empty() {
            use crate::commands::error_code::tool as tool_err;
            return (
                ErrorResponse::err_with_detail(
                    tool_err::PARAM_REQUIRED,
                    "web_search requires a query parameter",
                ),
                true,
            );
        }
        let text = if let Ok(providers) =
            axagent_dao::repo::search_provider::list_search_providers(db).await
        {
            if let Some(p) = providers.iter().find(|p| p.enabled) {
                let api_key = axagent_entities::search_providers::Entity::find_by_id(&p.id)
                    .one(db)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|e| e.api_key_ref)
                    .and_then(|enc| axagent_crypto::decrypt_key(&enc, master_key).ok())
                    .unwrap_or_default();
                axagent_search::search::execute_search_text(
                    &p.provider_type,
                    p.endpoint.as_deref(),
                    &api_key,
                    &query,
                    p.result_limit,
                    p.timeout_ms,
                )
                .await
            } else {
                axagent_search::search::execute_search_text("ddg", None, "", &query, 5, 10000).await
            }
        } else {
            axagent_search::search::execute_search_text("ddg", None, "", &query, 5, 10000).await
        };
        return (text, false);
    }

    let server_and_tool = axagent_dao::repo::mcp_server::find_server_for_tool(
        db,
        &tool_call.function.name,
        mcp_server_ids,
    )
    .await;

    let (server, _td) = match server_and_tool {
        Ok(Some(pair)) => pair,
        _ => {
            // Fallback: try local tool registry (Skill, Read, Write, etc.)
            {
                let mut registry = axagent_tools::registry::UnifiedToolRegistry::new();
                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                let input_str = serde_json::to_string(&args).unwrap_or_default();
                if let Ok(output) = registry.execute(&tool_call.function.name, &input_str).await {
                    return (output.content, output.is_error);
                }
            }
            use crate::commands::error_code::tool as tool_err;
            return (
                ErrorResponse::err_with_detail(
                    tool_err::NOT_FOUND,
                    format!(
                        "Tool {}' not found on any enabled MCP server",
                        tool_call.function.name
                    ),
                ),
                true,
            );
        },
    };

    let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let timeout_secs = server.execute_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let result = match server.transport.as_str() {
        "builtin" => {
            let input_str = serde_json::to_string(&arguments).unwrap_or_default();
            let mut reg = axagent_tools::registry::UnifiedToolRegistry::new();
            match tokio::time::timeout(
                timeout_duration,
                reg.execute(&tool_call.function.name, &input_str),
            )
            .await
            {
                Ok(Ok(r)) => Ok(axagent_mcp::mcp_client::McpToolResult {
                    content: r.content,
                    is_error: r.is_error,
                    progress: Vec::new(),
                }),
                Ok(Err(e)) => Err(axagent_harness::core_error::AxAgentError::Gateway(e.to_string())),
                Err(_) => {
                    use crate::commands::error_code::tool as tool_err;
                    return (
                        ErrorResponse::err_with_detail(
                            tool_err::EXECUTION_TIMEOUT,
                            format!("Tool execution timed out after {}s", timeout_secs),
                        ),
                        true,
                    );
                },
            }
        },
        "stdio" => {
            let command = match &server.command {
                Some(cmd) => cmd.clone(),
                None => {
                    use crate::commands::error_code::tool as tool_err;
                    return (ErrorResponse::err(tool_err::STDIO_NO_COMMAND), true);
                },
            };
            let args: Vec<String> = server
                .args_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let env: std::collections::HashMap<String, String> = server
                .env_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            match tokio::time::timeout(
                timeout_duration,
                axagent_mcp::mcp_client::call_tool_stdio(
                    &command,
                    &args,
                    &env,
                    &tool_call.function.name,
                    arguments,
                ),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    use crate::commands::error_code::tool as tool_err;
                    return (
                        ErrorResponse::err_with_detail(
                            tool_err::EXECUTION_TIMEOUT,
                            format!("Tool execution timed out after {}s", timeout_secs),
                        ),
                        true,
                    );
                },
            }
        },
        "http" => {
            let endpoint = match &server.endpoint {
                Some(ep) => ep.clone(),
                None => {
                    use crate::commands::error_code::tool as tool_err;
                    return (ErrorResponse::err(tool_err::HTTP_NO_ENDPOINT), true);
                },
            };
            match tokio::time::timeout(
                timeout_duration,
                axagent_mcp::mcp_client::call_tool_http(
                    &endpoint,
                    &tool_call.function.name,
                    arguments,
                    None,
                ),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    use crate::commands::error_code::tool as tool_err;
                    return (
                        ErrorResponse::err_with_detail(
                            tool_err::EXECUTION_TIMEOUT,
                            format!("Tool execution timed out after {}s", timeout_secs),
                        ),
                        true,
                    );
                },
            }
        },
        "sse" => {
            let endpoint = match &server.endpoint {
                Some(ep) => ep.clone(),
                None => {
                    use crate::commands::error_code::tool as tool_err;
                    return (ErrorResponse::err(tool_err::SSE_NO_ENDPOINT), true);
                },
            };
            match tokio::time::timeout(
                timeout_duration,
                axagent_mcp::mcp_client::call_tool_sse(
                    &endpoint,
                    &tool_call.function.name,
                    arguments,
                    None,
                ),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    use crate::commands::error_code::tool as tool_err;
                    return (
                        ErrorResponse::err_with_detail(
                            tool_err::EXECUTION_TIMEOUT,
                            format!("Tool execution timed out after {}s", timeout_secs),
                        ),
                        true,
                    );
                },
            }
        },
        other => {
            use crate::commands::error_code::tool as tool_err;
            return (
                ErrorResponse::err_with_detail(
                    tool_err::TRANSPORT_UNSUPPORTED,
                    format!("Unsupported transport {}'", other),
                )
                .to_string(),
                true,
            );
        },
    };

    match result {
        Ok(r) => (r.content, r.is_error),
        Err(e) => {
            use crate::commands::error_code::tool as tool_err;
            (
                ErrorResponse::err_with_detail(
                    tool_err::EXECUTION_ERROR,
                    format!("Error executing tool: {}", e),
                )
                .to_string(),
                true,
            )
        },
    }
}

// i18n-exempt: LLM system prompt for title generation — model interaction data, not UI
const DEFAULT_TITLE_PROMPT: &str = "You are a title generator. Based on the conversation below, generate a concise and descriptive title (maximum 30 characters). Reply with the title only, no quotes or extra text.";

/// 将多条 (role, content) 消息格式化为 "User: ... Assistant: ..." 交替文本。
/// 每条 Assistant 消息截断到 300 字符，总长度达 max_chars 时停止。
pub(crate) fn format_conversation_for_title(
    messages: &[(MessageRole, String)],
    max_chars: usize,
) -> String {
    let mut text = String::new();
    for (role, content) in messages {
        let prefix = match role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            _ => continue,
        };
        if text.len() >= max_chars {
            text.push_str("... (truncated)");
            break;
        }
        let preview: String = if matches!(role, MessageRole::Assistant) {
            content.chars().take(300).collect()
        } else {
            content.clone()
        };
        text.push_str(&format!("{}: {}\n\n", prefix, preview));
    }
    text
}

/// Generate an AI-powered conversation title using the configured title summary model.
/// Returns Err with the actual error message if generation fails.
///
/// `harness` 由调用方传入（通常 `&state.harness`），避免内部 `RuntimeHarness::new` 丢弃 adapter cache。
pub(crate) async fn generate_ai_title(
    harness: &axagent_runtime::harness::RuntimeHarness,
    conversation_messages: &[(MessageRole, String)],
    fallback: TitleFallbackModel<'_>,
    settings: &AppSettings,
) -> Result<String, String> {
    let db = harness.db();
    let master_key = harness.master_key();
    let TitleFallbackModel {
        provider: fallback_provider,
        ctx: fallback_ctx,
        model_id: fallback_model_id,
    } = fallback;
    // Helper: look up use_max_completion_tokens from model param_overrides
    let lookup_umc = |provider_id: &str, model_id: &str, db: &sea_orm::DatabaseConnection| {
        let pid = provider_id.to_string();
        let mid = model_id.to_string();
        let db = db.clone();
        async move {
            axagent_dao::repo::provider::get_model(&db, &pid, &mid)
                .await
                .ok()
                .and_then(|m| m.param_overrides)
                .and_then(|po| po.use_max_completion_tokens)
        }
    };

    // Resolve title summary provider/model: settings override → fallback to conversation model
    if let (Some(pid), Some(mid)) =
        (&settings.title_summary_provider_id, &settings.title_summary_model_id)
    {
        // Try to use the configured title summary provider
        let provider = match axagent_dao::repo::provider::get_provider(db, pid).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Title summary provider not found, falling back: {}", e);
                let umc = lookup_umc(&fallback_ctx.provider_id, fallback_model_id, db).await;
                return generate_ai_title_with(
                    fallback_provider,
                    fallback_ctx,
                    fallback_model_id,
                    conversation_messages,
                    settings,
                    umc,
                    &harness,
                )
                .await;
            },
        };
        let key_row = match axagent_dao::repo::provider::get_active_key(db, pid).await {
            Ok(k) => k,
            Err(e) => {
                tracing::warn!("Title summary provider has no active key, falling back: {}", e);
                let umc = lookup_umc(&fallback_ctx.provider_id, fallback_model_id, db).await;
                return generate_ai_title_with(
                    fallback_provider,
                    fallback_ctx,
                    fallback_model_id,
                    conversation_messages,
                    settings,
                    umc,
                    &harness,
                )
                .await;
            },
        };
        let dk = match axagent_crypto::decrypt_key(&key_row.key_encrypted, master_key) {
            Ok(dk) => dk,
            Err(e) => {
                tracing::warn!("Title summary key decrypt failed, falling back: {}", e);
                let umc = lookup_umc(&fallback_ctx.provider_id, fallback_model_id, db).await;
                return generate_ai_title_with(
                    fallback_provider,
                    fallback_ctx,
                    fallback_model_id,
                    conversation_messages,
                    settings,
                    umc,
                    &harness,
                )
                .await;
            },
        };
        let proxy = axagent_harness::types::provider_model::resolve_provider_proxy(&provider.proxy_config, settings);
        let ctx = ProviderRequestContext {
            api_key: dk,
            key_id: key_row.id.clone(),
            provider_id: provider.id.clone(),
            base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
            api_path: provider.api_path.clone(),
            proxy_config: proxy,
            custom_headers: provider
                .custom_headers
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };
        let umc = lookup_umc(pid, mid, db).await;
        generate_ai_title_with(&provider, &ctx, mid, conversation_messages, settings, umc, &harness)
            .await
    } else {
        // No title summary provider configured, use conversation model
        let umc = lookup_umc(&fallback_ctx.provider_id, fallback_model_id, db).await;
        generate_ai_title_with(
            fallback_provider,
            fallback_ctx,
            fallback_model_id,
            conversation_messages,
            settings,
            umc,
            &harness,
        )
        .await
    }
}

pub(crate) async fn generate_ai_title_with(
    provider: &ProviderConfig,
    ctx: &ProviderRequestContext,
    model_id: &str,
    conversation_messages: &[(MessageRole, String)],
    settings: &AppSettings,
    use_max_completion_tokens: Option<bool>,
    harness: &axagent_runtime::harness::RuntimeHarness,
) -> Result<String, String> {
    let prompt = settings
        .title_summary_prompt
        .as_deref()
        .unwrap_or(DEFAULT_TITLE_PROMPT);

    let conversation_text = format_conversation_for_title(conversation_messages, 3000);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(conversation_text),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
    ];

    let request = ChatRequest {
        model: model_id.to_string(),
        messages,
        stream: false,
        temperature: settings
            .title_summary_temperature
            .map(|v| v as f64)
            .or(Some(0.3)),
        top_p: settings.title_summary_top_p.map(|v| v as f64),
        max_tokens: settings.title_summary_max_tokens.or(Some(50)),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let registry_key = axagent_harness::types::provider_model::provider_registry_key(&provider.provider_type);
    let adapter = harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| {
            let err = format!("Adapter not found for provider type: {}", registry_key);
            tracing::error!("[title-gen] {}", err);
            err
        })?;

    let response = adapter.chat(ctx, request).await.map_err(|e| {
        let err = format!("Chat API error: {}", e);
        tracing::error!("[title-gen] {}", err);
        err
    })?;

    let title = response
        .content
        .trim()
        .trim_matches('"')
        .trim_matches('「')
        .trim_matches('」')
        .trim_matches('《')
        .trim_matches('》')
        .to_string();
    if title.is_empty() {
        // Fallback: use first line of raw response (before stripping), or user message
        let fallback: String = response
            .content
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect::<String>()
            .trim()
            .trim_matches('"')
            .to_string();
        if fallback.is_empty() {
            let first_user = conversation_messages
                .iter()
                .find(|(r, _)| matches!(r, MessageRole::User))
                .map(|(_, c)| c.chars().take(40).collect::<String>())
                .unwrap_or_default();
            tracing::warn!("[title-gen] AI empty, using fallback: {}", first_user);
            Ok(first_user)
        } else {
            tracing::warn!("[title-gen] AI empty after trim, using raw: {}", fallback);
            Ok(fallback)
        }
    } else {
        tracing::info!("[title-gen] Generated title: {}", title);
        Ok(title)
    }
}

