// SPDX-License-Identifier: AGPL-3.0-only

//! Provider 自动 Fallback 编排（Phase 4 / P2）
//!
//! - 健康检查：记录每个 Provider 的最近调用成功率、平均延迟
//! - 降级策略：主 Provider 故障时自动切换到备用 Provider
//! - 档次分组：优先同档次 → 跨档次降级
//!
//! 与前端 tracer 集成：所有 fallback 事件通过 tracerStore.recordLlmCall()
//! 回传 fallbackUsed / fallbackModelId 字段。

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ── Provider 健康状态 ──

/// 单个 Provider 的健康追踪信息
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// 最近 N 次调用的成功率（0.0 - 1.0）
    pub success_rate: f64,
    /// 平均响应延迟（毫秒）
    pub avg_latency_ms: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 总调用次数
    pub total_calls: u64,
    /// 总失败次数
    pub total_failures: u64,
    /// 上次健康检查时间
    pub last_check: Instant,
    /// 是否被标记为不可用
    pub marked_down: bool,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            success_rate: 1.0,
            avg_latency_ms: 0,
            consecutive_failures: 0,
            total_calls: 0,
            total_failures: 0,
            last_check: Instant::now(),
            marked_down: false,
        }
    }
}

impl ProviderHealth {
    /// 记录一次成功调用
    pub fn record_success(&mut self, latency_ms: u64) {
        self.total_calls += 1;
        self.consecutive_failures = 0;
        self.marked_down = false;
        self.last_check = Instant::now();

        // 指数移动平均更新延迟
        if self.avg_latency_ms == 0 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = (self.avg_latency_ms * 7 + latency_ms) / 8;
        }

        // total_calls 已经被 += 1,必 >= 1,无需再判 0
        let successes = self.total_calls - self.total_failures;
        self.success_rate = successes as f64 / self.total_calls as f64;
    }

    /// 记录一次失败调用
    pub fn record_failure(&mut self) {
        self.total_calls += 1;
        self.total_failures += 1;
        self.consecutive_failures += 1;
        self.last_check = Instant::now();

        // total_calls 已经被 += 1,必 >= 1,无需再判 0
        let successes = self.total_calls - self.total_failures;
        self.success_rate = successes as f64 / self.total_calls as f64;

        // 连续失败 3 次标记为 down
        if self.consecutive_failures >= 3 {
            self.marked_down = true;
        }
    }

    /// 超时检测：如果标记为 down 已超过 cooldown，恢复为可用
    pub fn maybe_recover(&mut self, cooldown: Duration) {
        if self.marked_down && self.last_check.elapsed() > cooldown {
            self.marked_down = false;
            self.consecutive_failures = 0;
        }
    }
}

// ── Provider 档次定义 ──

/// Provider 档次（用于降级优先级排序）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderTier {
    /// 最高档次（如 GPT-4o、Claude 3.5 Sonnet）
    Premium = 0,
    /// 标准档次（如 GPT-4o-mini、Claude 3 Haiku）
    Standard = 1,
    /// 经济档次（如本地模型 Ollama）
    Economy = 2,
    /// 最低档次（兜底）
    Fallback = 3,
}

impl ProviderTier {
    /// 返回当前档次及更低档次的优先级列表
    pub fn degradation_chain(&self) -> Vec<ProviderTier> {
        match self {
            ProviderTier::Premium => vec![
                ProviderTier::Premium,
                ProviderTier::Standard,
                ProviderTier::Economy,
                ProviderTier::Fallback,
            ],
            ProviderTier::Standard => vec![
                ProviderTier::Standard,
                ProviderTier::Premium, // 同档次内首选，但也可以升到 Premium
                ProviderTier::Economy,
                ProviderTier::Fallback,
            ],
            ProviderTier::Economy => vec![ProviderTier::Economy, ProviderTier::Fallback],
            ProviderTier::Fallback => vec![ProviderTier::Fallback],
        }
    }
}

// ── Provider 注册表 ──

/// 单个已注册的 Provider 条目
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub tier: ProviderTier,
    /// 适配器句柄（索引到外部 adapter 表）
    pub adapter_index: usize,
}

// ── Fallback 状态管理 ──

