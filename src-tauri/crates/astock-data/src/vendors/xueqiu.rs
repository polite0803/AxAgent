//! 雪球 (xueqiu.com) 数据源
//!
//! Token 通过共享 Arc<RwLock<String>> 注入，支持运行时动态更新。
//! 由前端"数据源"设置页管理，写入 workflow template 变量 vendor_xueqiu_token。
//! 未配置 token 时 vendor 静默跳过，不影响路由。

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;

pub struct XueqiuVendor {
    pub http: reqwest::Client,
    pub token: Arc<RwLock<String>>,
}

impl XueqiuVendor {
    /// 检查是否已配置有效 token
    async fn enabled(&self) -> bool {
        !self.token.read().await.is_empty()
    }

    /// 带 Cookie 的 GET 请求，自动检测 429 限流
    async fn xq_get(&self, url: &str) -> Result<reqwest::Response, DataError> {
        let token = self.token.read().await.clone();
        let resp = self
            .http
            .get(url)
            .header("Cookie", format!("xq_a_token={}", token))
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .header("Referer", "https://xueqiu.com/")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .send()
            .await
            .map_err(DataError::from)?;
        crate::check_response_429(&resp, "xueqiu")?;
        Ok(resp)
    }
}

fn to_xq_symbol(code: &str) -> String {
    if code.starts_with('6') || code.starts_with('9') {
        format!("SH{code}")
    } else if code.starts_with('8') || code.starts_with('4') {
        format!("BJ{code}")
    } else {
        format!("SZ{code}")
    }
}

