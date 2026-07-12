// SPDX-License-Identifier: AGPL-3.0-only

//! 市场数据契约层 — 纯 DTO + Trait 抽象
//!
//! 让 `quant` / `gateway` 等消费者通过 trait 调用数据源，
//! 无需直接依赖 `axagent-astock-data` 实现。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;

// ── DTOs ─────────────────────────────────────────────────────────────────

/// 实时行情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockQuote {
    pub code: String,
    pub name: String,
    pub price: f64,
    /// 昨收价
    pub pre_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    pub amount: f64,
    pub change_pct: f64,
    pub turnover_rate: f64,
    pub pe: Option<f64>,
    pub pb: Option<f64>,
    pub total_mv: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub circulating_mv: Option<f64>,
    /// 涨停价
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_up: Option<f64>,
    /// 跌停价
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_down: Option<f64>,
    /// 是否ST股票（含*ST）
    #[serde(default)]
    pub is_st: bool,
    pub timestamp: String,
}

/// K线数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KLine {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub amount: f64,
    pub turnover_rate: Option<f64>,
    /// 累计复权因子 (R3-A); None 表示未应用复权
    #[serde(default)]
    pub adj_factor: Option<f64>,
}

/// 复权类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AdjType {
    None,
    #[default]
    Forward,
    Backward,
}

/// 股票搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSearchResult {
    pub code: String,
    pub name: String,
    pub market: String,
}

/// 财务报告 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReport {
    pub stock_code: String,
    pub report_date: String,
    pub revenue: Option<f64>,
    pub net_profit: Option<f64>,
    pub eps: Option<f64>,
    pub bps: Option<f64>,
    pub roe: Option<f64>,
    pub debt_ratio: Option<f64>,
    pub gross_margin: Option<f64>,
    pub net_margin: Option<f64>,
    pub revenue_yoy: Option<f64>,
    pub profit_yoy: Option<f64>,
    #[serde(default)]
    pub total_assets: Option<f64>,
    #[serde(default)]
    pub operating_cash_flow: Option<f64>,
    #[serde(default)]
    pub capital_expenditure: Option<f64>,
    #[serde(default)]
    pub free_cash_flow: Option<f64>,
    #[serde(default)]
    pub current_ratio: Option<f64>,
    #[serde(default)]
    pub quick_ratio: Option<f64>,
}

impl FinancialReport {
    /// 检查该记录是否包含有效的核心财务数据
    pub fn has_valid_data(&self) -> bool {
        self.revenue.is_some()
            || self.net_profit.is_some()
            || self.eps.is_some()
            || self.bps.is_some()
            || self.roe.is_some()
            || self.debt_ratio.is_some()
            || self.gross_margin.is_some()
            || self.net_margin.is_some()
            || self.revenue_yoy.is_some()
            || self.profit_yoy.is_some()
    }
}

// ── MarketDataProvider Trait ─────────────────────────────────────────────

/// 市场数据提供者接口
///
/// 实现方：`axagent-astock-data` 的 `AStockClient`
/// 业务实现层（implementor）：`stock-analysis`（依赖 astock-data + dao + entities，属 implementor，非 consumer）
/// 消费者（consumer）：`quant`、`gateway`、`tools`
#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// 获取实时行情（含涨跌停价、ST标记）
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote>;

    /// 获取K线数据
    ///
    /// - `adj_type`: `Some(Forward)` 前复权 / `Some(Backward)` 后复权 / `None` 不复权
    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj_type: Option<AdjType>,
    ) -> Result<Vec<KLine>>;

    /// 搜索股票
    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>>;
}

// ── A 股市场工具函数 ────────────────────────────────────────────

/// 根据股票代码前缀识别市场板块
pub fn detect_market_type(code: &str) -> &str {
    match code.chars().next() {
        Some('6') if code.starts_with("688") => "star",
        Some('6') => "main_sh",
        Some('0') => "main_sz",
        Some('3') => "chinext",
        Some('8') => "bj",
        Some('4') => "neeq",
        Some('9') => "b_share",
        _ => "unknown",
    }
}

/// 获取A股各板块涨跌停幅度（百分比）
pub fn get_price_limit_pct(market_type: &str) -> f64 {
    match market_type {
        "star" | "chinext" => 20.0,
        "bj" => 30.0,
        _ => 10.0,
    }
}

/// 获取ST股票的涨跌停幅度
pub fn get_st_price_limit_pct(is_st: bool, market_type: &str) -> f64 {
    if is_st {
        5.0
    } else {
        get_price_limit_pct(market_type)
    }
}
