#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_large_err)]

pub mod adapter;
pub mod anthropic;
pub mod gemini;
pub mod hermes;
pub mod image_gen;
pub mod ollama;
pub mod openai;
pub mod openai_responses;
pub mod openclaw;
pub mod realtime_client;
pub mod registry;
pub mod screen_vision;
pub mod transport;
pub mod url_utils;

pub use image_gen::{
    DallEProvider, FluxProvider, GeneratedImage, ImageGenModelInfo, ImageGenProvider,
    ImageGenRequest, ImageGenResponse,
};
pub use transport::{
    AnthropicTransport, ChatCompletionsTransport, ResponsesTransport, TransportProvider,
    TransportRequest, TransportResponse, TransportStreamChunk,
};

use axagent_core::error::{AxAgentError, Result};
use axagent_harness::types::*;

/// Provide a human-readable diagnostic hint for a `reqwest::Error`.
/// Inspects the error kind to give specific, actionable guidance.
pub fn diagnose_reqwest_error(e: &reqwest::Error) -> String {
    let base = format!("{e}");
    if e.is_connect() {
        format!(
            "{base}. Possible causes: DNS resolution failure, server unreachable, \
            TLS/SSL handshake error, proxy connection refused, or firewall blocking the connection. \
            Check your network, proxy settings, and API host URL."
        )
    } else if e.is_timeout() {
        format!(
            "{base}. The request timed out. The server may be overloaded or your network may be slow. \
            Try again later or check your network connection."
        )
    } else if e.is_decode() {
        format!(
            "{base}. Failed to decode the response body. This can happen if the connection \
            was interrupted mid-stream, the server sent invalid data, or there was a TLS error. \
            Try again or check your proxy settings."
        )
    } else if e.is_redirect() {
        format!("{base}. Too many HTTP redirects. Check your API host URL and proxy configuration.")
    } else {
        format!("{base}. Check your network connection, proxy settings, and API host URL.")
    }
}

/// Provide a human-readable diagnostic hint for a non-2xx HTTP status code.
/// Returns a formatted error message with actionable guidance.
pub fn diagnose_http_status(
    provider_name: &str,
    status: reqwest::StatusCode,
    body: &str,
) -> String {
    let code = status.as_u16();
    let base = format!("{provider_name} API error {code}: {body}");
    match code {
        401 => format!(
            "{base}. Authentication failed: the API key is invalid or expired. \
            Please check your API key in the provider settings."
        ),
        403 => format!(
            "{base}. Access forbidden: your API key may lack the required permissions, \
            or your account may be suspended. Check your provider account status."
        ),
        429 => format!(
            "{base}. Rate limit exceeded: too many requests in a given time window. \
            Wait a moment and try again. Consider adding multiple API keys for rotation \
            or reducing concurrent requests."
        ),
        400 => format!(
            "{base}. Bad request: the request body may be malformed, the model may not support \
            the requested parameters, or the model ID may be invalid. Check your model and settings."
        ),
        404 => format!(
            "{base}. Not found: the API endpoint or model ID does not exist. \
            Verify the API host URL, API path, and model ID in your provider settings."
        ),
        408 => format!(
            "{base}. Request timeout: the server took too long to respond. \
            Try again later or use a smaller context."
        ),
        413 => format!(
            "{base}. Payload too large: the request body exceeds the provider's limit. \
            Try reducing the conversation length or using a model with a larger context window."
        ),
        500 => format!(
            "{base}. Internal server error: the provider experienced an unexpected failure. \
            This is a server-side issue — try again later."
        ),
        502 => format!(
            "{base}. Bad gateway: the provider's upstream server is unavailable. \
            This is a server-side issue — try again later."
        ),
        503 => format!(
            "{base}. Service unavailable: the provider is temporarily overloaded or in maintenance. \
            Try again later."
        ),
        504 => format!(
            "{base}. Gateway timeout: the provider's upstream server did not respond in time. \
            Try again later."
        ),
        _ => base,
    }
}

