//! 对话管理 — 会话 CRUD、消息流、上下文压缩。
//!
//! 子模块：
//! - streaming: SSE 流式消息发送与重新生成
//! - compress: 上下文压缩与消息操作

pub mod compress;
pub mod streaming;

use crate::AppState;
#[cfg(test)]
use crate::app_state::SemanticCacheState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::thinking as thinking_err;
use crate::commands::error_code::title as title_err;
#[cfg(test)]
use crate::commands::proactive::ProactiveService;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_providers::{ProviderRequestContext, extract_reasoning_from_text};
#[cfg(test)]
use axagent_runtime_core::prompt_cache::PromptCache;
use base64::Engine;
use dashmap::DashMap;
use futures::FutureExt;
use sea_orm::*;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::Emitter;
use tauri::State;
use tracing::instrument;

// ── Tauri command delegates (#[tauri::command] must be in mod.rs for generate_handler! to find __cmd__ items) ──

#[instrument(skip(app, state))]
#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: SendMessageParams,
) -> Result<Message, String> {
    streaming::send_message(app, state, params).await
}

#[tauri::command]
pub async fn regenerate_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: RegenerateMessageParams,
) -> Result<(), String> {
    streaming::regenerate_message(app, state, params).await
}

#[tauri::command]
pub async fn regenerate_with_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    params: RegenerateWithModelParams,
) -> Result<(), String> {
    streaming::regenerate_with_model(app, state, params).await
}

#[tauri::command]
pub async fn list_message_versions(
    state: State<'_, AppState>,
    conversation_id: String,
    parent_message_id: String,
) -> Result<Vec<Message>, String> {
    compress::list_message_versions(state, conversation_id, parent_message_id).await
}

#[tauri::command]
pub async fn switch_message_version(
    state: State<'_, AppState>,
    conversation_id: String,
    parent_message_id: String,
    message_id: String,
) -> Result<(), String> {
    compress::switch_message_version(state, conversation_id, parent_message_id, message_id).await
}

#[tauri::command]
pub async fn delete_message_group(
    state: State<'_, AppState>,
    conversation_id: String,
    user_message_id: String,
) -> Result<(), String> {
    compress::delete_message_group(state, conversation_id, user_message_id).await
}

#[tauri::command]
pub async fn compress_context(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    compress::compress_context(app, state, conversation_id).await
}

