//! WebFetchTool - 网页抓取工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

const MAX_CONTENT_LENGTH: usize = 200_000;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }
    fn description(&self) -> &str {
        "从指定 URL 抓取内容并转换为文本。用于获取网页、文档、API 数据。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的 URL"
                },
                "prompt": {
                    "type": "string",
                    "description": "从页面中提取什么的指令"
                }
            },
            "required": ["url"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("WebFetch", "缺少 url 参数"))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::invalid_input(
                "url 必须以 http:// 或 https:// 开头",
            ));
        }

        // SSRF 保护：禁止内网地址
        if url.contains("localhost") || url.contains("127.0.0.1") || url.contains("0.0.0.0") {
            return Err(ToolError::permission_denied("WebFetch", "禁止访问本地地址"));
        }

        if !ctx.allow_network {
            return Err(ToolError::permission_denied(
                "WebFetch",
                "当前上下文不允许网络请求",
            ));
        }

        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap();
        let prompt = input["prompt"].as_str().unwrap_or("提取页面主要内容");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("AxAgent/1.0 WebFetchTool")
            .build()
            .map_err(|e| ToolError::execution_failed(format!("HTTP 客户端创建失败: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::execution_failed(format!("HTTP 请求失败: {}", e)))?;

        let status = response.status();
        // 先获取 headers 信息（在消耗 body 之前）
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| ToolError::execution_failed(format!("读取响应失败: {}", e)))?;

        let extracted =
            if content_type.contains("text/html") || content_type.contains("application/xhtml") {
                // HTML -> 文本
                html_to_text(&body)
            } else if content_type.contains("application/json") {
                // JSON 美化
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    serde_json::to_string_pretty(&json).unwrap_or(body)
                } else {
                    body
                }
            } else {
                body
            };

        let result = format!(
            "## URL: {}\n状态: {}\nContent-Type: {}\n\n{}\n\n提示: {}\n",
            url,
            status,
            content_type,
            if extracted.len() > MAX_CONTENT_LENGTH {
                format!("{}{}", &extracted[..MAX_CONTENT_LENGTH], "\n[内容已截断]")
            } else {
                extracted
            },
            prompt
        );

        if result.len() > MAX_CONTENT_LENGTH {
            Ok(ToolResult::truncated(result, MAX_CONTENT_LENGTH))
        } else {
            Ok(ToolResult::success(result))
        }
    }
}

fn html_to_text(html: &str) -> String {
    // 使用 scraper 解析
    let document = scraper::Html::parse_document(html);

    // 移除 script 和 style
    let _selector = scraper::Selector::parse("script, style, nav, footer, header").unwrap();
    let mut text = String::new();

    // 优先提取 <main>, <article> 或 <body>
    let main_sel = scraper::Selector::parse("main, article, body").unwrap();
    if let Some(main) = document.select(&main_sel).next() {
        for node in main.text() {
            text.push_str(node);
        }
    } else {
        for node in document.root_element().text() {
            text.push_str(node);
        }
    }

    // 清理多余空白
    let cleaned = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    cleaned
}