#[async_trait]
impl StockVendor for XueqiuVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        if !self.enabled().await {
            return Err(DataError::VendorError {
                vendor: "xueqiu".into(),
                message: "XUEQIU_TOKEN 未配置".into(),
            });
        }
        let symbol = to_xq_symbol(stock_code);
        let url =
            format!("https://stock.xueqiu.com/v5/stock/quote.json?symbol={symbol}&extend=detail");
        let resp = self.xq_get(&url).await?;
        let json: serde_json::Value = resp.json().await?;
        let item = &json["data"]["quote"];
        if item.is_null() {
            return Err(DataError::NotFound(stock_code.into()));
        }
        let f = |k: &str| -> f64 { item[k].as_f64().unwrap_or(0.0) };
        let s = |k: &str| -> String { item[k].as_str().unwrap_or("").to_string() };
        Ok(StockQuote {
            code: stock_code.to_string(),
            name: s("name"),
            price: f("current"),
            pre_close: f("last_close"),
            open: f("open"),
            high: f("high"),
            low: f("low"),
            volume: f("volume"),
            amount: f("amount"),
            change_pct: f("percent"),
            turnover_rate: f("turnover_rate"),
            pe: Some(f("pe_ttm")).filter(|v| *v > 0.0),
            pb: Some(f("pb")).filter(|v| *v > 0.0),
            total_mv: Some(f("market_capital")).filter(|v| *v > 0.0),
            circulating_mv: Some(f("float_market_capital")).filter(|v| *v > 0.0),
            limit_up: None,
            limit_down: None,
            is_st: s("type").contains("ST"),
            timestamp: s("time"),
        })
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        if !self.enabled().await {
            return Ok(vec![]);
        }
        let symbol = to_xq_symbol(stock_code);
        let period_map = |p: &str| -> &str {
            match p {
                "5" | "Min5" => "5m",
                "15" | "Min15" => "15m",
                "30" | "Min30" => "30m",
                "60" | "Min60" => "60m",
                "daily" | "101" | "Daily" => "day",
                "weekly" | "102" | "Weekly" => "week",
                "monthly" | "103" | "Monthly" => "month",
                _ => "day",
            }
        };
        // begin=0 表示从最早开始
        let url = format!(
            "https://stock.xueqiu.com/v5/stock/chart/kline.json?symbol={symbol}&begin=0&period={}&type=before&count=-{}&indicator=kline,pe,pb",
            period_map(period),
            limit
        );
        let resp = self.xq_get(&url).await?;
        let json: serde_json::Value = resp.json().await?;
        let items = json["data"]["item"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("xueqiu klines: missing item array".into()))?;
        let mut klines: Vec<KLine> = items
            .iter()
            .map(|v| {
                let arr = v
                    .as_array()
                    .ok_or_else(|| DataError::ParseError("xueqiu kline: not an array".into()))?;
                // 雪球日K格式: [timestamp, open, high, low, close, volume, amount, ...]
                if arr.len() < 7 {
                    return Err(DataError::ParseError(format!(
                        "xueqiu kline: expected >=7 fields, got {}",
                        arr.len()
                    )));
                }
                let ts = arr[0].as_i64().unwrap_or(0) / 1000;
                let date = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                let g = |i: usize| -> f64 { arr[i].as_f64().unwrap_or(0.0) };
                Ok(KLine {
                    date,
                    open: g(1),
                    high: g(2),
                    low: g(3),
                    close: g(4),
                    volume: g(5),
                    amount: g(6),
                    turnover_rate: None,
                    adj_factor: None,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        klines.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(klines)
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        if !self.enabled().await {
            return Ok(vec![]);
        }
        let symbol = to_xq_symbol(stock_code);
        let url = format!(
            "https://stock.xueqiu.com/v5/stock/finance/cn/indicator.json?symbol={symbol}&type=all&is_detail=true&count=12"
        );
        let resp = self.xq_get(&url).await?;
        let json: serde_json::Value = resp.json().await?;
        let list = json["data"]["list"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("xueqiu financials: missing list array".into()))?;
        Ok(list
            .iter()
            .map(|item| {
                let f = |k: &str| -> Option<f64> { item[k].as_f64().filter(|v| !v.is_nan()) };
                let report_date = item["report_date"].as_str().unwrap_or("").to_string();
                FinancialReport {
                    stock_code: stock_code.to_string(),
                    report_date,
                    revenue: f("revenue"),
                    net_profit: f("net_profit"),
                    eps: f("basic_eps"),
                    bps: f("net_asset_value_per_share"),
                    roe: f("roe"),
                    debt_ratio: f("debt_asset_ratio"),
                    gross_margin: f("gross_profit_ratio"),
                    net_margin: f("net_profit_ratio"),
                    revenue_yoy: f("increase_revenue_ratio"),
                    profit_yoy: f("increase_net_profit_ratio"),
                    total_assets: f("total_assets"),
                    operating_cash_flow: f("operate_cash_flow"),
                    capital_expenditure: f("capital_expenditure"),
                    free_cash_flow: None,
                    current_ratio: f("current_ratio"),
                    quick_ratio: f("quick_ratio"),
                }
            })
            .collect())
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        if !self.enabled().await {
            return Ok(vec![]);
        }
        let symbol_id = format!(
            "{}{}",
            if stock_code.starts_with('6') {
                "SH"
            } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
                "BJ"
            } else {
                "SZ"
            },
            stock_code
        );
        let count = limit.min(50);
        // 雪球股票时间线接口：个股动态（新闻+讨论）
        // 注意：此接口被阿里云 WAF 保护，token 无效或网络环境触发 WAF 时会返回
        // text/html WAF 挑战页面而非 JSON。检查 Content-Type 以提前识别这种情形。
        let url = format!(
            "https://xueqiu.com/statuses/stock_timeline.json?symbol_id={symbol_id}&page=1&count={count}"
        );
        let resp = self.xq_get(&url).await?;

        // 预检 Content-Type，防止将 WAF HTML 页面当作 JSON 解析
        // 先克隆 Content-Type 字符串，避免借用 resp
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok().map(String::from))
            .unwrap_or_default();
        if ct.starts_with("text/html") || ct.starts_with("text/plain") {
            let body = resp.text().await.unwrap_or_default();
            let preview = &body[..body.len().min(200)];
            tracing::warn!("[xueqiu] 新闻接口返回非 JSON (Content-Type={ct}), preview={preview}");
            return Err(DataError::VendorError {
                vendor: "xueqiu".into(),
                message: format!("雪球新闻接口返回非 JSON (Content-Type={ct})，可能是阿里云 WAF 拦截或 API 已变更"),
            });
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(format!("xueqiu news json 解析失败: {e}")))?;
        let items = json["list"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("xueqiu news: missing list array".into()))?;
        Ok(items
            .iter()
            .map(|item| {
                let text = item["text"].as_str().unwrap_or("").to_string();
                // 雪球文本是 HTML，提取纯文本摘要
                let plain = text
                    .replace("<p>", " ")
                    .replace("</p>", " ")
                    .replace("<br />", " ")
                    .replace("<br>", " ")
                    .replace(['\n', '\r'], " ");
                let trimmed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
                let summary = trimmed.chars().take(200).collect::<String>();
                let created_at = item["created_at"].as_i64().unwrap_or(0);
                let ts = created_at / 1000;
                let publish_time = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                let source = item["user"]["screen_name"].as_str().unwrap_or("雪球");
                NewsItem {
                    title: item["title"].as_str().unwrap_or("").to_string(),
                    summary,
                    source: format!("{source}·雪球"),
                    url: format!(
                        "https://xueqiu.com/{}/{}",
                        item["user_id"].as_i64().unwrap_or(0),
                        item["id"].as_i64().unwrap_or(0)
                    ),
                    publish_time,
                    sentiment_score: None,
                }
            })
            .collect())
    }

    async fn get_money_flow(&self, _: &str) -> Result<Option<MoneyFlow>, DataError> {
        Ok(None)
    }

    async fn get_dragon_tiger(&self, _: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        Ok(vec![])
    }

    async fn get_lockup_schedule(&self, _: &str) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        if !self.enabled().await {
            return Ok(vec![]);
        }
        let url =
            format!("https://xueqiu.com/query/v1/search/web/search.json?q={keyword}&count=10");
        let resp = self.xq_get(&url).await?;
        let json: serde_json::Value = resp.json().await?;
        let items = json["data"]["stocks"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("xueqiu search: missing stocks array".into()))?;
        Ok(items
            .iter()
            .map(|item| StockSearchResult {
                code: item["code"].as_str().unwrap_or("").to_string(),
                name: item["name"].as_str().unwrap_or("").to_string(),
                market: item["exchange"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    // ── as-of 能力申报 ──
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        let _ = method;
        AsOfCapability::Fallthrough
    }
}
