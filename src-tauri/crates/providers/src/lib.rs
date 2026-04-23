pub mod adapter;
pub mod anthropic;
pub mod gemini;
pub mod hermes;
pub mod ollama;
pub mod openai;
pub mod openai_responses;
pub mod openclaw;
pub mod registry;

use axagent_core::error::{AxAgentError, Result};
use axagent_core::types::*;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

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
        format!(
            "{base}. Too many HTTP redirects. Check your API host URL and proxy configuration."
        )
    } else {
        format!(
            "{base}. Check your network connection, proxy settings, and API host URL."
        )
    }
}

/// Provide a human-readable diagnostic hint for a non-2xx HTTP status code.
/// Returns a formatted error message with actionable guidance.
pub fn diagnose_http_status(provider_name: &str, status: reqwest::StatusCode, body: &str) -> String {
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

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Result<ChatResponse>;

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>;

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>>;

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse>;

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        self.list_models(ctx).await.map(|_| true)
    }

    async fn get_response(&self, _ctx: &ProviderRequestContext, _response_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "get_response is not supported by this provider".to_string(),
        ))
    }

    async fn delete_response(&self, _ctx: &ProviderRequestContext, _response_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "delete_response is not supported by this provider".to_string(),
        ))
    }

    async fn list_jobs(&self, _ctx: &ProviderRequestContext) -> Result<String> {
        Err(AxAgentError::Provider(
            "list_jobs is not supported by this provider".to_string(),
        ))
    }

    async fn create_job(&self, _ctx: &ProviderRequestContext, _job_data: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "create_job is not supported by this provider".to_string(),
        ))
    }

    async fn get_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "get_job is not supported by this provider".to_string(),
        ))
    }

    async fn update_job(&self, _ctx: &ProviderRequestContext, _job_id: &str, _job_data: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "update_job is not supported by this provider".to_string(),
        ))
    }

    async fn delete_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "delete_job is not supported by this provider".to_string(),
        ))
    }

    async fn pause_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "pause_job is not supported by this provider".to_string(),
        ))
    }

    async fn resume_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "resume_job is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "trigger_job is not supported by this provider".to_string(),
        ))
    }

    async fn list_runs(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "list_runs is not supported by this provider".to_string(),
        ))
    }

    async fn get_run(&self, _ctx: &ProviderRequestContext, _job_id: &str, _run_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "get_run is not supported by this provider".to_string(),
        ))
    }

    async fn cancel_run(&self, _ctx: &ProviderRequestContext, _job_id: &str, _run_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "cancel_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_run_logs(&self, _ctx: &ProviderRequestContext, _job_id: &str, _run_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "get_run_logs is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_run(&self, _ctx: &ProviderRequestContext, _job_id: &str, _params: Option<&str>) -> Result<String> {
        Err(AxAgentError::Provider(
            "trigger_run is not supported by this provider".to_string(),
        ))
    }

    async fn retry_run(&self, _ctx: &ProviderRequestContext, _job_id: &str, _run_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "retry_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_job_schedule(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "get_job_schedule is not supported by this provider".to_string(),
        ))
    }

    async fn update_job_schedule(&self, _ctx: &ProviderRequestContext, _job_id: &str, _schedule: &str) -> Result<String> {
        Err(AxAgentError::Provider(
            "update_job_schedule is not supported by this provider".to_string(),
        ))
    }

    async fn enable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "enable_job is not supported by this provider".to_string(),
        ))
    }

    async fn disable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider(
            "disable_job is not supported by this provider".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct ProviderRequestContext {
    pub api_key: String,
    pub key_id: String,
    pub provider_id: String,
    pub base_url: Option<String>,
    pub api_path: Option<String>,
    pub proxy_config: Option<ProviderProxyConfig>,
    pub custom_headers: Option<std::collections::HashMap<String, String>>,
    pub api_mode: Option<String>,
    pub conversation: Option<String>,
    pub previous_response_id: Option<String>,
    pub store_response: Option<bool>,
}

/// Default version path for a given provider type.
pub fn default_version_for_type(provider_type: &ProviderType) -> &'static str {
    match provider_type {
        ProviderType::Gemini => "/v1beta",
        ProviderType::Ollama => "/v1",
        _ => "/v1",
    }
}

/// Resolve `api_host` into a usable base URL, using the provider type to
/// determine the default version path (e.g. `/v1` for OpenAI, `/v1beta` for Gemini).
///
/// - Trailing `!` → force mode: strip `!`, return as-is.
/// - Already ends with a versioned path (e.g. `/v1`, `/v1beta`) → return as-is.
/// - Otherwise → append the default version path for this provider type.
pub fn resolve_base_url_for_type(api_host: &str, provider_type: &ProviderType) -> String {
    let default_version = default_version_for_type(provider_type);
    resolve_base_url_inner(api_host, default_version)
}