/// Extract visible text content from a ChatContent enum.
/// For Text: returns the string directly. For Multipart: joins text parts with spaces.
pub fn extract_text_content(content: &ChatContent) -> String {
    match content {
        ChatContent::Text(text) => text.clone(),
        ChatContent::Multipart(parts) => parts
            .iter()
            .filter_map(|part| part.text.as_ref())
            .cloned()
            .collect::<Vec<String>>()
            .join(" "),
    }
}

/// Extract thinking content from text that contains `<think>...</think>` blocks.
/// Returns (visible_text, reasoning_content).
/// If no <think> blocks are present, reasoning_content is None.
pub fn extract_reasoning_from_text(text: &str) -> (String, Option<String>) {
    const THINK_OPEN: &str = "<think";
    const THINK_CLOSE: &str = "</think>";

    let mut result = String::with_capacity(text.len());
    let mut reasoning_parts: Vec<String> = Vec::new();
    let mut remaining = text;

    loop {
        let Some(start) = remaining.find(THINK_OPEN) else {
            result.push_str(remaining);
            break;
        };
        result.push_str(&remaining[..start]);
        let after_open = &remaining[start..];
        let tag_end = if let Some(close_bracket) = after_open.find('>') {
            if after_open.starts_with("<think")
                && let Some(think_close_pos) = after_open.find(THINK_CLOSE)
            {
                let content_start = close_bracket + 1;
                let reasoning = after_open[content_start..think_close_pos]
                    .trim()
                    .to_string();
                if !reasoning.is_empty() {
                    reasoning_parts.push(reasoning);
                }
                remaining = &after_open[think_close_pos + THINK_CLOSE.len()..];
                continue;
            }
            close_bracket + 1
        } else {
            result.push_str(remaining);
            break;
        };
        let search_from = tag_end;
        if let Some(end) = after_open[search_from..].find(THINK_CLOSE) {
            let reasoning = after_open[search_from..search_from + end]
                .trim()
                .to_string();
            if !reasoning.is_empty() {
                reasoning_parts.push(reasoning);
            }
            remaining = &after_open[search_from + end + THINK_CLOSE.len()..];
        } else {
            result.push_str(&after_open[search_from..]);
            break;
        }
    }

    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n\n"))
    };

    let visible = result.trim().to_string();
    if visible.is_empty() {
        (result, reasoning)
    } else {
        (visible, reasoning)
    }
}

#[doc(hidden)]
pub use axagent_harness::ProviderAdapter;
#[doc(hidden)]
pub use axagent_harness::ProviderRequestContext;
// 完整 ProviderAdapter trait 定义在 axagent-harness 中

/// Default version path for a given provider type.
/// 转发到 `axagent-harness::url_utils`（实际定义在 harness 契约层）。
pub use self::url_utils::{
    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,
};

// URL 解析函数的实际定义在 axagent-harness::url_utils，
// 本 crate 通过 url_utils 模块薄壳 re-export 保留向后兼容。
pub fn parse_base64_data_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("data:")?;
    let (mime_type, data) = rest.split_once(";base64,")?;
    if mime_type.is_empty() || data.is_empty() {
        return None;
    }
    Some((mime_type.to_string(), data.to_string()))
}

