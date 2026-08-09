//! 国际股票 vendor — 港股 + 美股 + 国际指数 + 外汇数据获取
//!
//! ## 设计
//!
//! 当前实现为 eastmoney 国际行情接口的封装。
//! eastmoney 支持通过特殊代码前缀获取港美股数据：
//! - 港股: `hk00700`（腾讯控股）
//! - 美股: `US_TSLA`（特斯拉）
//! - 中概: `US_BABA`（阿里巴巴）
//! - 国际指数: `US_SPX`（标普 500）/ `US_IXIC`（纳指）/ `hkHSI`（恒生）
//! - 外汇: `forex.USDCNY`（美元人民币）
//!
//! ## 使用
//!
//! 本 vendor 通过 `AStockClient::register_vendor` 注册为 "international"。
//! 调用方通过统一的 `get_quote` / `get_klines` 接口访问。

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use axagent_harness::plugin_hook::{
    ApiCallContext, ApiCallResult, HttpHookExecutor, PreApiHookOutcome,
};

use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;

/// 国际股票 vendor（港股 + 美股 + ETF + 国际指数 + 外汇）
///
/// G20: 可选注入 `HttpHookExecutor`，在每次 HTTP 调用前后触发
/// `pre_api_request` / `post_api_request` hook，用于限流/审计/SSRF 防护。
pub struct InternationalVendor {
    pub http: reqwest::Client,
    /// G20 API hook 执行器（None = 不启用 hook，零开销）
    pub hook_executor: Option<Arc<HttpHookExecutor>>,
}

impl InternationalVendor {
    /// 创建不带 hook 的 vendor（向后兼容）
    pub fn new() -> Self {
        Self { http: reqwest::Client::new(), hook_executor: None }
    }

    /// 创建带 hook 的 vendor（G20 接入点）
    pub fn with_hooks(
        http: reqwest::Client,
        hooks: Vec<axagent_harness::plugin_hook::SharedHook>,
    ) -> Self {
        if hooks.is_empty() {
            return Self { http, hook_executor: None };
        }
        Self { http, hook_executor: Some(Arc::new(HttpHookExecutor::new(hooks))) }
    }

    /// G20: 在 HTTP GET 调用前后包裹 hook。
    /// 返回 (响应文本, ApiCallResult) 以便调用方记录额外信息。
    async fn http_get_with_hooks(&self, url: &str, category: &str) -> Result<String, DataError> {
        // 无 hook 时走原始路径
        let Some(ref executor) = self.hook_executor else {
            let resp = self.http.get(url).send().await.map_err(|e| DataError::VendorError {
                vendor: "international".into(),
                message: format!("HTTP 请求失败: {e}"),
            })?;
            crate::check_response_429(&resp, "international")?;
            return resp.text().await.map_err(|e| DataError::VendorError {
                vendor: "international".into(),
                message: format!("读取响应失败: {e}"),
            });
        };

        // 有 hook：构建上下文 → pre → 执行 → post
        let ctx = ApiCallContext::new(url, "GET", category).with_service("eastmoney_international");
        let started = Instant::now();

        // pre_api_request
        match executor.pre_request(&ctx).await {
            PreApiHookOutcome::Allow => {},
            PreApiHookOutcome::Veto { reason, hook_name } => {
                tracing::warn!(
                    vendor = "international",
                    hook = %hook_name,
                    reason = %reason,
                    url = %url,
                    "[G20] 请求被 hook 否决"
                );
                return Err(DataError::VendorError {
                    vendor: "international".into(),
                    message: format!("请求被 hook '{hook_name}' 否决: {reason}"),
                });
            },
            PreApiHookOutcome::Modify { changes, hook_name } => {
                tracing::debug!(
                    hook = %hook_name,
                    ?changes,
                    "[G20] 请求被 hook 修改（ InternationalVendor 暂不应用 changes，仅记录）"
                );
                // 此处可按 changes 调整 headers/url/timeout，当前实现仅记录
            },
        }

        // 实际 HTTP 调用
        let resp_result = self.http.get(url).send().await;
        let duration_ms = started.elapsed().as_millis() as u64;

        let result = match resp_result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let success = resp.status().is_success();
                // 429 限流特殊处理（与无 hook 路径保持一致）
                if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let api_result = ApiCallResult {
                        url: url.to_string(),
                        status,
                        success: false,
                        headers: serde_json::json!({}),
                        body: Value::Null,
                        duration_ms,
                        error: Some("Rate limited (429)".to_string()),
                        retry_count: 0,
                    };
                    executor.post_request(&ctx, &api_result).await;
                    return Err(DataError::RateLimited { vendor: "international".to_string() });
                }
                if !success {
                    let text = resp.text().await.unwrap_or_default();
                    let api_result = ApiCallResult {
                        url: url.to_string(),
                        status,
                        success: false,
                        headers: serde_json::json!({}),
                        body: Value::String(text.clone()),
                        duration_ms,
                        error: Some(format!("HTTP {status}")),
                        retry_count: 0,
                    };
                    executor.post_request(&ctx, &api_result).await;
                    return Err(DataError::VendorError {
                        vendor: "international".into(),
                        message: format!("HTTP {status}: {text}"),
                    });
                }
                let text = resp.text().await.map_err(|e| DataError::VendorError {
                    vendor: "international".into(),
                    message: format!("读取响应失败: {e}"),
                })?;
                let api_result = ApiCallResult {
                    url: url.to_string(),
                    status,
                    success: true,
                    headers: serde_json::json!({}),
                    body: Value::String(text.clone()),
                    duration_ms,
                    error: None,
                    retry_count: 0,
                };
                executor.post_request(&ctx, &api_result).await;
                text
            },
            Err(e) => {
                let api_result = ApiCallResult {
                    url: url.to_string(),
                    status: 0,
                    success: false,
                    headers: serde_json::json!({}),
                    body: Value::Null,
                    duration_ms,
                    error: Some(e.to_string()),
                    retry_count: 0,
                };
                executor.post_request(&ctx, &api_result).await;
                return Err(DataError::VendorError {
                    vendor: "international".into(),
                    message: format!("HTTP 请求失败: {e}"),
                });
            },
        };

        Ok(result)
    }
}

