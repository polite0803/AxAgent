// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-mcp-serve — AxAgent MCP stdio server 独立入口。
//!
//! 用法：直接运行 `axagent-mcp-serve`，通过 stdio 协议接收 MCP 请求。
//! 默认 stub 模式（无 agent 实例，tool 返回占位）；生产版本由 wiring 层
//! 在 runtime/src/init/state.rs 中注入真实 Arc<dyn Agent> 后启动。

use axagent_mcp::server::{McpAgentServer, serve_stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化日志：默认只输出 warn 以上，RUST_LOG=debug 可调
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let server = McpAgentServer::stub();
    serve_stdio(server).await
}
