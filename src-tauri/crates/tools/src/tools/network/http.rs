// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct HttpRequestTool;

#[async_trait]
impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "HttpRequest"
    }
    fn description(&self) -> &str {
        "发送通用 HTTP 请求。支持 GET/POST/PUT/PATCH/DELETE，自定义 headers、body（JSON/表单/文本）、超时和重定向控制。返回状态码和响应体。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "请求 URL"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"], "default": "GET"},
                "headers": {"type": "object", "description": "自定义请求头，如 {\"Authorization\": \"Bearer xxx\"}"},
                "body": {"type": "string", "description": "请求体（JSON 字符串/表单/文本）"},
                "content_type": {"type": "string", "default": "application/json", "description": "Content-Type 头"},
                "timeout_secs": {"type": "integer", "default": 30, "description": "超时秒数"},
                "follow_redirects": {"type": "boolean", "default": true}
            },
            "required": ["url"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return Ok(ToolResult::error("Error: url 是必需的"));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult::error("url 必须以 http:// 或 https:// 开头"));
        }
        if let Err(e) = is_safe_url(&url) {
            return Ok(ToolResult::error(format!("Error: {}", e.message)));
        }

        let method = input["method"].as_str().unwrap_or("GET").to_uppercase();
        let timeout = input["timeout_secs"].as_u64().unwrap_or(30);
        let follow = input["follow_redirects"].as_bool().unwrap_or(true);
        let content_type = input["content_type"].as_str().unwrap_or("application/json");

        let mut client_builder =
            reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout));
        if !follow {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }
        let client = client_builder
            .build()
            .map_err(|e| ToolError::execution_failed(format!("创建 HTTP 客户端失败: {}", e)))?;

        let req_builder = match method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            "HEAD" => client.head(&url),
            _ => client.get(&url),
        };

        let mut req = req_builder.header("Content-Type", content_type);
        req = req.header("User-Agent", "AxAgent/1.0");

        // 自定义 headers
        if let Some(headers) = input["headers"].as_object() {
            for (k, v) in headers {
                if let Some(val) = v.as_str() {
                    req = req.header(k.as_str(), val);
                }
            }
        }

        // Body
        if let Some(body) = input["body"].as_str()
            && !body.is_empty()
        {
            req = req.body(body.to_string());
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let resp_headers: Vec<String> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("?")))
                    .collect();

                match resp.text().await {
                    Ok(body) => {
                        let truncated = truncate(&body, 50_000);
                        let mut result = format!(
                            "HTTP {} {}\n\n响应头:\n{}\n\n响应体 ({} bytes):\n{}",
                            status,
                            http_status_text(status),
                            resp_headers.join("\n"),
                            body.len(),
                            truncated
                        );
                        if body.len() > 50_000 {
                            result.push_str(&format!("\n... (截断，原 {} bytes)", body.len()));
                        }
                        Ok(ToolResult::success(result))
                    },
                    Err(e) => Ok(ToolResult::error(format!("读取响应体失败: {}", e))),
                }
            },
            Err(e) => Ok(ToolResult::error(format!("HTTP 请求失败: {}", e))),
        }
    }
}
