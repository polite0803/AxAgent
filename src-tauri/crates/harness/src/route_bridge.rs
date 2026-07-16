// SPDX-License-Identifier: AGPL-3.0-only

//! 路由决策桥接 — 连接 RouteDecision 与 ProviderRequestContext
//!
//! ## 背景（改进3 Phase D）
//!
//! `CostAwareRouter` 输出的 `RouteDecision` 只包含 tier（Budget/Balanced/Premium），
//! 但 `ProviderRequestContext` 需要具体的 `model_id` 和 `provider_id`。
//! 两者之间缺少一个"tier → 具体 model + provider"的映射层。
//!
//! ## 设计
//!
//! 本模块定义 `ModelTierResolver` trait，由应用层（wiring）实现，
//! 把 tier 字符串映射到具体的 `TierModelMapping`。
//! 这样 harness 层不需要依赖 smart_router 模块（应用层），
//! 应用层也不需要重定义 ProviderRequestContext（harness 层）。
//!
//! ## 架构位置
//!
//! - 定义层：harness（foundation）
//! - 实现层：应用层 wiring（`src/smart_router/` 或 `src/init/`）
//! - 调用层：在 `execute_llm` 之前，用 `apply_tier_to_request` 把 tier 映射注入 ChatRequest
//!
//! 符合 AGENTS.md 铁律：`组件 → harness ← 实现`

use crate::provider::ProviderRequestContext;
use crate::types::ChatRequest;

/// tier → 具体 model + provider 的映射结果
///
/// 由 `ModelTierResolver::resolve` 返回，应用层据此构建或调整 `ProviderRequestContext`。
#[derive(Debug, Clone)]
pub struct TierModelMapping {
    /// 解析出的模型 ID（如 "gpt-4o-mini" / "claude-3-sonnet"）
    pub model_id: String,
    /// 解析出的 provider ID（如 "openai" / "anthropic"）
    pub provider_id: String,
    /// 可选的 base URL 覆盖（用于自建端点 / 代理）
    pub base_url_override: Option<String>,
}

/// tier → model 映射器契约
///
/// 应用层实现此 trait，把 tier 字符串（"budget" / "balanced" / "premium"）
/// 解析为具体的 `TierModelMapping`。
///
/// 实现示例：
/// ```ignore
/// pub struct AppModelTierResolver {
///     mappings: HashMap<String, TierModelMapping>,
/// }
///
/// #[async_trait]
/// impl ModelTierResolver for AppModelTierResolver {
///     async fn resolve(&self, tier: &str) -> Option<TierModelMapping> {
///         self.mappings.get(tier).cloned()
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ModelTierResolver: Send + Sync {
    /// 把 tier 字符串解析为 `TierModelMapping`，找不到时返回 `None`
    async fn resolve(&self, tier: &str) -> Option<TierModelMapping>;
}

/// 把 tier 映射结果应用到 `ChatRequest`（设置 model 字段）
///
/// 如果 `resolver` 返回 `Some`，则用解析出的 `model_id` 覆盖 `request.model`；
/// 否则保持 `request` 原样（调用方自行决定降级策略）。
///
/// 返回 `(可能修改后的 request, Option<TierModelMapping>)`：
/// - `TierModelMapping` 为 `Some` 时，调用方还可据此调整 `ProviderRequestContext`
///   （如切换 provider_id / base_url）
pub async fn apply_tier_to_request(
    mut request: ChatRequest,
    tier: &str,
    resolver: &dyn ModelTierResolver,
) -> (ChatRequest, Option<TierModelMapping>) {
    if let Some(mapping) = resolver.resolve(tier).await {
        request.model = mapping.model_id.clone();
        (request, Some(mapping))
    } else {
        tracing::warn!(
            "[route_bridge] tier '{tier}' 无对应 model 映射，保持原 model: {}",
            request.model
        );
        (request, None)
    }
}

/// 用 tier 映射结果调整 `ProviderRequestContext`
///
/// 如果 `mapping` 中有 `provider_id` 或 `base_url_override`，
/// 则克隆并覆盖 `ctx` 的对应字段；否则原样返回 `ctx`。
///
/// 此函数不修改原始 `ctx`（因为 `ProviderRequestContext` 通常由调用方持有），
/// 返回调整后的克隆。
pub fn apply_mapping_to_context(
    ctx: &ProviderRequestContext,
    mapping: &TierModelMapping,
) -> ProviderRequestContext {
    ProviderRequestContext {
        provider_id: if mapping.provider_id.is_empty() {
            ctx.provider_id.clone()
        } else {
            mapping.provider_id.clone()
        },
        base_url: mapping.base_url_override.clone().or_else(|| ctx.base_url.clone()),
        api_key: ctx.api_key.clone(),
        key_id: ctx.key_id.clone(),
        api_path: ctx.api_path.clone(),
        proxy_config: ctx.proxy_config.clone(),
        custom_headers: ctx.custom_headers.clone(),
        api_mode: ctx.api_mode.clone(),
        conversation: ctx.conversation.clone(),
        previous_response_id: ctx.previous_response_id.clone(),
        store_response: ctx.store_response,
    }
}

