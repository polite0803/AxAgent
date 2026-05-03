//! Fork Session Bridge — 在父 agent 和 fork 子 agent 之间传递会话数据
//!
//! 当 fork 子 agent 创建时，父 agent 通过此模块将 system prompt 和消息历史
//! 传递给子 agent。子 agent 复用这些数据以确保 Anthropic prompt cache 命中。

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Fork 上下文 — 当 fork 子 agent 创建时存储父会话信息
/// 子 agent 启动时读取此上下文，继承父 agent 的消息历史以实现 prompt cache 共享
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForkSessionData {
    /// 父会话 ID
    pub parent_conversation_id: String,
    /// 子 agent 描述
    pub description: String,
    /// 任务 prompt
    pub prompt: String,
    /// 创建时间
    pub created_at: String,
    /// 父 agent 的 system prompt（用于子 agent 复用相同前缀以命中 cache）
    pub parent_system_prompt: Vec<String>,
    /// 父 agent 的消息历史（序列化为 JSON 以避免类型依赖）
    pub parent_messages_json: String,
    /// 子 agent 追加的 system prompt
    pub child_system_prompt: Option<String>,
}

static FORK_SESSIONS: LazyLock<RwLock<HashMap<String, ForkSessionData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 存储 fork session 数据（key = parent_conversation_id）
pub fn store_fork_session(data: ForkSessionData) {
    FORK_SESSIONS
        .write()
        .unwrap()
        .insert(data.parent_conversation_id.clone(), data);
}

/// 获取并移除 fork session 数据
pub fn take_fork_session(parent_id: &str) -> Option<ForkSessionData> {
    FORK_SESSIONS.write().unwrap().remove(parent_id)
}

/// 检查是否存在 fork session 数据
pub fn has_fork_session(parent_id: &str) -> bool {
    FORK_SESSIONS.read().unwrap().contains_key(parent_id)
}

/// 生成 fork 子 agent 的 system prompt（追加在父 prompt 之后，实现 cache 共享）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve_fork_session() {
        let data = ForkSessionData {
            parent_conversation_id: "test-session-1".into(),
            description: "测试 fork".into(),
            prompt: "分析代码".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            parent_system_prompt: vec!["你是 Claude".into()],
            parent_messages_json: "[]".into(),
            child_system_prompt: Some("子 agent 指令".into()),
        };
        store_fork_session(data);
        assert!(has_fork_session("test-session-1"));

        let loaded = take_fork_session("test-session-1").unwrap();
        assert_eq!(loaded.description, "测试 fork");
        assert_eq!(loaded.parent_system_prompt, vec!["你是 Claude"]);
        assert_eq!(loaded.child_system_prompt, Some("子 agent 指令".into()));

        // take 后应该被移除
        assert!(!has_fork_session("test-session-1"));
    }

    #[test]
    fn take_nonexistent_returns_none() {
        assert!(take_fork_session("nonexistent").is_none());
    }

    #[test]
    fn fork_prompt_includes_task_and_rules() {
        let prompt = build_fork_child_prompt("修复 bug");
        assert!(prompt.contains("修复 bug"));
        assert!(prompt.contains("Fork 规则"));
        assert!(prompt.contains("EnterPlanMode"));
    }
}
