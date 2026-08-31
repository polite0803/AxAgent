// SPDX-License-Identifier: AGPL-3.0-only

//! 会话状态存储（Session State）— 能力按需加载的写入/读取解耦点。
//!
//! # 为什么需要它
//!
//! 渐进式披露的「加载」动作必须是**写入状态**而非「立即返回正文」：
//! 工具调用（写）与系统提示注入（读）发生在不同的请求轮次，
//! 只有把加载结果落到会话状态，下一轮 Processor 才能读回并注入 Prompt。
//! 这是「检索 → 加载 → 注入」闭环的解耦点。
//!
//! # Key 规范
//!
//! ```text
//! {scope}:{namespace}:{conversation_id}:{agent_id}/{subject}
//! ```
//!
//! - `scope`：`temp`（会话结束即弃）/ `session`（会话生命周期）/ `persistent`（长期）
//! - `namespace`：业务命名空间，如 `skill:loaded`
//! - `agent_id`：Agent 作用域，**多 Agent 隔离的载体**。单 Agent 场景传 `"default"`
//!
//! 例：`temp:skill:loaded:conv-42:default/tool:read_file`
//!
//! # 分层合规
//!
//! 本模块只定义 trait 与 key 构造规则（纯契约），持久化实现在 `axagent-dao`
//! （`repo/session_state.rs`），由 wiring 层注入。

use serde::{Deserialize, Serialize};

/// 状态作用域 — 决定条目的生命周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateScope {
    /// 临时：随会话结束清理，适合「本轮加载了什么能力」
    Temp,
    /// 会话级：会话生命周期内保留
    Session,
    /// 持久：跨会话保留（用户偏好等）
    Persistent,
}

impl StateScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Temp => "temp",
            Self::Session => "session",
            Self::Persistent => "persistent",
        }
    }
}

impl std::fmt::Display for StateScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 能力加载状态的命名空间。
pub const NS_SKILL_LOADED: &str = "skill:loaded";

/// 单 Agent 场景（无多 Agent 分支）使用的默认 agent 作用域。
pub const DEFAULT_AGENT_ID: &str = "default";

/// 构造带 Agent 作用域的会话状态 key。
///
/// 格式：`{scope}:{namespace}:{conversation_id}:{agent_id}/{subject}`
///
/// `agent_id` 为空时回落到 [`DEFAULT_AGENT_ID`]，保证 key 始终含 agent 维度
/// —— 多 Agent 隔离依赖这个维度，缺失会让不同 Agent 的加载状态互相覆盖。
pub fn scoped_key(
    scope: StateScope,
    namespace: &str,
    conversation_id: &str,
    agent_id: Option<&str>,
    subject: &str,
) -> String {
    let agent = agent_id.filter(|a| !a.trim().is_empty()).unwrap_or(DEFAULT_AGENT_ID);
    format!("{}:{}:{}:{}/{}", scope.as_str(), namespace, conversation_id, agent, subject)
}

/// 命名空间前缀 —— 用于「列出某会话下所有已加载能力」这类范围查询。
///
/// 返回 `temp:skill:loaded:conv-42:` 形式，配合 `list_by_prefix` 可取出
/// 该会话下**全部 Agent** 的加载记录；再叠加 agent 段即可取单个 Agent 的。
pub fn namespace_prefix(
    scope: StateScope,
    namespace: &str,
    conversation_id: &str,
    agent_id: Option<&str>,
) -> String {
    let agent = agent_id.filter(|a| !a.trim().is_empty()).unwrap_or(DEFAULT_AGENT_ID);
    format!("{}:{}:{}:{}/", scope.as_str(), namespace, conversation_id, agent)
}

/// 会话状态条目（查询返回体）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateEntry {
    pub key: String,
    /// 值（JSON 字符串的原文，由业务方解释）
    pub value: String,
    pub scope: String,
    pub conversation_id: Option<String>,
    pub agent_id: Option<String>,
    pub updated_at_ms: i64,
    /// 过期时间戳（毫秒）；`None` 表示不过期
    pub expires_at_ms: Option<i64>,
}

