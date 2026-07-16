// SPDX-License-Identifier: AGPL-3.0-only

//! 高级股票数据服务契约 — 让 stock-analysis 等消费者通过 trait 调用，
//! 无需直接依赖 `axagent-astock-data`。
//!
//! 涵盖 `MarketDataProvider`（基础行情）之外的扩展能力：
//! 财务报告、技术指标、交易日历、市场类型识别等。

use async_trait::async_trait;
use serde_json::Value;

use axagent_harness::core_error::Result;
use axagent_harness::market_data::{FinancialReport, KLine, MarketDataProvider};

/// 高级股票数据服务 trait
///
/// 实现方：`axagent-astock-data` 的 `AStockClient`
/// 消费者：`axagent-stock-analysis`、工具系统
#[async_trait]
pub trait StockDataService: MarketDataProvider + Send + Sync {
    /// 获取财务报告
    async fn get_financial_report(&self, stock_code: &str) -> Result<FinancialReport>;

    /// 计算技术指标（基于 KLine 数据）
    async fn compute_technical_indicators(
        &self,
        stock_code: &str,
        klines: &[KLine],
    ) -> Result<Value>;

    /// 判断是否为交易日
    async fn is_trading_day(&self, date: &str) -> Result<bool>;

    /// 最近交易日（如果是非交易日，则返回前一个交易日）
    async fn latest_trading_day(&self, date: &str) -> Result<String>;
}
