use crate::error::DataError;
use crate::types::*;
use crate::vendors::eastmoney::classify_earnings_title;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 全局惰性预热标志 — 进程生命周期只做一次预热
static WARMED_UP: AtomicBool = AtomicBool::new(false);
static WARMUP_URL: &str = "https://www.eastmoney.com/";

async fn ensure_warmed_up(fetcher: &dyn BrowserHttpFetch) {
    if WARMED_UP.load(Ordering::Relaxed) {
        return;
    }
    match fetcher.fetch_text(WARMUP_URL).await {
        Ok(_) => {
            tracing::debug!("[browser_eastmoney] 预热成功({WARMUP_URL})");
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            WARMED_UP.store(true, Ordering::Relaxed);
        },
        Err(e) => {
            tracing::warn!("[browser_eastmoney] 预热失败({WARMUP_URL}): {e}，继续原始请求");
        },
    }
}

/// HTTP fetch 能力抽象的 trait（Harness 架构：依赖倒置，注入而非直接依赖 axagent-kit）
///
/// 实现方（如主应用层）可通过 axagent-kit 的 PlaywrightClient 实现此 trait，
/// 在浏览器上下文中执行 HTTP 请求，绕过 TLS 指纹封锁。
#[async_trait]
pub trait BrowserHttpFetch: Send + Sync {
    /// 发送 HTTP GET 请求，返回序列化的 `{ ok, status, body }` 结构
    /// 使用浏览器 fetch API（受 CORS 限制）
    async fn fetch_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, String>;

    /// 通过页面导航发送 HTTP GET 请求，返回诊断结构 `{ body, navigatedUrl, pageTitle, contentType }`
    /// 使用 page.goto() 导航（绕过 CORS 限制），适用于 JSON API
    async fn fetch_text(&self, url: &str) -> Result<Value, String>;
}

/// 通过浏览器内核请求东方财富 API 的 vendor
///
/// 东方财富 WAF 会通过 TLS 指纹（JA3）检测并封锁 curl/reqwest 等非浏览器 HTTP 客户端。
/// 此 vendor 通过注入的 `BrowserHttpFetch` 实现（通常在 Playwright/Chromium 上下文中执行 fetch），
/// 绕过 JA3 封锁。
pub struct BrowserEastMoneyVendor {
    pub fetcher: Option<Arc<dyn BrowserHttpFetch>>,
}

#[allow(clippy::new_without_default)]
impl BrowserEastMoneyVendor {
    pub fn new() -> Self {
        Self { fetcher: None }
    }

    pub fn with_fetcher(fetcher: Arc<dyn BrowserHttpFetch>) -> Self {
        Self { fetcher: Some(fetcher) }
    }
}

/// 构建东方财富 secid (1.600519, 0.000001)
fn to_em_secid(stock_code: &str) -> String {
    let market = if stock_code.starts_with('6') || stock_code.starts_with('9') {
        "1"
    } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
        "0"
    } else {
        "0"
    };
    format!("{market}.{stock_code}")
}

