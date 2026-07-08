//! K 线数据接入（M2 基础设施）
//!
//! 职责：
//! 1. 拉取 A 股 K 线（通过 MarketDataProvider trait）
//! 2. 拉取对应标的实时行情 `get_quote`（含涨跌停价 + ST 标记）
//! 3. 合并 KLine + StockQuote → 量化 `Bar`（`Bar::from_kline_with_quote`）
//! 4. 按日期区间过滤
//!
//! 设计原则：
//! - **纯函数优先**：`klines_to_bars` 与 `filter_bars_by_date` 不涉及 IO，
//!   单元测试可直接覆盖字段映射与日期边界。
//! - **降级策略**：`get_quote` 失败时降级为 `Bar::from_kline`（涨跌停/None, is_st=false），
//!   撮合器仍可跑（涨跌停分支跳过即可），不阻塞回测。
//! - **数据契约**：`start_date <= bar.date <= end_date`（含端点），与 `quant_backtest_run`
//!   的区间一致。

use axagent_harness::market_data::{AdjType, MarketDataProvider, StockQuote};

use crate::error::QuantError;
use crate::types::Bar;

/// 默认拉取上限（约 2 年交易日），调用方未指定时使用
pub const DEFAULT_KLINE_LIMIT: u32 = 504;

/// 拉取 K 线 + Quote → `Vec<Bar>`，按 `[start_date, end_date]` 过滤（闭区间）
///
/// # 参数
/// - `client`: 共享 `MarketDataProvider`（来自 AppState）
/// - `code`: A 股代码（"600519" / "000001" / "300750" 等）
/// - `start_date` / `end_date`: 形如 `"YYYY-MM-DD"`，含端点
/// - `limit`: 拉取上限，1 年 ~ 252 行，2 年 ~ 504
///
/// # 错误
/// - K 线拉取失败 → `QuantError::Data`
/// - 日期区间内无任何 K 线 → `QuantError::Data`
/// - Quote 拉取失败 → 降级为 `Bar::from_kline`（涨跌停 None, is_st=false），不回错
///
/// # 示例
/// ```no_run
/// # use axagent_quant::kline_provider::load_bars_with_quote;
/// # use axagent_harness::market_data::MarketDataProvider;
/// # async fn demo(client: &dyn MarketDataProvider) -> Result<(), axagent_quant::QuantError> {
/// let bars = load_bars_with_quote(client, "600519", "2023-01-01", "2024-12-31", 504).await?;
/// assert!(!bars.is_empty());
/// # Ok(())
/// # }
/// ```
pub async fn load_bars_with_quote(
    client: &dyn MarketDataProvider,
    code: &str,
    start_date: &str,
    end_date: &str,
    limit: u32,
) -> Result<Vec<Bar>, QuantError> {
    // 1) 并发拉 K 线 + Quote（Quote 失败不致命，klines 失败致命）
    let klines_fut = client.get_klines(code, "daily", limit, Some(AdjType::Forward));
    let quote_fut = client.get_quote(code);
    let (klines_res, quote_res) = tokio::join!(klines_fut, quote_fut);

    let klines =
        klines_res.map_err(|e| QuantError::Data(format!("K 线拉取失败 code={code}: {e}")))?;
    if klines.is_empty() {
        return Err(QuantError::Data(format!("股票 {code} 无 K 线数据（limit={limit}）")));
    }

    // 2) Quote 降级：失败/为空时涨跌停用 None（撮合器会跳过涨跌停分支）
    let quote_opt: Option<StockQuote> = match quote_res {
        Ok(q) if q.price > 0.0 => Some(q),
        Ok(_) => {
            tracing::warn!("[kline_provider] {code} quote 为空，涨跌停降级为 None");
            None
        },
        Err(e) => {
            tracing::warn!("[kline_provider] {code} quote 拉取失败: {e}，涨跌停降级为 None");
            None
        },
    };

    // 3) KLine + Quote → Bar，再按日期过滤
    let bars = klines_to_bars(code, &klines, quote_opt.as_ref());
    let filtered = filter_bars_by_date(bars, start_date, end_date);

    if filtered.is_empty() {
        return Err(QuantError::Data(format!(
            "区间 {start_date} ~ {end_date} 无 K 线（实际拉取 {klines_len} 行, 首日 {first}, 末日 {last}）",
            klines_len = klines.len(),
            first = klines.first().map(|k| k.date.as_str()).unwrap_or("?"),
            last = klines.last().map(|k| k.date.as_str()).unwrap_or("?"),
        )));
    }
    Ok(filtered)
}

