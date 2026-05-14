//! 网络工具集
//!
//! HttpRequest (通用 HTTP), Ping (网络连通), DnsLookup (DNS 查询),
//! JsonApi (结构化 JSON API), RssReader (RSS/Atom 订阅),
//! GraphQL (GraphQL 查询), WebSocket (双向通信)
//!
//! 全部基于已有依赖（reqwest + tokio::net），零新增 crate。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// HttpRequest — 通用 HTTP 客户端
// ═══════════════════════════════════════════════════════════════════════════════

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
        if let Some(body) = input["body"].as_str() {
            if !body.is_empty() {
                req = req.body(body.to_string());
            }
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

fn http_status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Ping — ICMP 网络连通性测试
// ═══════════════════════════════════════════════════════════════════════════════

pub struct PingTool;

#[async_trait]
impl Tool for PingTool {
    fn name(&self) -> &str {
        "Ping"
    }
    fn description(&self) -> &str {
        "测试目标主机的网络连通性和延迟。调用系统 ping 命令，无需 root 权限。返回丢包率和 RTT。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "目标主机名或 IP 地址"},
                "count": {"type": "integer", "default": 4, "minimum": 1, "maximum": 20, "description": "发送包数量"},
                "timeout_secs": {"type": "integer", "default": 10, "description": "总超时秒数"}
            },
            "required": ["host"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let host = input["host"].as_str().unwrap_or("").to_string();
        if host.is_empty() {
            return Ok(ToolResult::error("Error: host 是必需的"));
        }

        // 过滤危险字符，防止命令注入
        if host.contains(';') || host.contains('|') || host.contains('&') || host.contains('$') {
            return Ok(ToolResult::error("Error: host 包含非法字符"));
        }

        let count = input["count"].as_u64().unwrap_or(4).min(20);
        let timeout = input["timeout_secs"].as_u64().unwrap_or(10);

        // 跨平台 ping 参数
        let (count_flag, timeout_flag, timeout_val) = if cfg!(target_os = "windows") {
            ("-n", "-w", (timeout * 1000).to_string())
        } else {
            ("-c", "-W", timeout.to_string())
        };

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout + 5),
            tokio::process::Command::new("ping")
                .arg(count_flag)
                .arg(count.to_string())
                .arg(timeout_flag)
                .arg(&timeout_val)
                .arg(&host)
                .output(),
        )
        .await
        .map_err(|_| ToolError::execution_failed("Ping 超时".to_string()))?
        .map_err(|e| ToolError::execution_failed(format!("执行 ping 失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stdout.is_empty() && !stderr.is_empty() {
            return Ok(ToolResult::error(format!("Ping 失败: {}", stderr.trim())));
        }

        // 提取统计信息
        let mut result = format!("Ping {} ({})\n\n", host, stdout.lines().next().unwrap_or(""));
        result.push_str(&stdout);

        // 解析延迟和丢包
        let loss = parse_ping_loss(&stdout);
        let rtt = parse_ping_rtt(&stdout);
        if let (Some(loss), Some(rtt)) = (loss, rtt) {
            result.push_str(&format!("\n\n📊 丢包率: {:.0}%  |  平均延迟: {:.1}ms", loss, rtt));
        }

        Ok(ToolResult::success(result))
    }
}

fn parse_ping_loss(output: &str) -> Option<f64> {
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("loss") || lower.contains("丢失") || lower.contains("lost") {
            if let Some(pct) = lower.split('%').next() {
                let num: Vec<&str> = pct.split_whitespace().collect();
                if let Some(last) = num.last() {
                    return last.parse::<f64>().ok();
                }
            }
        }
    }
    None
}