/// 通过浏览器页面导航发送 GET 请求，解析 JSON 响应
/// 使用 page.goto() 而非 fetch()，绕过 CORS 限制
///
/// 添加预热机制：发送真实 API 请求前先导航到东方财富首页，
/// 让浏览器执行 JS 指纹脚本、设置 cookies，降低 WAF 误杀概率。
async fn browser_fetch(
    fetcher: Option<&Arc<dyn BrowserHttpFetch>>,
    url: &str,
) -> Result<Value, DataError> {
    let f = match fetcher {
        Some(f) => f,
        None => {
            // 修复 R3: fetcher 未注入时显式告警，避免静默失败导致调试黑洞
            // 常见原因：非浏览器环境启动（如纯 CLI / 测试），未调用 register_browser_fetcher
            tracing::warn!(
                url = %url,
                "[browser_eastmoney] fetcher 未配置，跳过浏览器请求（非浏览器环境或未注入 BrowserHttpFetch）"
            );
            return Err(DataError::VendorError {
                vendor: "browser_eastmoney".into(),
                message: "browser fetcher not configured (non-browser environment)".into(),
            });
        },
    };

    // ── 惰性预热：仅首次请求执行，后续跳过 ──
    ensure_warmed_up(f.as_ref()).await;

    let result = f.fetch_text(url).await.map_err(|e| DataError::VendorError {
        vendor: "browser_eastmoney".into(),
        message: format!("browser fetch failed: {e}"),
    })?;

    // 诊断信息
    let navigated_url = result["navigatedUrl"].as_str().unwrap_or("unknown");
    let content_type = result["contentType"].as_str().unwrap_or("unknown");
    let page_title = result["pageTitle"].as_str().unwrap_or("");

    let body = result["body"].as_str().unwrap_or("");
    let trimmed = body.trim();

    // 如果是 page.goto 错误，先记录诊断信息
    if trimmed.starts_with("PAGE_GOTO_ERROR:") {
        tracing::warn!(
            "[browser_eastmoney] page.goto 导航失败: {trimmed}, url={navigated_url}, contentType={content_type}"
        );
        return Err(DataError::VendorError {
            vendor: "browser_eastmoney".into(),
            message: format!("page.goto failed: {trimmed}"),
        });
    }

    if trimmed.is_empty() {
        tracing::warn!(
            "[browser_eastmoney] 返回空body, navigatedUrl={navigated_url}, contentType={content_type}, title={page_title}"
        );
        return Err(DataError::VendorError {
            vendor: "browser_eastmoney".into(),
            message: format!("empty body (url={navigated_url}, type={content_type})"),
        });
    }

    serde_json::from_str(trimmed).map_err(|e| {
        let snippet = &trimmed[..trimmed.len().min(200)];
        tracing::warn!(
            "[browser_eastmoney] JSON解析失败: {e}, body={snippet}, navigatedUrl={navigated_url}, contentType={content_type}"
        );
        DataError::ParseError(format!(
            "JSON error: {e} (url={navigated_url})"
        ))
    })
}

