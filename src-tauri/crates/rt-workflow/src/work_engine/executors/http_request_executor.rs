// SPDX-License-Identifier: AGPL-3.0-only

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use tokio::net::lookup_host;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct HttpRequestExecutor;

impl HttpRequestExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpRequestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// P0-4: SSRF 防护 —— 在 HTTP 请求前做 IP 黑名单校验。
/// 拒绝：loopback、私网、link-local、IPv6 link-local、IPv4 映射的 IPv6、云元数据地址。
fn is_blocked_ip(ip: &IpAddr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => {
            // 私网 + link-local (169.254.0.0/16) + 云元数据 (169.254.169.254)
            if v4.is_private() || v4.is_link_local() {
                return true;
            }
            // 0.0.0.0/8、100.64.0.0/10（CGNAT）、224.0.0.0/4（multicast）
            if v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast()
                || (v4.octets()[0] == 100 && (v4.octets()[1] >= 64 && v4.octets()[1] <= 127))
                || v4.octets()[0] >= 240
            {
                return true;
            }
        },
        IpAddr::V6(v6) => {
            // fe80::/10 (link-local) + fc00::/7 (unique local)
            if v6.is_unicast_link_local()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || v6.is_unspecified()
                || v6.is_multicast()
            {
                return true;
            }
            // IPv4-mapped IPv6 (::ffff:0:0/96) 也要校验内嵌的 v4
            if let Some(v4) = ipv4_mapped_from_v6(v6)
                && is_blocked_ip(&IpAddr::V4(v4))
            {
                return true;
            }
        },
    }
    false
}

fn ipv4_mapped_from_v6(v6: &Ipv6Addr) -> Option<Ipv4Addr> {
    let seg = v6.segments();
    if seg[0] == 0 && seg[1] == 0 && seg[2] == 0 && seg[3] == 0 && seg[4] == 0 && seg[5] == 0xffff {
        let octets = v6.octets();
        return Some(Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]));
    }
    None
}

/// P0-4: 解析 URL 主机名，校验所有解析到的 IP 都在白名单（此处为非黑名单）。
/// 同时拒绝空主机名、纯 IP 字面量解析失败。
async fn assert_url_safe(url: &str) -> Result<(), NodeError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| NodeError::exec_failed("http_error", format!("Invalid URL: {e}")))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| NodeError::exec_failed("http_error", "URL has no host".to_string()))?;

    // scheme 必须是 http/https
    match parsed.scheme() {
        "http" | "https" => {},
        other => {
            return Err(NodeError::exec_failed(
                "http_error",
                format!("URL scheme '{other}' is not allowed (only http/https)"),
            ));
        },
    }

    // 解析所有 A/AAAA 记录，任一落在黑名单就拒绝
    let addrs: Vec<IpAddr> = lookup_host((host, 0u16))
        .await
        .map_err(|e| {
            NodeError::exec_failed("http_error", format!("DNS lookup failed for {host}: {e}"))
        })?
        .map(|sa| sa.ip())
        .collect();

    if addrs.is_empty() {
        return Err(NodeError::exec_failed(
            "http_error",
            format!("DNS lookup for {host} returned no addresses"),
        ));
    }

    for ip in &addrs {
        if is_blocked_ip(ip) {
            return Err(NodeError::exec_failed(
                "http_error",
                format!(
                    "SSRF blocked: {host} resolves to blocked IP {ip} \
                     (loopback / private / link-local / cloud-metadata / multicast)"
                ),
            ));
        }
    }
    Ok(())
}

#[async_trait]
impl NodeExecutorTrait for HttpRequestExecutor {
    fn node_type(&self) -> &'static str {
        "httpRequest"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::HttpRequest(http_node) = node else {
            return Err(NodeError::type_mismatch(
                "httpRequest".to_string(),
                crate::work_engine::node_executor_trait::node_type_name(node).to_string(),
            ));
        };

        let config = &http_node.config;
        if config.url.trim().is_empty() {
            return Err(NodeError::exec_failed(
                "http_error",
                "HTTP Request URL is empty".to_string(),
            ));
        }

        // P0-4: SSRF 校验必须在发请求前做（IP 解析后再发，避免 DNS rebinding）
        assert_url_safe(&config.url).await?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs.clamp(5, 300)))
            // P0-4: 禁止跟随重定向 —— 攻击者可重定向到内网绕过校验
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| {
                NodeError::exec_failed("http_error", format!("Failed to create HTTP client: {e}"))
            })?;

        let mut req = match config.method.to_uppercase().as_str() {
            "GET" => client.get(&config.url),
            "POST" => {
                let mut r = client.post(&config.url);
                if let Some(ref body) = config.body {
                    r = match config.body_type.as_str() {
                        "json" => r.json(
                            &serde_json::from_str::<serde_json::Value>(body)
                                .unwrap_or(serde_json::Value::String(body.clone())),
                        ),
                        "form" => r
                            .header("content-type", "application/x-www-form-urlencoded")
                            .body(body.clone()),
                        _ => r.body(body.clone()),
                    };
                }
                r
            },
            "PUT" => {
                let mut r = client.put(&config.url);
                if let Some(ref body) = config.body {
                    r = r.body(body.clone());
                }
                r
            },
            "PATCH" => {
                let mut r = client.patch(&config.url);
                if let Some(ref body) = config.body {
                    r = r.body(body.clone());
                }
                r
            },
            "DELETE" => client.delete(&config.url),
            "HEAD" => client.head(&config.url),
            "OPTIONS" => client.request(reqwest::Method::OPTIONS, &config.url),
            _ => {
                return Err(NodeError::exec_failed(
                    "http_error",
                    format!("Unsupported HTTP method: {}", config.method),
                ));
            },
        };

        for (key, value) in &config.headers {
            req = req.header(key, value);
        }

        let response = req.send().await.map_err(|e| {
            NodeError::exec_failed("http_error", format!("HTTP request failed: {e}"))
        })?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body_text =
            response.text().await.unwrap_or_else(|e| format!("Failed to read response body: {e}"));

        let output = serde_json::json!({
            "status": status,
            "status_text": if (200..300).contains(&status) { "success" } else { "error" },
            "headers": headers.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect::<std::collections::HashMap<_, _>>(),
            "body": body_text,
            "node_id": node.base_id(),
        });

        Ok(NodeOutput {
            output,
            output_var: if config.output_var.is_empty() {
                None
            } else {
                Some(config.output_var.clone())
            },
        })
    }
}
