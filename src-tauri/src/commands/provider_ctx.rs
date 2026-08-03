// SPDX-License-Identifier: AGPL-3.0-only

use axagent_harness::types::ProviderType;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use std::sync::Arc;

/// 构建 ProviderRequestContext + adapter，供所有需要调用 LLM 的命令使用。
///
/// 从 screen_vision 提取而来，因为 multi_agent 等非视觉模块也需要此功能，
/// 不能被 `#[cfg(not(mobile))]` 门控排除。
pub(crate) fn resolve_provider_adapter(
    provider_type: &ProviderType,
) -> Result<Arc<dyn ProviderAdapter>, String> {
    match provider_type {
        ProviderType::OpenAI => Ok(Arc::new(axagent_providers::openai::OpenAIAdapter::new())),
        ProviderType::OpenAIResponses => {
            Ok(Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new()))
        },
        ProviderType::Anthropic => {
            Ok(Arc::new(axagent_providers::anthropic::AnthropicAdapter::new()))
        },
        ProviderType::Gemini => Ok(Arc::new(axagent_providers::gemini::GeminiAdapter::new())),
        ProviderType::OpenClaw => Ok(Arc::new(axagent_providers::openclaw::OpenClawAdapter::new())),
        ProviderType::Hermes => Ok(Arc::new(axagent_providers::hermes::HermesAdapter::new())),
        ProviderType::Ollama => Ok(Arc::new(axagent_providers::ollama::OllamaAdapter::new())),
        ProviderType::LlamaCpp => {
            Ok(Arc::new(axagent_providers::llama_cpp::LlamaCppAdapter::new()))
        },
    }
}

pub(crate) struct VisionContext {
    pub(crate) adapter: Arc<dyn ProviderAdapter>,
    pub(crate) ctx: ProviderRequestContext,
}

pub(crate) async fn build_vision_context(
    db: &sea_orm::DatabaseConnection,
    master_key: &[u8; 32],
    provider_id: &str,
) -> Result<VisionContext, String> {
    let provider =
        axagent_dao::repo::provider::get_provider(db, provider_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let key_row =
        axagent_dao::repo::provider::get_active_key(db, provider_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let decrypted_key =
        axagent_crypto::decrypt_key(&key_row.key_encrypted, master_key).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let global_settings = axagent_dao::repo::settings::get_settings(db).await.unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(
        &provider.proxy_config,
        &global_settings,
    );

    let adapter = resolve_provider_adapter(&provider.provider_type)?;

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id,
        base_url: Some(axagent_harness::url_utils::resolve_base_url_for_type(
            &provider.api_host,
            &provider.provider_type,
        )),
        api_path: provider.api_path,
        proxy_config: resolved_proxy,
        custom_headers: provider.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Ok(VisionContext { adapter, ctx })
}