#[async_trait]
impl StockVendor for BrowserEastMoneyVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f43,f44,f45,f46,f47,f48,f50,f51,f52,f57,f58,f60,f116,f117,f162,f167,f168,f169,f170"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let data = json["data"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("no data in quote response".into()))?;

        let g = |k: &str| -> f64 { data.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) };
        // 东方财富价格字段为整数，需除以 100 转为元（与 eastmoney.rs 一致）
        let gp = |k: &str| -> f64 { g(k) / 100.0 };

        Ok(StockQuote {
            code: stock_code.to_string(),
            name: data.get("f58").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            price: gp("f43"),
            pre_close: gp("f60"), // f60=昨收价
            open: gp("f46"),
            high: gp("f44"),           // f44=最高价
            low: gp("f45"),            // f45=最低价
            volume: g("f47"),          // 成交量（股），不除以 100
            amount: g("f48"),          // 成交额（元），不除以 100
            change_pct: gp("f170"),    // 涨跌幅
            turnover_rate: gp("f168"), // 换手率
            pe: Some(gp("f162")).filter(|v| *v > 0.0),
            pb: Some(gp("f167")).filter(|v| *v > 0.0),
            total_mv: Some(g("f116")).filter(|v| *v > 0.0), // 总市值，不乘 1e8
            circulating_mv: Some(g("f117")).filter(|v| *v > 0.0), // 流通市值，不乘 1e8
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let secid = to_em_secid(stock_code);
        let klt = match period {
            "daily" | "101" | "Daily" => "101",
            "weekly" | "102" | "Weekly" => "102",
            "monthly" | "103" | "Monthly" => "103",
            _ => "101",
        };
        // 修复 R3: 与 eastmoney vendor 一致，根据 adj 参数选择 fqt
        let fqt = match adj {
            None | Some(AdjType::None) => 0,
            Some(AdjType::Forward) => 1,
            Some(AdjType::Backward) => 2,
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={klt}&fqt={fqt}&end=20500101&lmt={limit}"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let klines = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("no klines array in response".into()))?;

        // vendor 已应用复权 → 标记 adj_factor = Some(1.0) 表示已处理
        let adj_marker = if fqt == 0 { None } else { Some(1.0) };
        let mut result = Vec::with_capacity(klines.len());
        for item in klines {
            let s = item.as_str().unwrap_or("");
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() < 11 {
                continue;
            }
            let p = |i: usize| -> f64 { parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0) };
            result.push(KLine {
                date: parts[0].to_string(),
                open: p(1),
                close: p(2),
                high: p(3),
                low: p(4),
                volume: p(5) * 100.0, // 东方财富 K线 f56 单位为"手"，×100 转为"股"
                amount: p(6),
                turnover_rate: if parts.len() > 10 { Some(p(10)) } else { None },
                // R3: vendor 已复权时标记，避免 lib 层二次应用
                adj_factor: adj_marker,
            });
        }
        Ok(result)
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        let secid = to_em_secid(stock_code);
        // 修复 M-RES-5: 系统时钟倒流时 unwrap_or_default 静默返回 0，
        // 导致 random=0 让 URL 缓存命中率异常。添加 warn 日志便于发现。
        let random = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_else(|e| {
                tracing::warn!("[browser_eastmoney] SystemTime 早于 UNIX_EPOCH（时钟倒流）: {e}");
                0
            });
        // 东方财富财务报表 API
        let url = format!(
            "https://datacenter.eastmoney.com/securities/api/data/v1/get?reportName=RPT_F10_FINANCE_MAINFINADATA&columns=ALL&filter=(SECUCODE=\"{secid}\")&pageNumber=1&pageSize=4&sortTypes=-1&sortColumns=REPORT_DATE&source=HSF10&client=PC&v={random}"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let items = json["result"]["data"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("no financials data".into()))?;

        let mut result = Vec::new();
        for item in items {
            let g = |k: &str| -> f64 {
                item.get(k)
                    .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0.0)
            };
            result.push(FinancialReport {
                stock_code: stock_code.to_string(),
                report_date: item["REPORT_DATE"].as_str().unwrap_or("").to_string(),
                revenue: Some(g("TOTAL_OPERATE_INCOME")).filter(|v| *v > 0.0),
                net_profit: Some(g("TOTAL_PROFIT")).filter(|v| *v > 0.0),
                eps: item["BASIC_EPS"]
                    .as_f64()
                    .or_else(|| item["BASIC_EPS"].as_str().and_then(|s| s.parse().ok())),
                bps: None,
                roe: item["WEIGHTAVG_ROE"]
                    .as_f64()
                    .or_else(|| item["WEIGHTAVG_ROE"].as_str().and_then(|s| s.parse().ok())),
                debt_ratio: None,
                gross_margin: None,
                net_margin: None,
                revenue_yoy: None,
                profit_yoy: None,
                total_assets: Some(g("TOTAL_ASSETS")).filter(|v| *v > 0.0),
                operating_cash_flow: None,
                capital_expenditure: None,
                free_cash_flow: None,
                current_ratio: None,
                quick_ratio: None,
                goodwill: None,
                accounts_receivable: None,
                estimated: Some(false),
            });
        }
        Ok(result)
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://search-api-web.eastmoney.com/search/jsonp?cb=jQuery&param=&type=14&secid={secid}&client=web&pageNum=1&pageSize={}&sort=date",
            limit.min(20)
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let list = json["data"]["list"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("no news list".into()))?;

        Ok(list
            .iter()
            .map(|item| NewsItem {
                title: item["title"].as_str().unwrap_or("").to_string(),
                summary: item["content"].as_str().unwrap_or("").chars().take(200).collect(),
                source: item["source"].as_str().unwrap_or("东方财富").to_string(),
                url: item["url"].as_str().unwrap_or("").to_string(),
                publish_time: item["date"].as_str().unwrap_or("").to_string(),
                sentiment_score: None,
            })
            .collect())
    }

    async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/fflow/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55&klt=1&lmt=1"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let klines = json["data"]["klines"].as_array();
        match klines {
            Some(arr) if !arr.is_empty() => {
                let s = arr[0].as_str().unwrap_or("");
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() < 5 {
                    return Ok(None);
                }
                let parse =
                    |i: usize| -> f64 { parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0) };
                Ok(Some(MoneyFlow {
                    date: parts[0].to_string(),
                    main_net_inflow: parse(1),
                    super_large_net: if parts.len() > 2 { parse(2) } else { 0.0 },
                    large_net: if parts.len() > 3 { parse(3) } else { 0.0 },
                    medium_net: if parts.len() > 4 { parse(4) } else { 0.0 },
                    small_net: if parts.len() > 5 { parse(5) } else { 0.0 },
                    history: Vec::new(),
                }))
            },
            _ => Ok(None),
        }
    }

    async fn get_dragon_tiger(&self, stock_code: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_DAILYBILLBOARD_DETAILS&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            pageSize=20&pageNumber=1&source=WEB&\
            sortColumns=TRADE_DATE&sortTypes=-1"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(vec![]),
        };
        rows.iter()
            .map(|r| {
                let trade_date = r["TRADE_DATE"].as_str().unwrap_or("");
                let date = if trade_date.len() >= 10 {
                    trade_date[..10].to_string()
                } else {
                    trade_date.to_string()
                };
                let buy_seat = r["BUY_SEAT_NEW"].as_i64().unwrap_or(0);
                let sell_seat = r["SELL_SEAT_NEW"].as_i64().unwrap_or(0);
                Ok(DragonTigerEntry {
                    stock_code: stock_code.to_string(),
                    date,
                    dept_name: format!("买入{}席位/卖出{}席位", buy_seat, sell_seat),
                    buy_amount: r["BILLBOARD_BUY_AMT"].as_f64().unwrap_or(0.0),
                    sell_amount: r["BILLBOARD_SELL_AMT"].as_f64().unwrap_or(0.0),
                    net_amount: r["BILLBOARD_NET_AMT"].as_f64().unwrap_or(0.0),
                    reason: r["EXPLANATION"].as_str().map(|s| s.to_string()),
                })
            })
            .collect()
    }

    async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPTA_WEB_RZRQ_GGMX&columns=ALL&\
            filter=(scode%3D%22{code}%22)&source=WEB&\
            sortColumns=DATE&sortTypes=-1&pageNumber=1&pageSize=1"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        if json["success"].as_bool() == Some(false) {
            return Ok(None);
        }
        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };
        let r = &rows[0];
        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date: r["DATE"].as_str().unwrap_or("").to_string(),
            margin_balance: r["RZYE"].as_f64().unwrap_or(0.0),
            short_balance: r["RQYE"].as_f64().unwrap_or(0.0),
            margin_buy: r["RZMRE"].as_f64().unwrap_or(0.0),
            short_sell_volume: r["RQMCL"].as_f64().unwrap_or(0.0),
        }))
    }

    async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_F10_EH_HOLDERS&columns=ALL&\
            filter=(SECURITY_CODE=%22{code}%22)(HOLDER_NAME=%22香港中央结算有限公司%22)&\
            pageSize=2&pageNumber=1&sortColumns=END_DATE&sortTypes=-1"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        if rows.is_empty() {
            return Ok(None);
        }
        let r = &rows[0];
        let hold_num = r["HOLD_NUM"].as_f64().unwrap_or(0.0);
        let total_shares = r["TOTAL_SHARES"].as_f64().unwrap_or(0.0);
        let ratio = if total_shares > 0.0 {
            hold_num / total_shares
        } else {
            0.0
        };
        let change = if rows.len() > 1 {
            let prev = rows[1]["HOLD_NUM"].as_f64().unwrap_or(0.0);
            hold_num - prev
        } else {
            0.0
        };
        Ok(Some(NorthBoundHolding {
            stock_code: stock_code.to_string(),
            date: r["END_DATE"].as_str().unwrap_or("").to_string(),
            holding_shares: hold_num,
            holding_ratio: ratio,
            change_shares: change,
        }))
    }

    async fn get_lockup_schedule(
        &self,
        _stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let url = format!(
            "https://searchadapter.eastmoney.com/api/suggest/get?input={keyword}&count=10&type=14"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;
        let list = json["data"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("no search data".into()))?;

        Ok(list
            .iter()
            .filter_map(|item| {
                let code = item["code"].as_str()?;
                let name = item["name"].as_str()?;
                Some(StockSearchResult {
                    code: code.to_string(),
                    name: name.to_string(),
                    market: if code.starts_with('6') || code.starts_with('9') {
                        "SH".to_string()
                    } else {
                        "SZ".to_string()
                    },
                })
            })
            .collect())
    }

    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        let url = "https://push2.eastmoney.com/api/qt/ulist.np/get?fields=f2,f3,f4,f12,f14&secids=1.000001,0.399001,0.399006,1.000688,1.000300&fltt=2";
        let json = browser_fetch(self.fetcher.as_ref(), url).await?;
        let list = json["data"]["diff"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("no index data".into()))?;

        Ok(list
            .iter()
            .map(|item| {
                let g = |k: &str| -> f64 { item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) };
                IndexQuote {
                    code: item["f12"].as_str().unwrap_or("").to_string(),
                    name: item["f14"].as_str().unwrap_or("").to_string(),
                    price: g("f2"),
                    change_pct: g("f3"),
                    pre_close: 0.0,
                    volume: 0.0,
                    amount: 0.0,
                }
            })
            .collect())
    }

    async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        let url = "https://push2his.eastmoney.com/api/qt/kamt.kline/get?fields1=f1,f2,f3&fields2=f51,f52,f53,f54&klt=101&lmt=5";
        let json = browser_fetch(self.fetcher.as_ref(), url).await?;
        let data = &json["data"];
        if data.is_null() {
            return Ok(None);
        }
        // 尝试解析 hk2sh（沪股通方向）
        let hk2sh = data["hk2sh"].as_str().unwrap_or("");
        if hk2sh.is_empty() {
            return Ok(None);
        }
        let parts: Vec<&str> = hk2sh.split(',').collect();
        if parts.len() < 4 {
            return Ok(None);
        }
        let parse = |i: usize| -> f64 { parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0) };
        let total = parse(1);
        let sz_flow = data["hk2sz"]
            .as_str()
            .and_then(|s| s.split(',').nth(1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        Ok(Some(NorthBoundFlow {
            date: parts[0].to_string(),
            sh_flow: parse(1),
            sz_flow,
            total_flow: total,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            recent_history: Vec::new(),
        }))
    }

    /// P3 修复(2026-07-25): 实现 earnings_calendar,作为 eastmoney datacenter-web
    /// 反爬时的浏览器 fallback 通道。复用 eastmoney.rs 的 classify_earnings_title
    /// 保持分类逻辑一致,通过 browser_fetch 绕过 JA3 TLS 指纹封锁。
    async fn get_earnings_calendar(
        &self,
        stock_code: &str,
    ) -> Result<Vec<EarningsEvent>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_NOTICE&columns=SECURITY_CODE,SECURITY_NAME_ABBR,NOTICE_DATE,TITLE,EQUITY_NOTICE_TYPE&filter=(SECURITY_CODE=\"{code}\")&pageSize=30&sortColumns=NOTICE_DATE&sortTypes=-1&pageNumber=1"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .filter_map(|r| {
                let title = r["TITLE"].as_str().unwrap_or("");
                let notice_date = r["NOTICE_DATE"].as_str().unwrap_or("");
                if title.is_empty() || notice_date.is_empty() {
                    return None;
                }

                let (event_type, period) = classify_earnings_title(title);

                // 只保留财报相关事件（与 eastmoney.rs 一致）
                if event_type == "other" && !title.contains("报告") && !title.contains("业绩") {
                    return None;
                }

                Some(EarningsEvent {
                    stock_code: stock_code.to_string(),
                    stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                    event_date: notice_date.to_string(),
                    event_type: event_type.to_string(),
                    period,
                    detail: Some(title.to_string()),
                    source: Some("browser_eastmoney".to_string()),
                    created_at: chrono::Utc::now().timestamp(),
                })
            })
            .collect())
    }

    async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        let req_trace = chrono::Utc::now().timestamp_millis();
        let url = format!(
            "https://np-listapi.eastmoney.com/comm/web/getNewsByColumns?client=web&biz=web_news_col&column=250&order=1&needInteractData=0&page_index=1&page_size=20&req_trace={req_trace}"
        );
        let json = browser_fetch(self.fetcher.as_ref(), &url).await?;

        let items = match json["data"]["list"].as_array() {
            Some(arr) => arr,
            None => match json["data"].as_array() {
                Some(arr) => arr,
                None => return Ok(vec![]),
            },
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let title = item.get("title")?.as_str()?.to_string();
                let content = item
                    .get("digest")
                    .or_else(|| item.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("showTime")
                    .or_else(|| item.get("publish_time"))
                    .or_else(|| item.get("ctime"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source = item
                    .get("source")
                    .or_else(|| item.get("mediaName"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(ClsFlashItem { title, content, publish_time, source })
            })
            .collect())
    }
}
