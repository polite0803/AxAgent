use super::helpers::AiChatMessage;
use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::provider as provider_err;
use crate::commands::workflow_ai::NODE_SCHEMAS_DOC;
use crate::commands::workflow_ai::UPSTREAM_EXTENSION_FOR_CHAT;
use axagent_harness::types::settings_chat::ChatContent;
use axagent_harness::types::{
    ChatMessage, ChatRequest, ChatStreamChunk, ChatStreamErrorEvent, ChatStreamEvent,
};

use super::helpers::{get_cancel_store, resolve_ai_provider};
use futures::StreamExt;
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
#[tracing::instrument(skip(app, state))]
pub async fn workflow_ai_chat_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    message: String,
    history: Vec<AiChatMessage>,
    current_nodes: Option<Vec<serde_json::Value>>,
    current_edges: Option<Vec<serde_json::Value>>,
    session_id: String,
) -> Result<(), String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            provider_err::ADAPTER_NOT_FOUND,
            format!("Provider adapter not found for type: {}", registry_key),
        )
    })?;

    let mut canvas_section = String::new();
    if let Some(nodes) = &current_nodes {
        if !nodes.is_empty() {
            let node_summary: Vec<String> = nodes
                .iter()
                .map(|n| {
                    let nt = n.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let title = n.get("title").and_then(|v| v.as_str()).unwrap_or(nt);
                    format!("- {} ({})", title, nt)
                })
                .collect();
            let edge_count = current_edges.as_ref().map(|e| e.len()).unwrap_or(0);
            canvas_section = format!(
                "\n\nCurrent workflow canvas:\nNodes ({}):\n{}\nEdges: {}",
                nodes.len(),
                node_summary.join("\n"),
                edge_count
            );
        }
    }

    let base_prompt = format!(
        r#"You are an AI assistant for a workflow editor. You help users create, modify, and optimize workflows through conversation.

=== 意图路由（务必先判断用户意图再决定输出形式）===
- 全新生成/重写工作流 → generate_workflow action
- 添加新节点 → add_node / add_nodes action
- 修改节点属性 → update_node action（仅传需要改的字段）
- 删除节点/边 → delete_node / delete_edge action
- 修改边 → add_edge / update_edge / delete_edge
- 优化 Agent 提示词 → optimize_prompt action
- 解释/分析/问问题 → 纯文本回复，不输出 action 块
- 单纯咨询（如"什么是 parallel 节点"）→ 纯文本

=== 你能做的事 ===
1. 全新生成完整工作流
2. 修改现有工作流（add/update/delete nodes, add/update/delete edges）
3. 优化 agent prompt
4. 推荐节点类型
5. 解释工作流概念

=== 完整节点类型（27 种，add_node 时 node.type 必须是下列之一）===
trigger, agent, llm, condition, switch, parallel, loop, merge, delay,
httpRequest, databaseQuery, tool, code, subWorkflow, documentParser,
vectorRetrieve, validation, notification, approval, fileOperation,
dataTransformer, webhookSend, logging, llmClassifier, aggregator, email, end

{NODE_SCHEMAS_DOC}

=== Action 协议（必须用 :::action 包裹 JSON）===
:::action
{{"action_type": "generate_workflow", "data": {{"nodes": [...], "edges": [...]}}}}
:::

:::action
{{"action_type": "add_node", "data": {{"node": {{...}}, "position": {{"x": 0, "y": 0}}}}}}
:::

:::action
{{"action_type": "add_nodes", "data": {{"nodes": [...]}}}}
:::

:::action
{{"action_type": "update_node", "data": {{"node_id": "...", "changes": {{"title": "新标题", "config": {{...}}, "position": {{...}}}}}}}}
:::

:::action
{{"action_type": "modify_node", "data": {{"node_id": "...", "changes": {{}}}}}}
:::

:::action
{{"action_type": "delete_node", "data": {{"node_id": "..."}}}}
:::

:::action
{{"action_type": "delete_nodes", "data": {{"node_ids": ["...", "..."]}}}}
:::

:::action
{{"action_type": "add_edge", "data": {{"edge": {{"id": "...", "source": "...", "target": "...", "edge_type": "direct", "label": "..."}}}}}}
:::

:::action
{{"action_type": "update_edge", "data": {{"edge_id": "...", "changes": {{"label": "...", "edge_type": "conditionTrue"}}}}}}
:::

:::action
{{"action_type": "delete_edge", "data": {{"edge_id": "..."}}}}
:::

:::action
{{"action_type": "optimize_prompt", "data": {{"node_id": "...", "optimized_prompt": "..."}}}}
:::

=== 强制规则 ===
1. 一次回复可包含多个 :::action 块（按依赖顺序排列）。
2. add_node/add_nodes 时严格按上面对应 node_type 的 schema 写 config；缺字段的未知参数会被丢弃。
3. update_node 是部分更新：只传需要改的字段。删除属性请传 null。
4. 涉及并行批量 → parallel，循环遍历 → loop，不要混用。
5. 描述里"审批"→ approval；"邮件"→ email；"HTTP/接口/REST"→ httpRequest；"数据库/SQL"→ databaseQuery；"Webhook 回调"→ webhookSend；"分类/打标"→ llmClassifier。
6. 在 action 块之前先用一句话解释你要做什么，让用户能跟得上。
7. 所有改动会先以 diff 形式展示给用户确认，不会自动落库。
8. Respond in the same language as the user's message.{}"#,
        canvas_section
    );

    let system_prompt = format!("{base_prompt}{UPSTREAM_EXTENSION_FOR_CHAT}");

    // 启动期 v2 prompt token 完整性自检(只在第一次调用时跑,后续无开销)。
    // 缺失关键 token 会让 LLM 行为静默回归,panic 早暴露更安全。
    assert_v2_prompts_well_formed();

    let mut chat_messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: ChatContent::Text(system_prompt),
        tool_calls: None,
        tool_call_id: None,
        thinking: None,
    }];

    for msg in &history {
        chat_messages.push(ChatMessage {
            role: msg.role.clone(),
            content: ChatContent::Text(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    }

    chat_messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::Text(message),
        tool_calls: None,
        tool_call_id: None,
        thinking: None,
    });

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: chat_messages,
        stream: true,
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(4096),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut store = get_cancel_store().lock().await;
        store.insert(session_id.clone(), cancel_flag.clone());
    }
    let _ = app.emit(
        "workflow-ai-chat-start",
        serde_json::json!({
            "session_id": session_id,
        }),
    );

    let llm_config = axagent_runtime_core::LlmCallConfig {
        session_id: Some(session_id.clone()),
        ..Default::default()
    };
    let mut stream = axagent_runtime_core::execute_llm_stream(
        &*adapter,
        &resolved.ctx,
        request,
        &llm_config,
        None,
    )
    .await
    .map_err(|e| format!("LLM 初始化失败: {e}"))?;
    let message_id = format!("wf-ai-{}", uuid::Uuid::new_v4());

    while let Some(result) = stream.next().await {
        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        match result {
            Ok(chunk) => {
                let is_done = chunk.done;
                let content_delta = chunk.content.clone();
                let thinking_delta = chunk.thinking.clone();

                let mut emit_content = String::new();
                if let Some(ref t) = thinking_delta {
                    if !t.is_empty() {
                        emit_content
                            .push_str(&format!("<think data-aq>\n{}\n</think data-aq>\n", t));
                    }
                }
                if let Some(ref c) = content_delta {
                    emit_content.push_str(c);
                }

                let emitted_chunk = ChatStreamChunk {
                    content: if emit_content.is_empty() {
                        None
                    } else {
                        Some(emit_content)
                    },
                    thinking: None,
                    done: is_done,
                    is_final: if is_done { Some(true) } else { None },
                    usage: chunk.usage.clone(),
                    tool_calls: None,
                };

                let _ = app.emit(
                    "workflow-ai-chat-chunk",
                    ChatStreamEvent {
                        conversation_id: session_id.clone(),
                        message_id: message_id.clone(),
                        model_id: Some(resolved.model_id.clone()),
                        provider_id: Some(resolved.ctx.provider_id.clone()),
                        chunk: emitted_chunk,
                    },
                );

                if is_done {
                    break;
                }
            },
            Err(e) => {
                let _ = app.emit(
                    "workflow-ai-chat-error",
                    ChatStreamErrorEvent {
                        conversation_id: session_id.clone(),
                        message_id: message_id.clone(),
                        error: e.to_string(),
                    },
                );
                break;
            },
        }
    }

    let _ = app.emit(
        "workflow-ai-chat-done",
        serde_json::json!({
            "session_id": session_id,
            "message_id": message_id,
        }),
    );

    {
        let mut store = get_cancel_store().lock().await;
        store.remove(&session_id);
    }

    Ok(())
}

