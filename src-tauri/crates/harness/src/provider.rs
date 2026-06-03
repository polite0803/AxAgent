//! LLM Provider 适配器契约 — 从 axagent-providers 提取的接口层
//!
//! 所有 LLM 提供商适配器必须实现 `ProviderAdapter` trait。
//! `ProviderRequestContext` 是每次调用携带的上下文。

use async_trait::async_trait;
use axagent_core::error::Result;
use axagent_core::types::*;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// ProviderProxyConfig 在 axagent-core::types 中定义
pub use axagent_core::types::ProviderProxyConfig;

/// LLM 提供商统一接口
///
/// 每个 LLM 提供商（OpenAI、Anthropic、Gemini 等）都必须实现此 trait。
/// `ctx` 提供 API key、base URL、代理等调用上下文信息。
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
        cancel_token: Option<Arc<AtomicBool>>,
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

    // ── Response API (OpenAI Responses API 特有) ──

    async fn get_response(
        &self,
        _ctx: &ProviderRequestContext,
        _response_id: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "get_response is not supported by this provider".to_string(),
        ))
    }

    async fn delete_response(
        &self,
        _ctx: &ProviderRequestContext,
        _response_id: &str,
    ) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "delete_response is not supported by this provider".to_string(),
        ))
    }

    // ── Job 管理 (Batch API 特有) ──

    async fn list_jobs(&self, _ctx: &ProviderRequestContext) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "list_jobs is not supported by this provider".to_string(),
        ))
    }

    async fn create_job(&self, _ctx: &ProviderRequestContext, _job_data: &str) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "create_job is not supported by this provider".to_string(),
        ))
    }

    async fn get_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "get_job is not supported by this provider".to_string(),
        ))
    }

    async fn update_job(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _job_data: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "update_job is not supported by this provider".to_string(),
        ))
    }

    async fn delete_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "delete_job is not supported by this provider".to_string(),
        ))
    }

    async fn pause_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "pause_job is not supported by this provider".to_string(),
        ))
    }

    async fn resume_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "resume_job is not supported by this provider".to_string(),
        ))
    }

    async fn enable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "enable_job is not supported by this provider".to_string(),
        ))
    }

    async fn disable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "disable_job is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "trigger_job is not supported by this provider".to_string(),
        ))
    }

    async fn list_runs(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "list_runs is not supported by this provider".to_string(),
        ))
    }

    async fn get_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "get_run is not supported by this provider".to_string(),
        ))
    }

    async fn cancel_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<()> {
        Err(axagent_core::error::AxAgentError::Provider(
            "cancel_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_run_logs(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "get_run_logs is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _params: Option<&str>,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "trigger_run is not supported by this provider".to_string(),
        ))
    }

    async fn retry_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "retry_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "get_job_schedule is not supported by this provider".to_string(),
        ))
    }

    async fn update_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _schedule: &str,
    ) -> Result<String> {
        Err(axagent_core::error::AxAgentError::Provider(
            "update_job_schedule is not supported by this provider".to_string(),
        ))
    }
}

/// 每次 LLM 调用携带的上下文信息
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

// ── URL 解析工具函数 ──────────────────────────────────────────

/// Default version path for a given provider type.
pub fn default_version_for_type(provider_type: &axagent_core::types::ProviderType) -> &'static str {
    match provider_type {
        axagent_core::types::ProviderType::Gemini => "/v1beta",
        axagent_core::types::ProviderType::Ollama => "/v1",
        _ => "/v1",
    }
}

/// Resolve `api_host` into a usable base URL, using the provider type to
/// determine the default version path (e.g. `/v1` for OpenAI, `/v1beta` for Gemini).
///
/// - Trailing `!` → force mode: strip `!`, return as-is.
/// - Already ends with a versioned path (e.g. `/v1`, `/v1beta`) → return as-is.
/// - Otherwise → append the default version path for this provider type.
pub fn resolve_base_url_for_type(
    api_host: &str,
    provider_type: &axagent_core::types::ProviderType,
) -> String {
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

fn has_version_suffix(url: &str) -> bool {
    let last_seg = url.rsplit('/').next().unwrap_or("");
    let bytes = last_seg.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'v' {
        return false;
    }
    let rest = &last_seg[1..];
    rest.starts_with(|c: char| c.is_ascii_digit())
}

/// Build the full chat/completion URL from resolved `base_url` and optional `api_path`.
pub fn resolve_chat_url(
    resolved_base: &str,
    api_path: Option<&str>,
    default_suffix: &str,
) -> String {
    let base = resolved_base.trim_end_matches('/');
    match api_path {
        Some(path) if !path.is_empty() => {
            if let Some(forced) = path.strip_suffix('!') {
                format!("{}/{}", base, forced.trim_matches('/'))
            } else {
                let path = path.trim_matches('/');
                if let Some(common) = find_common_version_prefix(base, path) {
                    format!("{}/{}", base, &path[common.len()..])
                } else {
                    format!("{}/{}", base, path)
                }
            }
        },
        _ => format!("{}{}", base, default_suffix),
    }
}

fn find_common_version_prefix<'a>(base: &str, path: &'a str) -> Option<&'a str> {
    let base_last = base.rsplit('/').next().unwrap_or("");
    if path.starts_with(base_last) && !base_last.is_empty() && base_last.starts_with('v') {
        Some(&path[..base_last.len()])
    } else {
        None
    }
}
