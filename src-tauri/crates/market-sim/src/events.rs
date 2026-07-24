//! 外部事件 DTO + 事件注入接口（P2-C9）
//!
//! market-sim 是 consumer，按 AGENTS.md 铁律不能依赖 astock-data（implementor）。
//! 因此本模块定义轻量级事件 DTO，由 wiring 层（commands/init）负责从 astock-data
//! 的 NewsItem / Announcement / EarningsEvent 转换为 ExternalEvent。
//!
//! ## 事件类型
//!
//! - `News` — 新闻事件（含情感分数）
//! - `Announcement` — 公告事件（如减持、回购、股权激励）
//! - `Earnings` — 财报事件（超预期/不及预期）
//! - `MarketShock` — 市场冲击事件（复用 oracle.rs 的 MarketEvent 语义）
//!
//! ## 注入流程
//!
//! ```text
//! astock-data::get_news() → Vec<NewsItem>
//!     ↓ wiring 层转换
//! ExternalEvent { kind: News, sentiment, impact, ... }
//!     ↓ kernel.inject_event()
//! SimEvent { scheduled_at, message: ExternalEvent(_) }
//!     ↓ 广播给所有 Agent
//! EventDrivenAgent::on_message(ExternalEvent) → 交易决策
//! ```

use serde::{Deserialize, Serialize};

use crate::types::SimTimestamp;

// ── 事件类型枚举 ──

/// 外部事件类别
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternalEventKind {
    /// 新闻事件（财经新闻、政策新闻、社交舆情）
    News,
    /// 公告事件（减持、回购、股权激励、重大合同等）
    Announcement,
    /// 财报事件（业绩预增/预亏、正式财报披露）
    Earnings,
    /// 市场冲击事件（价格冲击、成交量异常，复用 oracle 语义）
    MarketShock,
}

impl ExternalEventKind {
    pub fn as_str(&self) -> &str {
        match self {
            ExternalEventKind::News => "news",
            ExternalEventKind::Announcement => "announcement",
            ExternalEventKind::Earnings => "earnings",
            ExternalEventKind::MarketShock => "market_shock",
        }
    }
}

impl std::fmt::Display for ExternalEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── 外部事件 DTO ──

/// 外部事件数据（轻量级，不依赖 astock-data）
///
/// 由 wiring 层从 astock-data 的 NewsItem / Announcement / EarningsEvent
/// 转换而来，注入到 SimKernel 的事件队列中。
///
/// # 字段语义
///
/// - `sentiment`: 情感分数 [-1.0, 1.0]，正数利好，负数利空
/// - `impact`: 影响强度 [0.0, 1.0]，0 = 无影响，1 = 极强影响
/// - `scheduled_at`: 事件触发时间（模拟时间戳 ns）
/// - `stock_code`: 关联股票代码，None 表示市场级事件（影响所有股票）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalEvent {
    /// 事件类别
    pub kind: ExternalEventKind,
    /// 事件标题
    pub title: String,
    /// 事件摘要
    pub summary: String,
    /// 情感分数 [-1.0, 1.0]
    pub sentiment: f64,
    /// 影响强度 [0.0, 1.0]
    pub impact: f64,
    /// 事件触发时间（模拟时间戳 ns）
    pub scheduled_at: SimTimestamp,
    /// 关联股票代码（None = 市场级事件）
    pub stock_code: Option<String>,
}

impl ExternalEvent {
    /// 创建新闻事件
    pub fn news(
        title: impl Into<String>,
        summary: impl Into<String>,
        sentiment: f64,
        impact: f64,
        scheduled_at: SimTimestamp,
        stock_code: Option<String>,
    ) -> Self {
        Self {
            kind: ExternalEventKind::News,
            title: title.into(),
            summary: summary.into(),
            sentiment: sentiment.clamp(-1.0, 1.0),
            impact: impact.clamp(0.0, 1.0),
            scheduled_at,
            stock_code,
        }
    }

    /// 创建公告事件
    pub fn announcement(
        title: impl Into<String>,
        summary: impl Into<String>,
        sentiment: f64,
        impact: f64,
        scheduled_at: SimTimestamp,
        stock_code: Option<String>,
    ) -> Self {
        Self {
            kind: ExternalEventKind::Announcement,
            title: title.into(),
            summary: summary.into(),
            sentiment: sentiment.clamp(-1.0, 1.0),
            impact: impact.clamp(0.0, 1.0),
            scheduled_at,
            stock_code,
        }
    }

    /// 创建财报事件
    pub fn earnings(
        title: impl Into<String>,
        summary: impl Into<String>,
        sentiment: f64,
        impact: f64,
        scheduled_at: SimTimestamp,
        stock_code: Option<String>,
    ) -> Self {
        Self {
            kind: ExternalEventKind::Earnings,
            title: title.into(),
            summary: summary.into(),
            sentiment: sentiment.clamp(-1.0, 1.0),
            impact: impact.clamp(0.0, 1.0),
            scheduled_at,
            stock_code,
        }
    }

