// SPDX-License-Identifier: AGPL-3.0-only

//! AgentSessionBroker — 面向 MCP / CLI 等外部入口的 agent 会话查询与取消契约。
//!
//! 定义在 harness 层，consumer（mcp crate、CLI binary）仅依赖此 trait。
//! 实现由 agent crate 的 SessionManager 提供（SessionManager 实现本 trait）。
//!
//! 设计原则：
//! - 最小接口：只暴露 status 查询和 cancel 取消两个方法
//! - 结果类型独立于 agent crate 的 AgentSession，避免 consumer 越过 harness
//! - cancel 为 best-effort：如果会话已经结束则返回 already_terminal 语义的 Ok

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::session_state::SessionStatus;

/// MCP 工具 `agent_status` 的返回 DTO。
///
/// 独立于 agent crate 内部 AgentSession 结构，只暴露 consumer 关心的字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionStatusView {
    /// 会话唯一 ID
    pub session_id: String,
    /// 会话当前状态（复用 harness 统一的 8 态 SessionStatus）
    pub status: SessionStatus,
    /// 关联的 LLM provider 标识
    pub provider_id: String,
    /// 关联的 conversation_id（可能为空）
    pub conversation_id: Option<String>,
    /// 当前会话已完成的 turn 数（如果可获取）
    pub turn_count: Option<u32>,
    /// 是否为活跃态（initializing / running / waiting_approval）
    pub is_active: bool,
    /// 最近一次访问时间戳（epoch millis，内存会话可能不可用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_access_ms: Option<u64>,
    /// 最近一次错误信息（如果 failed）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// MCP / CLI 入口用于查询和取消 agent 会话的 broker 接口。
///
/// 实现要求：
/// - SessionManager（agent crate）是主实现，持有内存会话 HashMap
/// - wiring 层在初始化 McpAgentServer 时把 `Arc<dyn AgentSessionBroker>` 注入
/// - cancel 需幂等：已经 terminal 的会话返回 Ok(already_terminal=true)
#[async_trait]
pub trait AgentSessionBroker: Send + Sync + fmt::Debug {
    /// 查询指定会话的当前状态。
    ///
    /// 返回 Err 的语义：
    /// - "not_found" — 会话 ID 在内存和持久化层都不存在
    /// - 其他错误 — 持久化层读取失败等
    async fn get_session_status(&self, session_id: &str) -> Result<AgentSessionStatusView, String>;

    /// 取消指定会话的执行。
    ///
    /// 返回 Ok 的语义：
    /// - 如果会话正在运行 → 标记取消并返回 Ok
    /// - 如果会话已经 terminal（completed/failed/cancelled）→ Ok 且 no-op
    ///
    /// 返回 Err 的语义：
    /// - "not_found" — 会话不存在
    async fn cancel_session(&self, session_id: &str) -> Result<(), String>;

    /// 列出所有已知会话 ID（MCP 调试 / 运维用）。
    ///
    /// 默认实现返回空列表，具体实现可覆盖。
    async fn list_session_ids(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}
