// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct GraphQLTool;

#[async_trait]
impl Tool for GraphQLTool {
    fn name(&self) -> &str {
        "GraphQL"
    }
    fn description(&self) -> &str {
        "执行 GraphQL 查询或变更（mutation）。自动设置 Content-Type: application/json，支持变量替换。返回结构化 JSON 数据。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "endpoint": {"type": "string", "description": "GraphQL 端点 URL"},
                "query": {"type": "string", "description": "GraphQL 查询/变更字符串"},
                "variables": {"type": "object", "description": "查询变量（JSON 对象）"},
                "headers": {"type": "object", "description": "额外请求头，如 Authorization"},
                "timeout_secs": {"type": "integer", "default": 30}
            },
            "required": ["endpoint", "query"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let endpoint = input["endpoint"].as_str().unwrap_or("").to_string();
        let query = input["query"].as_str().unwrap_or("").to_string();
        if endpoint.is_empty() || query.is_empty() {
            return Ok(ToolResult::error("Error: endpoint 和 query 是必需的"));
        }
        let timeout = input["timeout_secs"].as_u64().unwrap_or(30);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| ToolError::execution_failed(format!("创建客户端失败: {}", e)))?;

        let mut body = serde_json::json!({ "query": query });
        if let Some(vars) = input["variables"].as_object() {
            body["variables"] = vars.clone().into();
        }

        let mut req = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("User-Agent", "AxAgent/1.0")
            .json(&body);

        if let Some(headers) = input["headers"].as_object() {
            for (k, v) in headers {
                if let Some(val) = v.as_str() {
                    req = req.header(k.as_str(), val);
                }
            }
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                match resp.json::<Value>().await {
                    Ok(json) => {
                        if let Some(errors) = json.get("errors") {
                            let pretty = serde_json::to_string_pretty(&errors).unwrap_or_default();
                            Ok(ToolResult::success(format!(
                                "GraphQL 错误 (HTTP {}):\n{}",
                                status, pretty
                            )))
                        } else if let Some(data) = json.get("data") {
                            let pretty = serde_json::to_string_pretty(data).unwrap_or_default();
                            Ok(ToolResult::success(format!(
                                "GraphQL 响应 (HTTP {}):\n{}",
                                status,
                                truncate(&pretty, 50_000)
                            )))
                        } else {
                            let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
                            Ok(ToolResult::success(format!(
                                "GraphQL 响应 (HTTP {}):\n{}",
                                status,
                                truncate(&pretty, 50_000)
                            )))
                        }
                    },
                    Err(e) => Ok(ToolResult::error(format!("解析 JSON 响应失败: {}", e))),
                }
            },
            Err(e) => Ok(ToolResult::error(format!("GraphQL 请求失败: {}", e))),
        }
    }
}

// WebSocket — WebSocket 客户端（基于 tokio::net::TcpStream）
