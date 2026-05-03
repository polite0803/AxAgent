//! MCP 传输适配器 - 实际 MCP 调用通过 axagent_core::mcp_client
//! 本文件仅保留类型定义，待后续与 core 层 MCP 客户端深度集成。

use crate::mcp::{McpServerConfig, McpToolDescriptor, McpTransport};

pub trait McpAdapter: Send + Sync {
    fn transport(&self) -> McpTransport;
    fn list_tools(&self, config: &McpServerConfig) -> Result<Vec<McpToolDescriptor>, String>;
    fn call_tool(
        &self,
        config: &McpServerConfig,
        tool_name: &str,
        input: &str,
    ) -> Result<String, String>;
}

pub struct StdioAdapter;
impl McpAdapter for StdioAdapter {
    fn transport(&self) -> McpTransport {
        McpTransport::Stdio
    }
    fn list_tools(&self, _config: &McpServerConfig) -> Result<Vec<McpToolDescriptor>, String> {
        Ok(Vec::new()) // 由 registry 层通过 core::mcp_client 实际实现
    }
    fn call_tool(
        &self,
        _config: &McpServerConfig,
        _tool_name: &str,
        _input: &str,
    ) -> Result<String, String> {
        Ok("{}".into())
    }
}