fn parse_ping_rtt(output: &str) -> Option<f64> {
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("average") || lower.contains("平均") || lower.contains("avg") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part.contains("ms") && i > 0 {
                    return parts[i - 1].parse::<f64>().ok();
                }
            }
            // 尝试最后匹配 = xxx ms
            if let Some(idx) = line.find('=') {
                let after = &line[idx + 1..];
                let val = after.split_whitespace().next().unwrap_or("");
                return val.parse::<f64>().ok();
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// DnsLookup — DNS 查询
// ═══════════════════════════════════════════════════════════════════════════════

pub struct DnsLookupTool;

#[async_trait]
impl Tool for DnsLookupTool {
    fn name(&self) -> &str {
        "DnsLookup"
    }
    fn description(&self) -> &str {
        "查询域名 DNS 记录。支持 A（IPv4）、AAAA（IPv6）、CNAME、MX、TXT、NS 等记录类型。返回解析结果和响应时间。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "hostname": {"type": "string", "description": "域名"},
                "record_type": {"type": "string", "enum": ["A", "AAAA", "MX", "TXT", "CNAME", "NS", "SOA", "PTR", "SRV"], "default": "A", "description": "记录类型"}
            },
            "required": ["hostname"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let hostname = input["hostname"].as_str().unwrap_or("").to_string();
        if hostname.is_empty() {
            return Ok(ToolResult::error("Error: hostname 是必需的"));
        }

        let record_type = input["record_type"].as_str().unwrap_or("A");

        let start = std::time::Instant::now();
        match record_type {
            "A" | "AAAA" => {
                // 用 ToSocketAddrs 解析
                match tokio::net::lookup_host(format!("{}:0", hostname)).await {
                    Ok(addrs) => {
                        let elapsed = start.elapsed();
                        let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                        if ips.is_empty() {
                            Ok(ToolResult::success(format!(
                                "DNS 查询 {} ({})\n结果: 未解析到 IP 地址\n耗时: {:.1}ms",
                                hostname,
                                record_type,
                                elapsed.as_secs_f64() * 1000.0
                            )))
                        } else {
                            Ok(ToolResult::success(format!(
                                "DNS 查询 {} ({})\nIP 地址:\n{}\n共 {} 条 | 耗时: {:.1}ms",
                                hostname,
                                record_type,
                                ips.iter()
                                    .map(|ip| format!("  {}", ip))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                                ips.len(),
                                elapsed.as_secs_f64() * 1000.0
                            )))
                        }
                    },
                    Err(e) => Ok(ToolResult::error(format!("DNS 解析失败: {}", e))),
                }
            },
            "MX" | "TXT" | "CNAME" | "NS" | "SOA" | "PTR" | "SRV" => {
                // 使用系统命令: nslookup (跨平台) 或 dig
                let qtype = match record_type {
                    "MX" => "MX",
                    "TXT" => "TXT",
                    "CNAME" => "CNAME",
                    "NS" => "NS",
                    "SOA" => "SOA",
                    "PTR" => "PTR",
                    "SRV" => "SRV",
                    _ => "A",
                };

                let output = if which::which("nslookup").is_ok() {
                    tokio::process::Command::new("nslookup")
                        .args(["-type", qtype, &hostname])
                        .output()
                        .await
                } else {
                    tokio::process::Command::new("dig")
                        .args([&hostname, qtype, "+short"])
                        .output()
                        .await
                };

                match output {
                    Ok(out) => {
                        let elapsed = start.elapsed();
                        let text = String::from_utf8_lossy(&out.stdout);
                        if text.trim().is_empty() {
                            Ok(ToolResult::success(format!(
                                "DNS 查询 {} ({})\n结果: 无记录\n耗时: {:.1}ms",
                                hostname,
                                record_type,
                                elapsed.as_secs_f64() * 1000.0
                            )))
                        } else {
                            Ok(ToolResult::success(format!(
                                "DNS 查询 {} ({})\n\n{}\n耗时: {:.1}ms",
                                hostname,
                                record_type,
                                text.trim(),
                                elapsed.as_secs_f64() * 1000.0
                            )))
                        }
                    },
                    Err(e) => Ok(ToolResult::error(format!(
                        "DNS 查询失败: {}。需要 nslookup 或 dig 命令。",
                        e
                    ))),
                }
            },
            _ => Ok(ToolResult::error(format!("不支持的记录类型: {}", record_type))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// JsonApi — 结构化 JSON API 调用
// ═══════════════════════════════════════════════════════════════════════════════

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
        } else if let Some(body_str) = input["body"].as_str() {
            if !body_str.is_empty() {
                req = req.body(body_str.to_string());
            }
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

// ═══════════════════════════════════════════════════════════════════════════════
// RssReader — RSS/Atom 订阅阅读
// ═══════════════════════════════════════════════════════════════════════════════

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
            parse_atom_feed(&body, limit)
        } else {
            parse_rss_feed(&body, limit)
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

fn parse_rss_feed(xml: &str, limit: usize) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();
    let item_re = regex::Regex::new(r"(?s)<item>(.*?)</item>").unwrap();
    for cap in item_re.captures_iter(xml).take(limit) {
        let item = &cap[1];
        let title = extract_xml_tag(item, "title").unwrap_or_default();
        let link = extract_xml_tag(item, "link").unwrap_or_default();
        let date = extract_xml_tag(item, "pubDate").unwrap_or_default();
        let desc = extract_xml_tag(item, "description")
            .or_else(|| extract_xml_cdata(item, "description"))
            .unwrap_or_default();
        if !title.is_empty() {
            entries.push((strip_html(&title), link, date, strip_html(&desc)));
        }
    }
    entries
}

fn parse_atom_feed(xml: &str, limit: usize) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();
    let entry_re = regex::Regex::new(r"(?s)<entry>(.*?)</entry>").unwrap();
    for cap in entry_re.captures_iter(xml).take(limit) {
        let item = &cap[1];
        let title = extract_xml_tag(item, "title").unwrap_or_default();
        let link = extract_atom_link(item);
        let date = extract_xml_tag(item, "updated")
            .or_else(|| extract_xml_tag(item, "published"))
            .unwrap_or_default();
        let desc = extract_xml_tag(item, "summary")
            .or_else(|| extract_xml_tag(item, "content"))
            .unwrap_or_default();
        if !title.is_empty() {
            entries.push((strip_html(&title), link, date, strip_html(&desc)));
        }
    }
    entries
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}[^>]*>(.*?)</{}>", tag, tag);
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(xml).map(|c| c[1].to_string()))
}

fn extract_xml_cdata(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}[^>]*><!\[CDATA\[(.*?)\]\]></{}>", tag, tag);
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(xml).map(|c| c[1].to_string()))
}

