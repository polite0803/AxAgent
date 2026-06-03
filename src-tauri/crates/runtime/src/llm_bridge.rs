//! LLM Bridge 工厂函数 — 从 DB 构建 ProviderLlmBridge
//!
//! 在 Harness 架构中，这些函数负责将具体 provider 实现注入到 agent，
//! 因此使用 `axagent-providers` 具体类型是合理的（runtime 是编排器层）。

use axagent_agent::ProviderLlmBridge;
use axagent_core::crypto;
use axagent_core::repo::provider;
use axagent_core::types::ProviderType;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use axagent_providers::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, hermes::HermesAdapter,
    ollama::OllamaAdapter, openai::OpenAIAdapter, openai_responses::OpenAIResponsesAdapter,
    openclaw::OpenClawAdapter, resolve_base_url_for_type,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 从数据库构建 LLM Bridge（自动选择首个启用的 provider）
pub async fn build_llm_bridge_from_db(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
) -> Option<ProviderLlmBridge> {
    build_llm_bridge_from_db_with(db, master_key, None, None).await
}

/// 从数据库构建 LLM Bridge（指定 provider 和 model）
pub async fn build_llm_bridge_from_db_with(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    preferred_provider_id: Option<&str>,
    preferred_model_id: Option<&str>,
) -> Option<ProviderLlmBridge> {
    let providers = provider::list_providers(db).await.ok()?;

    let prov = if let Some(pid) = preferred_provider_id {
        providers
            .into_iter()
            .find(|p| p.id == pid && p.enabled && p.keys.iter().any(|k| k.enabled))?
    } else {
        providers
            .into_iter()
            .find(|p| p.enabled && p.keys.iter().any(|k| k.enabled))?
    };

    let key = prov.keys.iter().find(|k| k.enabled)?;
    let api_key = crypto::decrypt_key(&key.key_encrypted, master_key).ok()?;

    let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
        ProviderType::OpenAI => Arc::new(OpenAIAdapter::new()),
        ProviderType::OpenAIResponses => Arc::new(OpenAIResponsesAdapter::new()),
        ProviderType::Anthropic => Arc::new(AnthropicAdapter::new()),
        ProviderType::Gemini => Arc::new(GeminiAdapter::new()),
        ProviderType::OpenClaw => Arc::new(OpenClawAdapter::new()),
        ProviderType::Hermes => Arc::new(HermesAdapter::new()),
        ProviderType::Ollama => Arc::new(OllamaAdapter::new()),
    };

    let base_url = Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type));
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url,
        api_path: prov.api_path.clone(),
        proxy_config: prov.proxy_config,
        custom_headers: prov
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let model = if let Some(mid) = preferred_model_id {
        mid.to_string()
    } else {
        prov.models
            .first()
            .map(|m| m.model_id.clone())
            .unwrap_or_else(|| "default".to_string())
    };

    Some(ProviderLlmBridge::new(adapter, ctx, model))
}