// ── 共享类型 re-export ──
// ModelTier / RouteDecision 等类型留在应用层（src/smart_router/），
// harness 层只定义桥接契约，不依赖 RouteDecision 的具体定义。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderRequestContext;
    use crate::types::{ChatContent, ChatMessage, ChatRequest};
    use std::sync::Arc;

    /// 测试用 ModelTierResolver：budget → gpt-4o-mini, premium → gpt-4o
    struct MockResolver {
        mappings: std::collections::HashMap<String, TierModelMapping>,
    }

    #[async_trait::async_trait]
    impl ModelTierResolver for MockResolver {
        async fn resolve(&self, tier: &str) -> Option<TierModelMapping> {
            self.mappings.get(tier).cloned()
        }
    }

    fn make_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text("hi".to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            ..Default::default()
        }
    }

    fn make_ctx(provider_id: &str) -> ProviderRequestContext {
        ProviderRequestContext {
            api_key: "sk-test".to_string(),
            key_id: "key1".to_string(),
            provider_id: provider_id.to_string(),
            base_url: Some("https://default.example.com".to_string()),
            api_path: None,
            proxy_config: None,
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        }
    }

    #[tokio::test]
    async fn test_apply_tier_with_mapping() {
        let resolver = MockResolver {
            mappings: std::collections::HashMap::from([
                (
                    "budget".to_string(),
                    TierModelMapping {
                        model_id: "gpt-4o-mini".to_string(),
                        provider_id: "openai".to_string(),
                        base_url_override: None,
                    },
                ),
                (
                    "premium".to_string(),
                    TierModelMapping {
                        model_id: "gpt-4o".to_string(),
                        provider_id: "openai".to_string(),
                        base_url_override: None,
                    },
                ),
            ]),
        };

        let request = make_request("default-model");
        let (req, mapping) = apply_tier_to_request(request, "budget", &resolver).await;
        assert_eq!(req.model, "gpt-4o-mini");
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().model_id, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn test_apply_tier_no_mapping_keeps_original() {
        let resolver = MockResolver { mappings: std::collections::HashMap::new() };

        let request = make_request("original-model");
        let (req, mapping) = apply_tier_to_request(request, "unknown-tier", &resolver).await;
        assert_eq!(req.model, "original-model");
        assert!(mapping.is_none());
    }

    #[test]
    fn test_apply_mapping_to_context_overrides_provider() {
        let ctx = make_ctx("old-provider");
        let mapping = TierModelMapping {
            model_id: "gpt-4o".to_string(),
            provider_id: "new-provider".to_string(),
            base_url_override: Some("https://custom.endpoint.com".to_string()),
        };

        let new_ctx = apply_mapping_to_context(&ctx, &mapping);
        assert_eq!(new_ctx.provider_id, "new-provider");
        assert_eq!(new_ctx.base_url, Some("https://custom.endpoint.com".to_string()));
        // api_key 等其他字段保持不变
        assert_eq!(new_ctx.api_key, "sk-test");
    }

    #[test]
    fn test_apply_mapping_to_context_empty_provider_keeps_original() {
        let ctx = make_ctx("original-provider");
        let mapping = TierModelMapping {
            model_id: "some-model".to_string(),
            provider_id: String::new(), // 空 → 保持原 provider
            base_url_override: None,
        };

        let new_ctx = apply_mapping_to_context(&ctx, &mapping);
        assert_eq!(new_ctx.provider_id, "original-provider");
        assert_eq!(new_ctx.base_url, Some("https://default.example.com".to_string()));
    }

    #[tokio::test]
    async fn test_apply_tier_premium() {
        let resolver = MockResolver {
            mappings: std::collections::HashMap::from([(
                "premium".to_string(),
                TierModelMapping {
                    model_id: "claude-opus".to_string(),
                    provider_id: "anthropic".to_string(),
                    base_url_override: None,
                },
            )]),
        };

        let request = make_request("placeholder");
        let (req, mapping) = apply_tier_to_request(request, "premium", &resolver).await;
        assert_eq!(req.model, "claude-opus");
        let mapping = mapping.unwrap();
        assert_eq!(mapping.provider_id, "anthropic");
    }

    #[tokio::test]
    async fn test_resolver_dyn_dispatch() {
        let resolver: Arc<dyn ModelTierResolver> = Arc::new(MockResolver {
            mappings: std::collections::HashMap::from([(
                "balanced".to_string(),
                TierModelMapping {
                    model_id: "gpt-4o".to_string(),
                    provider_id: "openai".to_string(),
                    base_url_override: None,
                },
            )]),
        });

        let result = resolver.resolve("balanced").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().model_id, "gpt-4o");

        let result = resolver.resolve("unknown").await;
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_mapping_preserves_all_context_fields() {
        let ctx = ProviderRequestContext {
            api_key: "sk-key".to_string(),
            key_id: "kid".to_string(),
            provider_id: "orig".to_string(),
            base_url: Some("https://orig.example.com".to_string()),
            api_path: Some("/v1".to_string()),
            proxy_config: None,
            custom_headers: Some(std::collections::HashMap::from([(
                "X-Custom".to_string(),
                "val".to_string(),
            )])),
            api_mode: Some("chat".to_string()),
            conversation: Some("conv-1".to_string()),
            previous_response_id: Some("resp-1".to_string()),
            store_response: Some(true),
        };

        let mapping = TierModelMapping {
            model_id: "model-x".to_string(),
            provider_id: "new".to_string(),
            base_url_override: None,
        };

        let new_ctx = apply_mapping_to_context(&ctx, &mapping);
        // 覆盖的字段
        assert_eq!(new_ctx.provider_id, "new");
        // 保持的字段
        assert_eq!(new_ctx.api_key, "sk-key");
        assert_eq!(new_ctx.key_id, "kid");
        assert_eq!(new_ctx.base_url, Some("https://orig.example.com".to_string()));
        assert_eq!(new_ctx.api_path, Some("/v1".to_string()));
        assert_eq!(new_ctx.api_mode, Some("chat".to_string()));
        assert_eq!(new_ctx.conversation, Some("conv-1".to_string()));
        assert_eq!(new_ctx.previous_response_id, Some("resp-1".to_string()));
        assert_eq!(new_ctx.store_response, Some(true));
    }
}