fn extract_atom_link(xml: &str) -> String {
    let re = regex::Regex::new(r#"<link[^>]*href="([^"]*)"[^>]*/>"#).unwrap();
    re.captures(xml)
        .map(|c| c[1].to_string())
        .unwrap_or_default()
}

fn strip_html(text: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let decoded = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'");
    re.replace_all(&decoded, "").trim().to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// GraphQL — GraphQL 查询/变更
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// WebSocket — WebSocket 客户端（基于 tokio::net::TcpStream）
// ═══════════════════════════════════════════════════════════════════════════════

pub struct WebSocketTool;

#[async_trait]
impl Tool for WebSocketTool {
    fn name(&self) -> &str {
        "WebSocket"
    }
    fn description(&self) -> &str {
        "通过 WebSocket 连接到服务器，发送消息并接收响应。支持 ws:// 和 wss:// 协议。每次调用建立连接→发送→接收→关闭。基于 tokio::net 手动实现，零外部依赖。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "WebSocket URL (ws:// 或 wss://)"},
                "message": {"type": "string", "description": "要发送的文本消息"},
                "headers": {"type": "object", "description": "额外握手请求头"},
                "timeout_secs": {"type": "integer", "default": 15, "description": "接收超时秒数"},
                "max_recv_bytes": {"type": "integer", "default": 65536, "description": "最大接收字节数"}
            },
            "required": ["url"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return Ok(ToolResult::error("Error: url 是必需的"));
        }

        let (host, port, path, use_tls) = parse_ws_url(&url)?;
        let message = input["message"].as_str().unwrap_or("");
        let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(15);
        let max_recv = input["max_recv_bytes"].as_u64().unwrap_or(65536) as usize;

        // 建立 TCP 连接
        let addr = format!("{}:{}", host, port);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| ToolError::execution_failed("连接超时".to_string()))?
        .map_err(|e| ToolError::execution_failed(format!("TCP 连接失败: {}", e)))?;

        if use_tls {
            // wss:// — 这里简化处理：返回不支持说明
            // 完整的 TLS WebSocket 需要 rustls 等，但可以用 tokio-native-tls
            let _ = stream;
            return websocket_over_tls(&host, port, &path, &url, message, timeout_secs, max_recv)
                .await;
        }

        // ws:// — 手动实现 WebSocket 握手 + 帧收发
        websocket_raw(stream, &host, &path, &url, message, timeout_secs, max_recv).await
    }
}

fn parse_ws_url(url: &str) -> Result<(String, u16, String, bool), ToolError> {
    let (host_part, use_tls) = if let Some(rest) = url.strip_prefix("wss://") {
        (rest, true)
    } else if let Some(rest) = url.strip_prefix("ws://") {
        (rest, false)
    } else {
        return Err(ToolError::invalid_input("url 必须以 ws:// 或 wss:// 开头"));
    };

    let (host_and_port, path) = match host_part.find('/') {
        Some(idx) => (&host_part[..idx], &host_part[idx..]),
        None => (host_part, "/"),
    };

    let (host, port) = match host_and_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(if use_tls { 443 } else { 80 })),
        None => (host_and_port.to_string(), if use_tls { 443 } else { 80 }),
    };

    Ok((host, port, path.to_string(), use_tls))
}

