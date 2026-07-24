// SPDX-License-Identifier: AGPL-3.0-only

//! 网关智能路由策略与 per-provider 延迟追踪。
//!
//! 仅在请求的 `model` 字段为 **bare model name**（不含 `provider/` 前缀）
//! 且该 model 被多个 enabled provider 同时支持时触发；其余场景保持
//! 显式指定 provider 的既有行为不变（参见 `handlers/models.rs` 的
//! `resolve_provider_for_model`）。
//!
//! ## 策略
//! - `Failover`（默认）：按 provider `sort_order` 升序选首选；失败时由
//!   `handle_non_stream_with_failover` 的 key-failover 机制兜底。
//! - `Priority`：始终选 `sort_order` 最小（优先级最高）的可用 provider。
//! - `Latency`：选最近 N 次请求平均延迟最低的 provider。
//! - `Cost`：选单 token 成本最低的 provider（综合 input/output 单价）。
//! - `RoundRobin`：在可用 provider 间轮询。
//!
//! ## 配置
//! 通过环境变量 `AXAGENT_GATEWAY_ROUTING_STRATEGY` 配置，取值
//! `failover` / `priority` / `latency` / `cost` / `round_robin`（大小写不敏感）。
//! 未设置或解析失败时回退到 `Failover`，与现有行为一致。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::types::{LoadBalanceStrategy, Model, ProviderConfig};
use axagent_harness::usage_pricing::pricing_for_model;
use parking_lot::Mutex;

/// 滑动窗口大小：每个 provider 保留最近 16 次请求的延迟样本。
///
/// 取 16 而非更大值是为了：
/// 1) 内存常驻、写路径无分配（满窗口后环形覆盖）；
/// 2) 对突发抖动有一定平滑作用，又不至于把历史均值拖得太长导致策略迟钝。
const LATENCY_WINDOW_SIZE: usize = 16;

/// per-provider 延迟样本环形缓冲。
///
/// 写入路径在 `record_usage` 完成后调用 [`LatencyTracker::record`]；
/// 读取路径在路由决策时调用 [`LatencyTracker::average_ms`]。
///
/// 用 `parking_lot::Mutex` 而非 `tokio::sync::Mutex`：临界区极短（仅数组写入/求和），
/// 不跨 await，`parking_lot` 的非异步锁更轻量且不会污染 async runtime。
#[derive(Debug, Default)]
struct ProviderLatencySamples {
    /// 环形缓冲；满窗口后从 0 开始覆盖。
    samples_ms: [u64; LATENCY_WINDOW_SIZE],
    /// 已写入样本数（<= LATENCY_WINDOW_SIZE）；用于区分"空窗口"和"全 0 延迟"。
    filled: usize,
    /// 下一个写入位置（mod LATENCY_WINDOW_SIZE）。
    next: usize,
}

impl ProviderLatencySamples {
    /// 追加一个延迟样本（毫秒）。满窗口后环形覆盖最旧样本。
    fn push(&mut self, latency_ms: u64) {
        self.samples_ms[self.next] = latency_ms;
        self.next = (self.next + 1) % LATENCY_WINDOW_SIZE;
        if self.filled < LATENCY_WINDOW_SIZE {
            self.filled += 1;
        }
    }

    /// 返回已有样本的平均延迟（毫秒）；窗口为空时返回 `None`。
    fn average_ms(&self) -> Option<u64> {
        if self.filled == 0 {
            return None;
        }
        let sum: u64 = self.samples_ms[..self.filled].iter().sum();
        Some(sum / self.filled as u64)
    }
}

/// 全局 per-provider 延迟追踪器。
///
/// 在 `GatewayAppState` 中以 `Arc` 共享，所有 chat / native handler
/// 共用同一份延迟统计。线程安全：内部用 `parking_lot::Mutex` 保护 HashMap。
#[derive(Debug, Default, Clone)]
pub struct LatencyTracker {
    inner: Arc<Mutex<HashMap<String, ProviderLatencySamples>>>,
}

impl LatencyTracker {
    /// 创建空的追踪器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次 provider 请求的延迟（毫秒）。
    ///
    /// `provider_id` 为空时直接返回，避免空键污染 HashMap。
    pub fn record(&self, provider_id: &str, latency_ms: u64) {
        if provider_id.is_empty() {
            return;
        }
        let mut guard = self.inner.lock();
        guard.entry(provider_id.to_string()).or_default().push(latency_ms);
    }

