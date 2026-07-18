// SPDX-License-Identifier: AGPL-3.0-only

//! 出站推送分发器（NotificationDispatcher）
//!
//! 借鉴 daily_stock_analysis 项目的多渠道推送能力，
//! 在 harness 契约层之上实现具体的分发逻辑：
//! - 渠道注册表（按 name 索引）
//! - 路由配置（route → channels 列表）
//! - 策略应用（去重 TTL / 冷却 / 静默时段 / 最低级别过滤）
//! - 并发分发 + 结果汇总
//!
//! 跳过原因分类（与 NotificationDispatchSummary 字段对应）：
//! - deduped_count: 同一 content hash 在 TTL 内重复
//! - cooldown_skipped_count: 同一 (route, channel) 在冷却期内重复
//! - quiet_hours_skipped_count: 当前时间在静默时段内
//! - severity_filtered_count: alert 级别低于 min_severity

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use axagent_harness::{
    AlertPayload, NotificationChannel, NotificationDispatchResult, NotificationDispatchSummary,
    NotificationPolicy, NotificationRoute, ReportPayload, RouteConfig,
};

/// 内容哈希（用于去重，sha256 前 16 字符 hex 编码）
fn content_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let bytes = hasher.finalize();
    hex_encode(&bytes)
}

/// 轻量 hex 编码（避免引入 hex crate 依赖）
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// 内置日志渠道（无外部依赖，用于测试和兜底）
pub struct LogChannel {
    name: String,
    display_name: String,
}

impl LogChannel {
    pub fn new(name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self { name: name.into(), display_name: display_name.into() }
    }
}

#[async_trait]
impl NotificationChannel for LogChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String> {
        tracing::info!(
            target: "axagent_notification::log_channel",
            name = %self.name,
            title = %payload.title,
            stocks_len = payload.stocks.len(),
            body_md_len = payload.body_md.len(),
            "推送报告"
        );
        Ok(format!("log-{}-{}", self.name, payload.generated_at.timestamp()))
    }

    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String> {
        tracing::warn!(
            target: "axagent_notification::log_channel",
            name = %self.name,
            severity = ?payload.severity,
            title = %payload.title,
            "推送告警"
        );
        Ok(format!("log-{}-{}", self.name, payload.generated_at.timestamp()))
    }

    async fn is_ready(&self) -> bool {
        true
    }
}

/// 策略检查结果
enum PolicyCheck {
    /// 跳过推送（已包含原因计数）
    Skipped(NotificationDispatchSummary),
    /// 通过检查，可以继续分发
    Proceed { targets: Vec<Arc<dyn NotificationChannel>>, cooldown_skipped: u32 },
}

/// 出站推送分发器
///
/// 线程安全：内部使用 RwLock + DashMap，可在多任务间共享（Arc<Dispatcher>）。
pub struct NotificationDispatcher {
    /// 已注册渠道（name → channel）
    channels: RwLock<HashMap<String, Arc<dyn NotificationChannel>>>,
    /// 路由配置（route → RouteConfig）
    routes: RwLock<HashMap<NotificationRoute, RouteConfig>>,
    /// 全局推送策略
    policy: RwLock<NotificationPolicy>,
    /// 去重缓存：content_hash → 首次推送时间
    dedup_cache: DashMap<String, DateTime<Utc>>,
    /// 冷却记录：(route, channel_name) → 上次推送时间
    cooldown_map: DashMap<(NotificationRoute, String), DateTime<Utc>>,
}

impl NotificationDispatcher {
    /// 创建空分发器（使用默认策略）
    pub fn new() -> Self {
        Self::with_policy(NotificationPolicy::default())
    }

