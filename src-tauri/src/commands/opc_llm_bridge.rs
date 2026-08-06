// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业学习 LLM 桥接 — `LlmInferencePort` 的 wiring 层实现。
//!
//! 让行业学习引擎（反思/进化/自我改进）从「规则打分占位」升级为真实 LLM 推理：
//! 复用 `RuntimeHarness` 解析默认提供商（第一个启用且含可用 key 的 provider），
//! 并经 `axagent_harness::execute_llm` 中心化入口调用（统一 PromptGuard/审计/重试）。
//!
//! 设计要点：
//! - **无状态**：仅持 `RuntimeHarness`（Clone），行业无关，天然满足行业隔离原则。
//! - **失败回退**：任何失败返回 `Err`，由 `IndustryLearningEngine` 自动回退规则评估，
//!   不阻塞行业工作流。
//! - **低配置起步**：`LlmCallConfig::default()`（缓存关闭），验证稳定后可开缓存。

use std::sync::Arc;

use async_trait::async_trait;
use axagent_crypto::decrypt_key;
use axagent_dao::repo::provider;
use axagent_dao::repo::settings;
use axagent_harness::llm_executor::{LlmCallConfig, execute_llm};
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext, resolve_base_url_for_type};
use axagent_orchestrator::LlmInferencePort;
use axagent_runtime::harness::RuntimeHarness;

/// 已解析的默认提供商（adapter + 请求上下文 + 默认模型）
struct ResolvedProvider {
    model_id: String,
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
}

/// 从 Harness 解析「第一个启用且含可用 key 的提供商」。
async fn resolve_default_provider(harness: &RuntimeHarness) -> Result<ResolvedProvider, String> {
    let providers = provider::list_providers(harness.db()).await.unwrap_or_default();

    let prov = providers
        .into_iter()
        .find(|p| p.enabled && p.keys.iter().any(|k| k.enabled))
        .ok_or_else(|| "没有启用的模型提供商".to_string())?;

    let key =
        prov.keys.iter().find(|k| k.enabled).ok_or_else(|| "没有可用的 API key".to_string())?;
    let api_key = decrypt_key(&key.key_encrypted, harness.master_key())
        .map_err(|e| format!("解密 API key 失败: {e}"))?;

    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
        api_path: prov.api_path.clone(),
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
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

    // 默认模型：取该 provider 模型列表的第一个
    let model_id = prov.models.first().map(|m| m.model_id.clone()).unwrap_or_default();

    Ok(ResolvedProvider { model_id, adapter, ctx })
}

/// 行业学习 LLM 桥接（无状态，仅持 `RuntimeHarness` 克隆）。
pub struct OpcLlmBridge {
    harness: RuntimeHarness,
    /// 输出 token 上限（反思/进化/自我改进输出通常较短）
    max_output_tokens: u32,
}

impl OpcLlmBridge {
    pub fn new(harness: RuntimeHarness) -> Self {
        Self { harness, max_output_tokens: 2048 }
    }
}

#[async_trait]
impl LlmInferencePort for OpcLlmBridge {
    async fn infer(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let resolved = resolve_default_provider(&self.harness).await?;

        let mut messages = Vec::with_capacity(2);
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(sys.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });

        let request = ChatRequest {
            model: resolved.model_id.clone(),
            messages,
            stream: false,
            temperature: Some(0.0),
            top_p: None,
            max_tokens: Some(self.max_output_tokens),
            ..Default::default()
        };

        let result = execute_llm(
            resolved.adapter.as_ref(),
            &resolved.ctx,
            request,
            &LlmCallConfig::default(),
        )
        .await?;

        Ok(result.response.content)
    }
}