    /// 返回某 provider 的最近平均延迟（毫秒）；无样本时返回 `None`。
    pub fn average_ms(&self, provider_id: &str) -> Option<u64> {
        let guard = self.inner.lock();
        guard.get(provider_id).and_then(ProviderLatencySamples::average_ms)
    }
}

/// Round-robin 游标（per-model 维度）。
///
/// 不同 model 的可用 provider 集合不同，故游标按 `model_id` 分桶。
/// 用 `parking_lot::Mutex` 保护：临界区仅取模 + 自增，无 await。
#[derive(Debug, Default, Clone)]
pub struct RoundRobinCursor {
    inner: Arc<Mutex<HashMap<String, usize>>>,
}

impl RoundRobinCursor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 取下一个 provider 索引（mod candidates_len）。
    ///
    /// `candidates_len` 为 0 时返回 0（调用方应保证非空）。
    pub fn next(&self, model_id: &str, candidates_len: usize) -> usize {
        if candidates_len == 0 {
            return 0;
        }
        let mut guard = self.inner.lock();
        let cur = guard.entry(model_id.to_string()).or_insert(0);
        let idx = *cur % candidates_len;
        *cur = (*cur + 1) % candidates_len;
        idx
    }
}

/// 从环境变量 `AXAGENT_GATEWAY_ROUTING_STRATEGY` 解析路由策略。
///
/// 未设置或解析失败时回退到 [`LoadBalanceStrategy::default`]（即 `Failover`），
/// 与既有行为保持一致，避免破坏存量部署。
#[must_use]
pub fn routing_strategy_from_env() -> LoadBalanceStrategy {
    let raw = std::env::var("AXAGENT_GATEWAY_ROUTING_STRATEGY")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let Some(raw) = raw else {
        return LoadBalanceStrategy::default();
    };

    match raw.as_str() {
        "failover" => LoadBalanceStrategy::Failover,
        "priority" => LoadBalanceStrategy::Priority,
        "latency" => LoadBalanceStrategy::Latency,
        "cost" => LoadBalanceStrategy::Cost,
        "round_robin" | "roundrobin" => LoadBalanceStrategy::RoundRobin,
        other => {
            tracing::warn!(
                strategy = other,
                "AXAGENT_GATEWAY_ROUTING_STRATEGY 取值无法识别，回退到默认 failover"
            );
            LoadBalanceStrategy::default()
        },
    }
}

