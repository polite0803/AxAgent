// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct RssReaderTool;

#[async_trait]
impl Tool for RssReaderTool {
    fn name(&self) -> &str {
        "RssReader"
    }
    fn description(&self) -> &str {
        "读取 RSS/Atom 订阅源。自动识别格式，提取标题、链接、发布日期和摘要。支持最多 50 条条目。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "RSS/Atom 订阅 URL"},
                "limit": {"type": "integer", "default": 20, "minimum": 1, "maximum": 50}
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

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return Ok(ToolResult::error("Error: url 是必需的"));
        }
        let limit = input["limit"].as_u64().unwrap_or(20).min(50) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| ToolError::execution_failed(format!("创建客户端失败: {}", e)))?;

        let resp = client
            .get(&url)
            .header("User-Agent", "AxAgent/1.0 RSS Reader")
            .header(
                "Accept",
                "application/rss+xml, application/atom+xml, application/xml, text/xml",
            )
            .send()
            .await
            .map_err(|e| ToolError::execution_failed(format!("请求失败: {}", e)))?;

        let body = resp
            .text()
            .await
            .map_err(|e| ToolError::execution_failed(format!("读取响应失败: {}", e)))?;

        // 自动识别 RSS 或 Atom
        let is_atom =
            body.contains("xmlns=\"http://www.w3.org/2005/Atom\"") || body.contains("<feed");

        let entries = if is_atom {
            parse_atom_feed(&body, limit)?
        } else {
            parse_rss_feed(&body, limit)?
        };

        if entries.is_empty() {
            return Ok(ToolResult::success(format!(
                "未在 {} 中找到条目。\n响应长度: {} bytes。响应可能是非标准格式。",
                url,
                body.len()
            )));
        }

        let mut result = format!("📰 RSS 订阅: {}\n\n{} 条条目:\n", url, entries.len());
        for (i, (title, link, date, desc)) in entries.iter().enumerate() {
            result.push_str(&format!(
                "{}. **{}**\n   链接: {}\n   日期: {}\n   摘要: {}\n\n",
                i + 1,
                title,
                link,
                date,
                truncate(desc, 300)
            ));
        }

        Ok(ToolResult::success(result))
    }
}

fn parse_rss_feed(
    xml: &str,
    limit: usize,
) -> Result<Vec<(String, String, String, String)>, ToolError> {
    let mut entries = Vec::new();
    let item_re = regex::Regex::new(r"(?s)<item>(.*?)</item>")
        .map_err(|e| ToolError::invalid_input(format!("正则表达式无效: {}", e)))?;
    for cap in item_re.captures_iter(xml).take(limit) {
        let item = &cap[1];
        let title = extract_xml_tag(item, "title").unwrap_or_default();
        let link = extract_xml_tag(item, "link").unwrap_or_default();
        let date = extract_xml_tag(item, "pubDate").unwrap_or_default();
        let desc = extract_xml_tag(item, "description")
            .or_else(|| extract_xml_cdata(item, "description"))
            .unwrap_or_default();
        if !title.is_empty() {
            entries.push((strip_html(&title)?, link, date, strip_html(&desc)?));
        }
    }
    Ok(entries)
}

fn parse_atom_feed(
    xml: &str,
    limit: usize,
) -> Result<Vec<(String, String, String, String)>, ToolError> {
    let mut entries = Vec::new();
    let entry_re = regex::Regex::new(r"(?s)<entry>(.*?)</entry>")
        .map_err(|e| ToolError::invalid_input(format!("正则表达式无效: {}", e)))?;
    for cap in entry_re.captures_iter(xml).take(limit) {
        let item = &cap[1];
        let title = extract_xml_tag(item, "title").unwrap_or_default();
        let link = extract_atom_link(item)?;
        let date = extract_xml_tag(item, "updated")
            .or_else(|| extract_xml_tag(item, "published"))
            .unwrap_or_default();
        let desc = extract_xml_tag(item, "summary")
            .or_else(|| extract_xml_tag(item, "content"))
            .unwrap_or_default();
        if !title.is_empty() {
            entries.push((strip_html(&title)?, link, date, strip_html(&desc)?));
        }
    }
    Ok(entries)
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}[^>]*>(.*?)</{}>", tag, tag);
    regex::Regex::new(&pattern).ok().and_then(|re| re.captures(xml).map(|c| c[1].to_string()))
}

fn extract_xml_cdata(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}[^>]*><!\[CDATA\[(.*?)\]\]></{}>", tag, tag);
    regex::Regex::new(&pattern).ok().and_then(|re| re.captures(xml).map(|c| c[1].to_string()))
}

fn extract_atom_link(xml: &str) -> Result<String, ToolError> {
    let re = regex::Regex::new(r#"<link[^>]*href="([^"]*)"[^>]*/>"#)
        .map_err(|e| ToolError::invalid_input(format!("正则表达式无效: {}", e)))?;
    Ok(re.captures(xml).map(|c| c[1].to_string()).unwrap_or_default())
}

fn strip_html(text: &str) -> Result<String, ToolError> {
    let re = regex::Regex::new(r"<[^>]+>")
        .map_err(|e| ToolError::invalid_input(format!("正则表达式无效: {}", e)))?;
    let decoded = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'");
    Ok(re.replace_all(&decoded, "").trim().to_string())
}

// GraphQL — GraphQL 查询/变更