impl SessionStateEntry {
    /// 是否已过期（`expires_at_ms` 早于 `now_ms`）。
    pub fn is_expired_at(&self, now_ms: i64) -> bool {
        self.expires_at_ms.is_some_and(|exp| exp <= now_ms)
    }
}

/// 会话状态存储契约。
///
/// # 实现位置
/// harness 层定义 trait，`axagent-dao` 提供 SQLite 实现，wiring 层注入。
#[async_trait::async_trait]
pub trait SessionStateStore: Send + Sync {
    /// 写入（已存在则覆盖）。`ttl_ms` 为 `None` 时不过期。
    async fn set(&self, key: &str, value: &str, ttl_ms: Option<i64>) -> Result<(), String>;

    /// 读取。过期条目视为不存在（返回 `None`）。
    async fn get(&self, key: &str) -> Result<Option<String>, String>;

    /// 删除。key 不存在时静默成功。
    async fn delete(&self, key: &str) -> Result<(), String>;

    /// 按前缀列出全部条目（已过滤过期项）。
    async fn list_by_prefix(&self, prefix: &str) -> Result<Vec<SessionStateEntry>, String>;

    /// 物理清理过期条目，返回删除条数。供启动与周期性任务调用。
    async fn purge_expired(&self) -> Result<usize, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_key_carries_agent_dimension() {
        let k = scoped_key(
            StateScope::Temp,
            NS_SKILL_LOADED,
            "conv-42",
            Some("agent-a"),
            "tool:read_file",
        );
        assert_eq!(k, "temp:skill:loaded:conv-42:agent-a/tool:read_file");
    }

    #[test]
    fn scoped_key_falls_back_to_default_agent() {
        let a = scoped_key(StateScope::Temp, NS_SKILL_LOADED, "c1", None, "x");
        let b = scoped_key(StateScope::Temp, NS_SKILL_LOADED, "c1", Some("  "), "x");
        assert_eq!(a, "temp:skill:loaded:c1:default/x");
        assert_eq!(a, b, "空白 agent_id 必须回落到 default");
    }

    #[test]
    fn different_agents_do_not_collide() {
        let a = scoped_key(StateScope::Temp, NS_SKILL_LOADED, "c1", Some("agent-a"), "x");
        let b = scoped_key(StateScope::Temp, NS_SKILL_LOADED, "c1", Some("agent-b"), "x");
        assert_ne!(a, b);
    }

    #[test]
    fn namespace_prefix_is_scoped_to_agent() {
        let p = namespace_prefix(StateScope::Temp, NS_SKILL_LOADED, "c1", Some("agent-a"));
        assert_eq!(p, "temp:skill:loaded:c1:agent-a/");
        let k = scoped_key(StateScope::Temp, NS_SKILL_LOADED, "c1", Some("agent-a"), "tool:x");
        assert!(k.starts_with(&p), "key 必须能被自身前缀命中");
    }

    #[test]
    fn entry_expiry_boundary() {
        let e = SessionStateEntry {
            key: "k".into(),
            value: "v".into(),
            scope: "temp".into(),
            conversation_id: None,
            agent_id: None,
            updated_at_ms: 0,
            expires_at_ms: Some(100),
        };
        assert!(!e.is_expired_at(99));
        assert!(e.is_expired_at(100), "过期时刻应视为已过期（左闭右开）");
        assert!(e.is_expired_at(101));
    }

    #[test]
    fn entry_without_expiry_never_expires() {
        let e = SessionStateEntry {
            key: "k".into(),
            value: "v".into(),
            scope: "temp".into(),
            conversation_id: None,
            agent_id: None,
            updated_at_ms: 0,
            expires_at_ms: None,
        };
        assert!(!e.is_expired_at(i64::MAX));
    }
}
