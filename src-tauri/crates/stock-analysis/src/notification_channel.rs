// SPDX-License-Identifier: AGPL-3.0-only

//! 出站推送通知渠道契约层
//!
//! 借鉴 daily_stock_analysis 项目的多渠道推送能力，
//! 定义统一的出站推送抽象（report / alert / system_error 三类路由）。
//!
//! 与 `channel_adapter::ChannelAdapter`（IM 消息收发）的区别：
//! - `ChannelAdapter` 面向双向 IM 通信（收消息 + 回复消息）
//! - `NotificationChannel` 面向单向出站推送（报告推送 + 告警推送）
//!
//! 实现 layer（notification crate）可以同时实现两个 trait，
//! 复用 HTTP 客户端基础设施。

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

// ── 推送载荷 DTO ──

/// 报告推送载荷（对应 DSA 的决策仪表盘推送）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportPayload {
    /// 报告标题（如 "贵州茅台(600519) 决策仪表盘"）
    pub title: String,
    /// Markdown 正文
    pub body_md: String,
    /// HTML 正文（可选，用于邮件渠道）
    pub body_html: Option<String>,
    /// 股票摘要列表（多只股票汇总时使用）
    pub stocks: Vec<ReportStockSummary>,
    /// 生成时间戳
    pub generated_at: DateTime<Utc>,
}

/// 报告中的股票摘要
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportStockSummary {
    pub stock_code: String,
    pub stock_name: String,
    pub action: String,
    pub score: u32,
    pub confidence: f64,
}

/// 告警推送载荷（对应 DSA 的 alert 路由）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlertPayload {
    /// 告警严重级别：info / warning / error / critical
    pub severity: AlertSeverity,
    /// 告警标题
    pub title: String,
    /// 告警正文
    pub body: String,
    /// 关联股票代码（可选）
    pub stock_code: Option<String>,
    /// 生成时间戳
    pub generated_at: DateTime<Utc>,
}

/// 告警严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertSeverity {
    /// 转为数字权重，用于 min_severity 过滤
    pub fn weight(&self) -> u8 {
        match self {
            AlertSeverity::Info => 0,
            AlertSeverity::Warning => 1,
            AlertSeverity::Error => 2,
            AlertSeverity::Critical => 3,
        }
    }

    /// 从权重值还原
    pub fn from_weight(w: u8) -> Self {
        match w {
            0 => AlertSeverity::Info,
            1 => AlertSeverity::Warning,
            2 => AlertSeverity::Error,
            _ => AlertSeverity::Critical,
        }
    }
}

// ── 推送路由 ──

/// 推送路由类型（借鉴 DSA 的三类路由）
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum NotificationRoute {
    /// 决策报告推送（每日荐股、仪表盘汇总）
    #[default]
    Report,
    /// 告警推送（价格突破、风险触发、系统异常）
    Alert,
    /// 系统错误推送（后台崩溃、数据源全部失败）
    SystemError,
}

/// 路由配置：某类路由推送到哪些渠道
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfig {
    /// 路由类型
    pub route: NotificationRoute,
    /// 目标渠道名称列表（如 ["feishu", "telegram", "email"]）
    pub channels: Vec<String>,
}

// ── 推送策略 ──

/// 推送策略（借鉴 DSA 的去重 / 冷却 / 静默 / 最低级别过滤）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPolicy {
    /// 去重 TTL（秒）：同一 content hash 在 TTL 内不重复推送
    pub dedup_ttl_seconds: u64,
    /// 冷却时间（秒）：同一渠道同一路由在冷却期内不重复推送
    pub cooldown_seconds: u64,
    /// 静默时段开始时间（如 22:00）；None 表示无静默
    pub quiet_hours_start: Option<NaiveTime>,
    /// 静默时段结束时间（如 08:00）；None 表示无静默
    pub quiet_hours_end: Option<NaiveTime>,
    /// 最低推送级别：低于此级别的 alert 不推送（仅对 Alert 路由生效）
    pub min_severity: AlertSeverity,
    /// 是否启用（全局开关）
    pub enabled: bool,
}

