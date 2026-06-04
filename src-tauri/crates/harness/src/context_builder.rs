//! `build_provider_request_context` — 公共 ProviderRequestContext 构造助手。
//!
//! 之前在多处（`llm_resolve.rs::resolve_provider_and_adapter`、
//! `platform_bridge.rs::call_llm`、以及若干 `commands/*.rs` 内的 4 处 3 步链）字节级
//! 重复构造 `ProviderRequestContext`（12 字段）。现统一收敛到本函数。
//!
//! 设计：本函数只关心"从已解密的 api_key + provider/key 配置 → ctx"这一纯数据变换，
//! 不涉及 `sea-orm` / `axagent-providers` 依赖，可放在 axagent-harness
//! （axagent-core 的 trait 抽象层），供所有 crate 复用。
//!
//! 注：URL 解析直接调用同 crate 的 `url_utils::resolve_base_url_for_type`，
//! 避免 harness → providers 反向依赖。

use crate::types::{ProviderConfig, ProviderKey};

use crate::provider::ProviderRequestContext;
use crate::url_utils::resolve_base_url_for_type;

/// 从 `ProviderConfig + ProviderKey + 已解密的 api_key` 构建 `ProviderRequestContext`。
///
/// 字段映射规则（与各调用方原逻辑保持一致）：
/// - `api_key` = 传入的明文 api_key
/// - `key_id` = `key.id`
/// - `provider_id` = `prov.id`
/// - `base_url` = `resolve_base_url_for_type(prov.api_host, prov.provider_type)`
/// - `api_path` = `prov.api_path.clone()`
/// - `proxy_config` = `prov.proxy_config.clone()`
/// - `custom_headers` = `prov.custom_headers` 解析 JSON 后的 Option
/// - 其余 5 个字段（api_mode/conversation/previous_response_id/store_response）= None
pub fn build_provider_request_context(
    prov: &ProviderConfig,
    key: &ProviderKey,
    api_key: String,
) -> ProviderRequestContext {
    ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: prov.proxy_config.clone(),
        custom_headers: prov
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    }
}
