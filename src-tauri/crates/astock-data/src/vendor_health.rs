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
    /// 持续故障标记（2026-07-31 新增）：
    /// 连接级反爬/网络故障（IncompleteMessage、error sending request、connection reset 等）
    /// 通常意味着 IP 被数据源封锁，属于"环境级持续故障"，不是 30s 瞬断。
    /// 标记后降级恢复只看硬超时（recovery_interval_secs），忽略滑动窗口老化，
    /// 避免长任务（如荐股扫描）内 eastmoney 每 30s 窗口老化就恢复→再被首选→再白打。
    pub sustained_failure: bool,
}

impl VendorHealth {
    /// P3-D12: pub 化以支持 stock_pipeline 测试构造健康快照
    pub fn new(name: &str) -> Self {
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
            sustained_failure: false,
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
        self.window_failures.iter().filter(|&&ts| ts >= cutoff).count()
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
    /// sustained_failure（连接级反爬）降级后的探测间隔（秒）。
    /// 2026-08-01 新增：此前 sustained 只按硬超时（1800s）恢复，期间首选
    /// vendor 完全缺席——若健康列表里恰好都是"空实现/无凭据"的 vendor，
    /// 整条链全空（AxInvest 趋势智选 2026-08-01 全链空实锤）。现在允许
    /// 每 probe_interval 秒单飞探测一次，成功即经 record_success 自动恢复，
    /// 失败则刷新 last_failure_at 顺延。兼顾防抖（不会每 30s 窗口白打）。
    pub probe_interval_secs: u64,
    /// 是否记录 fallback 路径
    pub track_fallback_path: bool,
}

impl Default for VendorHealthConfig {
    fn default() -> Self {
        Self {
            degraded_threshold: 8,        // 30s 窗口内 8 次失败才降级，burst 免疫
            recovery_interval_secs: 1800, // 兜底恢复间隔 30 分钟（2026-07-31 从 300s 上调：
            // 连接级反爬封锁是持续环境故障，5 分钟太短，
            // 每轮荐股 run 都会先白打首选 vendor 才降级）
            probe_interval_secs: 120, // sustained 降级探测间隔 2 分钟（2026-08-01 新增）
            track_fallback_path: true,
        }
    }
}

/// 健康追踪器
pub struct VendorHealthTracker {
    vendors: Arc<RwLock<HashMap<String, VendorHealth>>>,
    config: VendorHealthConfig,
    /// fallback 路径记录（环形缓冲区；修复 P1-13: 注释声称环形但实际是
    /// 无界 Vec，长时间运行（如荐股扫描 200 只股票 × 多次重试）会持续
    /// append，OOM 风险。改为 VecDeque 并设上限 1024 条，满时 pop_front）。
    fallback_log: Arc<RwLock<VecDeque<FallbackRecord>>>,
}

/// 环形缓冲上限；超出后弹出最旧条目，避免 OOM
const FALLBACK_LOG_CAP: usize = 1024;

/// 判断是否为连接级持续故障（反爬 RST / 网络中断 / 服务器异常断开）
///
/// 这类错误的共同特征：TCP/TLS 层握手成功但响应被掐断（或连接被拒），
/// 通常是出口 IP 被数据源反爬封锁，属于"环境级持续故障"而非 30s 瞬断。
/// 触发后 vendor 降级恢复只看硬超时，不随滑动窗口老化。
fn is_sustained_connection_error(error: &str) -> bool {
    let e = error.to_lowercase();
    e.contains("incompletemessage")
        || e.contains("error sending request")
        || e.contains("connection reset")
        || e.contains("connection aborted")
        || e.contains("close_notify")
        || e.contains("empty response")
        || e.contains("server closed")
        || e.contains("error on write")
}

use std::collections::{HashMap, VecDeque};

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
            fallback_log: Arc::new(RwLock::new(VecDeque::with_capacity(FALLBACK_LOG_CAP))),
        }
    }

    /// 记录成功 —— 重置连续失败计数器，不清除窗口（避免"假恢复→再降级"振荡）
    pub async fn record_success(&self, name: &str) {
        let mut vendors = self.vendors.write().await;
        let now = chrono::Utc::now().timestamp_millis();
        let entry = vendors.entry(name.to_string()).or_insert_with(|| VendorHealth::new(name));

        // P3-B5(B): Disabled vendor 完全冻结，不受事件影响
        if entry.status == VendorStatus::Disabled {
            return;
        }

        entry.consecutive_failures = 0;
        entry.total_successes += 1;
        entry.last_success_at = Some(now);
        // 一次成功证明链路恢复，清除持续故障标记
        entry.sustained_failure = false;

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
        let entry = vendors.entry(name.to_string()).or_insert_with(|| VendorHealth::new(name));

        // P3-B5(B): Disabled vendor 完全冻结，不受事件影响
        if entry.status == VendorStatus::Disabled {
            return false;
        }

        entry.consecutive_failures += 1;
        entry.total_failures += 1;
        entry.last_error = Some(error.to_string());
        entry.last_failure_at = Some(now);

        // 连接级反爬/网络故障 → 标记持续故障：
        // 此类错误（IncompleteMessage、error sending request 等）通常是 IP 被数据源
        // 封锁，属于环境级持续故障。降级后不随 30s 窗口老化恢复，只按硬超时。
        if is_sustained_connection_error(error) {
            entry.sustained_failure = true;
        }

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
                            // 持续故障（连接级反爬）：硬超时恢复 + 探测恢复（probe_interval）
                            if health.sustained_failure {
                                if let Some(last_fail) = health.last_failure_at {
                                    let probe_ms = (self.config.probe_interval_secs * 1000) as i64;
                                    return now - last_fail >= recovery_ms
                                        || now - last_fail >= probe_ms;
                                }
                                return false;
                            }
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

    /// P3-B5(B): 手动设置 vendor 状态
    /// 用于前端设置页手动启停 vendor：
    /// - `Healthy` → 重置失败计数，立即恢复
    /// - `Degraded` → 标记为降级，但允许窗口恢复
    /// - `Disabled` → 完全禁用，`try_vendors` 永远跳过（除非再次手动设为 Healthy）
    pub async fn set_vendor_status(&self, name: &str, status: VendorStatus) {
        let mut vendors = self.vendors.write().await;
        let now = chrono::Utc::now().timestamp_millis();
        let entry = vendors.entry(name.to_string()).or_insert_with(|| VendorHealth::new(name));
        let old_status = entry.status;
        entry.status = status;

        match status {
            VendorStatus::Healthy => {
                entry.consecutive_failures = 0;
                entry.window_failures.clear();
                entry.sustained_failure = false;
                entry.last_success_at = Some(now);
            },
            VendorStatus::Degraded => {
                entry.last_failure_at = Some(now);
            },
            VendorStatus::Disabled => {
                // Disabled 状态完全冻结，不修改其他字段
            },
        }

        tracing::info!("[VendorHealth] {} 状态手动变更: {:?} → {:?}", name, old_status, status);
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
        // 修复 P1-13: 用 VecDeque 真正的环形缓冲，超出上限弹出最旧
        if log.len() >= FALLBACK_LOG_CAP {
            log.pop_front();
        }
        log.push_back(FallbackRecord {
            data_type: data_type.to_string(),
            stock_code: stock_code.to_string(),
            primary_vendor: primary.to_string(),
            used_vendor: used.to_string(),
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 获取 fallback 日志（按时间顺序：旧 → 新）
    pub async fn get_fallback_log(&self) -> Vec<FallbackRecord> {
        self.fallback_log.read().await.iter().cloned().collect()
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
                    // 持续故障（连接级反爬）：硬超时恢复 + 探测恢复（probe_interval）
                    if h.sustained_failure {
                        if let Some(last_fail) = h.last_failure_at {
                            let probe_ms = (self.config.probe_interval_secs * 1000) as i64;
                            if now - last_fail >= recovery_ms || now - last_fail >= probe_ms {
                                recoverable.push(name);
                            }
                        }
                        continue;
                    }
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
        let healthy =
            tracker.get_healthy_vendors(&["tencent".to_string(), "sina".to_string()]).await;
        assert!(healthy.contains(&"tencent".to_string()));
    }

    #[tokio::test]
    async fn window_degraded_vendor_filtered() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        // 30s 窗口内 8 次失败 → 降级
        for _ in 0..8 {
            tracker.record_failure("bad-vendor", "timeout").await;
        }
        let healthy = tracker.get_healthy_vendors(&["bad-vendor".to_string()]).await;
        assert!(healthy.is_empty(), "降级后的 vendor 不应出现在健康列表");
    }

    #[tokio::test]
    async fn window_auto_recovery() {
        let config = VendorHealthConfig {
            degraded_threshold: 3,
            recovery_interval_secs: 3600, // 长间隔，验证窗口恢复而非硬超时
            probe_interval_secs: 3600,    // 长探测间隔，避免探测恢复干扰本测试
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
        tracker.record_fallback("quotes", "000001", "tencent", "sina", "timeout").await;
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

    /// P3-B5(B): set_vendor_status 手动切换状态
    #[tokio::test]
    async fn manual_disable_vendor_filters_it() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        // 初始为 Healthy，手动禁用
        tracker.set_vendor_status("bad-vendor", VendorStatus::Disabled).await;
        // try_vendors 应过滤掉 Disabled vendor
        let vendors = ["bad-vendor".to_string(), "good-vendor".to_string()];
        let available = tracker.try_vendors(&vendors).await;
        assert!(
            !available.iter().any(|v| v.as_str() == "bad-vendor"),
            "Disabled vendor 不应出现在 try_vendors 结果"
        );
        assert!(available.iter().any(|v| v.as_str() == "good-vendor"), "Healthy vendor 应保留");
    }

    /// P3-B5(B): 手动设为 Healthy 应清空失败窗口
    #[tokio::test]
    async fn manual_restore_clears_window() {
        let config = VendorHealthConfig { degraded_threshold: 3, ..Default::default() };
        let tracker = VendorHealthTracker::new(config);
        // 3 次失败 → 降级
        for _ in 0..3 {
            tracker.record_failure("flaky", "error").await;
        }
        let health = tracker.get_all_health().await;
        assert_eq!(
            health.iter().find(|h| h.name == "flaky").unwrap().status,
            VendorStatus::Degraded
        );

        // 手动恢复
        tracker.set_vendor_status("flaky", VendorStatus::Healthy).await;
        let health = tracker.get_all_health().await;
        let h = health.iter().find(|h| h.name == "flaky").unwrap();
        assert_eq!(h.status, VendorStatus::Healthy);
        assert_eq!(h.consecutive_failures, 0);
        assert!(h.window_failures.is_empty());
    }

    /// P3-B5(B): Disabled vendor 不受 record_success/record_failure 影响
    #[tokio::test]
    async fn disabled_vendor_ignores_events() {
        let tracker = VendorHealthTracker::new(VendorHealthConfig::default());
        tracker.set_vendor_status("frozen", VendorStatus::Disabled).await;

        // 即使 record_success 也不应恢复
        tracker.record_success("frozen").await;
        let health = tracker.get_all_health().await;
        let h = health.iter().find(|h| h.name == "frozen").unwrap();
        // record_success 的窗口恢复逻辑只对 Degraded 生效，Disabled 保持不变
        assert_eq!(h.status, VendorStatus::Disabled);
    }

    /// 2026-07-31: 连接级故障（反爬 RST）降级后不随窗口老化恢复
    #[tokio::test]
    async fn sustained_failure_ignores_window_aging() {
        let config = VendorHealthConfig {
            degraded_threshold: 3,
            // 长恢复间隔，验证持续故障只按硬超时恢复
            recovery_interval_secs: 3600,
            // 长探测间隔，避免探测恢复干扰本测试
            probe_interval_secs: 3600,
            track_fallback_path: true,
        };
        let tracker = VendorHealthTracker::new(config);
        // 连接级错误 3 次 → 降级 + sustained_failure 标记
        for _ in 0..3 {
            tracker.record_failure("blocked", "IncompleteMessage").await;
        }
        let health = tracker.get_all_health().await;
        let h = health.iter().find(|h| h.name == "blocked").unwrap();
        assert_eq!(h.status, VendorStatus::Degraded);
        assert!(h.sustained_failure, "连接级错误应标记持续故障");

        // 模拟窗口完全老化（等价于 30s 后），持续故障仍不应恢复
        {
            let mut vendors = tracker.vendors.write().await;
            if let Some(h) = vendors.get_mut("blocked") {
                h.window_failures.clear();
            }
        }
        let healthy = tracker.get_healthy_vendors(&["blocked".to_string()]).await;
        assert!(healthy.is_empty(), "持续故障在硬超时前不应随窗口老化恢复");

        // 一次成功 → 清除持续故障标记并恢复
        tracker.record_success("blocked").await;
        let healthy = tracker.get_healthy_vendors(&["blocked".to_string()]).await;
        assert_eq!(healthy.len(), 1, "成功后应清除持续故障并恢复");
    }

    /// 2026-08-01: sustained 降级在探测间隔到达后允许回归（首选 vendor 不再长期缺席）
    #[tokio::test]
    async fn sustained_failure_recovers_after_probe_interval() {
        let config = VendorHealthConfig {
            degraded_threshold: 3,
            recovery_interval_secs: 3600, // 硬超时很长，验证探测路径而非硬超时
            probe_interval_secs: 60,      // 探测间隔 60s
            track_fallback_path: true,
        };
        let tracker = VendorHealthTracker::new(config);
        // 连接级错误 3 次 → 降级 + sustained
        for _ in 0..3 {
            tracker.record_failure("blocked", "IncompleteMessage").await;
        }
        // 探测间隔未到 → 不恢复
        let healthy = tracker.get_healthy_vendors(&["blocked".to_string()]).await;
        assert!(healthy.is_empty(), "探测间隔未到不应恢复");
        // 把 last_failure_at 拨到 90s 前（模拟探测间隔已过）
        {
            let mut vendors = tracker.vendors.write().await;
            if let Some(h) = vendors.get_mut("blocked") {
                h.last_failure_at = Some(chrono::Utc::now().timestamp_millis() - 90_000);
            }
        }
        let healthy = tracker.get_healthy_vendors(&["blocked".to_string()]).await;
        assert_eq!(healthy.len(), 1, "探测间隔过后应允许单飞探测恢复");
    }

    /// 2026-07-31: 普通故障（如解析错误）仍随窗口老化恢复（不破坏原行为）
    #[tokio::test]
    async fn normal_failure_still_recovers_on_window_aging() {
        let config = VendorHealthConfig { degraded_threshold: 3, ..Default::default() };
        let tracker = VendorHealthTracker::new(config);
        for _ in 0..3 {
            tracker.record_failure("flaky", "parse error").await;
        }
        let health = tracker.get_all_health().await;
        let h = health.iter().find(|h| h.name == "flaky").unwrap();
        assert!(!h.sustained_failure, "解析错误不应标记持续故障");
        assert_eq!(h.status, VendorStatus::Degraded);

        // 模拟窗口老化 → 普通故障恢复
        {
            let mut vendors = tracker.vendors.write().await;
            if let Some(h) = vendors.get_mut("flaky") {
                h.window_failures.clear();
            }
        }
        tracker.record_success("flaky").await;
        let healthy = tracker.get_healthy_vendors(&["flaky".to_string()]).await;
        assert_eq!(healthy.len(), 1, "普通故障窗口老化后应恢复");
    }
}