/// 在多个候选 provider 中按策略选出首选 provider。
///
/// 输入：`candidates` 必须已过滤为「enabled 且支持该 model」的 provider。
/// 输出：选中的 provider 在 `candidates` 中的索引。
///
/// 各策略实现说明：
/// - `Failover` / `Priority`：按 `sort_order` 升序，取第一个（数字越小优先级越高）。
///   二者行为一致：`Failover` 的 fallback 由调用方的 key-failover 循环兜底，
///   此处只决定"首选"。
/// - `Latency`：取 [`LatencyTracker`] 中平均延迟最低的 provider；无样本时
///   退化为 `Priority`（避免冷启动时无据可依）。
/// - `Cost`：取单 token 综合成本最低的 provider。成本优先取 provider 的
///   `input_price_per_mtok` / `output_price_per_mtok`；缺失时回退到
///   [`pricing_for_model`] 查全局定价表；都无定价时退化为 `Priority`。
/// - `RoundRobin`：按 [`RoundRobinCursor`] 游标轮询。
///
/// `candidates` 为空时返回 `None`（调用方应提前校验非空）。
#[must_use]
pub fn select_provider_index(
    strategy: LoadBalanceStrategy,
    candidates: &[&ProviderConfig],
    model_id: &str,
    latency: &LatencyTracker,
    rr_cursor: &RoundRobinCursor,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }

    // 通用辅助：按 sort_order 升序取最小者的索引。
    // 数字越小优先级越高（与 providers 列表排序约定一致）。
    let priority_index = || {
        candidates.iter().enumerate().min_by_key(|(_, p)| p.sort_order).map(|(i, _)| i).unwrap_or(0)
    };

    match strategy {
        LoadBalanceStrategy::Failover | LoadBalanceStrategy::Priority => Some(priority_index()),
        LoadBalanceStrategy::Latency => {
            // 有样本的 provider 优先；都没样本时退化为 Priority。
            let mut best_idx = priority_index();
            let mut best_latency = u64::MAX;
            let mut has_any_sample = false;
            for (i, p) in candidates.iter().enumerate() {
                if let Some(avg) = latency.average_ms(&p.id) {
                    has_any_sample = true;
                    if avg < best_latency {
                        best_latency = avg;
                        best_idx = i;
                    }
                }
            }
            if has_any_sample {
                Some(best_idx)
            } else {
                Some(priority_index())
            }
        },
        LoadBalanceStrategy::Cost => {
            // 综合成本 = input 单价 + output 单价（USD / 1M tokens）。
            // 优先用 provider 自带的 per-model 价格；缺失时回退到全局定价表。
            let model_pricing = pricing_for_model(model_id);
            let cost_of = |p: &ProviderConfig| -> Option<f64> {
                if let Some(m) = find_model(p, model_id)
                    && let (Some(inp), Some(out)) =
                        (m.input_price_per_mtok, m.output_price_per_mtok)
                {
                    return Some(inp + out);
                }
                model_pricing.map(|mp| mp.input_cost_per_million + mp.output_cost_per_million)
            };

            let mut best_idx = priority_index();
            let mut best_cost = f64::MAX;
            let mut has_any_pricing = false;
            for (i, p) in candidates.iter().enumerate() {
                if let Some(cost) = cost_of(p) {
                    has_any_pricing = true;
                    if cost < best_cost {
                        best_cost = cost;
                        best_idx = i;
                    }
                }
            }
            if has_any_pricing {
                Some(best_idx)
            } else {
                Some(priority_index())
            }
        },
        LoadBalanceStrategy::RoundRobin => {
            let idx = rr_cursor.next(model_id, candidates.len());
            Some(idx)
        },
    }
}

