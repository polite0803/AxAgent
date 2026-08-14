// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;

use crate::ProviderAdapter;
use crate::anthropic::AnthropicAdapter;
use crate::deepseek::DeepSeekAdapter;
use crate::gemini::GeminiAdapter;
use crate::glm::GlmAdapter;
use crate::hermes::HermesAdapter;
use crate::kimi::KimiAdapter;
use crate::llama_cpp::LlamaCppAdapter;
use crate::ollama::OllamaAdapter;
use crate::openai::OpenAIAdapter;
use crate::openai_responses::OpenAIResponsesAdapter;
use crate::openclaw::OpenClawAdapter;
use crate::qwen::QwenAdapter;
use crate::wenxin::WenxinAdapter;

pub struct ProviderRegistry {
    adapters: HashMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { adapters: HashMap::new() }
    }

    /// Register a provider adapter (takes `Arc` to support the harness `ProviderRegistry` trait)
    pub fn register(&mut self, provider_type: &str, adapter: Arc<dyn ProviderAdapter>) {
        if self.adapters.contains_key(provider_type) {
            tracing::warn!(
                provider_type,
                "Provider adapter already registered; overwriting existing entry"
            );
        }
        self.adapters.insert(provider_type.to_string(), adapter);
    }

    /// Get a registered adapter by provider type name
    pub fn get(&self, provider_type: &str) -> Option<&Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_type)
    }

    /// Creates a registry pre-populated with built-in provider adapters.
    ///
    /// 包含：OpenAI / Anthropic / Gemini / OpenClaw / Hermes / Ollama
    /// 以及国内厂商原生适配器：DeepSeek / 通义千问 / 智谱 GLM / Kimi / 文心一言。
    ///
    /// 纯构造：只填充本地注册表，不产生任何全局副作用。将内置适配器注册进全局
    /// 能力注册表是「导演（Director）」职责，由 wiring 层显式调用
    /// [`ProviderRegistry::register_builtins_into_capability_registry`] 一次完成
    /// （缺陷 #10：副作用式全局注入收敛为 Director 注册）。
    pub fn create_default() -> Self {
        let mut registry = Self::new();
        registry.register_all_builtins();
        registry
    }

    /// 纯构造辅助：把 13 个内置适配器注册到本地注册表。
    fn register_all_builtins(&mut self) {
        self.register("openai", Arc::new(OpenAIAdapter::new()));
        self.register("openai_responses", Arc::new(OpenAIResponsesAdapter::new()));
        self.register("anthropic", Arc::new(AnthropicAdapter::new()));
        self.register("gemini", Arc::new(GeminiAdapter::new()));
        self.register("openclaw", Arc::new(OpenClawAdapter::new()));
        self.register("hermes", Arc::new(HermesAdapter::new()));
        self.register("ollama", Arc::new(OllamaAdapter::new()));
        self.register("llama_cpp", Arc::new(LlamaCppAdapter::new()));
        // 国内 LLM 厂商原生适配器
        self.register("deepseek", Arc::new(DeepSeekAdapter::new()));
        self.register("qwen", Arc::new(QwenAdapter::new()));
        self.register("glm", Arc::new(GlmAdapter::new()));
        self.register("kimi", Arc::new(KimiAdapter::new()));
        self.register("wenxin", Arc::new(WenxinAdapter::new()));
    }

    /// Director 入口：把全部内置适配器注册进全局能力注册表（P1 试点：内置核心走统一接缝）。
    ///
    /// 每个适配器以 `model.provider.{type}` 为能力 ID、`BuiltIn` 为来源注册，
    /// 与外部插件（`ExternalPlugin`）走同一注册路径，实现「内置与外部插件平权」。
    /// 走 `register_model_provider_for_type` 特化入口，使注册与检索落在同一张
    /// 特化 map（`get_provider_adapter_for(type)` 可取回），修复「注册即检索失败」。
    /// 幂等：同键内置重复注册静默跳过，重复调用（如 wiring 重复初始化）安全、与调用顺序无关
    /// （缺陷 #10）。
    pub fn register_builtins_into_capability_registry(&self) {
        let registry = axagent_harness::get_capability_registry();
        for (provider_type, adapter) in &self.adapters {
            let _ = registry.register_model_provider_for_type(provider_type, adapter.clone());
        }
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Harness ProviderRegistry trait 实现
// ============================================================

impl axagent_harness::registry::ProviderRegistry for ProviderRegistry {
    fn get(&self, provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_type).cloned()
    }
}
