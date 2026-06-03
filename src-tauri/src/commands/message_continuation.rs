// 消息续写 — 从截断/partial 消息处继续生成

use crate::AppState;
use axagent_core::entity::messages;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ContinueResult {
    message_id: String,
    branched: bool,
    content_preview: String,
}

/// 列出可续写的消息（partial 状态）
#[tauri::command]
pub async fn list_continuable_messages(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let db = app_state.harness.db();

    let msgs = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(&conversation_id))
        .filter(messages::Column::Role.eq("assistant"))
        .filter(messages::Column::Status.eq("partial"))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(msgs
        .iter()
        .filter_map(|m| {
            serde_json::to_value(serde_json::json!({
                "id": m.id,
                "parent_message_id": m.parent_message_id,
                "status": m.status,
                "content_preview": m.content.chars().take(100).collect::<String>(),
                "created_at": m.created_at,
            }))
            .ok()
        })
        .collect())
}

/// 续写消息 — 返回续写上下文供前端发送
/// 前端收到后发起 regenerate_with_model 调用
#[tauri::command]
pub async fn continue_message(
    app_state: State<'_, AppState>,
    conversation_id: String,
    message_id: String,
    branch: Option<bool>,
) -> Result<ContinueResult, String> {
    let db = app_state.harness.db();

    let msg = messages::Entity::find_by_id(&message_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("消息 {} 未找到", message_id))?;

    // 校验消息归属
    if msg.conversation_id != conversation_id {
        return Err("消息不属于指定对话".into());
    }

    if msg.role != "assistant" {
        return Err("只能续写 assistant 消息".into());
    }

    let preview: String = msg.content.chars().take(200).collect();

    // 分支续写：前端调用 regenerate_message
    // 追加续写：前端使用固定 system prompt 发送新消息

    Ok(ContinueResult {
        message_id: msg.id,
        branched: branch.unwrap_or(true),
        content_preview: preview,
    })
}