/// Build an HTTP client with optional proxy configuration.
/// - "system": use system proxy auto-detection (reqwest default)
/// - "http"/"socks5": use explicit proxy with address/port
/// - None or "none": disable all proxies
pub fn build_http_client(proxy_config: Option<&ProviderProxyConfig>) -> Result<reqwest::Client> {
    // Android TLS: aws-lc-rs 在 ARM 设备上经常不可用，ring 也可能缺失。
    // 使用 native-tls-vendored 静态链接 OpenSSL，确保 Android 上 HTTPS 可靠工作。
    #[cfg(target_os = "android")]
    let mut builder = reqwest::Client::builder().use_native_tls();
    #[cfg(not(target_os = "android"))]
    let mut builder = reqwest::Client::builder().use_rustls_tls();

    if let Some(config) = proxy_config {
        match config.proxy_type.as_deref() {
            Some("system") => {
                // Don't call .no_proxy() — let reqwest auto-detect system proxy
            },
            Some(proxy_type) if proxy_type != "none" => {
                if let (Some(addr), Some(port)) = (&config.proxy_address, &config.proxy_port) {
                    if !addr.is_empty() {
                        let scheme = if proxy_type == "socks5" {
                            "socks5"
                        } else {
                            "http"
                        };
                        let proxy_url = format!("{}://{}:{}", scheme, addr, port);
                        let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| {
                            AxAgentError::Provider(format!("Invalid proxy URL: {}", e))
                        })?;
                        builder = builder.proxy(proxy);
                    } else {
                        builder = builder.no_proxy();
                    }
                } else {
                    builder = builder.no_proxy();
                }
            },
            _ => {
                builder = builder.no_proxy();
            },
        }
    } else {
        builder = builder.no_proxy();
    }

    // Android 网络环境不稳定，连接超时缩短避免长时间挂起
    let connect_timeout = if cfg!(target_os = "android") { 15 } else { 30 };
    builder
        .tcp_nodelay(true)
        .connect_timeout(std::time::Duration::from_secs(connect_timeout))
        .timeout(std::time::Duration::from_secs(300))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("Failed to build HTTP client: {}", e)))
}

pub fn build_default_http_client() -> Result<reqwest::Client> {
    build_http_client(None)
}

/// Default User-Agent: `AxAgent-{os}_{arch}/{version}`
pub fn default_user_agent() -> String {
    format!(
        "AxAgent-{}_{}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION")
    )
}

/// Apply custom headers + default User-Agent to a request builder.
pub fn apply_request_headers(
    builder: reqwest::RequestBuilder,
    ctx: &ProviderRequestContext,
) -> reqwest::RequestBuilder {
    apply_headers_to_request(builder, &ctx.custom_headers)
}

/// Apply custom headers + default User-Agent from a raw headers map.
pub fn apply_headers_to_request(
    mut builder: reqwest::RequestBuilder,
    custom_headers: &Option<std::collections::HashMap<String, String>>,
) -> reqwest::RequestBuilder {
    let mut has_ua = false;
    if let Some(headers) = custom_headers {
        for (key, value) in headers {
            if key.eq_ignore_ascii_case("user-agent") {
                has_ua = true;
            }
            builder = builder.header(key, value);
        }
    }
    if !has_ua {
        builder = builder.header("User-Agent", default_user_agent());
    }
    builder
}

/// Force uncompressed transfer for streaming requests so SSE chunks are not
/// delayed by upstream/content-encoding buffering.
pub fn apply_stream_headers_to_request(
    builder: reqwest::RequestBuilder,
    custom_headers: &Option<std::collections::HashMap<String, String>>,
) -> reqwest::RequestBuilder {
    apply_headers_to_request(builder, custom_headers).header("Accept-Encoding", "identity")
}

/// Redact API key from URL query parameters (e.g., ?key=abc123 becomes ?key=[REDACTED])
pub fn redact_api_key_from_url(url: &str) -> String {
    if let Ok(mut parsed) = reqwest::Url::parse(url) {
        let mut pairs: Vec<_> = parsed.query_pairs().into_owned().collect();
        for (key, value) in pairs.iter_mut() {
            if key.eq_ignore_ascii_case("key")
                || key.eq_ignore_ascii_case("api_key")
                || key.eq_ignore_ascii_case("apikey")
            {
                *value = "[REDACTED]".to_string();
            }
        }
        parsed.query_pairs_mut().clear().extend_pairs(pairs);
        parsed.to_string()
    } else {
        // Fallback simple regex replacement if URL parsing fails
        url.to_string()
    }
}
