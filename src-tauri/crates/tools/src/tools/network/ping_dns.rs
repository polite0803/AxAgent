// SPDX-License-Identifier: AGPL-3.0-only

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_kit::utils::hide_window;
use serde_json::Value;

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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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
        if host.contains(';')
            || host.contains('|')
            || host.contains('&')
            || host.contains('$')
            || host.contains('`')
            || host.contains('\n')
            || host.contains('\r')
            || host.contains(' ')
        {
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

        let mut cmd = tokio::process::Command::new("ping");
        cmd.arg(count_flag).arg(count.to_string()).arg(timeout_flag).arg(&timeout_val).arg(&host);
        hide_window(cmd.as_std_mut());
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(timeout + 5), cmd.output())
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
        if (lower.contains("loss") || lower.contains("丢失") || lower.contains("lost"))
            && let Some(pct) = lower.split('%').next()
        {
            let num: Vec<&str> = pct.split_whitespace().collect();
            if let Some(last) = num.last() {
                return last.parse::<f64>().ok();
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

// DnsLookup — DNS 查询

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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let hostname = input["hostname"].as_str().unwrap_or("").to_string();
        if hostname.is_empty() {
            return Ok(ToolResult::error("Error: hostname 是必需的"));
        }
        let dangerous_chars = [';', '|', '&', '$', '`', '\n', '\r', ' ', '>', '<'];
        if hostname.chars().any(|c| dangerous_chars.contains(&c)) {
            return Ok(ToolResult::error("Error: hostname 包含非法字符"));
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
                    let mut cmd = tokio::process::Command::new("nslookup");
                    cmd.args(["-type", qtype, &hostname]);
                    hide_window(cmd.as_std_mut());
                    cmd.output().await
                } else {
                    let mut cmd = tokio::process::Command::new("dig");
                    cmd.args([&hostname, qtype, "+short"]);
                    hide_window(cmd.as_std_mut());
                    cmd.output().await
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

// JsonApi — 结构化 JSON API 调用