    /// 创建分发器并指定初始策略
    pub fn with_policy(policy: NotificationPolicy) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            routes: RwLock::new(HashMap::new()),
            policy: RwLock::new(policy),
            dedup_cache: DashMap::new(),
            cooldown_map: DashMap::new(),
        }
    }

    /// 注册渠道（同名渠道会被覆盖）
    pub async fn register_channel(&self, channel: Arc<dyn NotificationChannel>) {
        let name = channel.name().to_string();
        self.channels.write().await.insert(name, channel);
    }

    /// 注销渠道
    pub async fn unregister_channel(&self, name: &str) -> Option<Arc<dyn NotificationChannel>> {
        self.channels.write().await.remove(name)
    }

    /// 列出所有已注册渠道名称
    pub async fn list_channel_names(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// 配置路由（route → channels）
    pub async fn configure_route(&self, config: RouteConfig) {
        self.routes.write().await.insert(config.route.clone(), config);
    }

    /// 移除路由配置
    pub async fn remove_route(&self, route: &NotificationRoute) -> Option<RouteConfig> {
        self.routes.write().await.remove(route)
    }

    /// 更新推送策略
    pub async fn set_policy(&self, policy: NotificationPolicy) {
        *self.policy.write().await = policy;
    }

    /// 读取当前策略快照
    pub async fn policy(&self) -> NotificationPolicy {
        self.policy.read().await.clone()
    }

    /// 清去过期去重条目（可由外部定时调用，避免内存泄漏）
    pub fn cleanup_expired_dedup(&self, now: DateTime<Utc>) {
        let ttl_secs = self.dedup_ttl_secs_blocking();
        let cutoff = now - chrono::Duration::seconds(ttl_secs as i64);
        self.dedup_cache.retain(|_, ts| *ts > cutoff);
    }

    fn dedup_ttl_secs_blocking(&self) -> u64 {
        // 用 try_read 避免在 sync 上下文中阻塞；失败则用默认 1h
        self.policy.try_read().map(|p| p.dedup_ttl_seconds).unwrap_or(3600)
    }

    /// 分发报告
    pub async fn dispatch_report(
        &self,
        payload: &ReportPayload,
        now: DateTime<Utc>,
    ) -> NotificationDispatchSummary {
        let route = NotificationRoute::Report;
        let content_key = content_hash(&format!("report:{}:{}", payload.title, payload.body_md));

        match self.check_policy_and_targets(&route, &content_key, now).await {
            PolicyCheck::Skipped(s) => s,
            PolicyCheck::Proceed { targets, cooldown_skipped } => {
                // 并发发送
                let futures: Vec<_> = targets
                    .into_iter()
                    .map(|ch| {
                        let ch_clone = Arc::clone(&ch);
                        async move { (ch, ch_clone.send_report(payload).await) }
                    })
                    .collect();
                let results_raw = futures::future::join_all(futures).await;
                self.finalize_results(route, content_key, now, cooldown_skipped, results_raw).await
            },
        }
    }

    /// 分发告警
    pub async fn dispatch_alert(
        &self,
        payload: &AlertPayload,
        now: DateTime<Utc>,
    ) -> NotificationDispatchSummary {
        // 级别过滤（仅在 Alert 路由生效）
        let policy = self.policy.read().await.clone();
        if !policy.passes_severity_filter(payload.severity) {
            return NotificationDispatchSummary {
                total: 0,
                severity_filtered_count: 1,
                ..Default::default()
            };
        }

        let route = NotificationRoute::Alert;
        let content_key = content_hash(&format!(
            "alert:{}:{}:{:?}",
            payload.title, payload.body, payload.severity
        ));

        match self.check_policy_and_targets(&route, &content_key, now).await {
            PolicyCheck::Skipped(s) => s,
            PolicyCheck::Proceed { targets, cooldown_skipped } => {
                let futures: Vec<_> = targets
                    .into_iter()
                    .map(|ch| {
                        let ch_clone = Arc::clone(&ch);
                        async move { (ch, ch_clone.send_alert(payload).await) }
                    })
                    .collect();
                let results_raw = futures::future::join_all(futures).await;
                self.finalize_results(route, content_key, now, cooldown_skipped, results_raw).await
            },
        }
    }

    /// 分发系统错误
    pub async fn dispatch_system_error(
        &self,
        title: &str,
        body: &str,
        now: DateTime<Utc>,
    ) -> NotificationDispatchSummary {
        let route = NotificationRoute::SystemError;
        let content_key = content_hash(&format!("syserr:{}:{}", title, body));

        match self.check_policy_and_targets(&route, &content_key, now).await {
            PolicyCheck::Skipped(s) => s,
            PolicyCheck::Proceed { targets, cooldown_skipped } => {
                let title_owned = title.to_string();
                let body_owned = body.to_string();
                let futures: Vec<_> = targets
                    .into_iter()
                    .map(|ch| {
                        let ch_clone = Arc::clone(&ch);
                        let t = title_owned.clone();
                        let b = body_owned.clone();
                        async move { (ch, ch_clone.send_system_error(&t, &b).await) }
                    })
                    .collect();
                let results_raw = futures::future::join_all(futures).await;
                self.finalize_results(route, content_key, now, cooldown_skipped, results_raw).await
            },
        }
    }

    /// 策略检查 + 目标渠道解析 + 冷却过滤
    async fn check_policy_and_targets(
        &self,
        route: &NotificationRoute,
        content_key: &str,
        now: DateTime<Utc>,
    ) -> PolicyCheck {
        let policy = self.policy.read().await.clone();

        // 全局开关
        if !policy.enabled {
            return PolicyCheck::Skipped(NotificationDispatchSummary {
                total: 0,
                ..Default::default()
            });
        }

        // 静默时段
        let now_time = now.naive_utc().time();
        if policy.is_in_quiet_hours(now_time) {
            return PolicyCheck::Skipped(NotificationDispatchSummary {
                total: 0,
                quiet_hours_skipped_count: 1,
                ..Default::default()
            });
        }

        // 去重检查
        if let Some(prev) = self.dedup_cache.get(content_key) {
            let ttl = chrono::Duration::seconds(policy.dedup_ttl_seconds as i64);
            if now - *prev < ttl {
                return PolicyCheck::Skipped(NotificationDispatchSummary {
                    total: 0,
                    deduped_count: 1,
                    ..Default::default()
                });
            }
        }

        // 取路由目标渠道
        let routes = self.routes.read().await;
        let route_config = match routes.get(route) {
            Some(c) => c.clone(),
            None => {
                return PolicyCheck::Skipped(NotificationDispatchSummary {
                    total: 0,
                    ..Default::default()
                });
            },
        };
        drop(routes);

        let channels = self.channels.read().await;
        let mut targets: Vec<Arc<dyn NotificationChannel>> = Vec::new();
        for name in &route_config.channels {
            if let Some(ch) = channels.get(name) {
                targets.push(Arc::clone(ch));
            } else {
                tracing::warn!(
                    target: "axagent_notification::dispatcher",
                    route = ?route,
                    channel = %name,
                    "渠道未注册，跳过"
                );
            }
        }
        drop(channels);

        if targets.is_empty() {
            return PolicyCheck::Skipped(NotificationDispatchSummary::default());
        }

        // 冷却检查（按 (route, channel_name) 维度）
        let mut cooldown_skipped: u32 = 0;
        let mut send_targets: Vec<Arc<dyn NotificationChannel>> = Vec::new();
        for ch in targets {
            let key = (route.clone(), ch.name().to_string());
            if let Some(prev) = self.cooldown_map.get(&key) {
                let cd = chrono::Duration::seconds(policy.cooldown_seconds as i64);
                let diff = now - *prev;
                if diff < cd {
                    cooldown_skipped += 1;
                    continue;
                }
            }
            send_targets.push(ch);
        }

        // 所有渠道都被冷却跳过：返回 Skipped，total 计入 cooldown_skipped
        if send_targets.is_empty() {
            return PolicyCheck::Skipped(NotificationDispatchSummary {
                total: cooldown_skipped,
                cooldown_skipped_count: cooldown_skipped,
                ..Default::default()
            });
        }

        PolicyCheck::Proceed { targets: send_targets, cooldown_skipped }
    }

    /// 汇总结果 + 更新冷却/去重缓存
    async fn finalize_results(
        &self,
        route: NotificationRoute,
        content_key: String,
        now: DateTime<Utc>,
        cooldown_skipped: u32,
        results_raw: Vec<(Arc<dyn NotificationChannel>, Result<String, String>)>,
    ) -> NotificationDispatchSummary {
        let total = (results_raw.len() as u32) + cooldown_skipped;
        let mut results = Vec::with_capacity(results_raw.len());
        let mut success_count: u32 = 0;
        let mut failure_count: u32 = 0;

        for (ch, res) in results_raw {
            let channel_name = ch.name().to_string();
            let (success, message_id, error) = match res {
                Ok(id) => {
                    success_count += 1;
                    (true, Some(id), None)
                },
                Err(e) => {
                    failure_count += 1;
                    (false, None, Some(e))
                },
            };
            if success {
                self.cooldown_map.insert((route.clone(), channel_name.clone()), now);
            }
            results.push(NotificationDispatchResult {
                channel: channel_name,
                success,
                message_id,
                error,
                timestamp: now,
            });
        }

        // 更新去重缓存（仅当至少有一个成功推送时）
        if success_count > 0 {
            self.dedup_cache.insert(content_key, now);
        }

        NotificationDispatchSummary {
            total,
            success_count,
            failure_count,
            deduped_count: 0,
            cooldown_skipped_count: cooldown_skipped,
            quiet_hours_skipped_count: 0,
            severity_filtered_count: 0,
            results,
        }
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::{AlertSeverity, ReportStockSummary};

    fn make_report(title: &str, body: &str) -> ReportPayload {
        ReportPayload {
            title: title.to_string(),
            body_md: body.to_string(),
            body_html: None,
            stocks: vec![ReportStockSummary {
                stock_code: "600519".to_string(),
                stock_name: "贵州茅台".to_string(),
                action: "增持".to_string(),
                score: 85,
                confidence: 0.82,
            }],
            generated_at: Utc::now(),
        }
    }

    fn make_alert(severity: AlertSeverity, title: &str) -> AlertPayload {
        AlertPayload {
            severity,
            title: title.to_string(),
            body: "test body".to_string(),
            stock_code: Some("600519".to_string()),
            generated_at: Utc::now(),
        }
    }

    fn utc_at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        let naive =
            chrono::NaiveDate::from_ymd_opt(y, mo, d).unwrap().and_hms_opt(h, mi, 0).unwrap();
        DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
    }

    #[tokio::test]
    async fn test_register_and_dispatch_report() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        let summary = dispatcher.dispatch_report(&make_report("t1", "b1"), now).await;
        assert!(summary.is_all_success());
        assert_eq!(summary.total, 1);
        assert_eq!(summary.success_count, 1);
    }

    #[tokio::test]
    async fn test_dedup_same_content() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        let s1 = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s1.success_count, 1);

        // 同一内容立刻再推 → 去重跳过
        let s2 = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s2.deduped_count, 1);
        assert_eq!(s2.success_count, 0);
    }

    #[tokio::test]
    async fn test_cooldown_skipped() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        // 首次推送（不同内容避开去重）
        let s1 = dispatcher.dispatch_report(&make_report("t1", "b1"), now).await;
        assert_eq!(s1.success_count, 1);

        // 立刻再推不同内容 → 去重不命中，但冷却命中
        let s2 = dispatcher.dispatch_report(&make_report("t2", "b2"), now).await;
        assert_eq!(s2.cooldown_skipped_count, 1);
        assert_eq!(s2.success_count, 0);
        assert_eq!(s2.total, 1);
    }

    #[tokio::test]
    async fn test_severity_filter() {
        let policy =
            NotificationPolicy { min_severity: AlertSeverity::Error, ..Default::default() };
        let dispatcher = NotificationDispatcher::with_policy(policy);
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Alert,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        // Info 级别被过滤
        let s1 = dispatcher.dispatch_alert(&make_alert(AlertSeverity::Info, "low"), now).await;
        assert_eq!(s1.severity_filtered_count, 1);

        // Critical 级别通过
        let s2 = dispatcher.dispatch_alert(&make_alert(AlertSeverity::Critical, "high"), now).await;
        assert_eq!(s2.success_count, 1);
    }

    #[tokio::test]
    async fn test_quiet_hours_skip() {
        let policy = NotificationPolicy {
            quiet_hours_start: Some(chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap()),
            quiet_hours_end: Some(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap()),
            ..Default::default()
        };
        let dispatcher = NotificationDispatcher::with_policy(policy);
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log".to_string()],
            })
            .await;

        // 构造 UTC 13:30 时刻（naive_utc().time() 取的就是 UTC 时间部分）
        let now = utc_at(2024, 1, 1, 13, 30);

        let s = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s.quiet_hours_skipped_count, 1);
        assert_eq!(s.success_count, 0);
    }

    #[tokio::test]
    async fn test_disabled_policy() {
        let policy = NotificationPolicy { enabled: false, ..Default::default() };
        let dispatcher = NotificationDispatcher::with_policy(policy);
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        let s = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s.total, 0);
        assert_eq!(s.success_count, 0);
    }

    #[tokio::test]
    async fn test_no_route_configured() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        // 不配置路由
        let now = Utc::now();
        let s = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s.total, 0);
    }

    #[tokio::test]
    async fn test_multi_channel_dispatch() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log1", "日志1"))).await;
        dispatcher.register_channel(Arc::new(LogChannel::new("log2", "日志2"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::Report,
                channels: vec!["log1".to_string(), "log2".to_string()],
            })
            .await;

        let now = Utc::now();
        let s = dispatcher.dispatch_report(&make_report("t", "b"), now).await;
        assert_eq!(s.total, 2);
        assert_eq!(s.success_count, 2);
        assert!(s.is_all_success());
    }

    #[tokio::test]
    async fn test_unregister_channel() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        assert_eq!(dispatcher.list_channel_names().await.len(), 1);

        let removed = dispatcher.unregister_channel("log").await;
        assert!(removed.is_some());
        assert_eq!(dispatcher.list_channel_names().await.len(), 0);
    }

    #[tokio::test]
    async fn test_system_error_dispatch() {
        let dispatcher = NotificationDispatcher::new();
        dispatcher.register_channel(Arc::new(LogChannel::new("log", "日志"))).await;
        dispatcher
            .configure_route(RouteConfig {
                route: NotificationRoute::SystemError,
                channels: vec!["log".to_string()],
            })
            .await;

        let now = Utc::now();
        let s = dispatcher.dispatch_system_error("后端崩溃", "数据源全部失败", now).await;
        assert_eq!(s.total, 1);
        assert_eq!(s.success_count, 1);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let h1 = content_hash("test");
        let h2 = content_hash("test");
        assert_eq!(h1, h2);
        assert_ne!(h1, content_hash("other"));
        // hex 编码长度 = 64 字符（sha256 32 字节）
        assert_eq!(h1.len(), 64);
    }
}