impl Default for NotificationPolicy {
    fn default() -> Self {
        Self {
            dedup_ttl_seconds: 3600, // 1 小时去重
            cooldown_seconds: 300,   // 5 分钟冷却
            quiet_hours_start: None, // 默认无静默
            quiet_hours_end: None,
            min_severity: AlertSeverity::Info, // 默认全部推送
            enabled: true,
        }
    }
}

impl NotificationPolicy {
    /// 判断当前时间是否在静默时段内
    ///
    /// 支持跨午夜：如 22:00-08:00 表示晚 10 点到次日早 8 点
    pub fn is_in_quiet_hours(&self, now: NaiveTime) -> bool {
        match (self.quiet_hours_start, self.quiet_hours_end) {
            (Some(start), Some(end)) => {
                if start <= end {
                    // 不跨午夜：如 09:00-12:00
                    now >= start && now < end
                } else {
                    // 跨午夜：如 22:00-08:00
                    now >= start || now < end
                }
            },
            _ => false,
        }
    }

    /// 判断告警级别是否达到最低推送门槛
    pub fn passes_severity_filter(&self, severity: AlertSeverity) -> bool {
        severity.weight() >= self.min_severity.weight()
    }
}

// ── 推送渠道 trait ──

/// 出站推送通知渠道 trait
///
/// 每个推送渠道（飞书 / Telegram / 邮件 / PushPlus 等）实现此 trait，
/// 注册到 NotificationDispatcher 后按路由分发。
///
/// 与 ChannelAdapter 的关系：
/// - IM 类渠道（飞书/Telegram/Discord 等）可同时实现 ChannelAdapter + NotificationChannel
/// - 非 IM 类渠道（邮件/PushPlus/Server酱/ntfy/Gotify/Webhook）只实现 NotificationChannel
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// 渠道标识名称（如 "feishu"、"telegram"、"email"）
    fn name(&self) -> &str;

    /// 渠道显示名称（如 "飞书"、"Telegram"、"邮件"）
    fn display_name(&self) -> &str;

    /// 推送决策报告
    ///
    /// 实现方应将 ReportPayload 转换为渠道特定格式后发送。
    /// 返回渠道侧消息 ID（如有）。
    async fn send_report(&self, payload: &ReportPayload) -> Result<String, String>;

    /// 推送告警
    ///
    /// 实现方应根据 AlertPayload.severity 决定格式（如 critical 用红色标记）。
    async fn send_alert(&self, payload: &AlertPayload) -> Result<String, String>;

    /// 推送系统错误
    ///
    /// 默认实现：复用 send_alert，severity=Critical。
    async fn send_system_error(&self, title: &str, body: &str) -> Result<String, String> {
        let payload = AlertPayload {
            severity: AlertSeverity::Critical,
            title: title.to_string(),
            body: body.to_string(),
            stock_code: None,
            generated_at: Utc::now(),
        };
        self.send_alert(&payload).await
    }

    /// 渠道是否就绪（配置完整、连接正常）
    ///
    /// Dispatcher 在推送前检查，跳过未就绪的渠道。
    async fn is_ready(&self) -> bool {
        true
    }
}

// ── 推送结果 ──
//
// 注意：本模块的 `NotificationDispatchResult` / `NotificationDispatchSummary`
// 与 `webhook_subscription::DispatchResult`（批量 webhook 推送统计，含 success_count/failure_count/errors）
// 语义不同 —— 后者面向"一批 webhook 订阅的聚合统计"，
// 本模块面向"出站通知单渠道/汇总结果"（含 channel/message_id/timestamp/跳过原因分类）。
// 二者保持独立，避免语义混淆。

/// 单次推送结果（一个渠道一次推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDispatchResult {
    /// 渠道名称
    pub channel: String,
    /// 是否成功
    pub success: bool,
    /// 渠道侧消息 ID（成功时）
    pub message_id: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 推送时间戳
    pub timestamp: DateTime<Utc>,
}