impl Default for InternationalVendor {
    fn default() -> Self {
        Self::new()
    }
}

/// eastmoney secid 市场前缀
///
/// - 0.<code>  — 港股/美股/外汇/国际指数
/// - 1.<code>  — 上交所
/// - 0.51xxxx  — 上证 ETF
fn to_em_secid(intl_code: &str) -> String {
    // eastmoney 国际行情统一走 secid=0.<intl_code>
    // （与 eastmoney.rs A 股 secid=1.<6位数字> 不同）
    format!("0.{intl_code}")
}

/// 将国际股票代码转为 eastmoney API 兼容格式
///
/// 规则:
/// - "00700" / "00700.HK" → "hk00700"
/// - "TSLA" / "TSLA.US" → "US_TSLA"
/// - "BABA" / "BABA.US" → "US_BABA"
/// - "US_SPX" / "hkHSI" / "forex.USDCNY" → 原样保留（已编码）
/// - 其他保留原样（由 API 自行处理）
fn to_international_code(stock_code: &str) -> String {
    // 已编码格式直接返回
    if stock_code.starts_with("US_")
        || stock_code.starts_with("hk")
        || stock_code.starts_with("forex.")
    {
        return stock_code.to_string();
    }
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
    /// 获取港股/美股/国际指数/外汇行情
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let intl_code = to_international_code(stock_code);
        let secid = to_em_secid(&intl_code);
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f43,f44,f45,f46,f47,f48,f50,f51,f52,f57,f58,f60,f116,f117,f162,f168,f170"
        );

        let text = self.http_get_with_hooks(&url, "data_source").await?;

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
        let secid = to_em_secid(&intl_code);
        let period_code = match period {
            "daily" => "101",
            "weekly" => "102",
            "monthly" => "103",
            _ => "101",
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3&fields2=f51,f52,f53,f54,f55,f56,f57&klt={}&fqt={}&end=20500101&lmt={}",
            period_code,
            if matches!(adj, Some(AdjType::Forward) | None) {
                "1"
            } else {
                "0"
            },
            limit.min(1000)
        );

        let text = self.http_get_with_hooks(&url, "data_source").await?;

        Self::parse_klines_json(&text, stock_code)
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let url = format!(
            "https://searchadapter.eastmoney.com/api/suggest/get?input={}&type=14&token=ANY",
            urlencoding(keyword)
        );

        let text = self.http_get_with_hooks(&url, "data_source").await?;

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

        // 字段映射参考 browser_eastmoney.rs:169-175 的正确映射
        // f43=最新价 f60=昨收 f46=开盘 f44=最高 f45=最低 f47=成交量(手) f48=成交额(元)
        let price = parse_f64("f43");
        let pre_close = parse_f64("f60");
        let open = parse_f64("f46");
        let high = parse_f64("f44");
        let low = parse_f64("f45");
        let volume = parse_f64("f47");
        let amount = parse_f64("f48");
        let turnover_rate = parse_f64("f168");

        Ok(StockQuote {
            code: stock_code.to_string(),
            name: data.get("f58").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            price,
            pre_close,
            open,
            high,
            low,
            // f47 单位为"手"，1 手 = 100 股，转换为"股"
            volume: volume * 100.0,
            amount,
            change_pct: if pre_close > 0.0 {
                (price - pre_close) / pre_close * 100.0
            } else {
                0.0
            },
            turnover_rate,
            pe: Some(parse_f64("f162")),
            pb: Some(parse_f64("f167")),
            // f116/f117 单位为"元"，与 browser_eastmoney.rs:180-181 保持一致，不乘系数
            total_mv: Some(parse_f64("f116")),
            circulating_mv: Some(parse_f64("f117")),
            limit_up: None,
            limit_down: None,
            is_st: false,
            // 使用 UTC+8（北京时间）固定时区，避免跨时区部署时时间戳偏移
            timestamp: chrono::Utc::now()
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
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
