// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleChatInput {
    pub conversation_id: String,
    pub messages: Vec<SimpleChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// 轻量级一次性对话补全（stub，供 AgentGeneratorModal 使用）
///
/// TODO: 对接 agent_query 或 session manager 的完整推理路径
#[tauri::command]
pub async fn simple_chat_completion(
    _state: State<'_, AppState>,
    input: SimpleChatInput,
) -> Result<String, String> {
    // 当前返回 stub 信息，待后续对接真实 LLM 调用
    let last_content = input
        .messages
        .last()
        .map(|m| m.content.as_str())
        .unwrap_or("");
    tracing::warn!(
        "[simple_chat_completion] stub called, conv={}, last_msg_len={}",
        input.conversation_id,
        last_content.len()
    );
    Err("simple_chat_completion: 尚未接入真实 LLM 调用，请在设置中手动配置智能体".to_string())
}
