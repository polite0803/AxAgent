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

// ── MarketDataProvider Trait ─────────────────────────────────────────────

/// 市场数据提供者接口
///
/// 实现方：`axagent-astock-data` 的 `AStockClient`
/// 消费者：`quant`、`gateway`、`tools`、`stock-analysis`
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
