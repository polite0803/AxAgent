//! 数据源健康追踪与 fallback 链增强 (P1.5-3)
//!
//! 借鉴 TradingAgents-AShare 的 "failsafe fallback 链" 设计:
//! - 追踪每个 vendor 在 30s 滑动窗口内的失败次数
//! - 窗口内失败数超过阈值后自动降级该 vendor
//! - 窗口内失败数回落后自动恢复（无需固定恢复间隔）
//! - 记录 fallback 路径供调试
//!
//! ## 设计原理
//!
//! 传统连续失败计数在批量请求场景（如荐股扫描→200次并发请求）下，
//! vendor 可能在 1 秒内积累 8 次连续失败→触发降级。
//! 滑动窗口方案：
//! - 30s 窗口内积累 8 次失败才降级（burst 免疫）
//! - 窗口老化后自然恢复（无需 5min 固定间隔）
//! - 单次成功不清除窗口（避免"假恢复→再降级"振荡）

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// 失败追踪窗口大小（秒）
const FAILURE_WINDOW_SECS: u64 = 30;

/// 单个 vendor 的健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorHealth {
    /// vendor 名称
    pub name: String,
    /// 连续失败次数（保留用于日志/显示，降级决策用窗口）
    pub consecutive_failures: u32,
    /// 30s 滑动窗口内失败时间戳（epoch ms）
    #[serde(skip)]
    pub window_failures: VecDeque<i64>,
    /// 总成功次数
    pub total_successes: u64,
    /// 总失败次数
    pub total_failures: u64,
    /// 当前状态
    pub status: VendorStatus,
    /// 最后错误信息
    pub last_error: Option<String>,
    /// 最后成功时间 (epoch ms)
    pub last_success_at: Option<i64>,
    /// 最后失败时间 (epoch ms)
    pub last_failure_at: Option<i64>,
}

impl VendorHealth {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            consecutive_failures: 0,
            window_failures: VecDeque::with_capacity(64),
            total_successes: 0,
            total_failures: 0,
            status: VendorStatus::Healthy,
            last_error: None,
            last_success_at: None,
            last_failure_at: None,
        }
    }

    /// 清理窗口内过期条目并返回有效失败数（需要 &mut self）
    fn prune_window(&mut self, now: i64) -> usize {
        let cutoff = now - (FAILURE_WINDOW_SECS as i64 * 1000);
        while let Some(&ts) = self.window_failures.front() {
            if ts < cutoff {
                self.window_failures.pop_front();
            } else {
                break;
            }
        }
        self.window_failures.len()
    }

    /// 只读计算窗口内有效失败数（不需要 &mut self，用于 read-lock 路径）
    fn window_failure_count(&self, now: i64) -> usize {
        let cutoff = now - (FAILURE_WINDOW_SECS as i64 * 1000);
        self.window_failures
            .iter()
            .filter(|&&ts| ts >= cutoff)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorStatus {
    /// 正常
    Healthy,
    /// 已降级（窗口内失败数超过阈值）
    Degraded,
    /// 完全禁用（手动或严重错误）
    Disabled,
}

/// 健康检查配置
#[derive(Debug, Clone)]
pub struct VendorHealthConfig {
    /// 30s 窗口内多少次失败后降级
    pub degraded_threshold: u32,
    /// 降级后多久恢复尝试（秒）—— 窗口方案下作为降级尝试间隔的兜底
    pub recovery_interval_secs: u64,
    /// 是否记录 fallback 路径
    pub track_fallback_path: bool,
}

impl Default for VendorHealthConfig {
    fn default() -> Self {
        Self {
            degraded_threshold: 8,       // 30s 窗口内 8 次失败才降级，burst 免疫
            recovery_interval_secs: 300, // 兜底恢复间隔（窗口老化的同时仍保留此兜底）
            track_fallback_path: true,
        }
    }
}

/// 健康追踪器
pub struct VendorHealthTracker {
    vendors: Arc<RwLock<HashMap<String, VendorHealth>>>,
    config: VendorHealthConfig,
    /// fallback 路径记录（环形缓冲区）
    fallback_log: Arc<RwLock<Vec<FallbackRecord>>>,
}

// Use std::collections::HashMap inside the struct
use std::collections::HashMap;

/// 一次 fallback 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackRecord {
    /// 数据类 (quote, klines, news...)
    pub data_type: String,
    /// 股票代码
    pub stock_code: String,
    /// 首选 vendor
    pub primary_vendor: String,
    /// 实际使用的 vendor
    pub used_vendor: String,
    /// 失败原因
    pub reason: String,
    /// 时间戳
    pub timestamp: i64,
}

