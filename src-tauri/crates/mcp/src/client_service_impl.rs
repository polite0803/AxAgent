// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::mcp_service::McpClientService` 的默认实现。
//!
//! 包装本 crate 的 `mcp_client::discover_tools_unified` / `call_tool_unified`，
//! 让 gateway 通过 harness trait 调用 MCP，而不直接依赖 `axagent-mcp`。

use async_trait::async_trait;

use axagent_harness::mcp_service::{
    DiscoveredMcpTool, McpClientService, McpServerConfig, McpToolCallResult,
};
// McpResource / McpResourceContent / McpPrompt / McpPromptResult 定义在
// `axagent_harness::mcp_types` 并在 lib.rs 顶层 pub use；从这里直接走顶层路径
// 避免在 mcp_service 子模块内被识别为 private 导入。
use axagent_harness::{McpPrompt, McpPromptResult, McpResource, McpResourceContent};

use crate::mcp_client;

/// 基于 `mcp_client` unified 入口的 MCP 客户端服务实现。
#[derive(Debug, Default)]
pub struct DefaultMcpClientService;

#[async_trait]
impl McpClientService for DefaultMcpClientService {
    async fn discover_tools(
        &self,
        server: &McpServerConfig,
    ) -> std::result::Result<Vec<DiscoveredMcpTool>, String> {
        let tools = mcp_client::discover_tools_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(tools
            .into_iter()
            .map(|t| DiscoveredMcpTool {
                name: t.name,
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema.unwrap_or(serde_json::Value::Null),
            })
            .collect())
    }

    async fn call_tool(
        &self,
        server: &McpServerConfig,
        tool_name: &str,
        args: serde_json::Value,
    ) -> std::result::Result<McpToolCallResult, String> {
        let result = mcp_client::call_tool_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            tool_name,
            args,
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(McpToolCallResult {
            success: !result.is_error,
            // mcp_client 的 content 为纯文本字符串，序列化为 JSON 字符串保持
            // 原 HTTP 响应形态（此前直接 `json!({ "content": result.content })`）。
            content: serde_json::Value::String(result.content),
        })
    }

    // H1: resources / prompts support
    async fn list_resources(
        &self,
        server: &McpServerConfig,
    ) -> std::result::Result<Vec<McpResource>, String> {
        let resources = mcp_client::list_resources_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(resources)
    }

    async fn read_resource(
        &self,
        server: &McpServerConfig,
        uri: &str,
    ) -> std::result::Result<Vec<McpResourceContent>, String> {
        let contents = mcp_client::read_resource_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            uri,
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(contents)
    }

    async fn list_prompts(
        &self,
        server: &McpServerConfig,
    ) -> std::result::Result<Vec<McpPrompt>, String> {
        let prompts = mcp_client::list_prompts_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(prompts)
    }

    async fn get_prompt(
        &self,
        server: &McpServerConfig,
        name: &str,
        args: serde_json::Value,
    ) -> std::result::Result<McpPromptResult, String> {
        let result = mcp_client::get_prompt_unified(
            &server.transport,
            server.command.as_deref(),
            server.args.as_deref(),
            server.env.as_ref(),
            server.endpoint.as_deref(),
            name,
            args,
            Some(&server.id),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(result)
    }
}

/// 构造 `McpClientService`，供 gateway wiring 层注入。
pub fn build_mcp_client_service() -> std::sync::Arc<dyn McpClientService> {
    std::sync::Arc::new(DefaultMcpClientService)
}