/// Resolve `api_host` into a usable base URL (defaults to `/v1`).
pub fn resolve_base_url(api_host: &str) -> String {
    resolve_base_url_inner(api_host, "/v1")
}

fn resolve_base_url_inner(api_host: &str, default_version: &str) -> String {
    let trimmed = api_host.trim_end_matches('/');
    if let Some(forced) = trimmed.strip_suffix('!') {
        forced.trim_end_matches('/').to_string()
    } else if has_version_suffix(trimmed) {
        trimmed.to_string()
    } else {
        format!("{}{}", trimmed, default_version)
    }
}

/// Check whether the URL already ends with a versioned path segment
/// like `/v1`, `/v1beta`, `/v2`, `/v1beta1`, etc.
fn has_version_suffix(url: &str) -> bool {
    let last_seg = url.rsplit('/').next().unwrap_or("");
    // Match patterns like v1, v2, v1beta, v1beta1, v1alpha, etc.
    let bytes = last_seg.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'v' {
        return false;
    }
    // After 'v', must start with digit(s), optionally followed by alpha tag
    let rest = &last_seg[1..];
    rest.starts_with(|c: char| c.is_ascii_digit())
}

/// Build the full chat/completion URL from resolved `base_url` and optional `api_path`.
///
/// When `api_path` is provided:
/// - Trailing `!` on api_path → force: concat resolved base + raw path (strip `!`).
/// - No `!` → auto-dedup: if both resolved base and api_path share a common
///   versioned prefix (e.g. `/v1`, `/v1beta`), strip the duplicate from api_path.
///
/// When `api_path` is absent, returns `resolved_base_url + default_suffix`
/// (e.g. `/chat/completions`).
pub fn resolve_chat_url(
    resolved_base: &str,
    api_path: Option<&str>,
    default_suffix: &str,
) -> String {
    let base = resolved_base.trim_end_matches('/');
    match api_path {
        Some(path) if !path.is_empty() => {
            if let Some(forced) = path.strip_suffix('!') {
                // Force mode: concat as-is
                let p = if forced.starts_with('/') {
                    forced.to_string()
                } else {
                    format!("/{}", forced)
                };
                format!("{}{}", base, p)
            } else {
                let p = if path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/{}", path)
                };
                // Auto dedup: if base ends with a version prefix that matches
                // the start of api_path, strip it from api_path
                if let Some(ver) = extract_version_prefix(base) {
                    if p.starts_with(&ver) {
                        return format!("{}{}", base, &p[ver.len()..]);
                    }
                }
                format!("{}{}", base, p)
            }
        }
        _ => format!("{}{}", base, default_suffix),
    }
}

/// Extract the trailing version prefix from a URL (e.g. "/v1", "/v1beta").
fn extract_version_prefix(url: &str) -> Option<String> {
    let last_seg = url.rsplit('/').next()?;
    let bytes = last_seg.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'v' {
        return None;
    }
    let rest = &last_seg[1..];
    if rest.starts_with(|c: char| c.is_ascii_digit()) {
        Some(format!("/{}", last_seg))
    } else {
        None
    }
}

pub(crate) fn parse_base64_data_url(url: &str) -> Option<(String, String)> {
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
    let mut builder = reqwest::Client::builder().use_rustls_tls();

    if let Some(config) = proxy_config {
        match config.proxy_type.as_deref() {
            Some("system") => {
                // Don't call .no_proxy() — let reqwest auto-detect system proxy
            }
            Some(proxy_type) if proxy_type != "none" => {
                if let (Some(addr), Some(port)) = (&config.proxy_address, &config.proxy_port) {
                    if !addr.is_empty() {
                        let scheme = if proxy_type == "socks5" {
                            "socks5"
                        } else {
                            "http"
                        };
                        let proxy_url = format!("{}://{}:{}", scheme, addr, port);
                        let proxy = reqwest::Proxy::all(&proxy_url)
                            .map_err(|e| AxAgentError::Provider(format!("Invalid proxy URL: {}", e)))?;
                        builder = builder.proxy(proxy);
                    } else {
                        builder = builder.no_proxy();
                    }
                } else {
                    builder = builder.no_proxy();
                }
            }
            _ => {
                builder = builder.no_proxy();
            }
        }
    } else {
        builder = builder.no_proxy();
    }

    builder
        .tcp_nodelay(true)
        .connect_timeout(std::time::Duration::from_secs(30))
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
    if let Some(ref headers) = custom_headers {
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
