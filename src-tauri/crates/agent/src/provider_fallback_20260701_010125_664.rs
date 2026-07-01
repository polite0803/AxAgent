// SPDX-License-Identifier: AGPL-3.0-only

//! Provider 自动 Fallback 编排
//!
//! 核心能力：
//! - 健康检查：超时检测 + 错误率追踪
//! - 主 Provider 故障时自动切换备用 Provider
//! - 降级策略：同档次优先 → 跨档次降级

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Provider 降级档次 — 用于决定 fallback 优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderTier {
    /// 顶级商用模型 (GPT-4o, Claude 3.5 Sonnet, Gemini 2.0 Pro)
    Premium = 0,
    /// 中档模型 (GPT-4o-mini, Claude 3.5 Haiku, Gemini 2.0 Flash)
    Standard = 1,
    /// 轻量 / 本地模型 (Ollama, Hermes)
    Light = 2,
}

/// 单个 Provider 的健康状态追踪
#[derive(Debug, Clone)]
struct ProviderHealth {
    /// 最近 N 次调用的成败记录 (true = 成功)
    recent_results: Vec<bool>,
    /// 最近一次失败的时间戳
    last_failure: Option<Instant>,
    /// 连续失败次数
    consecutive_failures: u32,
    /// 总调用次数
    total_calls: u64,
    /// 总失败次数
    total_failures: u64,
    /// 当前是否标记为不健康
    is_unhealthy: bool,
}

impl ProviderHealth {
    fn new() -> Self {
        Self {
            recent_results: Vec::with_capacity(20),
            last_failure: None,
            consecutive_failures: 0,
            total_calls: 0,
            total_failures: 0,
            is_unhealthy: false,
        }
    }

    fn record_success(&mut self) {
        self.recent_results.push(true);
        if self.recent_results.len() > 20 {
            self.recent_results.remove(0);
        }
        self.consecutive_failures = 0;
        self.total_calls += 1;
        self.is_unhealthy = false;
    }

    fn record_failure(&mut self) {
        self.recent_results.push(false);
        if self.recent_results.len() > 20 {
            self.recent_results.remove(0);
        }
        self.consecutive_failures += 1;
        self.total_calls += 1;
        self.total_failures += 1;
        self.last_failure = Some(Instant::now());

        // 连续 3 次失败 → 标记为不健康
        if self.consecutive_failures >= 3 {
            self.is_unhealthy = true;
        }
    }

    /// 最近 20 次调用中的错误率
    fn recent_error_rate(&self) -> f64 {
        if self.recent_results.is_empty() {
            return 0.0;
        }
        let failures = self.recent_results.iter().filter(|&&r| !r).count();
        failures as f64 / self.recent_results.len() as f64
    }

    /// 是否可以尝试恢复（进入不健康状态后至少等 30s 再尝试）
    fn can_retry(&self, cooldown: Duration) -> bool {
        match self.last_failure {
            Some(t) => t.elapsed() >= cooldown,
            None => true,
        }
    }
}

/// Provider 注册信息
#[derive(Debug, Clone)]
pub struct FallbackProvider {
    pub provider_type: String,
    pub model_id: String,
    pub tier: ProviderTier,
}

/// Fallback 编排引擎
pub struct ProviderFallbackEngine {
    /// 已注册的 fallback providers (key = provider_type)
    providers: HashMap<String, FallbackProvider>,
    /// 健康状态追踪 (key = provider_type)
    health: Mutex<HashMap<String, ProviderHealth>>,
    /// 降级策略：失败后使用哪个 provider_type
    fallback_chain: HashMap<String, Vec<String>>,
    /// 不健康 provider 冷却时间
    cooldown: Duration,
}