#[tauri::command]
pub async fn get_compression_summary(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<ConversationSummary>, String> {
    compress::get_compression_summary(state, conversation_id).await
}

#[tauri::command]
pub async fn delete_compression(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    compress::delete_compression(state, conversation_id).await
}

#[tauri::command]
pub async fn send_system_message(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> Result<Message, String> {
    compress::send_system_message(state, conversation_id, content).await
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageOptions {
    pub enabled_mcp_server_ids: Option<Vec<String>>,
    pub thinking_budget: Option<u32>,
    pub enabled_knowledge_base_ids: Option<Vec<String>>,
    pub enabled_memory_namespace_ids: Option<Vec<String>>,
    pub enabled_wiki_ids: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageParams {
    pub conversation_id: String,
    pub content: String,
    pub attachments: Vec<AttachmentInput>,
    pub options: SendMessageOptions,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateMessageParams {
    pub conversation_id: String,
    pub user_message_id: Option<String>,
    pub options: SendMessageOptions,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateWithModelParams {
    pub conversation_id: String,
    pub user_message_id: String,
    pub target_provider_id: String,
    pub target_model_id: String,
    pub options: SendMessageOptions,
    pub is_companion: Option<bool>,
}

pub(crate) struct StreamConsumptionParams<'a> {
    conversation_id: &'a str,
    message_id: &'a str,
    model_id: &'a str,
    provider_id: &'a str,
    cancel_flag: &'a AtomicBool,
    suppress_thinking: bool,
}

pub(crate) struct TitleFallbackModel<'a> {
    provider: &'a ProviderConfig,
    ctx: &'a ProviderRequestContext,
    model_id: &'a str,
}

pub(crate) struct StreamTaskParams {
    pub conversation_id: String,
    pub assistant_message_id: String,
    pub conversation: Conversation,
    pub provider: ProviderConfig,
    pub ctx: ProviderRequestContext,
    pub chat_messages: Vec<ChatMessage>,
    pub is_first_message: bool,
    pub user_content: String,
    pub parent_message_id: String,
    pub version_index: i32,
    pub tools: Option<Vec<ChatTool>>,
    pub thinking_budget: Option<u32>,
    pub mcp_server_ids: Vec<String>,
    pub override_created_at: Option<i64>,
    pub use_max_completion_tokens: Option<bool>,
    pub force_max_tokens: Option<bool>,
    pub thinking_param_style: Option<String>,
    pub request_delay_ms: Option<u64>,
    pub settings: AppSettings,
    pub cancel_flag: Arc<AtomicBool>,
    pub cancel_flags: Arc<DashMap<String, Arc<AtomicBool>>>,
    pub content_prefix: String,
    pub create_inactive: bool,
    pub skip_placeholder_create: bool,
}

pub(crate) struct CompressProviderInfo<'a> {
    provider: &'a ProviderConfig,
    decrypted_key: &'a str,
    key_id: &'a str,
    proxy_config: &'a Option<ProviderProxyConfig>,
    model_id: &'a str,
    use_max_completion_tokens: Option<bool>,
}

pub(crate) struct CompressContext<'a> {
    conversation_id: &'a str,
    history_messages: &'a [ChatMessage],
    existing_summary: Option<&'a str>,
    settings: &'a AppSettings,
    master_key: &'a [u8; 32],
}

/// 获取思考块开始标记
pub(crate) fn get_thinking_block_start() -> String {
    format!("<think data-axagent=\"{}\" data-code=\"{}\">\n", "1", thinking_err::BLOCK_START)
}

/// 获取思考块结束标记
pub(crate) fn get_thinking_block_end() -> String {
    "\n</think>\n\n".to_string()
}

/// Resolve effective system prompt with priority: Conversation → Category → Global Default
pub(crate) async fn resolve_system_prompt(
    db: &DatabaseConnection,
    conversation: &Conversation,
) -> Option<String> {
    // 1. Conversation-level system prompt (highest priority)
    if let Some(s) = &conversation.system_prompt
        && !s.is_empty()
    {
        return Some(s.clone());
    }

    if let Some(ref cat_id) = conversation.category_id {
        if let Ok(categories) =
            axagent_dao::repo::conversation_category::list_conversation_categories(db).await
        {
            if let Some(cat) = categories.iter().find(|c| &c.id == cat_id)
                && let Some(ref s) = cat.system_prompt
                && !s.is_empty()
            {
                return Some(s.clone());
            }
        }
    }

    // 3. Global default system prompt (lowest priority)
    let settings = axagent_dao::repo::settings::get_settings(db)
        .await
        .unwrap_or_default();
    settings.default_system_prompt.filter(|s| !s.is_empty())
}

pub(crate) async fn persist_attachments(
    state: &AppState,
    conversation_id: &str,
    attachments: &[AttachmentInput],
) -> axagent_harness::core_error::Result<Vec<Attachment>> {
    axagent_storage::storage_paths::ensure_documents_dirs()?;
    let file_store = axagent_storage::file_store::FileStore::new();

    let mut persisted = Vec::with_capacity(attachments.len());
    for attachment in attachments {
        // Safety limit: reject base64 payloads larger than 100MB to prevent OOM
        const MAX_ATTACHMENT_BASE64_SIZE: usize = 100 * 1024 * 1024; // 100 MB
        if attachment.data.len() > MAX_ATTACHMENT_BASE64_SIZE {
            return Err(axagent_harness::core_error::AxAgentError::Validation(format!(
                "Attachment '{}' base64 data is too large ({} bytes, max {} bytes)",
                attachment.file_name,
                attachment.data.len(),
                MAX_ATTACHMENT_BASE64_SIZE,
            )));
        }

        let data = base64::engine::general_purpose::STANDARD
            .decode(&attachment.data)
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::Validation(format!(
                    "Invalid attachment base64 for {}: {}",
                    attachment.file_name, e
                ))
            })?;

        // Safety limit: reject decoded data larger than 50MB
        const MAX_ATTACHMENT_DECODED_SIZE: usize = 50 * 1024 * 1024; // 50 MB
        if data.len() > MAX_ATTACHMENT_DECODED_SIZE {
            return Err(axagent_harness::core_error::AxAgentError::Validation(format!(
                "Attachment '{}' decoded content is too large ({} bytes, max {} bytes)",
                attachment.file_name,
                data.len(),
                MAX_ATTACHMENT_DECODED_SIZE,
            )));
        }
        let saved = file_store.save_file(&data, &attachment.file_name, &attachment.file_type)?;
        let stored_file_id = axagent_kit::utils::gen_id();
        axagent_dao::repo::stored_file::create_stored_file(
            state.harness.db(),
            &stored_file_id,
            &saved.hash,
            &attachment.file_name,
            &attachment.file_type,
            saved.size_bytes,
            &saved.storage_path,
            Some(conversation_id),
        )
        .await?;

        persisted.push(Attachment {
            id: stored_file_id,
            file_type: attachment.file_type.clone(),
            file_name: attachment.file_name.clone(),
            file_path: saved.storage_path,
            file_size: attachment.file_size,
            data: None,
        });
    }

    Ok(persisted)
}

