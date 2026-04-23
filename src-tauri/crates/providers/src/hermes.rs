use axagent_core::error::{AxAgentError, Result};
use axagent_core::types::*;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

use crate::{build_http_client, ProviderAdapter, ProviderRequestContext};

use crate::openai::OpenAIAdapter;
use crate::openai_responses::OpenAIResponsesAdapter;
use crate::anthropic::AnthropicAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMode {
    ChatCompletions,
    CodexResponses,
    AnthropicMessages,
}

impl ApiMode {
    fn detect_from_url(url: &str) -> Self {
        let url = url.trim_end_matches('/').to_lowercase();
        if url.contains("/anthropic") || url.contains("/v1/messages") {
            Self::AnthropicMessages
        } else if url.contains("/responses") || url.contains("/v1/responses") {
            Self::CodexResponses
        } else {
            Self::ChatCompletions
        }
    }
}

pub struct HermesAdapter {
    chat_completions: Arc<OpenAIAdapter>,
    codex_responses: Arc<OpenAIResponsesAdapter>,
    anthropic: Arc<AnthropicAdapter>,
}

impl HermesAdapter {
    pub fn new() -> Self {
        Self {
            chat_completions: Arc::new(OpenAIAdapter::new()),
            codex_responses: Arc::new(OpenAIResponsesAdapter::new()),
            anthropic: Arc::new(AnthropicAdapter::new()),
        }
    }

    fn resolve_api_mode(ctx: &ProviderRequestContext) -> ApiMode {
        if let Some(mode) = ctx.api_mode.as_deref() {
            match mode.to_lowercase().as_str() {
                "chat_completions" | "chatcompletions" => return ApiMode::ChatCompletions,
                "codex_responses" | "responses" | "openai_responses" => return ApiMode::CodexResponses,
                "anthropic_messages" | "anthropic" | "messages" => return ApiMode::AnthropicMessages,
                _ => {}
            }
        }

        ctx.api_path
            .as_deref()
            .and_then(|p| {
                if p.contains("anthropic") || p.contains("/messages") {
                    Some(ApiMode::AnthropicMessages)
                } else if p.contains("responses") {
                    Some(ApiMode::CodexResponses)
                } else {
                    Some(ApiMode::ChatCompletions)
                }
            })
            .unwrap_or_else(|| {
                let base = ctx.base_url.as_deref().unwrap_or("");
                ApiMode::detect_from_url(base)
            })
    }

    async fn chat_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        mode: ApiMode,
    ) -> Result<ChatResponse> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.chat(ctx, request).await,
            ApiMode::CodexResponses => self.codex_responses.chat(ctx, request).await,
            ApiMode::AnthropicMessages => self.anthropic.chat(ctx, request).await,
        }
    }

    fn chat_stream_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        mode: ApiMode,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.chat_stream(ctx, request),
            ApiMode::CodexResponses => self.codex_responses.chat_stream(ctx, request),
            ApiMode::AnthropicMessages => self.anthropic.chat_stream(ctx, request),
        }
    }

    async fn list_models_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        mode: ApiMode,
    ) -> Result<Vec<Model>> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.list_models(ctx).await,
            ApiMode::CodexResponses => self.codex_responses.list_models(ctx).await,
            ApiMode::AnthropicMessages => self.anthropic.list_models(ctx).await,
        }
    }

    async fn validate_key_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        mode: ApiMode,
    ) -> Result<bool> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.validate_key(ctx).await,
            ApiMode::CodexResponses => self.codex_responses.validate_key(ctx).await,
            ApiMode::AnthropicMessages => self.anthropic.validate_key(ctx).await,
        }
    }

    async fn embed_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
        mode: ApiMode,
    ) -> Result<EmbedResponse> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.embed(ctx, request).await,
            ApiMode::CodexResponses => self.codex_responses.embed(ctx, request).await,
            ApiMode::AnthropicMessages => {
                Err(AxAgentError::Provider(
                    "Embed endpoint is not supported in anthropic_messages mode".to_string(),
                ))
            }
        }
    }

    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8642".to_string())
    }

    fn get_client(ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        match &ctx.proxy_config {
            Some(c) if c.proxy_type.as_deref() != Some("none") => build_http_client(Some(c)),
            _ => Ok(reqwest::Client::new()),
        }
    }

    async fn hermes_request(
        ctx: &ProviderRequestContext,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<String> {
        let url = format!("{}{}", Self::base_url(ctx), path);
        let client = Self::get_client(ctx)?;

        let mut req = client.request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), &url)
            .header("Authorization", format!("Bearer {}", ctx.api_key))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            req = req.body(body.to_string());
        }

        let resp = req.send().await
            .map_err(|e| AxAgentError::Provider(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!(
                "Hermes API error {status}: {text}"
            )));
        }

        resp.text().await.map_err(|e| AxAgentError::Provider(format!("Read error: {e}")))
    }
}

