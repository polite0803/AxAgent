//! LLM provider + adapter 公共解析助手。
//!
//! 集中 4 个 executor 重复的 `resolve_model_for_node → decrypt_key → registry.get` 三步。

use axagent_core::types::{ProviderConfig, ProviderKey};
use axagent_harness::{ProviderAdapter, registry::ProviderRegistry};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::work_engine::node_executor_trait::{NodeError, error_code};

/// 解析 provider + key + model + adapter + api_key。
///
/// 调用方传 node_model / session_model / session_provider_id / profile_suggested，
/// helper 内部完成：
/// 1. `axagent_core::repo::provider::resolve_model_for_node` 拿到 (prov, key, model)
/// 2. `axagent_core::crypto::decrypt_key` 解密 api key
/// 3. `provider_registry.get(prov.provider_type.registry_key())` 拿 adapter
///
/// 返回值 `(prov, key, model, adapter, api_key)` 供调用方继续构建 request。
pub(crate) async fn resolve_provider_and_adapter(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_registry: Option<&Arc<dyn ProviderRegistry>>,
    node_model: Option<&str>,
    session_model: Option<&str>,
    session_provider_id: Option<&str>,
    profile_suggested_provider: Option<&str>,
    executor_label: &str,
) -> Result<(ProviderConfig, ProviderKey, String, Arc<dyn ProviderAdapter>, String), NodeError> {
    let (prov, key, model) = axagent_core::repo::provider::resolve_model_for_node(
        db,
        node_model,
        session_model,
        session_provider_id,
        profile_suggested_provider,
    )
    .await
    .map_err(|e| NodeError::exec_failed(error_code::UNSUPPORTED_PROVIDER, e))?;

    let api_key =
        axagent_core::crypto::decrypt_key(&key.key_encrypted, master_key).map_err(|e| {
            NodeError::exec_failed(
                error_code::UNSUPPORTED_PROVIDER,
                format!("API key decryption failed: {e}"),
            )
        })?;

    let registry_key = prov.provider_type.registry_key();
    let adapter: Arc<dyn ProviderAdapter> = provider_registry
        .and_then(|reg| reg.get(registry_key))
        .ok_or_else(|| {
            NodeError::exec_failed(
                error_code::UNSUPPORTED_PROVIDER,
                format!("{executor_label} 未找到 ProviderAdapter for type: {registry_key}"),
            )
        })?;

    Ok((prov, key, model, adapter, api_key))
}
