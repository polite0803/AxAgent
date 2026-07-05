// SPDX-License-Identifier: AGPL-3.0-only

//! 网络工具集
//!
//! HttpRequest (通用 HTTP), Ping (网络连通), DnsLookup (DNS 查询),
//! JsonApi (结构化 JSON API), RssReader (RSS/Atom 订阅),
//! GraphQL (GraphQL 查询), WebSocket (双向通信)
//!
//! 全部基于已有依赖（reqwest + tokio::net），零新增 crate。

use crate::ToolError;

/// 检查 URL 是否安全（不指向内网地址）
fn is_safe_url(url: &str) -> Result<(), ToolError> {
    let parsed = url::Url::parse(url).map_err(|_| ToolError::invalid_input("无效的 URL"))?;
    let host = parsed.host_str().unwrap_or("");
    let blocked_hosts = [
        "127.0.0.1",
        "0.0.0.0",
        "localhost",
        "::1",
        "169.254.169.254",
    ];
    for blocked in &blocked_hosts {
        if host == *blocked {
            return Err(ToolError::permission_denied(
                "Network",
                &format!("不允许访问内部地址: {}", host),
            ));
        }
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>()
        && (ip.is_loopback() || is_link_local_ip(&ip) || is_private_ip(&ip))
    {
        return Err(ToolError::permission_denied(
            "Network",
            &format!("不允许访问内部地址: {}", host),
        ));
    }
    Ok(())
}

fn is_link_local_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_link_local(),
        std::net::IpAddr::V6(v6) => v6.segments()[0] & 0xFFC0 == 0xFE80,
    }
}

fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private(),
        std::net::IpAddr::V6(_v6) => false,
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

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

pub mod graphql;
pub mod http;
pub mod json_api;
pub mod ping_dns;
pub mod rss;
pub mod websocket;

pub use graphql::GraphQLTool;
pub use http::HttpRequestTool;
pub use json_api::JsonApiTool;
pub use ping_dns::{DnsLookupTool, PingTool};
pub use rss::RssReaderTool;
pub use websocket::WebSocketTool;
