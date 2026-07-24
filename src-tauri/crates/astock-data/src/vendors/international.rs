//! 国际股票 vendor — 港股 + 美股数据获取
//!
//! ## 设计
//!
//! 当前实现为 eastmoney 国际行情接口的封装。
//! eastmoney 支持通过特殊代码前缀获取港美股数据：
//! - 港股: `hk00700`（腾讯控股）
//! - 美股: `US_TSLA`（特斯拉）
//! - 中概: `US_BABA`（阿里巴巴）
//!
//! ## 使用
//!
//! 本 vendor 通过 `AStockClient::register_vendor` 注册为 "international"。
//! 调用方通过统一的 `get_quote` / `get_klines` 接口访问。

use async_trait::async_trait;
use serde_json::Value;

use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;

/// 国际股票 vendor（港股 + 美股 + ETF）
pub struct InternationalVendor {
    pub http: reqwest::Client,
}

/// 将国际股票代码转为 eastmoney API 兼容格式
///
/// 规则:
/// - "00700" / "00700.HK" → "hk00700"
/// - "TSLA" / "TSLA.US" → "US_TSLA"
/// - "BABA" / "BABA.US" → "US_BABA"
/// - 其他保留原样（由 API 自行处理）
fn to_international_code(stock_code: &str) -> String {
    let (code, suffix) = if let Some((before, after)) = stock_code.split_once('.') {
        (before, after.to_uppercase())
    } else {
        // 未带后缀：数字=港股，字母=美股
        if stock_code.chars().all(|c| c.is_ascii_digit()) {
            (stock_code, "HK".to_string())
        } else {
            (stock_code, "US".to_string())
        }
    };

    match suffix.as_str() {
        "HK" => format!("hk{code}"),
        "US" => format!("US_{code}"),
        _ => stock_code.to_string(),
    }
}

#[async_trait]
impl StockVendor for InternationalVendor {
    /// 获取港股/美股行情
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let intl_code = to_international_code(stock_code);
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get?secid=0.{}&fields=f43,f44,f45,f46,f47,f48,f50,f51,f52,f57,f58,f60,f116,f117,f162,f168,f170",
            intl_code
        );

        let resp = self.http.get(&url).send().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("HTTP 请求失败: {e}"),
        })?;

        crate::check_response_429(&resp, "international")?;
        let text = resp.text().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("读取响应失败: {e}"),
        })?;

        Self::parse_quote_json(&text, stock_code)
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let intl_code = to_international_code(stock_code);
        let period_code = match period {
            "daily" => "101",
            "weekly" => "102",
            "monthly" => "103",
            _ => "101",
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid=0.{}&fields1=f1,f2,f3&fields2=f51,f52,f53,f54,f55,f56,f57&klt={}&fqt={}&end=20500101&lmt={}",
            intl_code, period_code, if matches!(adj, Some(AdjType::Forward) | None) { "1" } else { "0" }, limit.min(1000)
        );

        let resp = self.http.get(&url).send().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("HTTP 请求失败: {e}"),
        })?;

        let text = resp.text().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("读取响应失败: {e}"),
        })?;

        Self::parse_klines_json(&text, stock_code)
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let url = format!(
            "https://searchadapter.eastmoney.com/api/suggest/get?input={}&type=14&token=ANY",
            urlencoding(keyword)
        );

        let resp = self.http.get(&url).send().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("HTTP 请求失败: {e}"),
        })?;

        let text = resp.text().await.map_err(|e| DataError::VendorError {
            vendor: "international".into(),
            message: format!("读取响应失败: {e}"),
        })?;

        let json: Value =
            serde_json::from_str(&text).map_err(|e| DataError::ParseError(e.to_string()))?;
        let mut results = Vec::new();

        if let Some(suggestions) = json["QuotationCodeTable"]["Data"].as_array() {
            for item in suggestions {
                let code = item["Code"].as_str().unwrap_or("").to_string();
                let name = item["Name"].as_str().unwrap_or("").to_string();
                let market = item["MarketType"].as_str().unwrap_or("").to_string();
                // 只保留港股/美股
                if market == "HK" || market == "US" {
                    results.push(StockSearchResult {
                        code: format!("{}.{}", code, market),
                        name,
                        market: if market == "HK" {
                            "港股".into()
                        } else {
                            "美股".into()
                        },
                    });
                }
            }
        }

        Ok(results)
    }

    async fn get_financials(&self, _stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        // TODO: 国际股票财务报表通过 eastmoney F10 API 获取
        Ok(vec![])
    }

    async fn get_news(&self, _stock_code: &str, _limit: u32) -> Result<Vec<NewsItem>, DataError> {
        Ok(vec![])
    }

    async fn get_money_flow(&self, _stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        Ok(None)
    }

    async fn get_dragon_tiger(
        &self,
        _stock_code: &str,
    ) -> Result<Vec<DragonTigerEntry>, DataError> {
        Ok(vec![])
    }

    async fn get_lockup_schedule(
        &self,
        _stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn get_sector_info(&self, _stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        Ok(None)
    }
}

impl InternationalVendor {
    fn parse_quote_json(text: &str, stock_code: &str) -> Result<StockQuote, DataError> {
        let json: Value = serde_json::from_str(text)
            .map_err(|e| DataError::ParseError(format!("国际行情 JSON 解析失败: {e}")))?;

        let data =
            json["data"].as_object().ok_or_else(|| DataError::NotFound(stock_code.to_string()))?;

        let parse_f64 =
            |key: &str| -> f64 { data.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) };

        let price = parse_f64("f43");
        let pre_close = parse_f64("f44");
        let open = parse_f64("f45");
        let high = parse_f64("f46");
        let low = parse_f64("f47");
        let volume = parse_f64("f48");
        let amount = parse_f64("f50");
        let turnover_rate = parse_f64("f168");

        Ok(StockQuote {
            code: stock_code.to_string(),
            name: data.get("f58").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            price,
            pre_close,
            open,
            high,
            low,
            volume: volume * 100.0, // eastmoney 手→股
            amount,
            change_pct: if pre_close > 0.0 {
                (price - pre_close) / pre_close * 100.0
            } else {
                0.0
            },
            turnover_rate,
            pe: Some(parse_f64("f162")),
            pb: Some(parse_f64("f167")),
            total_mv: Some(parse_f64("f116") * 1_0000.0),
            circulating_mv: Some(parse_f64("f117") * 1_0000.0),
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    fn parse_klines_json(text: &str, stock_code: &str) -> Result<Vec<KLine>, DataError> {
        let json: Value = serde_json::from_str(text)
            .map_err(|e| DataError::ParseError(format!("国际K线 JSON 解析失败: {e}")))?;

        let klines_data = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::NotFound(format!("{} K线数据为空", stock_code)))?;

        let mut klines = Vec::with_capacity(klines_data.len());
        for item in klines_data {
            let line = item.as_str().unwrap_or("");
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 6 {
                continue;
            }
            klines.push(KLine {
                date: parts[0].to_string(),
                open: parts[1].parse().unwrap_or(0.0),
                close: parts[2].parse().unwrap_or(0.0),
                high: parts[3].parse().unwrap_or(0.0),
                low: parts[4].parse().unwrap_or(0.0),
                volume: parts[5].parse().unwrap_or(0.0),
                amount: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                turnover_rate: None,
                adj_factor: None,
            });
        }

        Ok(klines)
    }
}

fn urlencoding(s: &str) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            },
            b' ' => result.push_str("%20"),
            _ => {
                let _ = write!(result, "%{byte:02X}");
            },
        }
    }
    result
}