/// Strip `<think ...>...</think>` blocks from content (all variants).
pub(crate) fn strip_think_tags(content: &str) -> String {
    let mut s = content.to_string();
    loop {
        if let Some(start) = s.find("<think") {
            // Ensure it's a tag (next char is '>' or ' ')
            let after_tag = &s[start + 6..];
            let is_tag = after_tag.starts_with('>') || after_tag.starts_with(' ');
            if !is_tag {
                break;
            }
            if let Some(end_offset) = s[start..].find("</think>") {
                let end = start + end_offset + "</think>".len();
                let before = s[..start].trim_end_matches('\n');
                let after = s[end..].trim_start_matches('\n');
                s = format!("{}{}", before, after);
                continue;
            }
            s.truncate(start);
        }
        break;
    }
    s
}

#[derive(Default)]
pub(crate) struct DisabledThinkingStripState {
    in_think_block: bool,
    trailing_fragment: String,
}

pub(crate) fn think_tag_partial_suffix_len(input: &str, tag: &str) -> usize {
    let max_len = input.len().min(tag.len().saturating_sub(1));
    for len in (1..=max_len).rev() {
        if input.ends_with(&tag[..len]) {
            return len;
        }
    }
    0
}

pub(crate) fn strip_disabled_thinking_content(content: &str) -> String {
    strip_think_tags(content)
}

pub(crate) fn strip_disabled_thinking_delta(
    delta: &str,
    state: &mut DisabledThinkingStripState,
) -> String {
    if delta.is_empty() && state.trailing_fragment.is_empty() {
        return String::new();
    }

    let mut combined = std::mem::take(&mut state.trailing_fragment);
    combined.push_str(delta);

    const THINK_OPEN: &str = "<think";
    const THINK_CLOSE: &str = "</think>";

    let mut stripped = String::with_capacity(combined.len());
    let mut cursor = 0usize;

    loop {
        if cursor >= combined.len() {
            return stripped;
        }

        if state.in_think_block {
            if let Some(end_offset) = combined[cursor..].find(THINK_CLOSE) {
                cursor += end_offset + THINK_CLOSE.len();
                state.in_think_block = false;
                continue;
            }

            let remaining = &combined[cursor..];
            let suffix_len = think_tag_partial_suffix_len(remaining, THINK_CLOSE);
            if suffix_len > 0 {
                state.trailing_fragment = remaining[remaining.len() - suffix_len..].to_string();
            }
            return stripped;
        }

        if let Some(start_offset) = combined[cursor..].find(THINK_OPEN) {
            let start = cursor + start_offset;
            stripped.push_str(&combined[cursor..start]);

            let after_tag = &combined[start + THINK_OPEN.len()..];
            let is_tag = after_tag.starts_with('>') || after_tag.starts_with(' ');
            if !is_tag {
                stripped.push_str(THINK_OPEN);
                cursor = start + THINK_OPEN.len();
                continue;
            }

            if let Some(close_offset) = combined[start..].find('>') {
                cursor = start + close_offset + 1;
                state.in_think_block = true;
                continue;
            }

            state.trailing_fragment = combined[start..].to_string();
            return stripped;
        }

        let remaining = &combined[cursor..];
        let suffix_len = think_tag_partial_suffix_len(remaining, THINK_OPEN);
        if suffix_len > 0 {
            let safe_len = remaining.len() - suffix_len;
            stripped.push_str(&remaining[..safe_len]);
            state.trailing_fragment = remaining[safe_len..].to_string();
        } else {
            stripped.push_str(remaining);
        }
        return stripped;
    }
}