impl VendorHealthTracker {
    pub fn new(config: VendorHealthConfig) -> Self {
        Self {
            vendors: Arc::new(RwLock::new(HashMap::new())),
            config,
            fallback_log: Arc::new(RwLock::new(Vec::with_capacity(100))),
        }
    }

    /// 记录成功 —— 重置连续失败计数器，不清除窗口（避免"假恢复→再降级"振荡）
    pub async fn record_success(&self, name: &str) {
        let mut vendors = self.vendors.write().await;
        let now = chrono::Utc::now().timestamp_millis();
        let entry = vendors
            .entry(name.to_string())
            .or_insert_with(|| VendorHealth::new(name));
        entry.consecutive_failures = 0;
        entry.total_successes += 1;
        entry.last_success_at = Some(now);

        // 窗口失败数回落后自动恢复
        let window_count = entry.prune_window(now);
        if entry.status == VendorStatus::Degraded
            && (window_count as u32) < self.config.degraded_threshold
        {
            entry.status = VendorStatus::Healthy;
            warn!(
                "[VendorHealth] {} 自动恢复（窗口失败数 {} < 阈值 {}）",
                name, window_count, self.config.degraded_threshold
            );
        }
    }

    /// 记录失败，返回是否已降级
    pub async fn record_failure(&self, name: &str, error: &str) -> bool {
        let mut vendors = self.vendors.write().await;
        let now = chrono::Utc::now().timestamp_millis();
        let entry = vendors
            .entry(name.to_string())
            .or_insert_with(|| VendorHealth::new(name));
        entry.consecutive_failures += 1;
        entry.total_failures += 1;
        entry.last_error = Some(error.to_string());
        entry.last_failure_at = Some(now);

        // 添加失败时间戳到滑动窗口
        entry.window_failures.push_back(now);
        let window_count = entry.prune_window(now);

        if entry.status == VendorStatus::Healthy
            && (window_count as u32) >= self.config.degraded_threshold
        {
            entry.status = VendorStatus::Degraded;
            warn!(
                "[VendorHealth] {} 30s 窗口内失败 {} 次（共 {} 次），已降级。最后错误: {}",
                name, window_count, entry.total_failures, error
            );
            true
        } else {
            false
        }
    }

    // (is_vendor_degraded removed — 决策内联到 get_healthy_vendors / try_vendors)

    /// 获取可用 vendor 列表（按优先级排序，排除降级的）
    pub async fn get_healthy_vendors(&self, names: &[String]) -> Vec<String> {
        let vendors = self.vendors.read().await;
        let now = chrono::Utc::now().timestamp_millis();
        let recovery_ms = (self.config.recovery_interval_secs * 1000) as i64;

        names
            .iter()
            .filter(|name| {
                if let Some(health) = vendors.get(name.as_str()) {
                    match health.status {
                        VendorStatus::Healthy => true,
                        VendorStatus::Degraded => {
                            // 窗口方案：检查窗口失败数是否已回落
                            let window_count = health.window_failure_count(now);
                            if (window_count as u32) < self.config.degraded_threshold {
                                return true;
                            }
                            // 兜底：仍检查恢复间隔（给上游缓存留的硬超时）
                            if let Some(last_fail) = health.last_failure_at {
                                now - last_fail >= recovery_ms
                            } else {
                                false
                            }
                        },
                        VendorStatus::Disabled => false,
                    }
                } else {
                    true // 未知 vendor 默认可用
                }
            })
            .cloned()
            .collect()
    }

