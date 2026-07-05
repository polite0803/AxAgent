// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct JsonApiTool;

#[async_trait]
impl Tool for JsonApiTool {
    fn name(&self) -> &str {
        "JsonApi"
    }
    fn description(&self) -> &str {
        "调用 JSON API 并提取结构化数据。自动设置 Content-Type: application/json，解析 JSON 响应，支持用 JSON 路径（如 data.items[0].name）提取特定字段。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "API URL"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"], "default": "GET"},
                "headers": {"type": "object", "description": "额外请求头"},
                "body": {"type": "object", "description": "请求体（JSON 对象）"},
                "extract_path": {"type": "string", "description": "提取路径，如 data.items[0].name。不填返回完整 JSON。"},
                "timeout_secs": {"type": "integer", "default": 30}
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
        if let Err(e) = is_safe_url(&url) {
            return Ok(ToolResult::error(format!("Error: {}", e.message)));
        }

        let method = input["method"].as_str().unwrap_or("GET").to_uppercase();
        let timeout = input["timeout_secs"].as_u64().unwrap_or(30);
        let extract_path = input["extract_path"].as_str().unwrap_or("");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| ToolError::execution_failed(format!("创建客户端失败: {}", e)))?;

        let req_builder = match method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => client.get(&url),
        };

        let mut req = req_builder
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "AxAgent/1.0");

        if let Some(headers) = input["headers"].as_object() {
            for (k, v) in headers {
                if let Some(val) = v.as_str() {
                    req = req.header(k.as_str(), val);
                }
            }
        }

        if let Some(body) = input["body"].as_object() {
            req = req.json(body);
        } else if let Some(body_str) = input["body"].as_str()
            && !body_str.is_empty()
        {
            req = req.body(body_str.to_string());
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                match serde_json::from_str::<Value>(&body) {
                    Ok(json) => {
                        if status >= 400 {
                            return Ok(ToolResult::success(format!(
                                "API 错误 {} {}\n\n{}",
                                status,
                                http_status_text(status),
                                serde_json::to_string_pretty(&json).unwrap_or_default()
                            )));
                        }

                        if extract_path.is_empty() {
                            let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
                            return Ok(ToolResult::success(truncate(&pretty, 50_000)));
                        }

                        // JSON 路径提取
                        match json_path_get(&json, extract_path) {
                            Some(value) => {
                                let pretty =
                                    serde_json::to_string_pretty(&value).unwrap_or_default();
                                Ok(ToolResult::success(format!(
                                    "提取路径: {}\n\n{}",
                                    extract_path,
                                    truncate(&pretty, 50_000)
                                )))
                            },
                            None => Ok(ToolResult::success(format!(
                                "路径 '{}' 未匹配到值。\n\n完整响应:\n{}",
                                extract_path,
                                truncate(
                                    &serde_json::to_string_pretty(&json).unwrap_or_default(),
                                    10_000
                                )
                            ))),
                        }
                    },
                    Err(_) => Ok(ToolResult::success(format!(
                        "HTTP {} — 非 JSON 响应\n\n{}",
                        status,
                        truncate(&body, 20_000)
                    ))),
                }
            },
            Err(e) => Ok(ToolResult::error(format!("API 请求失败: {}", e))),
        }
    }
}

/// 简单 JSON 路径取值：data.items[0].name
fn json_path_get<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        // 处理数组索引，如 items[0]
        if let Some(bracket) = segment.find('[') {
            let field = &segment[..bracket];
            let rest = &segment[bracket..];
            if !field.is_empty() {
                current = current.get(field)?;
            }
            // 处理 [0] 索引
            for part in rest.split(']') {
                let idx_str = part.trim_start_matches('[');
                if idx_str.is_empty() {
                    continue;
                }
                let idx: usize = idx_str.parse().ok()?;
                current = current.get(idx)?;
            }
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

// RssReader — RSS/Atom 订阅阅读
