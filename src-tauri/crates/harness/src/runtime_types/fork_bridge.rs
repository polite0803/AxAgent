// SPDX-License-Identifier: AGPL-3.0-only
//
// Fork Session Bridge — 在父 agent 和 fork 子 agent 之间传递会话数据

#![allow(clippy::disallowed_types)]

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Fork 上下文 — 当 fork 子 agent 创建时存储父会话信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkSessionData {
    /// 父会话 ID
    pub parent_conversation_id: String,
    /// 子 agent 描述
    pub description: String,
    /// 任务 prompt
    pub prompt: String,
    /// 创建时间
    pub created_at: String,
    /// 父 agent 的 system prompt
    pub parent_system_prompt: Vec<String>,
    /// 父 agent 的消息历史（序列化为 JSON 以避免类型依赖）
    pub parent_messages_json: String,
    /// 子 agent 追加的 system prompt
    pub child_system_prompt: Option<String>,
}

static FORK_SESSIONS: LazyLock<RwLock<HashMap<String, ForkSessionData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 存储 fork session 数据
pub fn store_fork_session(data: ForkSessionData) {
    FORK_SESSIONS.write().insert(data.parent_conversation_id.clone(), data);
}

/// 获取并移除 fork session 数据
pub fn take_fork_session(parent_id: &str) -> Option<ForkSessionData> {
    FORK_SESSIONS.write().remove(parent_id)
}

/// 检查是否存在 fork session 数据
pub fn has_fork_session(parent_id: &str) -> bool {
    FORK_SESSIONS.read().contains_key(parent_id)
}

/// 生成 fork 子 agent 的 system prompt
pub fn build_fork_child_prompt(task: &str) -> String {
    format!(
        "## Fork 子 Agent 指令\n\n\
         你是父 Agent 的 fork 子进程。你拥有父 agent 的完整对话历史作为上下文。\
         请完成以下任务：\n\n{}\n\n\
         ## Fork 规则\n\
         - 不使用 EnterPlanMode/ExitPlanMode\n\
         - 不递归创建子 agent\n\
         - 完成后直接返回结果，不继续对话\n\
         - 只读操作优先于写入操作",
        task
    )
}