/// Strip display-only tags from assistant message content so they aren't sent to the AI.
/// Strips: `<knowledge-retrieval data-axagent="1">` and `<memory-retrieval data-axagent="1">` tags,
/// `:::mcp ... :::` fenced blocks, and `<think>...</think>` blocks.
pub(crate) fn strip_display_tags(content: &str) -> String {
    // Strip <think> blocks first
    let content = strip_think_tags(content);
    // Strip knowledge-retrieval and memory-retrieval tags with data-axagent attribute
    // Also strip <memory-item> and <retrieved-context> boundary tags (injected into LLM context)
    let content = {
        let mut s = content.to_string();
        for tag_name in &[
            "knowledge-retrieval",
            "memory-retrieval",
            "memory-item",
            "retrieved-context",
        ] {
            let tag_start = format!("<{} ", tag_name);
            let tag_start_bare = format!("<{}>", tag_name);
            let tag_end = format!("</{}>", tag_name);
            loop {
                let start_pos = if let Some(pos) = s.find(&tag_start) {
                    Some(pos)
                } else if tag_name == &"retrieved-context" || tag_name == &"memory-item" {
                    s.find(&tag_start_bare)
                } else {
                    None
                };
                if let Some(start_pos) = start_pos
                    && let Some(end_offset) = s[start_pos..].find(&tag_end)
                {
                    let after = &s[start_pos + end_offset + tag_end.len()..];
                    let before = &s[..start_pos];
                    s = format!(
                        "{}{}",
                        before.trim_end_matches('\n'),
                        after.trim_start_matches('\n')
                    );
                    continue;
                }
                break;
            }
        }
        s
    };

    // Strip :::mcp blocks
    let mut result = String::with_capacity(content.len());
    let mut remaining = content.as_str();
    while let Some(start) = remaining.find(":::mcp ") {
        // Only match at start of line
        let at_line_start = start == 0 || remaining.as_bytes().get(start - 1) == Some(&b'\n');
        if !at_line_start {
            result.push_str(&remaining[..start + 7]);
            remaining = &remaining[start + 7..];
            continue;
        }
        result.push_str(remaining[..start].trim_end_matches('\n'));
        // Find the closing :::
        if let Some(end_offset) = remaining[start..].find("\n:::\n") {
            remaining = &remaining[start + end_offset + 4..]; // skip past \n:::\n
        } else if remaining[start..].ends_with("\n:::") {
            remaining = "";
        } else {
            // No closing fence found — keep the content
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }
    result.push_str(remaining);
    let trimmed = result.trim().to_string();
    if trimmed.is_empty() && !content.trim().is_empty() {
        // If stripping removed everything, return empty (content was all display tags)
        String::new()
    } else {
        trimmed
    }
}

pub(crate) fn build_message_content(
    file_store: &axagent_storage::file_store::FileStore,
    message: &Message,
) -> axagent_harness::core_error::Result<ChatContent> {
    // Strip display-only tags from all messages (not just assistant)
    // to prevent prompt injection via <knowledge-retrieval> or <memory-retrieval> tags
    let content = strip_display_tags(&message.content);

    let image_attachments = message
        .attachments
        .iter()
        .filter(|attachment| attachment.file_type.starts_with("image/"))
        .collect::<Vec<_>>();

    if image_attachments.is_empty() {
        return Ok(ChatContent::Text(content));
    }

    let mut parts = Vec::new();
    if !content.is_empty() {
        parts.push(ContentPart {
            r#type: "text".to_string(),
            text: Some(content.clone()),
            image_url: None,
        });
    }

    for attachment in image_attachments {
        let data_url = if attachment.file_path.is_empty() {
            let base64_data = attachment.data.as_ref().ok_or_else(|| {
                axagent_harness::core_error::AxAgentError::Validation(format!(
                    "Attachment {} is missing both file_path and inline data",
                    attachment.file_name
                ))
            })?;
            format!("data:{};base64,{}", attachment.file_type, base64_data)
        } else {
            match file_store.read_file(&attachment.file_path) {
                Ok(data) => format!(
                    "data:{};base64,{}",
                    attachment.file_type,
                    base64::engine::general_purpose::STANDARD.encode(data)
                ),
                Err(_) => continue, // skip deleted/missing attachments
            }
        };
        parts.push(ContentPart {
            r#type: "image_url".to_string(),
            text: None,
            image_url: Some(ImageUrl { url: data_url }),
        });
    }

    // If only text part remains (all images were missing), simplify to Text
    if parts.len() <= 1 && parts.iter().all(|p| p.r#type == "text") {
        return Ok(ChatContent::Text(content));
    }

    Ok(ChatContent::Multipart(parts))
}

pub(crate) fn chat_message_from_message(
    file_store: &axagent_storage::file_store::FileStore,
    message: &Message,
) -> axagent_harness::core_error::Result<ChatMessage> {
    let tool_calls: Option<Vec<ToolCall>> = message
        .tool_calls_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(ChatMessage {
        role: match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        }
        .to_string(),
        content: build_message_content(file_store, message)?,
        tool_calls,
        tool_call_id: message.tool_call_id.clone(),
        thinking: message.thinking.clone(),
    })
}

