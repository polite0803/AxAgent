//! 外部集成工具
//!
//! DifyListBases / DifySearch — Dify 知识库平台集成

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

fn truncate_text(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

pub struct DifyListBasesTool;

#[async_trait]
impl Tool for DifyListBasesTool {
    fn name(&self) -> &str {
        "DifyListBases"
    }
    fn description(&self) -> &str {
        "列出 Dify 平台上的知识库。需要 api_base（Dify API 地址）和 api_key（访问令牌）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"api_base":{"type":"string"},"api_key":{"type":"string"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let api_base = input.get("api_base").and_then(|v| v.as_str()).unwrap_or("");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        if api_base.is_empty() || api_key.is_empty() {
            return Ok(ToolResult::error(
                "Error: api_base 和 api_key 是必需的。请在设置中配置 Dify 连接。",
            ));
        }
        let url = format!("{}/v1/knowledge-bases", api_base.trim_end_matches('/'));
        let client = reqwest::Client::new();
        match client.get(&url).bearer_auth(api_key).send().await {
            Ok(resp) => match resp.text().await {
                Ok(body) => {
                    Ok(ToolResult::success(format!("Dify 知识库:\n{}", truncate_text(&body, 8000))))
                },
                Err(e) => Ok(ToolResult::error(format!("读取响应失败: {}", e))),
            },
            Err(e) => Ok(ToolResult::error(format!("请求 Dify API 失败: {}", e))),
        }
    }
}

pub struct DifySearchTool;

#[async_trait]
impl Tool for DifySearchTool {
    fn name(&self) -> &str {
        "DifySearch"
    }
    fn description(&self) -> &str {
        "在 Dify 知识库中搜索文档。需要 api_base、api_key、base_id 和 query。返回 top_k 结果。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"api_base":{"type":"string"},"api_key":{"type":"string"},"base_id":{"type":"string"},"query":{"type":"string"},"top_k":{"type":"integer","default":5}},"required":["api_base","api_key","base_id","query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let api_base = input.get("api_base").and_then(|v| v.as_str()).unwrap_or("");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        let base_id = input.get("base_id").and_then(|v| v.as_str()).unwrap_or("");
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let top_k = input.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5);

        if api_base.is_empty() || api_key.is_empty() {
            return Ok(ToolResult::error("Error: api_base 和 api_key 是必需的"));
        }
        if base_id.is_empty() || query.is_empty() {
            return Ok(ToolResult::error("Error: base_id 和 query 是必需的"));
        }
        let url =
            format!("{}/v1/knowledge-bases/{}/search", api_base.trim_end_matches('/'), base_id);
        let body = serde_json::json!({"query": query, "top_k": top_k});
        let client = reqwest::Client::new();
        match client
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(body) => Ok(ToolResult::success(format!(
                    "Dify 搜索结果:\n{}",
                    truncate_text(&body, 8000)
                ))),
                Err(e) => Ok(ToolResult::error(format!("读取响应失败: {}", e))),
            },
            Err(e) => Ok(ToolResult::error(format!("请求 Dify API 失败: {}", e))),
        }
    }
}