/// 在 provider 的 models 列表中查找指定 model_id（enabled）。
fn find_model<'a>(provider: &'a ProviderConfig, model_id: &str) -> Option<&'a Model> {
    provider.models.iter().find(|m| m.enabled && m.model_id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::{Model, ModelType, ProviderConfig, ProviderType};

    fn make_provider(id: &str, sort_order: i32, models: Vec<Model>) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            name: id.to_string(),
            provider_type: ProviderType::OpenAI,
            api_host: String::new(),
            api_path: None,
            enabled: true,
            models,
            keys: Vec::new(),
            proxy_config: None,
            tool_adaptation: None,
            tool_adaptation_marker_prefix: None,
            custom_headers: None,
            icon: None,
            builtin_id: None,
            sort_order,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn make_model(model_id: &str, input: Option<f64>, output: Option<f64>) -> Model {
        Model {
            provider_id: String::new(),
            model_id: model_id.to_string(),
            name: model_id.to_string(),
            group_name: None,
            model_type: ModelType::Chat,
            capabilities: Vec::new(),
            max_tokens: None,
            max_output_tokens: None,
            enabled: true,
            param_overrides: None,
            input_price_per_mtok: input,
            output_price_per_mtok: output,
        }
    }

    #[test]
    fn empty_candidates_returns_none() {
        let latency = LatencyTracker::new();
        let rr = RoundRobinCursor::new();
        let idx = select_provider_index(LoadBalanceStrategy::Priority, &[], "m", &latency, &rr);
        assert!(idx.is_none());
    }

    #[test]
    fn priority_picks_lowest_sort_order() {
        let m = make_model("gpt-5", None, None);
        let p1 = make_provider("a", 10, vec![m.clone()]);
        let p2 = make_provider("b", 1, vec![m.clone()]);
        let candidates: Vec<&ProviderConfig> = vec![&p1, &p2];
        let latency = LatencyTracker::new();
        let rr = RoundRobinCursor::new();
        let idx = select_provider_index(
            LoadBalanceStrategy::Priority,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        assert_eq!(idx, Some(1)); // p2.sort_order=1 更小
    }

    #[test]
    fn latency_picks_lowest_average() {
        let m = make_model("gpt-5", None, None);
        let p1 = make_provider("a", 0, vec![m.clone()]);
        let p2 = make_provider("b", 0, vec![m.clone()]);
        let candidates: Vec<&ProviderConfig> = vec![&p1, &p2];
        let latency = LatencyTracker::new();
        latency.record("a", 500);
        latency.record("a", 700);
        latency.record("b", 100);
        latency.record("b", 200);
        let rr = RoundRobinCursor::new();
        let idx = select_provider_index(
            LoadBalanceStrategy::Latency,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        assert_eq!(idx, Some(1)); // b 平均更低
    }

    #[test]
    fn latency_falls_back_to_priority_when_no_samples() {
        let m = make_model("gpt-5", None, None);
        let p1 = make_provider("a", 10, vec![m.clone()]);
        let p2 = make_provider("b", 1, vec![m.clone()]);
        let candidates: Vec<&ProviderConfig> = vec![&p1, &p2];
        let latency = LatencyTracker::new();
        let rr = RoundRobinCursor::new();
        let idx = select_provider_index(
            LoadBalanceStrategy::Latency,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        assert_eq!(idx, Some(1)); // 退化为 priority → sort_order 更小者
    }

    #[test]
    fn cost_picks_lowest_combined_price() {
        let m_a = make_model("gpt-5", Some(2.0), Some(8.0)); // 合计 10
        let m_b = make_model("gpt-5", Some(0.5), Some(1.5)); // 合计 2
        let p1 = make_provider("a", 0, vec![m_a]);
        let p2 = make_provider("b", 0, vec![m_b]);
        let candidates: Vec<&ProviderConfig> = vec![&p1, &p2];
        let latency = LatencyTracker::new();
        let rr = RoundRobinCursor::new();
        let idx =
            select_provider_index(LoadBalanceStrategy::Cost, &candidates, "gpt-5", &latency, &rr);
        assert_eq!(idx, Some(1)); // b 合计 2 < a 合计 10
    }

    #[test]
    fn round_robin_cycles_through_candidates() {
        let m = make_model("gpt-5", None, None);
        let p1 = make_provider("a", 0, vec![m.clone()]);
        let p2 = make_provider("b", 0, vec![m.clone()]);
        let p3 = make_provider("c", 0, vec![m]);
        let candidates: Vec<&ProviderConfig> = vec![&p1, &p2, &p3];
        let latency = LatencyTracker::new();
        let rr = RoundRobinCursor::new();
        // 连续调用应循环 0 → 1 → 2 → 0
        let i1 = select_provider_index(
            LoadBalanceStrategy::RoundRobin,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        let i2 = select_provider_index(
            LoadBalanceStrategy::RoundRobin,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        let i3 = select_provider_index(
            LoadBalanceStrategy::RoundRobin,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        let i4 = select_provider_index(
            LoadBalanceStrategy::RoundRobin,
            &candidates,
            "gpt-5",
            &latency,
            &rr,
        );
        assert_eq!(i1, Some(0));
        assert_eq!(i2, Some(1));
        assert_eq!(i3, Some(2));
        assert_eq!(i4, Some(0));
    }

    #[test]
    fn env_parsing_defaults_to_failover() {
        // 未设置环境变量时回退到默认（本测试不污染环境变量）。
        // SAFETY: 测试为单线程执行，无并发 env 操作；此处仅清理本测试专用的环境变量。
        unsafe {
            std::env::remove_var("AXAGENT_GATEWAY_ROUTING_STRATEGY");
        }
        assert_eq!(routing_strategy_from_env(), LoadBalanceStrategy::Failover);
    }

    #[test]
    fn latency_tracker_window_evicts_oldest() {
        let tracker = LatencyTracker::new();
        // 填满窗口 + 1，验证环形覆盖后平均值基于最近 LATENCY_WINDOW_SIZE 个样本。
        for i in 0..(LATENCY_WINDOW_SIZE as u64 + 1) {
            tracker.record("p", 100 + i);
        }
        // 样本为 101..=116（100 被覆盖），平均 = (101+116)*16/2 / 16 = 108.5 → 整除得 108。
        let avg = tracker.average_ms("p").expect("应有样本");
        assert_eq!(avg, 108);
    }

    #[test]
    fn latency_tracker_empty_returns_none() {
        let tracker = LatencyTracker::new();
        assert!(tracker.average_ms("unknown").is_none());
    }
}
