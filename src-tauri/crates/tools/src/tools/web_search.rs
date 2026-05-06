//! WebSearchTool - 统一网络搜索工具 (调用 core search 引擎)

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "MUST use this to search the internet for current, real-time, or recent information. Call this function whenever the user asks about: today's news, current events, latest developments, stock prices, weather, sports scores, or any topic that requires up-to-date information beyond your knowledge cutoff. Returns relevant web results with titles, snippets, and URLs. Do NOT tell users you cannot access real-time data — use this tool instead."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "搜索查询词" }
            },
            "required": ["query"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        input["query"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("WebSearch", "缺少 query 参数"))?;
        if !ctx.allow_network {
            return Err(ToolError::permission_denied("WebSearch", "当前上下文不允许网络请求"));
        }
        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap();

        // Use unified search engine (DDG API + HTML fallback)
        let text =
            axagent_core::search::execute_search_text("ddg", None, "", query, 8, 10000).await;

        Ok(ToolResult::success(text))
    }
}