impl Default for HermesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderAdapter for HermesAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let mode = Self::resolve_api_mode(ctx);
        self.chat_with_mode(ctx, request, mode).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        let mode = Self::resolve_api_mode(ctx);
        self.chat_stream_with_mode(ctx, request, mode)
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        let mode = Self::resolve_api_mode(ctx);
        self.list_models_with_mode(ctx, mode).await
    }

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        let mode = Self::resolve_api_mode(ctx);
        self.validate_key_with_mode(ctx, mode).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        let mode = Self::resolve_api_mode(ctx);
        self.embed_with_mode(ctx, request, mode).await
    }

    async fn get_response(&self, ctx: &ProviderRequestContext, response_id: &str) -> Result<String> {
        self.codex_responses.get_response(ctx, response_id).await
    }

    async fn delete_response(&self, ctx: &ProviderRequestContext, response_id: &str) -> Result<()> {
        self.codex_responses.delete_response(ctx, response_id).await
    }

    async fn list_jobs(&self, ctx: &ProviderRequestContext) -> Result<String> {
        Self::hermes_request(ctx, "GET", "/api/jobs", None).await
    }

    async fn create_job(&self, ctx: &ProviderRequestContext, job_data: &str) -> Result<String> {
        Self::hermes_request(ctx, "POST", "/api/jobs", Some(job_data)).await
    }

    async fn get_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "GET", &format!("/api/jobs/{}", job_id), None).await
    }

    async fn update_job(&self, ctx: &ProviderRequestContext, job_id: &str, job_data: &str) -> Result<String> {
        Self::hermes_request(ctx, "PATCH", &format!("/api/jobs/{}", job_id), Some(job_data)).await
    }

    async fn delete_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "DELETE", &format!("/api/jobs/{}", job_id), None).await?;
        Ok(())
    }

    async fn pause_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/pause", job_id), None).await?;
        Ok(())
    }

    async fn resume_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/resume", job_id), None).await?;
        Ok(())
    }

    async fn trigger_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/run", job_id), None).await?;
        Ok(())
    }

    async fn list_runs(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "GET", &format!("/api/jobs/{}/runs", job_id), None).await
    }

    async fn get_run(&self, ctx: &ProviderRequestContext, job_id: &str, run_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "GET", &format!("/api/jobs/{}/runs/{}", job_id, run_id), None).await
    }

    async fn cancel_run(&self, ctx: &ProviderRequestContext, job_id: &str, run_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/runs/{}/cancel", job_id, run_id), None).await?;
        Ok(())
    }

    async fn get_run_logs(&self, ctx: &ProviderRequestContext, job_id: &str, run_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "GET", &format!("/api/jobs/{}/runs/{}/logs", job_id, run_id), None).await
    }

    async fn trigger_run(&self, ctx: &ProviderRequestContext, job_id: &str, params: Option<&str>) -> Result<String> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/runs", job_id), params).await
    }

    async fn retry_run(&self, ctx: &ProviderRequestContext, job_id: &str, run_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/runs/{}/retry", job_id, run_id), None).await
    }

    async fn get_job_schedule(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<String> {
        Self::hermes_request(ctx, "GET", &format!("/api/jobs/{}/schedule", job_id), None).await
    }

    async fn update_job_schedule(&self, ctx: &ProviderRequestContext, job_id: &str, schedule: &str) -> Result<String> {
        Self::hermes_request(ctx, "PUT", &format!("/api/jobs/{}/schedule", job_id), Some(schedule)).await
    }

    async fn enable_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/enable", job_id), None).await?;
        Ok(())
    }

    async fn disable_job(&self, ctx: &ProviderRequestContext, job_id: &str) -> Result<()> {
        Self::hermes_request(ctx, "POST", &format!("/api/jobs/{}/disable", job_id), None).await?;
        Ok(())
    }
}