/// 批量推送结果汇总（一次 dispatch 调用跨多渠道的聚合结果）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDispatchSummary {
    /// 总渠道数
    pub total: u32,
    /// 成功数
    pub success_count: u32,
    /// 失败数
    pub failure_count: u32,
    /// 被去重跳过的数量
    pub deduped_count: u32,
    /// 被冷却跳过的数量
    pub cooldown_skipped_count: u32,
    /// 被静默跳过的数量
    pub quiet_hours_skipped_count: u32,
    /// 被级别过滤跳过的数量
    pub severity_filtered_count: u32,
    /// 各渠道详细结果
    pub results: Vec<NotificationDispatchResult>,
}

impl NotificationDispatchSummary {
    /// 是否全部成功
    pub fn is_all_success(&self) -> bool {
        self.failure_count == 0 && self.total > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_hours_non_crossing() {
        let policy = NotificationPolicy {
            quiet_hours_start: Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            quiet_hours_end: Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
            ..Default::default()
        };
        // 在静默时段内
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(10, 0, 0).unwrap()));
        // 在静默时段外
        assert!(!policy.is_in_quiet_hours(NaiveTime::from_hms_opt(13, 0, 0).unwrap()));
        // 边界：start 包含，end 不包含
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(9, 0, 0).unwrap()));
        assert!(!policy.is_in_quiet_hours(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
    }

    #[test]
    fn test_quiet_hours_crossing_midnight() {
        let policy = NotificationPolicy {
            quiet_hours_start: Some(NaiveTime::from_hms_opt(22, 0, 0).unwrap()),
            quiet_hours_end: Some(NaiveTime::from_hms_opt(8, 0, 0).unwrap()),
            ..Default::default()
        };
        // 跨午夜：晚 10 点到次日早 8 点
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(23, 0, 0).unwrap()));
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(2, 0, 0).unwrap()));
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(7, 59, 0).unwrap()));
        // 不在静默时段
        assert!(!policy.is_in_quiet_hours(NaiveTime::from_hms_opt(8, 0, 0).unwrap()));
        assert!(!policy.is_in_quiet_hours(NaiveTime::from_hms_opt(15, 0, 0).unwrap()));
        assert!(policy.is_in_quiet_hours(NaiveTime::from_hms_opt(22, 0, 0).unwrap()));
    }

    #[test]
    fn test_quiet_hours_none() {
        let policy = NotificationPolicy::default();
        assert!(!policy.is_in_quiet_hours(NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
    }

    #[test]
    fn test_severity_filter() {
        let policy =
            NotificationPolicy { min_severity: AlertSeverity::Warning, ..Default::default() };
        assert!(!policy.passes_severity_filter(AlertSeverity::Info));
        assert!(policy.passes_severity_filter(AlertSeverity::Warning));
        assert!(policy.passes_severity_filter(AlertSeverity::Error));
        assert!(policy.passes_severity_filter(AlertSeverity::Critical));
    }

    #[test]
    fn test_severity_weight_ordering() {
        assert!(AlertSeverity::Info.weight() < AlertSeverity::Warning.weight());
        assert!(AlertSeverity::Warning.weight() < AlertSeverity::Error.weight());
        assert!(AlertSeverity::Error.weight() < AlertSeverity::Critical.weight());
    }

    #[test]
    fn test_dispatch_summary_all_success() {
        let summary = NotificationDispatchSummary {
            total: 3,
            success_count: 3,
            failure_count: 0,
            deduped_count: 0,
            cooldown_skipped_count: 0,
            quiet_hours_skipped_count: 0,
            severity_filtered_count: 0,
            results: vec![],
        };
        assert!(summary.is_all_success());
    }

    #[test]
    fn test_dispatch_summary_has_failure() {
        let summary = NotificationDispatchSummary {
            total: 3,
            success_count: 2,
            failure_count: 1,
            deduped_count: 0,
            cooldown_skipped_count: 0,
            quiet_hours_skipped_count: 0,
            severity_filtered_count: 0,
            results: vec![],
        };
        assert!(!summary.is_all_success());
    }

    #[test]
    fn test_default_policy_values() {
        let policy = NotificationPolicy::default();
        assert_eq!(policy.dedup_ttl_seconds, 3600);
        assert_eq!(policy.cooldown_seconds, 300);
        assert!(policy.enabled);
        assert_eq!(policy.min_severity, AlertSeverity::Info);
    }
}
