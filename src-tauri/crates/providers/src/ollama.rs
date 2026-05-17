//! Ollama local provider adapter.
//!
//! Connects to a locally running [Ollama](https://ollama.com) instance.
//! Ollama exposes an OpenAI-compatible `/v1/chat/completions` endpoint, so
//! chat and streaming are delegated to [`OpenAIAdapter`].
//!
//! This adapter overrides:
//!
//! - **`list_models`** — uses Ollama's native `/api/tags` endpoint which
//!   returns model names in Ollama's own format (e.g. `llama3:latest`).
//! - **`validate_key`** — Ollama does not require an API key, so this
//!   probes `/api/tags` to check if the server is reachable.

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_core::error::{AxAgentError, Result};
use axagent_core::types::*;
use futures::Stream;
use serde::Deserialize;
use std::pin::Pin;

/// Default base URL for a local Ollama instance.
const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";

/// Default API path for Ollama's OpenAI-compatible chat endpoint.
const DEFAULT_OLLAMA_PATH: &str = "/v1/chat/completions";

/// Provider adapter for Ollama local inference.
///
/// Chat and streaming delegate to the inner OpenAI adapter because Ollama
/// speaks the OpenAI-compatible API protocol on the `/v1/` prefix.
/// Model listing and key validation use Ollama's native `/api/` endpoints.
pub struct OllamaAdapter {
    inner: OpenAIAdapter,
}

impl Default for OllamaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaAdapter {
    pub fn new() -> Self {
        Self {
            inner: OpenAIAdapter::new(),
        }
    }

    /// Resolve the effective base URL for an Ollama instance.
    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OLLAMA_HOST.to_string())
    }

    /// Resolve the effective chat URL for Ollama.
    fn effective_chat_url(ctx: &ProviderRequestContext) -> String {
        let base = Self::base_url(ctx);
        let path = ctx.api_path.as_deref().unwrap_or(DEFAULT_OLLAMA_PATH);
        crate::resolve_chat_url(&base, Some(path), DEFAULT_OLLAMA_PATH)
    }

    /// Build an HTTP client, respecting proxy configuration.
    #[allow(clippy::result_large_err)]
    fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        self.inner.get_client(ctx)
    }
}

// --- Ollama native API response types ---

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    details: Option<OllamaModelDetails>,
}

#[derive(Deserialize)]
struct OllamaModelDetails {
    family: Option<String>,
    parameter_size: Option<String>,
}

#[async_trait]
impl ProviderAdapter for OllamaAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        self.inner.chat(ctx, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// List models using Ollama's native `/api/tags` endpoint.
    ///
    /// Falls back to the OpenAI-compatible `/v1/models` endpoint if the
    /// native endpoint is unavailable (e.g. older Ollama versions).
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        let base = Self::base_url(ctx);
        let url = format!("{}/api/tags", base.trim_end_matches('/'));

        let client = self.get_client(ctx)?;
        let resp = crate::apply_request_headers(client.get(&url), ctx)
            .send()
            .await
            .map_err(|e| AxAgentError::Provider(super::diagnose_reqwest_error(&e)))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            // Fall back to OpenAI-compatible /v1/models
            if s.as_u16() == 404 {
                return self.inner.list_models(ctx).await;
            }
            return Err(AxAgentError::Provider(super::diagnose_http_status("Ollama", s, &t)));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AxAgentError::Provider(format!("Read error: {e}")))?;

        let tags: OllamaTagsResponse = serde_json::from_str(&body).map_err(|e| {
            AxAgentError::Provider(format!(
                "Failed to parse Ollama /api/tags response: {e}. Body: {}",
                &body[..body.len().min(200)]
            ))
        })?;

        let models = tags
            .models
            .into_iter()
            .map(|m| {
                let model_type = ModelType::detect(&m.name);
                let mut caps = match model_type {
                    ModelType::Chat => vec![ModelCapability::TextChat],
                    ModelType::Embedding => vec![],
                    ModelType::Voice => vec![ModelCapability::RealtimeVoice],
                };
                // 从 details.family 推断部分能力
                if let Some(ref details) = m.details {
                    if let Some(ref family) = details.family {
                        let fam = family.to_lowercase();
                        if fam.contains("llava")
                            || fam.contains("bakllava")
                            || fam.contains("minicpm")
                            || fam.contains("moondream")
                        {
                            caps.push(ModelCapability::Vision);
                        }
                    }
                }
                let group_name = m.details.as_ref().and_then(|d| d.family.clone());
                let max_tokens = axagent_core::model_knowledge::get_model_context_window(&m.name);
                let name = m
                    .details
                    .as_ref()
                    .and_then(|d| d.parameter_size.clone())
                    .map(|ps| format!("{} ({})", m.name, ps))
                    .unwrap_or(m.name.clone());
                Model {
                    provider_id: ctx.provider_id.clone(),
                    model_id: m.name.clone(),
                    name,
                    group_name,
                    model_type,
                    capabilities: caps,
                    max_tokens,
                    max_output_tokens: None,
                    enabled: true,
                    param_overrides: None,
                    input_price_per_mtok: None,
                    output_price_per_mtok: None,
                }
            })
            .collect();

        Ok(models)
    }

    /// Validate that the Ollama server is reachable.
    ///
    /// Ollama does not require an API key, so we simply probe the
    /// `/api/tags` endpoint. If it responds, the server is running.
    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        let base = Self::base_url(ctx);
        let url = format!("{}/api/tags", base.trim_end_matches('/'));
        let chat_url = Self::effective_chat_url(ctx);

        let client = self.get_client(ctx)?;
        let resp = crate::apply_request_headers(client.get(&url), ctx)
            .send()
            .await
            .map_err(|e| {
                AxAgentError::Provider(format!(
                    "Ollama server not reachable at {}: {}. \
                     Make sure Ollama is running locally. You can start it with 'ollama serve'.",
                    base, e
                ))
            })?;

        if resp.status().is_success() {
            tracing::debug!("[Ollama] Tags endpoint OK, chat URL resolved to: {}", chat_url);
            Ok(true)
        } else {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            Err(AxAgentError::Provider(format!("Ollama server returned error {s}: {t}")))
        }
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }
}