/// KLine 列表 → Bar 列表（纯函数，可单测）
///
/// - 有 quote：使用 `Bar::from_kline_with_quote`（含涨跌停 + ST 标记 + 优先用 quote.amount）
/// - 无 quote：使用 `Bar::from_kline`（涨跌停 None, is_st=false）
pub fn klines_to_bars(
    code: &str,
    klines: &[axagent_harness::market_data::KLine],
    quote: Option<&StockQuote>,
) -> Vec<Bar> {
    klines
        .iter()
        .map(|k| match quote {
            Some(q) => Bar::from_kline_with_quote(code, k, q),
            None => Bar::from_kline(code, k),
        })
        .collect()
}

/// 按日期区间过滤 Bar（闭区间，纯函数，可单测）
///
/// - `bar.date` 形如 `"YYYY-MM-DD"`，与 `start_date` / `end_date` 字典序比较即可
/// - 字符串为空时跳过（避免空字段误匹配）
pub fn filter_bars_by_date(bars: Vec<Bar>, start_date: &str, end_date: &str) -> Vec<Bar> {
    bars.into_iter()
        .filter(|b| b.date.as_str() >= start_date && b.date.as_str() <= end_date)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::market_data::KLine;

    fn kline(date: &str, close: f64) -> KLine {
        KLine {
            date: date.to_string(),
            open: close - 0.1,
            high: close + 0.2,
            low: close - 0.2,
            close,
            volume: 1000.0,
            amount: close * 1000.0,
            turnover_rate: Some(0.5),
            adj_factor: Some(1.0),
        }
    }

    fn quote_with_limit() -> StockQuote {
        StockQuote {
            code: "600519".to_string(),
            name: "测试".to_string(),
            price: 100.0,
            pre_close: 99.0,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            volume: 5000.0,
            amount: 500_000.0,
            change_pct: 1.0,
            turnover_rate: 0.5,
            pe: Some(20.0),
            pb: Some(5.0),
            total_mv: Some(1_000_000_000.0),
            circulating_mv: Some(800_000_000.0),
            limit_up: Some(108.9),  // 99 * 1.10
            limit_down: Some(89.1), // 99 * 0.90
            is_st: false,
            timestamp: "2024-01-15 15:00:00".to_string(),
        }
    }

    #[test]
    fn test_klines_to_bars_without_quote() {
        let klines = vec![kline("2024-01-15", 100.5)];
        let bars = klines_to_bars("600519", &klines, None);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].code, "600519");
        assert_eq!(bars[0].date, "2024-01-15");
        assert_eq!(bars[0].close, 100.5);
        assert!(bars[0].limit_up.is_none());
        assert!(bars[0].limit_down.is_none());
        assert!(!bars[0].is_st);
    }

    #[test]
    fn test_klines_to_bars_with_quote() {
        let klines = vec![kline("2024-01-15", 100.5)];
        let q = quote_with_limit();
        let bars = klines_to_bars("600519", &klines, Some(&q));
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].limit_up, Some(108.9));
        assert_eq!(bars[0].limit_down, Some(89.1));
        assert!(!bars[0].is_st);
        // 优先用 quote.amount
        assert_eq!(bars[0].amount, 500_000.0);
    }

    #[test]
    fn test_filter_bars_inclusive_bounds() {
        let klines = vec![
            kline("2023-12-29", 100.0),
            kline("2023-12-30", 100.0),
            kline("2024-01-01", 100.0),
            kline("2024-01-15", 100.0),
            kline("2024-01-31", 100.0),
            kline("2024-02-01", 100.0),
        ];
        let bars = klines_to_bars("600519", &klines, None);
        let filtered = filter_bars_by_date(bars, "2024-01-01", "2024-01-31");
        // 闭区间：含 2024-01-01 与 2024-01-31
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].date, "2024-01-01");
        assert_eq!(filtered[2].date, "2024-01-31");
    }

    #[test]
    fn test_filter_bars_empty_when_no_match() {
        let klines = vec![kline("2024-01-15", 100.0)];
        let bars = klines_to_bars("600519", &klines, None);
        let filtered = filter_bars_by_date(bars, "2025-01-01", "2025-12-31");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_bars_single_day_window() {
        let klines = vec![
            kline("2024-01-14", 100.0),
            kline("2024-01-15", 101.0),
            kline("2024-01-16", 102.0),
        ];
        let bars = klines_to_bars("600519", &klines, None);
        let filtered = filter_bars_by_date(bars, "2024-01-15", "2024-01-15");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].date, "2024-01-15");
    }

    #[test]
    fn test_klines_to_bars_preserves_turnover_rate() {
        let klines = vec![kline("2024-01-15", 100.5)];
        let bars = klines_to_bars("600519", &klines, None);
        assert_eq!(bars[0].turnover_rate, Some(0.5));
        assert_eq!(bars[0].adj_factor, Some(1.0));
    }
}
