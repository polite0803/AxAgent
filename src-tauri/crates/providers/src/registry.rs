// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;

use crate::ProviderAdapter;
use crate::anthropic::AnthropicAdapter;
use crate::gemini::GeminiAdapter;
use crate::hermes::HermesAdapter;
use crate::ollama::OllamaAdapter;
use crate::openai::OpenAIAdapter;
use crate::openai_responses::OpenAIResponsesAdapter;
use crate::openclaw::OpenClawAdapter;

pub struct ProviderRegistry {
    adapters: HashMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register a provider adapter (takes `Arc` to support the harness `ProviderRegistry` trait)
    pub fn register(&mut self, provider_type: &str, adapter: Arc<dyn ProviderAdapter>) {
        self.adapters.insert(provider_type.to_string(), adapter);
    }

    /// Get a registered adapter by provider type name
    pub fn get(&self, provider_type: &str) -> Option<&Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_type)
    }

    /// Creates a registry pre-populated with OpenAI, Anthropic, Gemini, OpenClaw, and Hermes adapters.
    pub fn create_default() -> Self {
        let mut registry = Self::new();
        registry.register("openai", Arc::new(OpenAIAdapter::new()));
        registry.register("openai_responses", Arc::new(OpenAIResponsesAdapter::new()));
        registry.register("anthropic", Arc::new(AnthropicAdapter::new()));
        registry.register("gemini", Arc::new(GeminiAdapter::new()));
        registry.register("openclaw", Arc::new(OpenClawAdapter::new()));
        registry.register("hermes", Arc::new(HermesAdapter::new()));
        registry.register("ollama", Arc::new(OllamaAdapter::new()));
        registry
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