    /// 创建市场冲击事件
    pub fn market_shock(
        title: impl Into<String>,
        summary: impl Into<String>,
        sentiment: f64,
        impact: f64,
        scheduled_at: SimTimestamp,
    ) -> Self {
        Self {
            kind: ExternalEventKind::MarketShock,
            title: title.into(),
            summary: summary.into(),
            sentiment: sentiment.clamp(-1.0, 1.0),
            impact: impact.clamp(0.0, 1.0),
            scheduled_at,
            stock_code: None, // 市场级事件
        }
    }

    /// 是否为利好事件（sentiment > 0）
    pub fn is_positive(&self) -> bool {
        self.sentiment > 0.0
    }

    /// 是否为利空事件（sentiment < 0）
    pub fn is_negative(&self) -> bool {
        self.sentiment < 0.0
    }

    /// 是否为强影响事件（impact >= 0.5）
    pub fn is_high_impact(&self) -> bool {
        self.impact >= 0.5
    }

    /// 综合信号强度 = |sentiment| * impact ∈ [0, 1]
    ///
    /// 用于 EventDrivenAgent 判断是否触发交易
    pub fn signal_strength(&self) -> f64 {
        self.sentiment.abs() * self.impact
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_news_event_creation() {
        let event = ExternalEvent::news(
            "某公司业绩预增",
            "净利润同比增长50%",
            0.8,
            0.7,
            1_000_000_000,
            Some("000001".to_string()),
        );
        assert_eq!(event.kind, ExternalEventKind::News);
        assert!(event.is_positive());
        assert!(event.is_high_impact());
        assert!((event.signal_strength() - 0.56).abs() < 1e-10);
    }

    #[test]
    fn test_announcement_event_creation() {
        let event = ExternalEvent::announcement(
            "股东减持公告",
            "大股东拟减持不超过3%",
            -0.5,
            0.6,
            2_000_000_000,
            Some("000002".to_string()),
        );
        assert_eq!(event.kind, ExternalEventKind::Announcement);
        assert!(event.is_negative());
        assert!(event.is_high_impact());
        assert!((event.signal_strength() - 0.30).abs() < 1e-10);
    }

    #[test]
    fn test_earnings_event_creation() {
        let event = ExternalEvent::earnings(
            "财报超预期",
            "营收超预期20%",
            0.9,
            0.9,
            3_000_000_000,
            Some("600000".to_string()),
        );
        assert_eq!(event.kind, ExternalEventKind::Earnings);
        assert!(event.is_positive());
        assert!(event.is_high_impact());
    }

    #[test]
    fn test_market_shock_event_creation() {
        let event =
            ExternalEvent::market_shock("市场冲击", "突发利好消息", 0.6, 0.8, 5_000_000_000);
        assert_eq!(event.kind, ExternalEventKind::MarketShock);
        assert_eq!(event.stock_code, None);
        assert!(event.is_positive());
    }

    #[test]
    fn test_sentiment_clamped() {
        let event = ExternalEvent::news("test", "", 2.0, 0.5, 0, None);
        assert_eq!(event.sentiment, 1.0);
        let event = ExternalEvent::news("test", "", -2.0, 0.5, 0, None);
        assert_eq!(event.sentiment, -1.0);
    }

    #[test]
    fn test_impact_clamped() {
        let event = ExternalEvent::news("test", "", 0.5, 2.0, 0, None);
        assert_eq!(event.impact, 1.0);
        let event = ExternalEvent::news("test", "", 0.5, -1.0, 0, None);
        assert_eq!(event.impact, 0.0);
    }

    #[test]
    fn test_signal_strength() {
        let event = ExternalEvent::news("test", "", 0.5, 0.4, 0, None);
        assert!((event.signal_strength() - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_is_positive_negative() {
        let positive = ExternalEvent::news("利好", "", 0.1, 0.1, 0, None);
        assert!(positive.is_positive());
        assert!(!positive.is_negative());

        let negative = ExternalEvent::news("利空", "", -0.1, 0.1, 0, None);
        assert!(!negative.is_positive());
        assert!(negative.is_negative());

        let neutral = ExternalEvent::news("中性", "", 0.0, 0.1, 0, None);
        assert!(!neutral.is_positive());
        assert!(!neutral.is_negative());
    }

    #[test]
    fn test_kind_as_str() {
        assert_eq!(ExternalEventKind::News.as_str(), "news");
        assert_eq!(ExternalEventKind::Announcement.as_str(), "announcement");
        assert_eq!(ExternalEventKind::Earnings.as_str(), "earnings");
        assert_eq!(ExternalEventKind::MarketShock.as_str(), "market_shock");
    }
}