    /// 获取所有 vendor 的健康状态
    pub async fn get_all_health(&self) -> Vec<VendorHealth> {
        let vendors = self.vendors.read().await;
        let mut result: Vec<VendorHealth> = vendors.values().cloned().collect();
        // clone 时 window_failures VecDeque 也会 clone（跳过 serde）
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// 记录 fallback 路径
    pub async fn record_fallback(
        &self,
        data_type: &str,
        stock_code: &str,
        primary: &str,
        used: &str,
        reason: &str,
    ) {
        if !self.config.track_fallback_path {
            return;
        }
        let mut log = self.fallback_log.write().await;
        if log.len() >= 100 {
            log.remove(0);
        }
        log.push(FallbackRecord {
            data_type: data_type.to_string(),
            stock_code: stock_code.to_string(),
            primary_vendor: primary.to_string(),
            used_vendor: used.to_string(),
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 获取 fallback 日志
    pub async fn get_fallback_log(&self) -> Vec<FallbackRecord> {
        self.fallback_log.read().await.clone()
    }

    /// 获取缺省 vendor 列表（Healthy 优先，再试 Degraded [窗口回落后]）
    pub async fn try_vendors<'a>(&self, names: &'a [String]) -> Vec<&'a String> {
        let vendors = self.vendors.read().await;
        let now = chrono::Utc::now().timestamp_millis();
        let recovery_ms = (self.config.recovery_interval_secs * 1000) as i64;
        let threshold = self.config.degraded_threshold;

        let mut healthy = Vec::new();
        let mut recoverable = Vec::new();

        for name in names {
            match vendors.get(name.as_str()) {
                Some(h) if h.status == VendorStatus::Healthy => healthy.push(name),
                Some(h) if h.status == VendorStatus::Degraded => {
                    // 窗口方案：检查窗口失败数是否已回落 < 阈值 → 可恢复
                    let window_count = h.window_failure_count(now);
                    if (window_count as u32) < threshold {
                        recoverable.push(name);
                    } else {
                        // 兜底：硬超时恢复
                        if let Some(last_fail) = h.last_failure_at {
                            if now - last_fail >= recovery_ms {
                                recoverable.push(name);
                            }
                        }
                    }
                },
                None => healthy.push(name),
                _ => {},
            }
        }

        healthy.extend(recoverable);
        healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthy_vendor_available() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        tracker.record_success("tencent").await;
        let healthy = tracker
            .get_healthy_vendors(&["tencent".to_string(), "sina".to_string()])
            .await;
        assert!(healthy.contains(&"tencent".to_string()));
    }

    #[tokio::test]
    async fn window_degraded_vendor_filtered() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        // 30s 窗口内 8 次失败 → 降级
        for _ in 0..8 {
            tracker.record_failure("bad-vendor", "timeout").await;
        }
        let healthy = tracker
            .get_healthy_vendors(&["bad-vendor".to_string()])
            .await;
        assert!(healthy.is_empty(), "降级后的 vendor 不应出现在健康列表");
    }

    #[tokio::test]
    async fn window_auto_recovery() {
        let config = VendorHealthConfig {
            degraded_threshold: 3,
            recovery_interval_secs: 3600, // 长间隔，验证窗口恢复而非硬超时
            track_fallback_path: true,
        };
        let tracker = VendorHealthTracker::new(config);
        // 3 次失败 → 降级
        for _ in 0..3 {
            tracker.record_failure("flaky", "error").await;
        }
        // 此时 flaky 应被降级
        let healthy = tracker.get_healthy_vendors(&["flaky".to_string()]).await;
        assert_eq!(healthy.len(), 0, "3次失败后应降级");

        // 模拟窗口老化：直接清空窗口并记录一次成功（触发自动恢复）
        {
            let mut vendors = tracker.vendors.write().await;
            if let Some(h) = vendors.get_mut("flaky") {
                h.window_failures.clear(); // 模拟窗口老化
            }
        }
        // 一次成功应在窗口清理后自动恢复
        tracker.record_success("flaky").await;
        let healthy = tracker.get_healthy_vendors(&["flaky".to_string()]).await;
        assert_eq!(healthy.len(), 1, "窗口老化+成功后应自动恢复");
    }

    #[tokio::test]
    async fn tracks_fallback_path() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        tracker
            .record_fallback("quotes", "000001", "tencent", "sina", "timeout")
            .await;
        let log = tracker.get_fallback_log().await;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].primary_vendor, "tencent");
    }

    #[tokio::test]
    async fn success_resets_failures() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        tracker.record_failure("flaky", "error").await;
        tracker.record_success("flaky").await;
        let health = tracker.get_all_health().await;
        let h = health.iter().find(|h| h.name == "flaky").unwrap();
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.status, VendorStatus::Healthy);
    }
}
