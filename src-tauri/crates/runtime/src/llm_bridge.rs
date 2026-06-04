//! LLM Bridge 工厂函数 — 从 DB 构建 ProviderLlmBridge
//!
//! 在 Harness 架构中，这些函数负责将具体 provider 实现注入到 agent，
//! 因此使用 `axagent-providers` 具体类型是合理的（runtime 是编排器层）。
//!
//! **重写注意**：原实现手写 `match prov.provider_type { ... }` 把 ProviderType 映射到
//! 具体 Adapter 实现，与 `ProviderRegistry::get(registry_key)` 等价但绕过 registry。
//! 现改用 registry 单源查表；本文件不再依赖具体 Adapter 类型（OpenAIAdapter / AnthropicAdapter / ...）。

use axagent_agent::ProviderLlmBridge;
use axagent_core::crypto;
use axagent_core::repo::provider;
use axagent_harness::registry::ProviderRegistry;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use axagent_providers::resolve_base_url_for_type;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 从数据库构建 LLM Bridge（自动选择首个启用的 provider；使用默认 registry）
pub async fn build_llm_bridge_from_db(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
) -> Option<ProviderLlmBridge> {
    let registry = default_registry();
    build_llm_bridge_from_db_with(db, master_key, &registry, None, None).await
}

/// 从数据库构建 LLM Bridge（指定 provider 和 model；调用方提供 registry）
pub async fn build_llm_bridge_from_db_with(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_registry: &Arc<dyn ProviderRegistry>,
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

    // 单源查表：用 ProviderRegistry 取代手写 match
    let registry_key = prov.provider_type.registry_key();
    let adapter: Arc<dyn ProviderAdapter> = provider_registry.get(registry_key)?;

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
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

/// 默认 ProviderRegistry（懒创建单例，避免每次 build_llm_bridge_from_db 都新建一份
/// `axagent_providers::registry::ProviderRegistry`）
fn default_registry() -> Arc<dyn ProviderRegistry> {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Arc<dyn ProviderRegistry>> = OnceLock::new();
    DEFAULT
        .get_or_init(|| {
            Arc::new(axagent_providers::registry::ProviderRegistry::create_default())
                as Arc<dyn ProviderRegistry>
        })
        .clone()
}
