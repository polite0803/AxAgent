// SPDX-License-Identifier: AGPL-3.0-only

//! LLM Provider 适配器契约 — 从 axagent-providers 提取的接口层
//!
//! 所有 LLM 提供商适配器必须实现 `ProviderAdapter` trait。
//! `ProviderRequestContext` 是每次调用携带的上下文。

use crate::core_error::Result;
use crate::speech::{AudioChunkStream, SpeakRequest, SpeechCapabilities, SpeechInput};
use crate::types::*;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// ProviderProxyConfig 在 harness::types 中定义
pub use crate::types::ProviderProxyConfig;

/// LLM 提供商统一接口
///
/// 每个 LLM 提供商（OpenAI、Anthropic、Gemini 等）都必须实现此 trait。
/// `ctx` 提供 API key、base URL、代理等调用上下文信息。
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
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

    // ── Realtime Voice API ──

    /// 返回连接 LLM Realtime API 所需的 WebSocket URL 和认证信息。
    /// 默认实现返回不支持错误，仅支持 RealtimeVoice 的 provider 需要覆写。
    async fn realtime_config(
        &self,
        _ctx: &ProviderRequestContext,
        _model: &str,
    ) -> Result<RealtimeProviderConfig> {
        Err(crate::core_error::AxAgentError::Provider(
            "realtime voice is not supported by this provider".to_string(),
        ))
    }

    // ── Speech: STT / TTS ──

    /// 声明该 provider 是否支持语音识别(STT)与语音合成(TTS)。
    /// 默认不支持；支持语音的 provider（如 OpenAI）覆写此方法。
    fn supports_speech(&self) -> SpeechCapabilities {
        SpeechCapabilities::none()
    }

    /// 语音识别（Speech-to-Text）。
    ///
    /// 输入一段原始音频（`SpeechInput`），返回识别出的文本。
    /// 默认返回「不支持」错误；支持 STT 的 provider 覆写此方法。
    async fn transcribe(
        &self,
        _ctx: &ProviderRequestContext,
        _input: SpeechInput,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "transcribe (STT) is not supported by this provider".to_string(),
        ))
    }

    /// 语音合成（Text-to-Speech），返回流式音频字节。
    ///
    /// 每个 chunk 为特定编码的原始音频（通常为 PCM16 小端），调用方按
    /// `SpeakRequest.format.encoding` 解码后边收边播。
    /// 默认返回一个立即以「不支持」错误结束的流。
    async fn speech(
        &self,
        _ctx: &ProviderRequestContext,
        _req: SpeakRequest,
    ) -> Result<AudioChunkStream> {
        Ok(crate::speech::unsupported_speech_stream())
    }

    // ── Response API (OpenAI Responses API 特有) ──

    async fn get_response(
        &self,
        _ctx: &ProviderRequestContext,
        _response_id: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "get_response is not supported by this provider".to_string(),
        ))
    }

    async fn delete_response(
        &self,
        _ctx: &ProviderRequestContext,
        _response_id: &str,
    ) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "delete_response is not supported by this provider".to_string(),
        ))
    }

    // ── Job 管理 (Batch API 特有) ──

    async fn list_jobs(&self, _ctx: &ProviderRequestContext) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "list_jobs is not supported by this provider".to_string(),
        ))
    }

    async fn create_job(&self, _ctx: &ProviderRequestContext, _job_data: &str) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "create_job is not supported by this provider".to_string(),
        ))
    }

    async fn get_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "get_job is not supported by this provider".to_string(),
        ))
    }

    async fn update_job(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _job_data: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "update_job is not supported by this provider".to_string(),
        ))
    }

    async fn delete_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "delete_job is not supported by this provider".to_string(),
        ))
    }

    async fn pause_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "pause_job is not supported by this provider".to_string(),
        ))
    }

    async fn resume_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "resume_job is not supported by this provider".to_string(),
        ))
    }

    async fn enable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "enable_job is not supported by this provider".to_string(),
        ))
    }

    async fn disable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "disable_job is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "trigger_job is not supported by this provider".to_string(),
        ))
    }

    async fn list_runs(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "list_runs is not supported by this provider".to_string(),
        ))
    }

    async fn get_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "get_run is not supported by this provider".to_string(),
        ))
    }

    async fn cancel_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<()> {
        Err(crate::core_error::AxAgentError::Provider(
            "cancel_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_run_logs(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "get_run_logs is not supported by this provider".to_string(),
        ))
    }

    async fn trigger_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _params: Option<&str>,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "trigger_run is not supported by this provider".to_string(),
        ))
    }

    async fn retry_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "retry_run is not supported by this provider".to_string(),
        ))
    }

    async fn get_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
            "get_job_schedule is not supported by this provider".to_string(),
        ))
    }

    async fn update_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _schedule: &str,
    ) -> Result<String> {
        Err(crate::core_error::AxAgentError::Provider(
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

// ── Realtime Voice ────────────────────────────────────────────

/// Provider 返回的 Realtime WebSocket 连接配置
#[derive(Debug, Clone)]
pub struct RealtimeProviderConfig {
    /// LLM 提供商的 Realtime WebSocket URL（如 wss://api.openai.com/v1/realtime）
    pub ws_url: String,
    /// API Key（用于 WebSocket 认证）
    pub api_key: String,
    /// 额外的自定义 HTTP Headers（如 OpenAI 的 Authorization 已自动处理）
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// 提供商类型（openai / gemini 等），用于 RealtimeClient 选择协议适配
    pub provider_type: String,
}

// ── URL 解析工具函数 ──────────────────────────────────────────
//
// `resolve_base_url`, `resolve_base_url_for_type`, `resolve_chat_url` 等函数
// 已迁移至 `axagent-providers::url_utils`。
// 使用方请导入：
//   use axagent_providers::url_utils::{resolve_base_url_for_type, resolve_chat_url};
//
