// SPDX-License-Identifier: AGPL-3.0-only

//! LLM Bridge 工厂函数 — 从 DB 构建 ProviderLlmBridge
//!
//! 在 Harness 架构中，这些函数负责将具体 provider 实现注入到 agent，
//! 因此使用 `axagent-providers` 具体类型是合理的（runtime 是编排器层）。
//!
//! **重写注意**：原实现手写 `match prov.provider_type { ... }` 把 ProviderType 映射到
//! 具体 Adapter 实现，与 `ProviderRegistry::get(registry_key)` 等价但绕过 registry。
//! 现改用 registry 单源查表；本文件不再依赖具体 Adapter 类型（OpenAIAdapter / AnthropicAdapter / ...）。

use axagent_agent::ProviderLlmBridge;
use axagent_crypto::crypto;
use axagent_harness::registry::ProviderRegistry;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use axagent_providers::url_utils::resolve_base_url_for_type;
use std::sync::Arc;

/// 从数据库构建 LLM 组件三元组（adapter + ctx + model）。
///
/// 这是 wiring 层的基础工厂：所有需要直接使用 `(adapter, ctx, model)` 的
/// 消费者（如 `LlmDrivenReasoningProvider`、`LlmBasedDecomposer`）都应调用本函数
/// 获取组件，而非各自重复 DB 查询逻辑。
///
/// 选用首个启用的 provider；使用默认 registry。
pub async fn build_llm_components_from_db(
    master_key: &[u8; 32],
) -> Option<(Arc<dyn ProviderAdapter>, ProviderRequestContext, String)> {
    let registry = default_registry();
    build_llm_components_from_db_with(master_key, &registry, None, None).await
}

/// 从数据库构建 LLM 组件三元组（指定 provider 和 model；调用方提供 registry）。
///
/// 选择优先级（修复前：直接取第一个启用的 provider + models[0]，无视用户设置的默认模型）：
/// 1. `preferred_provider_id` / `preferred_model_id`：调用方显式指定，找不到直接失败
///    （不静默回退，保持调用方意图）
/// 2. 用户设置的默认模型：`settings.default_provider_id` / `settings.default_model_id`
///    （settings 表 key-value，与 `agency_expert.rs` 同款读法；默认配置不可用时回退）
/// 3. 回退：第一个启用的 provider + 其第一个启用的模型（原逻辑）
pub async fn build_llm_components_from_db_with(
    master_key: &[u8; 32],
    provider_registry: &Arc<dyn ProviderRegistry>,
    preferred_provider_id: Option<&str>,
    preferred_model_id: Option<&str>,
) -> Option<(Arc<dyn ProviderAdapter>, ProviderRequestContext, String)> {
    let providers =
        axagent_harness::repositories::provider_repository().list_providers().await.ok()?;

    // 读取用户设置的默认 provider/model（注册表未初始化时返回 None，安全回退）
    let settings = if let Some(repo) = axagent_harness::repositories::try_settings_repository() {
        repo.get_settings().await.ok()
    } else {
        None
    };

    let prov = if let Some(pid) = preferred_provider_id {
        // 调用方显式指定：找不到直接失败，不静默回退
        providers
            .iter()
            .find(|p| p.id == pid && p.enabled && p.keys.iter().any(|k| k.enabled))?
            .clone()
    } else {
        let default_pid = settings.as_ref().and_then(|s| s.default_provider_id.as_deref());
        default_pid
            .and_then(|pid| {
                providers
                    .iter()
                    .find(|p| p.id == pid && p.enabled && p.keys.iter().any(|k| k.enabled))
                    .cloned()
            })
            .or_else(|| {
                // 默认 provider 未配置或不可用 → 回退第一个启用的 provider
                providers.iter().find(|p| p.enabled && p.keys.iter().any(|k| k.enabled)).cloned()
            })?
    };

    let key = prov.keys.iter().find(|k| k.enabled)?;
    let api_key = crypto::decrypt_key(&key.key_encrypted, master_key).ok()?;

    // 模型选择：显式指定 > settings 默认模型 > provider 第一个启用模型。
    // 必须在构造 ctx 前计算——ctx 会 partial move `prov.proxy_config`，
    // 之后 `first_enabled_model(&prov)` 将无法借用。
    let model = if let Some(mid) = preferred_model_id {
        mid.to_string()
    } else if let Some(mid) = settings.as_ref().and_then(|s| s.default_model_id.as_deref()) {
        // settings 默认模型必须存在于该 provider 的启用模型中，否则回退
        if prov.models.iter().any(|m| m.enabled && m.model_id == mid) {
            mid.to_string()
        } else {
            first_enabled_model(&prov)
        }
    } else {
        first_enabled_model(&prov)
    };

    // 单源查表：用 ProviderRegistry 取代手写 match
    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&prov.provider_type);
    let adapter: Arc<dyn ProviderAdapter> = provider_registry.get(registry_key)?;

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: prov.proxy_config,
        custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Some((adapter, ctx, model))
}

/// 返回 provider 第一个启用的模型；全部未启用时取第一个；都没有则 "default"。
///
/// 修复点：原逻辑 `prov.models.first()` 不检查 `model.enabled`，
/// 可能选到被用户禁用的模型。
fn first_enabled_model(prov: &axagent_harness::types::provider_model::ProviderConfig) -> String {
    prov.models
        .iter()
        .find(|m| m.enabled)
        .or_else(|| prov.models.first())
        .map(|m| m.model_id.clone())
        .unwrap_or_else(|| "default".to_string())
}

/// 从数据库构建 LLM Bridge（自动选择首个启用的 provider；使用默认 registry）
pub async fn build_llm_bridge_from_db(master_key: &[u8; 32]) -> Option<ProviderLlmBridge> {
    let (adapter, ctx, model) = build_llm_components_from_db(master_key).await?;
    Some(ProviderLlmBridge::new(adapter, ctx, model))
}

/// 从数据库构建 LLM Bridge（指定 provider 和 model；调用方提供 registry）
pub async fn build_llm_bridge_from_db_with(
    master_key: &[u8; 32],
    provider_registry: &Arc<dyn ProviderRegistry>,
    preferred_provider_id: Option<&str>,
    preferred_model_id: Option<&str>,
) -> Option<ProviderLlmBridge> {
    let (adapter, ctx, model) = build_llm_components_from_db_with(
        master_key,
        provider_registry,
        preferred_provider_id,
        preferred_model_id,
    )
    .await?;
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