#[tauri::command]
pub async fn workflow_ai_chat_cancel(
    _state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let store = get_cancel_store().lock().await;
    if let Some(flag) = store.get(&session_id) {
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    Ok(())
}

/// 启动期 v2 prompt 完整性自检(防 system_prompt 关键 token 回归)
///
/// 在第一次被访问时(本 crate 任意一处读 `UPSTREAM_EXTENSION_FOR_CHAT` /
/// `UPSTREAM_EXTENSION_FOR_DIAGNOSE`)强制跑一遍关键 token 断言。若 token 缺失,
/// 直接 `panic!` — 因为这些 const 是 LLM 行为契约,缺失会让 LLM 静默回归,
/// 比 panic 更难发现。
///
/// 之所以选 `OnceLock`(而非 `#[test]`)是因为本 crate 的 lib test binary 在
/// 当前 Windows + rustc 1.95 环境下启动时 `STATUS_ENTRYPOINT_NOT_FOUND`
/// (`0xC0000139`),内联 `#[test]` 跑不起来;改用启动期断言保证约束不丢。
pub fn assert_v2_prompts_well_formed() {
    use std::sync::OnceLock;
    static CHECK: OnceLock<()> = OnceLock::new();
    CHECK.get_or_init(|| {
        for token in [
            "\"action_type\":\"update_variable\"",
            "\"action_type\":\"rollback_to_version\"",
            "\"action_type\":\"update_input_mapping\"",
            "\"action_type\":\"edit_asset_file\"",
            "\"action_type\":\"apply_diff_with_validation\"",
        ] {
            assert!(
                UPSTREAM_EXTENSION_FOR_CHAT.contains(token),
                "UPSTREAM_EXTENSION_FOR_CHAT missing required action_type token: {token}"
            );
        }
        for token in ["\"inject_context\":\"version_history\"", "\"inject_context\":\"diagnostic\""]
        {
            assert!(
                UPSTREAM_EXTENSION_FOR_CHAT.contains(token),
                "UPSTREAM_EXTENSION_FOR_CHAT missing required inject_context marker: {token}"
            );
        }
    });
}
