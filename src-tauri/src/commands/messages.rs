// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_harness::types::*;
use tauri::State;

#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "列出对话消息")]
#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "分页列出对话消息")]
#[tauri::command]
pub async fn list_messages_page(
    state: State<'_, AppState>,
    conversation_id: String,
    limit: Option<u64>,
    before_message_id: Option<String>,
) -> Result<MessagePage, String> {
    axagent_dao::repo::message::list_messages_page(
        state.harness.db(),
        &conversation_id,
        limit.unwrap_or(10),
        before_message_id.as_deref(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = conversations, safety = Dangerous, call_mode = StateInput, description = "删除消息")]
#[tauri::command]
pub async fn delete_message(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::message::delete_message(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = conversations, safety = Caution, call_mode = StateInput, description = "更新消息内容")]
#[tauri::command]
pub async fn update_message_content(
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<Message, String> {
    axagent_dao::repo::message::update_message_content(state.harness.db(), &id, &content)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[agent_command(domain = conversations, safety = Dangerous, call_mode = StateInput, description = "清空对话消息")]
#[tauri::command]
pub async fn clear_conversation_messages(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<u64, String> {
    let rows = axagent_dao::repo::message::clear_conversation_messages(
        state.harness.db(),
        &conversation_id,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // Also clear the agent session's SDK context so the agent doesn't retain old history
    let _ = axagent_dao::repo::agent_session::clear_sdk_context_by_conversation_id(
        state.harness.db(),
        &conversation_id,
    )
    .await;

    Ok(rows)
}

#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "导出对话")]
#[tauri::command]
pub async fn export_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    format: String,
) -> Result<String, String> {
    let conversation =
        axagent_dao::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    match format.as_str() {
        "json" => serde_json::to_string_pretty(&serde_json::json!({
            "conversation": conversation,
            "messages": messages,
        }))
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        }),
        "markdown" => {
            let mut md = format!("# {}\n\n", conversation.title);
            for msg in &messages {
                let role = match msg.role {
                    MessageRole::System => "System",
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::Tool => "Tool",
                };
                md.push_str(&format!("## {}\n\n{}\n\n", role, msg.content));
            }
            Ok(md)
        },
        _ => Err(format!("Unsupported export format: {}", format)),
    }
}

#[agent_command(domain = conversations, safety = Safe, call_mode = StateInput, description = "获取对话统计信息")]
#[tauri::command]
pub async fn get_conversation_stats(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationStats, String> {
    axagent_dao::repo::message::get_conversation_stats(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}
