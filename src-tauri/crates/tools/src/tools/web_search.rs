//! WebSearchTool - 网络搜索工具

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
        "通过网络搜索引擎搜索信息，返回相关结果及摘要。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索查询词"
                },
                "allowed_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "限制搜索结果的域名列表"
                },
                "blocked_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "排除的域名列表"
                }
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
        let _query = input["query"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("WebSearch", "缺少 query 参数"))?;

        if !ctx.allow_network {
            return Err(ToolError::permission_denied(
                "WebSearch",
                "当前上下文不允许网络请求",
            ));
        }

        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap();

        // 使用 DuckDuckGo HTML 搜索（免 API key）
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("AxAgent/1.0 WebSearchTool")
            .build()
            .map_err(|e| ToolError::execution_failed(format!("HTTP 客户端创建失败: {}", e)))?;

        let response = client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| ToolError::execution_failed(format!("搜索请求失败: {}", e)))?;

        let body = response
            .text()
            .await
            .map_err(|e| ToolError::execution_failed(format!("读取搜索结果失败: {}", e)))?;

        // 解析 DuckDuckGo HTML 结果
        let document = scraper::Html::parse_document(&body);
        let result_sel = scraper::Selector::parse(".result").unwrap();
        let title_sel = scraper::Selector::parse(".result__title").unwrap();
        let snippet_sel = scraper::Selector::parse(".result__snippet").unwrap();
        let url_sel = scraper::Selector::parse(".result__url").unwrap();

        let mut results = Vec::new();
        for result_el in document.select(&result_sel) {
            let title = result_el
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_default()
                .trim()
                .to_string();

            let snippet = result_el
                .select(&snippet_sel)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_default()
                .trim()
                .to_string();

            let url = result_el
                .select(&url_sel)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            if !title.is_empty() && !snippet.is_empty() {
                results.push(format!("**{}**\n  {} \n  {}", title, url, snippet));
            }
        }

        if results.is_empty() {
            return Ok(ToolResult::success(format!(
                "搜索 '{}' 未找到结果。",
                query
            )));
        }

        let mut output = format!("## 搜索结果: \"{}\"\n\n", query);
        for (i, result) in results.iter().take(10).enumerate() {
            output.push_str(&format!("{}. {}\n\n", i + 1, result));
        }

        Ok(ToolResult::success(output))
    }
}
