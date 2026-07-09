// SPDX-License-Identifier: AGPL-3.0-only

//! AgentSession 持久化契约 trait。
//!
//! 定义在 harness 层，业务层（agent）仅依赖此 trait，
//! 实现由 dao 层提供。

use async_trait::async_trait;

use crate::core_error::Result;
use crate::types::AgentSession;

/// Agent 会话持久化操作。
#[async_trait]
pub trait AgentSessionRepository: Send + Sync {
    /// Upsert：若对应 conversation 不存在则创建，否则更新 cwd / permission_mode。
    async fn upsert_agent_session(
        &self,
        conversation_id: &str,
        cwd: Option<&str>,
        permission_mode: Option<&str>,
    ) -> Result<AgentSession>;

    /// 更新运行时状态。
    async fn update_agent_session_status(&self, id: &str, runtime_status: &str) -> Result<()>;

    /// 查询完成后更新 sdk_context / tokens / cost。
    async fn update_agent_session_after_query(
        &self,
        id: &str,
        runtime_status: &str,
        sdk_context_json: Option<&str>,
        tokens_delta: i32,
        cost_delta: f64,
    ) -> Result<()>;

    /// 按 conversation_id 清空 sdk_context_json。
    async fn clear_sdk_context_by_conversation_id(&self, conversation_id: &str) -> Result<()>;
}
