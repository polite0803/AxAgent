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
pub async fn build_llm_components_from_db_with(
    master_key: &[u8; 32],
    provider_registry: &Arc<dyn ProviderRegistry>,
    preferred_provider_id: Option<&str>,
    preferred_model_id: Option<&str>,
) -> Option<(Arc<dyn ProviderAdapter>, ProviderRequestContext, String)> {
    let providers =
        axagent_harness::repositories::provider_repository().list_providers().await.ok()?;

    let prov = if let Some(pid) = preferred_provider_id {
        providers
            .into_iter()
            .find(|p| p.id == pid && p.enabled && p.keys.iter().any(|k| k.enabled))?
    } else {
        providers.into_iter().find(|p| p.enabled && p.keys.iter().any(|k| k.enabled))?
    };

    let key = prov.keys.iter().find(|k| k.enabled)?;
    let api_key = crypto::decrypt_key(&key.key_encrypted, master_key).ok()?;

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

    let model = if let Some(mid) = preferred_model_id {
        mid.to_string()
    } else {
        prov.models.first().map(|m| m.model_id.clone()).unwrap_or_else(|| "default".to_string())
    };

    Some((adapter, ctx, model))
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

// ── FleetIntentLlm 适配器 ────────────────────────────────────────────────
//
// `axagent_agent::ProviderLlmBridge` 已具备 `call_llm(system, user)` 能力，
// 但它没有实现 `axagent_harness::fleet::FleetIntentLlm` trait（agent crate 不能
// 依赖 providers，但 runtime 是 wiring 层，可以同时依赖两者）。
//
// 本适配器在 wiring 层桥接二者，使 `LlmDispatcher` 能用真实 LLM 做意图分类，
// 替换 P0 阶段的 `NoopFleetIntentLlm` 兜底实现。

/// 用 `ProviderLlmBridge` 包装出的 `FleetIntentLlm` 实现。
///
/// - `route(system_prompt, user_prompt)` 调用 `ProviderLlmBridge::call_llm`
///   完成 LLM 意图分类，返回 LLM 原始响应文本（期望 JSON：
///   `{"agent_slug": "...", "reason": "..."}`）。
/// - 解析失败由 `LlmDispatcher` 兜底为第一个可用成员。
pub struct BridgeFleetIntentLlm {
    bridge: axagent_agent::ProviderLlmBridge,
}

impl BridgeFleetIntentLlm {
    pub fn new(bridge: axagent_agent::ProviderLlmBridge) -> Self {
        Self { bridge }
    }
}

#[async_trait::async_trait]
impl axagent_harness::fleet::FleetIntentLlm for BridgeFleetIntentLlm {
    async fn route(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        // 调用 ProviderLlmBridge 的 call_llm（默认 temp=0.7, max_tokens=2048）
        // 意图分类属轻量调用，单轮对话足够。
        self.bridge.call_llm(system_prompt, user_prompt).await
    }
}

/// 从数据库构建 `BridgeFleetIntentLlm` 实例（自动选择首个启用的 provider）。
///
/// 返回 `Arc<dyn FleetIntentLlm>`，可直接注入到 `LlmDispatcher::new`。
/// 若数据库无可用 provider，返回 `None`，调用方应回退到 `NoopFleetIntentLlm`。
pub async fn build_fleet_intent_llm_from_db(
    master_key: &[u8; 32],
) -> Option<std::sync::Arc<dyn axagent_harness::fleet::FleetIntentLlm>> {
    let bridge = build_llm_bridge_from_db(master_key).await?;
    Some(std::sync::Arc::new(BridgeFleetIntentLlm::new(bridge)))
}
