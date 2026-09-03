// SPDX-License-Identifier: AGPL-3.0-only

//! McpAgentServer — 把 AxAgent 的 agent 能力暴露为 MCP stdio server。
//!
//! 外部 MCP 宿主（Claude Desktop、Cline、VSCode MCP 扩展等）可直接调用：
//! - gent_run — 给一个 goal，让 agent 自主执行并返回结果
//! - gent_status — 查询指定会话状态（接 SessionManager via AgentSessionBroker）
//! - gent_cancel — 取消指定会话（接 SessionManager via AgentSessionBroker）
//!
//! 传输层 stdio（MCP 标准 stdio transport）。
//!
//! 分层：此 crate 通过 harness trait 与 agent 实现解耦：
//! - Arc<dyn Agent>（execute / plan）
//! - Arc<dyn AgentSessionBroker>（status / cancel）
//!
//! wiring 层在 runtime/src/init/state.rs 里注入真实实例。

use axagent_harness::{Agent, AgentExecuteRequest, AgentSessionBroker};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

// ── MCP Tool 请求结构 ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentRunRequest {
    #[schemars(description = "Agent 要完成的目标描述（自然语言 prompt）")]
    pub goal: String,
    #[schemars(description = "可选：给 agent 的额外上下文（历史对话、文档片段等）")]
    pub context: Option<String>,
    #[schemars(description = "可选：最大执行步数上限，防止 agent 死循环")]
    pub max_steps: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentStatusRequest {
    #[schemars(description = "要查询的会话 ID")]
    pub session_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentCancelRequest {
    #[schemars(description = "要取消的会话 ID")]
    pub session_id: String,
}

// ── Server 本体 ────────────────────────────────────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
pub struct McpAgentServer {
    agent: Option<Arc<dyn Agent>>,
    session_broker: Option<Arc<dyn AgentSessionBroker>>,
}

impl std::fmt::Debug for McpAgentServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpAgentServer")
            .field("agent", &self.agent.is_some())
            .field("session_broker", &self.session_broker.is_some())
            .finish()
    }
}

impl McpAgentServer {
    pub fn new(
        agent: Option<Arc<dyn Agent>>,
        session_broker: Option<Arc<dyn AgentSessionBroker>>,
    ) -> Self {
        Self { agent, session_broker }
    }

    /// 无依赖构造：所有 tool 返回 "not configured" 占位。
    /// 用于独立测试或尚未 wiring 注入的场景。
    pub fn stub() -> Self {
        Self { agent: None, session_broker: None }
    }

    /// 注入 agent 实例（wiring 层调用）。
    pub fn with_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.agent = Some(agent);
        self
    }

    /// 注入 session broker（wiring 层调用）。
    pub fn with_session_broker(mut self, broker: Arc<dyn AgentSessionBroker>) -> Self {
        self.session_broker = Some(broker);
        self
    }
}

// ── Tool Router ─────────────────────────────────────────────────────────────

#[tool_router]
impl McpAgentServer {
    #[tool(
        description = "给 agent 一个目标（自然语言），让它自主规划并执行，返回最终结果。每次调用独立运行，不维护会话状态。"
    )]
    async fn agent_run(
        &self,
        Parameters(req): Parameters<AgentRunRequest>,
    ) -> Result<String, String> {
        let agent = self
            .agent
            .as_ref()
            .ok_or_else(|| "McpAgentServer 未配置 agent 实例（stub 模式）".to_string())?;

        let result = agent
            .execute(AgentExecuteRequest {
                goal: req.goal,
                context: req.context,
                max_steps: req.max_steps,
            })
            .await?;

        let status = if result.success { "success" } else { "failed" };
        let session_tag =
            result.session_id.as_ref().map(|sid| format!(" sessionId={sid}")).unwrap_or_default();
        Ok(format!(
            "[{status}] steps_taken={steps}{session_tag}\n{output}",
            steps = result.steps_taken,
            output = result.output,
        ))
    }

    #[tool(
        description = "查询指定 agent 会话的当前状态。返回 sessionStatus、isActive、turnCount 等字段。"
    )]
    async fn agent_status(
        &self,
        Parameters(req): Parameters<AgentStatusRequest>,
    ) -> Result<String, String> {
        let broker = self
            .session_broker
            .as_ref()
            .ok_or_else(|| "McpAgentServer 未配置 session broker（stub 模式）".to_string())?;

        let view = broker.get_session_status(&req.session_id).await?;
        let json = serde_json::to_string_pretty(&view).map_err(|e| format!("序列化错误: {e}"))?;
        Ok(json)
    }

    #[tool(description = "尝试取消指定 agent 会话的执行。幂等：terminal 状态会话直接返回 ok。")]
    async fn agent_cancel(
        &self,
        Parameters(req): Parameters<AgentCancelRequest>,
    ) -> Result<String, String> {
        let broker = self
            .session_broker
            .as_ref()
            .ok_or_else(|| "McpAgentServer 未配置 session broker（stub 模式）".to_string())?;

        broker.cancel_session(&req.session_id).await?;
        Ok(format!("cancelled: {}", req.session_id))
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────

impl ServerHandler for McpAgentServer {
    fn get_info(&self) -> ServerInfo {
        let caps = ServerCapabilities::builder().enable_tools().build();
        ServerInfo::new(caps)
            .with_server_info(Implementation::new("AxAgent", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "AxAgent MCP Server — 把 agent 能力暴露为 MCP tools。\
                 agent_run 接收自然语言 goal，agent 会自主规划并执行；\
                 agent_status 查询会话状态，agent_cancel 取消运行中会话。",
            )
    }
}

// ── 便捷：stdio 启动入口 ─────────────────────────────────────────────────────

/// 在当前 tokio runtime 上启动 stdio MCP server。
/// 阻塞直到客户端断开或进程退出。
pub async fn serve_stdio(
    server: McpAgentServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("McpAgentServer starting on stdio transport");
    let transport = rmcp::transport::stdio();
    let _running = server.serve(transport).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info() {
        let server = McpAgentServer::stub();
        let info = server.get_info();
        assert_eq!(info.server_info.name.as_str(), "AxAgent");
        assert!(info.instructions.is_some());
    }

    #[tokio::test]
    async fn test_agent_run_stub_returns_error() {
        let server = McpAgentServer::stub();
        let result = server
            .agent_run(Parameters(AgentRunRequest {
                goal: "hello".to_string(),
                context: None,
                max_steps: None,
            }))
            .await;
        assert!(result.is_err(), "stub mode should return err");
    }

    #[tokio::test]
    async fn test_agent_status_stub_returns_error() {
        let server = McpAgentServer::stub();
        let result = server
            .agent_status(Parameters(AgentStatusRequest { session_id: "any".to_string() }))
            .await;
        assert!(result.is_err(), "stub mode should return err");
    }
}
