// SPDX-License-Identifier: AGPL-3.0-only

//! LLM 调用公共辅助 — 从 `commands/fleet/executor.rs` 提取，
//! 供 fleet executor 和 task_shape LLM 分类器共用。
//!
//! 消除铁律 12「禁止重复定义」违规：`resolve_default_provider` 原为
//! fleet/executor.rs 私有函数，现提取为 runtime 公开 API。

use std::sync::Arc;

use axagent_crypto;
use axagent_dao::repo::provider;
use axagent_dao::repo::settings;
use axagent_harness::types::provider_model::resolve_provider_proxy;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext, resolve_base_url_for_type};

use crate::harness::RuntimeHarness;

/// 已解析的默认提供商上下文（供路由与执行共用）。
pub struct ResolvedProvider {
    pub provider_id: String,
    pub model_id: String,
    pub adapter: Arc<dyn ProviderAdapter>,
    pub ctx: ProviderRequestContext,
}

/// 解析「默认提供商 + 默认模型」。
///
/// 选取优先级（与前端会话实际使用的模型保持一致，修复 task_shape 等
/// 后台 LLM 调用随机落到其他 provider 导致 400 的问题）：
///   1. settings.default_provider_id 指定的提供商（enabled 且含可用 key）
///   2. 兜底：第一个 enabled 且含可用 key 的提供商（历史行为）
///
/// 模型选取：
///   1. settings.default_model_id（须属于该提供商的模型列表）
///   2. 兜底：该提供商模型列表第一个
///
/// 供 fleet executor 和 task_shape LLM 分类器共用。
pub async fn resolve_default_provider(
    harness: &RuntimeHarness,
) -> Result<ResolvedProvider, String> {
    let providers = provider::list_providers(harness.db()).await.unwrap_or_default();
    let cfg = settings::get_settings(harness.db()).await.unwrap_or_default();
    let default_pid = cfg.default_provider_id.as_deref();
    let default_mid = cfg.default_model_id.as_deref().unwrap_or("");

    let is_usable = |p: &axagent_harness::types::provider_model::ProviderConfig| {
        p.enabled && p.keys.iter().any(|k| k.enabled)
    };

    let prov = providers
        .iter()
        .find(|p| default_pid.is_some_and(|pid| p.id == pid) && is_usable(p))
        .or_else(|| providers.iter().find(|p| is_usable(p)))
        .cloned()
        .ok_or_else(|| "没有启用的模型提供商".to_string())?;

    let key =
        prov.keys.iter().find(|k| k.enabled).ok_or_else(|| "没有可用的 API key".to_string())?;
    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, harness.master_key())
        .map_err(|e| format!("解密 API key 失败: {e}"))?;

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: resolve_provider_proxy(
            &prov.proxy_config,
            &settings::get_settings(harness.db()).await.unwrap_or_default(),
        ),
        custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = harness
        .get_adapter_for_provider(&prov)
        .await
        .ok_or_else(|| format!("无适配器可用: {:?}", prov.provider_type))?;

    // 默认模型：settings.default_model_id 优先（须属于该提供商），回退模型列表第一个
    let model_id = (!default_mid.is_empty()
        && prov.models.iter().any(|m| m.model_id == default_mid))
    .then(|| default_mid.to_string())
    .or_else(|| prov.models.first().map(|m| m.model_id.clone()))
    .unwrap_or_default();

    Ok(ResolvedProvider { provider_id: prov.id, model_id, adapter, ctx })
}

/// 用默认提供商跑一次非流式 chat，返回响应文本。
///
/// 通用轻量 LLM 调用入口（system + user 两轮消息，temperature=0，max_tokens 可配）。
pub async fn chat_with_default_provider(
    harness: &RuntimeHarness,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let resolved = resolve_default_provider(harness).await?;

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.0),
        top_p: None,
        max_tokens: Some(max_tokens),
        ..Default::default()
    };

    let resp = resolved
        .adapter
        .chat(&resolved.ctx, Arc::new(request))
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    Ok(resp.content)
}