/// Provider Fallback 管理器
pub struct ProviderFallbackManager {
    /// health[provider_id] = 健康状态
    health: RwLock<HashMap<String, ProviderHealth>>,
    /// 已注册的 Provider 列表
    providers: RwLock<Vec<ProviderEntry>>,
    /// 全局配置
    config: RwLock<FallbackConfig>,
}

#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// 连续失败阈值，达到后标记为 down
    pub consecutive_failure_threshold: u32,
    /// down 之后的冷却时间（超过后自动恢复）
    pub cooldown_duration: Duration,
    /// 超时阈值（毫秒），单次调用超过此值视为失败
    pub timeout_ms: u64,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            consecutive_failure_threshold: 3,
            cooldown_duration: Duration::from_secs(30),
            timeout_ms: 30_000,
        }
    }
}

impl ProviderFallbackManager {
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            health: RwLock::new(HashMap::new()),
            providers: RwLock::new(Vec::new()),
            config: RwLock::new(config),
        }
    }

    /// 注册一个 Provider
    pub async fn register(&self, entry: ProviderEntry) {
        let mut providers = self.providers.write().await;
        let mut health = self.health.write().await;
        health
            .entry(entry.provider_id.clone())
            .or_insert_with(ProviderHealth::default);
        providers.push(entry);
    }

    /// 记录成功调用
    pub async fn record_success(&self, provider_id: &str, latency_ms: u64) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(provider_id) {
            h.record_success(latency_ms);
        }
    }

    /// 记录失败调用
    pub async fn record_failure(&self, provider_id: &str) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(provider_id) {
            h.record_failure();
        }
    }

    /// 带超时的调用封装
    pub async fn record_call(&self, provider_id: &str, latency_ms: u64, is_error: bool) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(provider_id) {
            let timeout_ms = self.config.read().await.timeout_ms;
            if latency_ms > timeout_ms || is_error {
                h.record_failure();
            } else {
                h.record_success(latency_ms);
            }
        }
    }

    /// 选择下一个可用 Provider。
    ///
    /// 返回 `(provider_entry, is_fallback)`：
    /// - 优先返回首选（preferred）Provider
    /// - 如果首选不可用，按降级链搜索同档次 → 跨档次
    /// - is_fallback = true 表示发生了降级
    pub async fn select_provider(
        &self,
        preferred_id: Option<&str>,
    ) -> Option<(ProviderEntry, bool)> {
        // 统一锁顺序: config -> providers -> health
        // 整个决策在一次临界区内完成,避免 TOCTOU 与中途状态变更
        let config_guard = self.config.read().await;
        let cooldown = config_guard.cooldown_duration;
        let providers_guard = self.providers.read().await;
        let mut health_guard = self.health.write().await;

        // recover 与决策合并到同一临界区,避免重入读取不一致状态
        for h in health_guard.values_mut() {
            h.maybe_recover(cooldown);
        }

        // 1. 如果有首选且健康，直接返回
        if let Some(pref_id) = preferred_id
            && let Some(entry) = providers_guard.iter().find(|p| p.provider_id == pref_id)
            && let Some(h) = health_guard.get(pref_id)
            && !h.marked_down
        {
            return Some((entry.clone(), false));
        }

        // 2. 找到首选对应档次
        let preferred_tier = preferred_id
            .and_then(|id| providers_guard.iter().find(|p| p.provider_id == id))
            .map(|p| p.tier)
            .unwrap_or(ProviderTier::Standard);

        // 3. 按降级链搜索
        let chain = preferred_tier.degradation_chain();
        for tier in &chain {
            for entry in providers_guard.iter() {
                if entry.tier == *tier
                    && let Some(h) = health_guard.get(&entry.provider_id)
                    && !h.marked_down
                {
                    // P1-5 修复:边界错误 —— 使用 map_or 避免 preferred_id 为 None 时
                    // unwrap_or("") 永远产生空字符串,导致 entry.provider_id != "" 恒为 true
                    let is_fallback = preferred_id.is_some_and(|id| entry.provider_id != id);
                    return Some((entry.clone(), is_fallback));
                }
            }
        }

        // 4. 所有 Provider 都 down 了,强制恢复冷却最短的那个,避免服务彻底不可用
        //    (P1-4 修复:不直接返回第一个,而是选择"冷得最透"的,即 last_check 最早的)
        if let Some(entry) = providers_guard.iter().min_by_key(|p| {
            health_guard
                .get(&p.provider_id)
                .map(|h| h.last_check)
                .unwrap_or_else(Instant::now)
        }) {
            if let Some(h) = health_guard.get_mut(&entry.provider_id) {
                h.marked_down = false;
                h.consecutive_failures = 0;
            }
            return Some((entry.clone(), true));
        }

        None
    }

    /// 获取所有 Provider 的健康状态摘要
    pub async fn health_summary(&self) -> Vec<ProviderHealthSummary> {
        let health = self.health.read().await;
        let providers = self.providers.read().await;

        providers
            .iter()
            .map(|p| {
                let h = health.get(&p.provider_id).cloned().unwrap_or_default();
                ProviderHealthSummary {
                    provider_id: p.provider_id.clone(),
                    provider_name: p.provider_name.clone(),
                    model_id: p.model_id.clone(),
                    tier: p.tier,
                    health: h,
                }
            })
            .collect()
    }

    /// 更新超时配置
    pub async fn set_timeout(&self, timeout_ms: u64) {
        self.config.write().await.timeout_ms = timeout_ms;
    }
}

