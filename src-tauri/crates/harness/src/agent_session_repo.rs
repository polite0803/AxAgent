// SPDX-License-Identifier: AGPL-3.0-only

//! AgentSession 持久化契约 trait。
//!
//! 定义在 harness 层，业务层（agent）仅依赖此 trait，
//! 实现由 dao 层提供。

use async_trait::async_trait;

use crate::core_error::Result;
use crate::types::AgentSession;
use crate::types::session_state::SessionStatus;

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

    /// 更新运行时状态（字符串形式，向后兼容）。
    async fn update_agent_session_status(&self, id: &str, runtime_status: &str) -> Result<()>;

    /// 更新运行时状态（类型安全枚举形式，新代码优先使用）。
    async fn update_session_status_enum(&self, id: &str, status: SessionStatus) -> Result<()> {
        self.update_agent_session_status(id, status.as_str()).await
    }

    /// 查询完成后更新 sdk_context / tokens / cost（字符串形式，向后兼容）。
    async fn update_agent_session_after_query(
        &self,
        id: &str,
        runtime_status: &str,
        sdk_context_json: Option<&str>,
        tokens_delta: i64,
        cost_delta: f64,
    ) -> Result<()>;

    /// 查询完成后更新 sdk_context / tokens / cost（枚举形式，新代码优先使用）。
    async fn update_session_after_query_enum(
        &self,
        id: &str,
        status: SessionStatus,
        sdk_context_json: Option<&str>,
        tokens_delta: i64,
        cost_delta: f64,
    ) -> Result<()> {
        self.update_agent_session_after_query(
            id,
            status.as_str(),
            sdk_context_json,
            tokens_delta,
            cost_delta,
        )
        .await
    }

    /// 按 conversation_id 清空 sdk_context_json。
    async fn clear_sdk_context_by_conversation_id(&self, conversation_id: &str) -> Result<()>;

    /// 按主键查询 AgentSession（DB 回退用，内存未命中时查 DB）。
    async fn get_by_id(&self, id: &str) -> Result<Option<AgentSession>>;

    /// 按 conversation_id 查询 AgentSession（DB 回退用，内存未命中时查 DB）。
    async fn get_by_conversation_id(&self, conversation_id: &str) -> Result<Option<AgentSession>>;

    /// 列出全部持久化 AgentSession（会话管理面板 / 历史列表用）。
    async fn list_all(&self) -> Result<Vec<AgentSession>>;
}