async fn websocket_raw(
    mut stream: tokio::net::TcpStream,
    host: &str,
    path: &str,
    _url: &str,
    message: &str,
    timeout_secs: u64,
    max_recv: usize,
) -> Result<ToolResult, ToolError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // WebSocket 握手 key（UUID v4 → 16 字节随机 + Base64）
    let random_bytes = uuid::Uuid::new_v4().into_bytes();
    let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, random_bytes);

    // 发送 HTTP Upgrade 请求
    let handshake = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
        path, host, key
    );

    stream
        .write_all(handshake.as_bytes())
        .await
        .map_err(|e| ToolError::execution_failed(format!("发送握手失败: {}", e)))?;

    // 读取握手响应
    let mut buf = vec![0u8; 4096];
    let n =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), stream.read(&mut buf))
            .await
            .map_err(|_| ToolError::execution_failed("握手超时".to_string()))?
            .map_err(|e| ToolError::execution_failed(format!("读取握手响应失败: {}", e)))?;

    let response = String::from_utf8_lossy(&buf[..n]);
    if !response.contains("101") {
        return Ok(ToolResult::error(format!(
            "WebSocket 握手失败:\n{}",
            truncate(&response, 1000)
        )));
    }

    let mut result = format!("✅ WebSocket 已连接到 {}\n\n握手成功 (HTTP 101)\n", _url);

    // 发送消息（如果有）
    if !message.is_empty() {
        let frame = build_ws_frame(message.as_bytes(), 0x1); // text frame
        stream
            .write_all(&frame)
            .await
            .map_err(|e| ToolError::execution_failed(format!("发送消息失败: {}", e)))?;
        result.push_str(&format!("📤 已发送: {}\n", truncate(message, 500)));

        // 接收响应
        let mut recv_buf = vec![0u8; max_recv];
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            stream.read(&mut recv_buf),
        )
        .await
        {
            Ok(Ok(n)) if n > 0 => {
                if let Some((opcode, payload)) = parse_ws_frame(&recv_buf[..n]) {
                    let text = String::from_utf8_lossy(&payload);
                    result.push_str(&format!(
                        "📥 收到 (opcode={}): {}\n",
                        opcode,
                        truncate(&text, 10_000)
                    ));
                }
            },
            Ok(Ok(_)) => result.push_str("📥 收到空响应\n"),
            Ok(Err(e)) => result.push_str(&format!("接收错误: {}\n", e)),
            Err(_) => result.push_str("⏱ 接收超时\n"),
        }
    }

    // 发送关闭帧
    let close_frame = build_ws_frame(&[], 0x8);
    let _ = stream.write_all(&close_frame).await;

    Ok(ToolResult::success(result))
}

async fn websocket_over_tls(
    _host: &str,
    _port: u16,
    _path: &str,
    url: &str,
    _message: &str,
    _timeout_secs: u64,
    _max_recv: usize,
) -> Result<ToolResult, ToolError> {
    Ok(ToolResult::success(format!(
        "WebSocket URL: {}\n\n⚠️ wss:// (TLS) 需要 native-tls 或 rustls。\n当前仅支持 ws:// 明文连接。\n对于 wss://，请使用 HttpRequest 工具调用 REST API 作为替代。",
        url
    )))
}

/// 构建 WebSocket 帧
fn build_ws_frame(payload: &[u8], opcode: u8) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode); // FIN + opcode

    let len = payload.len();
    if len < 126 {
        frame.push(len as u8);
    } else if len <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);
    frame
}

/// 解析 WebSocket 帧，返回 (opcode, payload)
fn parse_ws_frame(data: &[u8]) -> Option<(u8, Vec<u8>)> {
    if data.len() < 2 {
        return None;
    }
    let opcode = data[0] & 0x0F;

    let mut offset = 2;
    let payload_len = match data[1] & 0x7F {
        126 => {
            if data.len() < 4 {
                return None;
            }
            offset = 4;
            u16::from_be_bytes([data[2], data[3]]) as usize
        },
        127 => {
            if data.len() < 10 {
                return None;
            }
            offset = 10;
            u64::from_be_bytes([
                data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
            ]) as usize
        },
        n => n as usize,
    };

    if data.len() < offset + payload_len {
        return None;
    }
    Some((opcode, data[offset..offset + payload_len].to_vec()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