impl ProviderFallbackEngine {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            health: Mutex::new(HashMap::new()),
            fallback_chain: HashMap::new(),
            cooldown: Duration::from_secs(30),
        }
    }

    /// 注册一个 fallback provider
    pub fn register(
        &mut self,
        provider_type: &str,
        model_id: &str,
        tier: ProviderTier,
    ) {
        self.providers.insert(
            provider_type.to_string(),
            FallbackProvider {
                provider_type: provider_type.to_string(),
                model_id: model_id.to_string(),
                tier,
            },
        );
        self.health
            .lock()
            .unwrap()
            .entry(provider_type.to_string())
            .or_insert_with(ProviderHealth::new);
    }

    /// 设置 fallback 链：primary → [fallback1, fallback2, ...]
    pub fn set_fallback_chain(
        &mut self,
        primary: &str,
        fallbacks: Vec<String>,
    ) {
        self.fallback_chain
            .insert(primary.to_string(), fallbacks);
    }

    /// 自动构建 fallback 链：同档次 → 降档次，按注册顺序排列
    pub fn build_default_chain(&mut self) {
        let mut entries: Vec<&FallbackProvider> = self.providers.values().collect();
        entries.sort_by_key(|p| p.tier as u8);

        let provider_types: Vec<String> = entries
            .iter()
            .map(|p| p.provider_type.clone())
            .collect();

        // 为每个 provider 构建 fallback 链：跳过自己，优先同 tier 再跨 tier
        for p in &entries {
            let current_tier = p.tier;
            let mut chain: Vec<String> = Vec::new();

            // 先加同档次的其他 provider
            for other in &entries {
                if other.provider_type != p.provider_type && other.tier == current_tier {
                    chain.push(other.provider_type.clone());
                }
            }
            // 再加更低档次的 provider
            for other in &entries {
                if other.provider_type != p.provider_type && other.tier > current_tier {
                    chain.push(other.provider_type.clone());
                }
            }

            self.fallback_chain
                .insert(p.provider_type.clone(), chain);
        }
    }

    /// 记录调用成功
    pub fn record_success(&self, provider_type: &str) {
        let mut health_map = self.health.lock().unwrap();
        health_map
            .entry(provider_type.to_string())
            .or_insert_with(ProviderHealth::new)
            .record_success();
    }

    /// 记录调用失败
    pub fn record_failure(&self, provider_type: &str) {
        let mut health_map = self.health.lock().unwrap();
        health_map
            .entry(provider_type.to_string())
            .or_insert_with(ProviderHealth::new)
            .record_failure();
    }

    /// 检查 provider 是否健康
    pub fn is_healthy(&self, provider_type: &str) -> bool {
        let health_map = self.health.lock().unwrap();
        match health_map.get(provider_type) {
            Some(h) => !h.is_unhealthy || h.can_retry(self.cooldown),
            None => true, // 未追踪 = 假设健康
        }
    }

    /// 获取 provider 的错误率
    pub fn error_rate(&self, provider_type: &str) -> f64 {
        let health_map = self.health.lock().unwrap();
        health_map
            .get(provider_type)
            .map(|h| h.recent_error_rate())
            .unwrap_or(0.0)
    }

    /// 解析 fallback：返回下一个可用的 provider_type + model_id
    /// 如果当前 provider 健康，直接返回 None（不需要 fallback）
    /// 如果当前不健康，按 fallback_chain 顺序返回第一个健康的
    pub fn resolve_fallback(
        &self,
        failed_provider: &str,
    ) -> Option<(String, String)> {
        // 如果当前 provider 健康，不需要 fallback
        if self.is_healthy(failed_provider) {
            return None;
        }

        let chain = match self.fallback_chain.get(failed_provider) {
            Some(c) => c,
            None => return None,
        };

        for candidate in chain {
            if self.is_healthy(candidate) {
                if let Some(provider) = self.providers.get(candidate) {
                    return Some((
                        provider.provider_type.clone(),
                        provider.model_id.clone(),
                    ));
                }
            }
        }

        None
    }

    /// 获取所有 provider 的健康状态快照（用于调试/监控）
    pub fn health_snapshot(&self) -> HashMap<String, ProviderHealth> {
        self.health.lock().unwrap().clone()
    }

    /// 重置所有健康状态
    pub fn reset_all(&self) {
        let mut health_map = self.health.lock().unwrap();
        health_map.clear();
    }
}

impl Default for ProviderFallbackEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_recording() {
        let engine = ProviderFallbackEngine::new();
        engine.register("openai", "gpt-4o", ProviderTier::Premium);
        engine.register("anthropic", "claude-sonnet", ProviderTier::Premium);

        // 初始状态：都健康
        assert!(engine.is_healthy("openai"));
        assert!(engine.is_healthy("anthropic"));

        // 3 次失败 → 不健康
        engine.record_failure("openai");
        engine.record_failure("openai");
        assert!(engine.is_healthy("openai")); // 还健康
        engine.record_failure("openai");
        assert!(!engine.is_healthy("openai")); // 不健康
    }

    #[test]
    fn test_fallback_resolution() {
        let engine = ProviderFallbackEngine::new();
        engine.register("openai", "gpt-4o", ProviderTier::Premium);
        engine.register("anthropic", "claude-sonnet", ProviderTier::Premium);
        engine.register("ollama", "llama3", ProviderTier::Light);

        engine.set_fallback_chain(
            "openai",
            vec!["anthropic".to_string(), "ollama".to_string()],
        );

        // 健康时不需要 fallback
        assert!(engine.resolve_fallback("openai").is_none());

        // openai 故障 → fallback 到 anthropic
        for _ in 0..3 {
            engine.record_failure("openai");
        }
        let fb = engine.resolve_fallback("openai");
        assert!(fb.is_some());
        let (ptype, model) = fb.unwrap();
        assert_eq!(ptype, "anthropic");
        assert_eq!(model, "claude-sonnet");
    }

    #[test]
    fn test_build_default_chain() {
        let mut engine = ProviderFallbackEngine::new();
        engine.register("openai", "gpt-4o", ProviderTier::Premium);
        engine.register("anthropic", "claude-sonnet", ProviderTier::Premium);
        engine.register("gemini", "gemini-pro", ProviderTier::Standard);
        engine.register("ollama", "llama3", ProviderTier::Light);

        engine.build_default_chain();

        let chain = engine.fallback_chain.get("openai").unwrap();
        // 同档次：anthropic，然后 standard，然后 light
        assert!(chain.contains(&"anthropic".to_string()));
        let anthropic_idx = chain.iter().position(|s| s == "anthropic").unwrap();
        let gemini_idx = chain.iter().position(|s| s == "gemini").unwrap();
        let ollama_idx = chain.iter().position(|s| s == "ollama").unwrap();
        // 同档次优先
        assert!(anthropic_idx < gemini_idx);
        assert!(gemini_idx < ollama_idx);
    }

    #[test]
    fn test_error_rate() {
        let engine = ProviderFallbackEngine::new();
        engine.register("openai", "gpt-4o", ProviderTier::Premium);

        // 10 次成功 + 5 次失败
        for _ in 0..10 {
            engine.record_success("openai");
        }
        for _ in 0..5 {
            engine.record_failure("openai");
        }
        // 最近 15 次记录，5 次失败 = 33%
        let rate = engine.error_rate("openai");
        assert!((rate - 0.333).abs() < 0.05);
    }
}