/// Provider 健康摘要（用于前端展示）
#[derive(Debug, Clone)]
pub struct ProviderHealthSummary {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub tier: ProviderTier,
    pub health: ProviderHealth,
}

// ── Default 实例 ──

impl Default for ProviderFallbackManager {
    fn default() -> Self {
        Self::new(FallbackConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_health_record_success() {
        let mut h = ProviderHealth::default();
        h.record_success(200);
        assert_eq!(h.total_calls, 1);
        assert_eq!(h.total_failures, 0);
        assert_eq!(h.consecutive_failures, 0);
        assert!(!h.marked_down);
    }

    #[test]
    fn test_provider_health_marks_down_after_3_failures() {
        let mut h = ProviderHealth::default();
        h.record_failure();
        assert!(!h.marked_down);
        h.record_failure();
        assert!(!h.marked_down);
        h.record_failure();
        assert!(h.marked_down);
    }

    #[test]
    fn test_provider_health_recovers_after_cooldown() {
        let mut h = ProviderHealth::default();
        h.record_failure();
        h.record_failure();
        h.record_failure();
        assert!(h.marked_down);

        // 模拟冷却时间已过
        h.last_check = Instant::now() - Duration::from_secs(60);
        h.maybe_recover(Duration::from_secs(30));
        assert!(!h.marked_down);
    }

    #[tokio::test]
    async fn test_select_preferred_when_healthy() {
        let mgr = ProviderFallbackManager::default();
        mgr.register(ProviderEntry {
            provider_id: "openai".into(),
            provider_name: "OpenAI".into(),
            model_id: "gpt-4o".into(),
            tier: ProviderTier::Premium,
            adapter_index: 0,
        })
        .await;
        mgr.register(ProviderEntry {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic".into(),
            model_id: "claude-3-sonnet".into(),
            tier: ProviderTier::Premium,
            adapter_index: 1,
        })
        .await;

        let (entry, is_fallback) = mgr
            .select_provider(Some("openai"))
            .await
            .expect("should find provider");
        assert_eq!(entry.provider_id, "openai");
        assert!(!is_fallback);
    }

    #[tokio::test]
    async fn test_fallback_when_preferred_down() {
        let mgr = ProviderFallbackManager::default();
        mgr.register(ProviderEntry {
            provider_id: "openai".into(),
            provider_name: "OpenAI".into(),
            model_id: "gpt-4o".into(),
            tier: ProviderTier::Premium,
            adapter_index: 0,
        })
        .await;
        mgr.register(ProviderEntry {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic".into(),
            model_id: "claude-3-sonnet".into(),
            tier: ProviderTier::Premium,
            adapter_index: 1,
        })
        .await;

        // 模拟 OpenAI 连续失败
        {
            let mut health = mgr.health.write().await;
            if let Some(h) = health.get_mut("openai") {
                h.record_failure();
                h.record_failure();
                h.record_failure();
            }
        }

        let (entry, is_fallback) = mgr
            .select_provider(Some("openai"))
            .await
            .expect("should fallback");
        assert_eq!(entry.provider_id, "anthropic");
        assert!(is_fallback);
    }
}
