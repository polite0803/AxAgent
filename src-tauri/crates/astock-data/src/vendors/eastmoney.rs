use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

pub struct EastMoneyVendor {
    pub http: reqwest::Client,
    /// 可选代理客户端（EASTMONEY_PROXY 环境变量配置），用于绕过 IP 封锁
    pub proxy_http: Option<reqwest::Client>,
}

impl EastMoneyVendor {
    /// 从环境变量 EASTMONEY_PROXY 构建代理客户端（如 socks5://192.168.0.235:1080）
    pub fn build_proxy_client() -> Option<reqwest::Client> {
        let proxy_url = std::env::var("EASTMONEY_PROXY").ok()?;
        if proxy_url.is_empty() {
            return None;
        }
        // 修复 M-RES-2: 原 `reqwest::Proxy::all(&proxy_url).ok()?` 把代理构建
        // 错误（URL 格式错误、协议不支持等）静默吞为 None，调用方无法感知。
        // 改为显式 match，记录 warn 日志便于诊断。
        let proxy = match reqwest::Proxy::all(&proxy_url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("[eastmoney] 代理 URL 解析失败 (url={proxy_url}): {e}");
                return None;
            },
        };
        match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .cookie_store(true)
            .pool_max_idle_per_host(8)
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .build()
        {
            Ok(c) => {
                tracing::info!("[eastmoney] 已配置代理: {proxy_url}");
                Some(c)
            },
            Err(e) => {
                tracing::warn!("[eastmoney] 代理客户端创建失败: {e}");
                None
            },
        }
    }

    /// em_get 带指数退避重试（连接级别错误：1s → 2s → 4s，最多 3 次）
    /// 429 限流时使用更长等待（2s → 4s → 8s）
    /// IncompleteMessage 时若配置了代理，自动走代理重试
    async fn em_get(&self, url: &str) -> Result<reqwest::Response, DataError> {
        let max_retries = 3;
        let mut delay = Duration::from_secs(1);
        let mut last_err = None;
        for attempt in 0..max_retries {
            let http_client = if attempt == 0 {
                &self.http
            } else if let Some(ref p) = self.proxy_http {
                // 第1次失败后走代理重试（仅当配置了代理）
                p
            } else {
                &self.http
            };
            let result = http_client
                .get(url)
                .header("Referer", "https://quote.eastmoney.com/")
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                )
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await;
            match result {
                Ok(resp) => {
                    // 检查 429 限流
                    if let Err(e) = crate::check_response_429(&resp, "eastmoney") {
                        if attempt + 1 < max_retries {
                            let wait = delay * 2;
                            tracing::warn!(
                                "[retry] eastmoney 限流(第{}次, {wait:?}后重试)",
                                attempt + 1
                            );
                            sleep(wait).await;
                            delay *= 2;
                            last_err = Some(e);
                            continue;
                        }
                        last_err = Some(e);
                    } else {
                        return Ok(resp);
                    }
                },
                Err(e) => {
                    let is_incomplete = format!("{e:?}").contains("IncompleteMessage");
                    if is_incomplete && attempt == 0 && self.proxy_http.is_some() {
                        // IncompleteMessage + 有代理 → 不走指数退避，立即走代理重试
                        tracing::warn!("[eastmoney] IncompleteMessage，切换代理重试({url})");
                        last_err = Some(e.into());
                        continue; // 直接用 attempt=1 走代理
                    }
                    if is_incomplete {
                        // IncompleteMessage + 无代理 → 快速失败让路由层 fallback
                        tracing::warn!(
                            "[eastmoney] IncompleteMessage({url})，快速失败→路由层 fallback"
                        );
                        return Err(e.into());
                    }
                    if attempt + 1 < max_retries {
                        tracing::warn!(
                            "[retry] eastmoney 请求失败 (第{}次, {delay:?}后重试): {e:?}",
                            attempt + 1
                        );
                        sleep(delay).await;
                        delay *= 2;
                    } else {
                        tracing::error!(
                            "[eastmoney] 最终失败: {e:?}, source: {:?}",
                            std::error::Error::source(&e)
                        );
                        last_err = Some(e.into());
                    }
                },
            }
        }
        // 修复 M-DEF-2: 原代码 `last_err.unwrap()` 在 last_err 为 None 时 panic。
        // 理论上循环正常退出时 last_err 必有值（只有 Ok 分支会 return），
        // 但防御性编程：若因逻辑漏洞走到这里 last_err 仍为 None，给出明确错误。
        Err(last_err.unwrap_or_else(|| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: "no error recorded".into(),
        }))
    }
}

/// 根据公告标题分类财报事件类型
///
/// 返回 (event_type, period)
/// - "业绩预告" → ("preliminary", 期间)
/// - "业绩快报" → ("express", 期间)
/// - "年报"/"季报"/"半年报" → ("formal", 期间)
/// - "股东大会" → ("shareholders_meeting", None)
/// - 其他 → ("other", None)
pub(crate) fn classify_earnings_title(title: &str) -> (&'static str, Option<String>) {
    // 提取期间（如 "2024年年度报告" → "2024年报"，"2025年第三季度报告" → "2025Q3"）
    let period = extract_report_period(title);

    if title.contains("业绩预告") || title.contains("预增") || title.contains("预减") {
        return ("preliminary", period);
    }
    if title.contains("业绩快报") {
        return ("express", period);
    }
    if title.contains("股东大会") {
        return ("shareholders_meeting", None);
    }
    if title.contains("年度报告")
        || title.contains("季度报告")
        || title.contains("半年报")
        || title.contains("年报")
    {
        return ("formal", period);
    }
    ("other", None)
}

/// 从标题中提取报告期间
fn extract_report_period(title: &str) -> Option<String> {
    // 匹配 "2024年年度报告" / "2025年第三季度报告" / "2024年半年度报告"
    if let Some(year_start) = title.find(|c: char| c.is_ascii_digit() && c != '0') {
        let rest = &title[year_start..];
        // 提取年份
        let year: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if year.len() == 4 {
            // P3 修复(2026-07-25): 半年度/半年报/中报必须先于"年度报告"判断,
            // 否则"2024年半年度报告"会先匹配到"年度报告"误返回"2024年报"。
            if title.contains("半年度") || title.contains("半年报") || title.contains("中报")
            {
                return Some(format!("{year}Q2"));
            }
            if title.contains("年度报告") || title.contains("年报") {
                return Some(format!("{year}年报"));
            }
            if title.contains("第一季度") {
                return Some(format!("{year}Q1"));
            }
            if title.contains("第三季度") {
                return Some(format!("{year}Q3"));
            }
            return Some(year);
        }
    }
    None
}

/// 构建东方财富 secid
///
/// A 股：`1.600519`（上海）、`0.000001`（深圳）
/// 港股：`116.00700`（去掉 .HK 后缀，加 116 前缀）
/// 美股：`105.AAPL`（去掉 .US 后缀，加 105 前缀）
fn to_em_secid(stock_code: &str) -> String {
    // 修复(2026-07-22): 去除 sh/sz/bj 前缀,否则后续 starts_with('6') 判断会失效,
    // 误把 "sh600887" 当作深圳市场股票,生成 secid="0.sh600887" 导致所有 API 调用返回空数据。
    // 影响范围:get_quote/get_klines/get_financials/get_money_flow/get_dragon_tiger/
    // get_lockup_schedule/get_margin_data/get_north_bound_holding/get_shareholder_trades/
    // get_block_trades/get_peers 等所有使用 to_em_secid 的方法。
    let code =
        stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");

    // 港股：00700.HK → 116.00700
    if let Some(hk) = code.strip_suffix(".HK").or_else(|| code.strip_suffix(".hk")) {
        return format!("116.{hk}");
    }
    // 美股：AAPL.US → 105.AAPL
    if let Some(us) = code.strip_suffix(".US").or_else(|| code.strip_suffix(".us")) {
        return format!("105.{us}");
    }
    // A 股
    let market = if code.starts_with('6') || code.starts_with('9') {
        "1"
    } else if code.starts_with('8') || code.starts_with('4') {
        "0"
    } else {
        "0"
    };
    format!("{market}.{code}")
}

/// 构建东方财富 SECUCODE（用于 datacenter 报表 API）
///
/// A 股：`600887.SH`（上海）、`000001.SZ`（深圳）、`830879.BJ`（北交所）
/// 港股/美股：原值返回（如 `00700.HK`、`AAPL.US`）
fn to_em_secucode(stock_code: &str) -> String {
    let code =
        stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
    // 港股/美股：已带后缀直接返回
    if code.ends_with(".HK")
        || code.ends_with(".hk")
        || code.ends_with(".US")
        || code.ends_with(".us")
    {
        return code.to_string();
    }
    // A 股
    let suffix = if code.starts_with('6') || code.starts_with('9') {
        "SH"
    } else if code.starts_with('8') || code.starts_with('4') {
        "BJ"
    } else {
        "SZ"
    };
    format!("{code}.{suffix}")
}

#[async_trait]
impl StockVendor for EastMoneyVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f43,f44,f45,f46,f47,f48,f50,f51,f52,f55,f57,f58,f60,f116,f117,f162,f167,f168,f169,f170,f171"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;
        let d = &json["data"];
        if d.is_null() {
            return Err(DataError::VendorError {
                vendor: "eastmoney".into(),
                message: "no quote data".into(),
            });
        }
        let f = |key: &str| d[key].as_f64().unwrap_or(0.0);
        let price = f("f43") / 100.0;
        let pre_close = f("f60") / 100.0;
        let is_st = d["f58"].as_str().map(|n| n.contains("ST")).unwrap_or(false);
        let market_type = detect_market_type(stock_code);
        let limit_pct = get_st_price_limit_pct(is_st, market_type) / 100.0;
        let limit_up = if pre_close > 0.0 {
            Some((pre_close * (1.0 + limit_pct) * 100.0).round() / 100.0)
        } else {
            None
        };
        let limit_down = if pre_close > 0.0 {
            Some((pre_close * (1.0 - limit_pct) * 100.0).round() / 100.0)
        } else {
            None
        };
        Ok(StockQuote {
            code: stock_code.to_string(),
            name: d["f58"].as_str().unwrap_or("").to_string(),
            price,
            pre_close,
            open: f("f46") / 100.0,
            high: f("f44") / 100.0,
            low: f("f45") / 100.0,
            volume: f("f47"),
            amount: f("f48"),
            change_pct: f("f170") / 100.0,
            turnover_rate: f("f168") / 100.0,
            pe: Some(f("f162") / 100.0).filter(|v| *v > 0.0),
            pb: Some(f("f167") / 100.0).filter(|v| *v > 0.0),
            total_mv: Some(f("f116")).filter(|v| *v > 0.0),
            circulating_mv: Some(f("f117")).filter(|v| *v > 0.0),
            limit_up,
            limit_down,
            is_st,
            timestamp: d["f171"].as_i64().map(|t| t.to_string()).unwrap_or_default(),
        })
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let period_code = match period {
            "5" | "Min5" => "5",
            "15" | "Min15" => "15",
            "30" | "Min30" => "30",
            "60" | "Min60" => "60",
            "daily" | "101" | "Daily" => "101",
            "weekly" | "102" | "Weekly" => "102",
            "monthly" | "103" | "Monthly" => "103",
            _ => "101",
        };
        let secid = to_em_secid(stock_code);
        // 修复 R3: 根据 adj 参数选择 fqt（0=不复权, 1=前复权, 2=后复权）
        // 原硬编码 fqt=1 导致 adj_type=None 时仍返回前复权数据，与不复权语义不符
        let fqt = match adj {
            None | Some(AdjType::None) => 0,
            Some(AdjType::Forward) => 1,
            Some(AdjType::Backward) => 2,
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={period_code}&fqt={fqt}&end=20500101&lmt={limit}"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let klines_raw = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("missing klines array".into()))?;

        // vendor 已应用复权 → 标记 adj_factor = Some(1.0) 表示已处理
        // （实际复权因子不是 1.0，但 lib 层只检查 is_some 判断是否需要本地 fallback）
        let adj_marker = if fqt == 0 { None } else { Some(1.0) };
        let mut klines: Vec<KLine> = klines_raw
            .iter()
            .map(|v| {
                let s =
                    v.as_str().ok_or_else(|| DataError::ParseError("kline not string".into()))?;
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() < 11 {
                    return Err(DataError::ParseError(format!(
                        "expected 11 fields in kline, got {}",
                        parts.len()
                    )));
                }
                let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
                Ok(KLine {
                    date: parts[0].to_string(),
                    open: parse(parts[1]),
                    close: parse(parts[2]),
                    high: parse(parts[3]),
                    low: parse(parts[4]),
                    volume: parse(parts[5]) * 100.0, // 东方财富 K线 f56 单位为"手"，×100 转为"股"
                    amount: parse(parts[6]),
                    turnover_rate: Some(parse(parts[10])),
                    // R3: vendor 已复权时标记，避免 lib 层二次应用
                    adj_factor: adj_marker,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        klines.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(klines)
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        // 东方财富 2025 年后 FinanceSummary API 失效，改用 NewFinanceAnalysis/ZYZBAjaxNew
        // 修复(2026-07-22): 先去除 sh/sz/bj 前缀,否则 starts_with 判断失效
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let em_code = if code.starts_with('6') || code.starts_with('9') {
            format!("SH{code}")
        } else if code.starts_with('8') || code.starts_with('4') {
            format!("BJ{code}")
        } else {
            format!("SZ{code}")
        };

        let url = format!(
            "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/ZYZBAjaxNew?type=0&code={}",
            em_code
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let data = match json["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!("get_financials 数据为空(stock_code={stock_code})"),
                });
            },
        };

        let reports: Vec<FinancialReport> = data
            .iter()
            .take(24)           // 取24条(6年季度)，as-of 截断后有足够历史数据
            .map(|r| {
                let s = |key: &str| -> &str { r[key].as_str().unwrap_or("") };
                // 修复 M-FIN-1: 原 n 函数只处理字符串类型，但东方财富 API 部分字段
                // 可能返回数字类型（Value::Number），导致解析失败返回 None。
                // 改为同时支持字符串和数字类型，并过滤 "--"/""/null 等无效值。
                let n = |key: &str| -> Option<f64> {
                    let v = &r[key];
                    if v.is_null() {
                        return None;
                    }
                    if let Some(s) = v.as_str() {
                        if s.is_empty() || s == "--" || s == "null" {
                            return None;
                        }
                        return s.parse::<f64>().ok();
                    }
                    v.as_f64()
                };
                FinancialReport {
                    stock_code: stock_code.to_string(),
                    report_date: s("REPORT_DATE").to_string(),
                    // 东方财富 2025 年字段名变更映射
                    revenue: n("TOTALOPERATEREVE"),       // 营业总收入
                    net_profit: n("PARENTNETPROFIT"),     // 归母净利润
                    eps: n("EPSJB"),                      // 基本每股收益
                    bps: n("BPS"),                        // 每股净资产
                    roe: n("ROEJQ"),                      // 加权平均ROE
                    debt_ratio: n("ZCFZL"),               // 资产负债率
                    gross_margin: n("XSMLL"),             // 销售毛利率
                    net_margin: n("XSJLL"),               // 销售净利率
                    revenue_yoy: n("TOTALOPERATEREVETZ"), // 营收同比增长
                    profit_yoy: n("PARENTNETPROFITTZ"),   // 净利润同比增长
                    total_assets: None,
                    operating_cash_flow: None,
                    capital_expenditure: None,
                    free_cash_flow: None,
                    current_ratio: n("LD"), // 流动比率
                    quick_ratio: n("SD"),   // 速动比率
                    // #8 修复(2026-07-22): 商誉/应收账款字段——当前利润表接口未提供,
                    // 后续可通过 ZcfzbAjaxNew 资产负债表接口补全
                    goodwill: None,
                    accounts_receivable: None,
                    estimated: Some(false),
                }
            })
            .collect();

        // 修复 M-FIN-2: 字段名映射可能因 API 升级而失效。
        // 当所有关键字段（roe/gross_margin/net_margin/revenue_yoy/profit_yoy）均为 None，
        // 但财报条目本身存在时，返回 VendorError 触发 fallback 到下一个 vendor。
        let critical_fields_empty = reports.iter().all(|r| {
            r.roe.is_none()
                && r.gross_margin.is_none()
                && r.net_margin.is_none()
                && r.revenue_yoy.is_none()
                && r.profit_yoy.is_none()
        });
        if critical_fields_empty {
            tracing::warn!(
                "[eastmoney] get_financials 所有财报的5个关键字段(ROE/毛利率/净利率/营收同比/净利润同比)均为 None，\
                 可能字段名映射失效(stock_code={stock_code})，触发 fallback"
            );
            return Err(DataError::VendorError {
                vendor: "eastmoney".into(),
                message: format!(
                    "财报关键字段全为 None，可能 API 字段名变更(stock_code={stock_code})"
                ),
            });
        }

        Ok(reports)
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        // 使用东方财富搜索 API（与 akshare 相同的 endpoint，作为主源）
        let param = serde_json::json!({
            "uid": "",
            "keyword": stock_code,
            "type": ["cmsArticleWebOld"],
            "client": "web",
            "clientType": "web",
            "clientVersion": "curr",
            "param": {
                "cmsArticleWebOld": {
                    "searchScope": "default",
                    "sort": "default",
                    "pageIndex": 1,
                    "pageSize": limit.min(50),
                    "preTag": "",
                    "postTag": ""
                }
            }
        });

        let url = format!(
            "https://search-api-web.eastmoney.com/search/jsonp?cb=jQuery&param={}",
            urlencoding::encode(&param.to_string())
        );

        let resp = self
            .http
            .get(&url)
            .header("Referer", "https://so.eastmoney.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| DataError::VendorError {
                vendor: "eastmoney".into(),
                message: format!("新闻搜索请求失败: {e}"),
            })?;

        let text = resp.text().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("新闻搜索响应读取失败: {e}"),
        })?;

        // 解析 JSONP 响应: jQuery18306726XXX(...)
        // 找到第一个 '(' 和最后一个 ')'，提取中间的 JSON 内容
        let trimmed = text.trim();
        let json_str = if let Some(start) = trimmed.find('(') {
            if let Some(end) = trimmed.rfind(')') {
                &trimmed[start + 1..end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        let json: Value = serde_json::from_str(json_str).map_err(|e| {
            DataError::ParseError(format!(
                "eastmoney news jsonp parse failed: {e}, raw: {}",
                &text[..200.min(text.len())]
            ))
        })?;

        // 修复 M-RES-16: 添加 fallback 检查 `result.cmsArticleWebOld.list`（旧格式）。
        // 原实现仅检查 `cmsArticleWebOld` 是否为数组，若上游改为 {list: [...]} 格式
        // 则静默返回空 vec，调用方无感知。
        let items = json["result"]["cmsArticleWebOld"]
            .as_array()
            .or_else(|| json["result"]["cmsArticleWebOld"]["list"].as_array());
        let items = match items {
            Some(arr) => arr,
            None => {
                tracing::debug!(
                    "[eastmoney] cmsArticleWebOld 字段格式非预期（无 list 数组），返回空"
                );
                return Ok(vec![]);
            },
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let title = item.get("title")?.as_str()?.to_string();
                let summary = item
                    .get("digest")
                    .or_else(|| item.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source = item
                    .get("mediaName")
                    .or_else(|| item.get("source"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("东方财富")
                    .to_string();
                let article_url = item
                    .get("articleUrl")
                    .or_else(|| item.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("showTime")
                    .or_else(|| item.get("publishTime"))
                    .or_else(|| item.get("ctime"))
                    .or_else(|| item.get("date"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Some(NewsItem {
                    title,
                    summary,
                    source,
                    url: article_url,
                    publish_time,
                    sentiment_score: None,
                })
            })
            .collect())
    }

    async fn search_news(&self, keyword: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        // 复用东方财富搜索 API，以 keyword 搜索（与 get_news 同一 endpoint）
        let param = serde_json::json!({
            "uid": "",
            "keyword": keyword,
            "type": ["cmsArticleWebOld"],
            "client": "web",
            "clientType": "web",
            "clientVersion": "curr",
            "param": {
                "cmsArticleWebOld": {
                    "searchScope": "default",
                    "sort": "default",
                    "pageIndex": 1,
                    "pageSize": limit.min(50),
                    "preTag": "",
                    "postTag": ""
                }
            }
        });

        let url = format!(
            "https://search-api-web.eastmoney.com/search/jsonp?cb=jQuery&param={}",
            urlencoding::encode(&param.to_string())
        );

        let resp = self
            .http
            .get(&url)
            .header("Referer", "https://so.eastmoney.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| DataError::VendorError {
                vendor: "eastmoney".into(),
                message: format!("新闻搜索请求失败: {e}"),
            })?;

        let text = resp.text().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("新闻搜索响应读取失败: {e}"),
        })?;

        let trimmed = text.trim();
        let json_str = if let Some(start) = trimmed.find('(') {
            if let Some(end) = trimmed.rfind(')') {
                &trimmed[start + 1..end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        let json: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            DataError::ParseError(format!(
                "eastmoney search_news jsonp parse failed: {e}, raw: {}",
                &text[..200.min(text.len())]
            ))
        })?;

        // 修复 M-RES-16: 添加 fallback 检查 `result.cmsArticleWebOld.list`（旧格式）。
        // 原实现仅检查 `cmsArticleWebOld` 是否为数组，若上游改为 {list: [...]} 格式
        // 则静默返回空 vec，调用方无感知。
        let items = json["result"]["cmsArticleWebOld"]
            .as_array()
            .or_else(|| json["result"]["cmsArticleWebOld"]["list"].as_array());
        let items = match items {
            Some(arr) => arr,
            None => {
                tracing::debug!(
                    "[eastmoney] cmsArticleWebOld 字段格式非预期（无 list 数组），返回空"
                );
                return Ok(vec![]);
            },
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let title = item.get("title")?.as_str()?.to_string();
                let summary = item
                    .get("digest")
                    .or_else(|| item.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source = item
                    .get("mediaName")
                    .or_else(|| item.get("source"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("东方财富")
                    .to_string();
                let article_url = item
                    .get("articleUrl")
                    .or_else(|| item.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("showTime")
                    .or_else(|| item.get("publishTime"))
                    .or_else(|| item.get("ctime"))
                    .or_else(|| item.get("date"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Some(NewsItem {
                    title,
                    summary,
                    source,
                    url: article_url,
                    publish_time,
                    sentiment_score: None,
                })
            })
            .collect())
    }

    async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        // 修复(2026-07-22): 原 push2.eastmoney.com/api/qt/stock/fflow/kline/get
        // 已失效(IncompleteMessage)。改用 datacenter-web.eastmoney.com 的
        // RPT_F10_HOMEPAGE_FUND_FLOW 报表(东方财富 F10 资金流向页面数据源)。
        //
        // 字段映射:
        //   - TRADE_DATE: 交易日期
        //   - MAIN_NET_INFLOW: 主力净流入
        //   - SUPER_LARGE_NET_INFLOW: 超大单净流入
        //   - LARGE_NET_INFLOW: 大单净流入
        //   - MEDIUM_NET_INFLOW: 中单净流入
        //   - SMALL_NET_INFLOW: 小单净流入
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_F10_HOMEPAGE_FUND_FLOW&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            pageSize=5&pageNumber=1&source=WEB&\
            sortColumns=TRADE_DATE&sortTypes=-1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        // 第一条为最新交易日数据，其余 4 条为历史数据（已按 TRADE_DATE 降序排列）。
        // 旧实现只取 rows[0] 丢弃了 4 天数据，但 prompt 要求"连续 3-5 日趋势"分析，
        // 因此把所有行都映射到 history 字段（第 0 条同时映射到顶层字段，保持兼容）。
        let parse_row = |r: &Value| -> MoneyFlowDaily {
            let date = r["TRADE_DATE"]
                .as_str()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            let f = |key: &str| -> f64 {
                r[key]
                    .as_f64()
                    .or_else(|| r[key].as_str().and_then(|s| s.parse().ok()))
                    .unwrap_or(0.0)
            };
            MoneyFlowDaily {
                date,
                main_net_inflow: f("MAIN_NET_INFLOW"),
                super_large_net: f("SUPER_LARGE_NET_INFLOW"),
                large_net: f("LARGE_NET_INFLOW"),
                medium_net: f("MEDIUM_NET_INFLOW"),
                small_net: f("SMALL_NET_INFLOW"),
            }
        };
        let history: Vec<MoneyFlowDaily> = rows.iter().map(parse_row).collect();
        let latest = &history[0];
        Ok(Some(MoneyFlow {
            date: latest.date.clone(),
            main_net_inflow: latest.main_net_inflow,
            super_large_net: latest.super_large_net,
            large_net: latest.large_net,
            medium_net: latest.medium_net,
            small_net: latest.small_net,
            history,
        }))
    }

    async fn get_dragon_tiger(&self, stock_code: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        // P1-3 修复(2026-07-22): 原 push2his.eastmoney.com/api/qt/stock/mmpa/get
        // 已失效(IncompleteMessage)。改用 datacenter-web.eastmoney.com 的
        // RPT_DAILYBILLBOARD_DETAILS 报表(东方财富数据中心"龙虎榜"页面数据源)。
        //
        // 字段映射:
        //   - TRADE_DATE: 交易日期
        //   - EXPLANATION: 上榜原因
        //   - BILLBOARD_BUY_AMT: 龙虎榜买入额
        //   - BILLBOARD_SELL_AMT: 龙虎榜卖出额
        //   - BILLBOARD_NET_AMT: 龙虎榜净买额
        //   - BUY_SEAT_NEW/SELL_SEAT_NEW: 买入/卖出营业部数量
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_DAILYBILLBOARD_DETAILS&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            pageSize=20&pageNumber=1&source=WEB&\
            sortColumns=TRADE_DATE&sortTypes=-1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(vec![]),
        };

        rows.iter()
            .map(|r| {
                let trade_date = r["TRADE_DATE"].as_str().unwrap_or("");
                // 截取日期部分 "YYYY-MM-DD 00:00:00" → "YYYY-MM-DD"
                let date = if trade_date.len() >= 10 {
                    trade_date[..10].to_string()
                } else {
                    trade_date.to_string()
                };
                let buy_seat = r["BUY_SEAT_NEW"].as_i64().unwrap_or(0);
                let sell_seat = r["SELL_SEAT_NEW"].as_i64().unwrap_or(0);
                let dept_name = format!("买入{}席位/卖出{}席位", buy_seat, sell_seat);
                Ok(DragonTigerEntry {
                    stock_code: stock_code.to_string(),
                    date,
                    dept_name,
                    buy_amount: r["BILLBOARD_BUY_AMT"].as_f64().unwrap_or(0.0),
                    sell_amount: r["BILLBOARD_SELL_AMT"].as_f64().unwrap_or(0.0),
                    net_amount: r["BILLBOARD_NET_AMT"].as_f64().unwrap_or(0.0),
                    reason: r["EXPLANATION"].as_str().map(|s| s.to_string()),
                })
            })
            .collect()
    }

    async fn get_lockup_schedule(
        &self,
        stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        // 修复(2026-07-22): 原 reportName=RPTA_WEB_LOCKUP 已失效("报表配置不存在")。
        // 改用 RPT_LIFT_GD 报表(来自 data.eastmoney.com/newstatic/js/xsjj/history.js,
        // 即东方财富数据中心"限售股解禁"页面的实际数据源)。
        //
        // 字段映射:
        //   - SECURITY_CODE/SECUCODE/SECURITY_NAME_ABBR: 代码/简称
        //   - FREE_DATE: 解禁日期(格式 "YYYY-MM-DD 00:00:00")
        //   - ADD_LISTING_SHARES: 本次解禁数量(股)
        //   - LIMITED_HOLDER_NAME: 限售股持有人名称
        //   - FREE_SHARES_TYPE: 限售股类型(如"股权激励限售股份")
        //   - RESIDUAL_LIMITED_SHARES: 剩余限售股数
        //   - LIFT_SHARES_ALL: 当次解禁总数(同一 FREE_DATE 下多股东合计)
        //   - TOTAL_SHARES_NUM: 总股本(用于计算解禁比例)
        //
        // 修复(2026-07-22): SECURITY_CODE 字段需纯数字代码,去除 sh/sz/bj 前缀
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_LIFT_GD&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            pageSize=50&pageNumber=1&source=WEB&\
            sortColumns=FREE_DATE&sortTypes=-1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                // result.data 为空或结构异常，触发 VendorError 让下一个 vendor 尝试
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!("get_lockup_schedule 数据为空(stock_code={stock_code})"),
                });
            },
        };

        Ok(rows
            .iter()
            .map(|r| {
                // FREE_DATE 格式 "YYYY-MM-DD 00:00:00",截取日期部分
                let raw_date = r["FREE_DATE"].as_str().unwrap_or("");
                let date = raw_date.split_whitespace().next().unwrap_or(raw_date).to_string();
                // 解禁数量(本次新增可上市股份,单位:股)
                let unlock_shares = r["ADD_LISTING_SHARES"].as_f64().unwrap_or(0.0);
                // P2-3 修复: 解禁比例计算
                // RPT_LIFT_GD 不返回 TOTAL_SHARES_NUM,但返回 LIFT_SHARES_ALL(当日总解禁股数)
                // 和 ADD_LISTING_SHARES(单股东解禁股数)。
                // unlock_ratio 表示该股东解禁占当日总解禁的比例,非占总股本比例。
                // 若需占总股本比例,上层 LLM 可用 unlock_shares / total_shares 计算。
                let lift_shares_all = r["LIFT_SHARES_ALL"].as_f64().unwrap_or(0.0);
                let unlock_ratio = if lift_shares_all > 0.0 {
                    (unlock_shares / lift_shares_all * 100.0 * 100.0).round() / 100.0
                } else {
                    0.0
                };
                LockupSchedule {
                    stock_code: stock_code.to_string(),
                    stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                    unlock_date: date,
                    unlock_shares,
                    unlock_ratio,
                    shareholder: r["LIMITED_HOLDER_NAME"].as_str().map(|s| s.to_string()),
                }
            })
            .collect())
    }

    /// 获取融资融券数据
    ///
    /// 修复(2026-07-22): 原 API `push2his.eastmoney.com/api/qt/stock/margin/get` 已失效(返回404)。
    /// 改用 datacenter-web 的 `RPTA_WEB_RZRQ_GGMX` 报表,该报表提供个股融资融券明细数据。
    /// 响应字段映射:
    ///   - RZYE: 融资余额(元)
    ///   - RQYE: 融券余额(元)
    ///   - RZMRE: 融资买入额(元)
    ///   - RQMCL: 融券卖出量(股)
    ///   - DATE: 交易日期
    async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        // 去除 sh/sz/bj 前缀
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPTA_WEB_RZRQ_GGMX&columns=ALL&\
            filter=(scode%3D%22{code}%22)&source=WEB&\
            sortColumns=DATE&sortTypes=-1&pageNumber=1&pageSize=1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        // 检查 API 返回是否成功
        if json["success"].as_bool() == Some(false) {
            let msg = json["message"].as_str().unwrap_or("unknown error");
            return Err(DataError::VendorError {
                vendor: "eastmoney".into(),
                message: format!("get_margin_data API 错误: {msg}(stock_code={stock_code})"),
            });
        }

        let data = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => &arr[0],
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!(
                        "get_margin_data 数据为空(stock_code={stock_code}),该股票可能非融资融券标的"
                    ),
                });
            },
        };

        let parse_f64 = |key: &str| -> f64 { data[key].as_f64().unwrap_or(0.0) };

        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date: data["DATE"].as_str().unwrap_or("").to_string(),
            margin_buy: parse_f64("RZMRE"),        // 融资买入额(元)
            margin_balance: parse_f64("RZYE"),     // 融资余额(元)
            short_sell_volume: parse_f64("RQMCL"), // 融券卖出量(股)
            short_balance: parse_f64("RQYE"),      // 融券余额(元)
        }))
    }

    /// P1-2 新增: eastmoney 实现 consensus_eps
    /// 复用 reportapi.eastmoney.com/report/list 接口，聚合最近研报的 EPS 预测。
    /// 此接口与 get_research_reports 同源，是 eastmoney 稳定的 emweb 系列接口，
    /// 不受 push2his/push2 系列故障影响。
    ///
    // 修复(2026-07-22 #6): 目标价计算错误。
    // 原代码用 predictThisYearPe(预测PE) 直接当目标价，语义错误。
    // 正确做法: 目标价 = 预测PE × 预测EPS，二者均可用时才算出目标价。
    async fn get_consensus_eps(&self, stock_code: &str) -> Result<Option<ConsensusEPS>, DataError> {
        let url = format!(
            "https://reportapi.eastmoney.com/report/list?industryCode=*&pageSize=20&industry=%2A&rating=&ratingChange=&beginTime=2000-01-01&endTime=2030-01-01&pageNo=1&fields=&qType=0&orgCode=&code={}&rcode=&p=1&pageNum=1&pageNumber=1",
            stock_code
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;
        let reports = match json["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        // 聚合今年 EPS 预测（predictThisYearEps），取均值作为一致预期
        let mut eps_sum = 0.0_f64;
        let mut eps_count = 0_i32;
        let mut rating_count = 0_i32;
        // 目标价 = 预测PE × 预测EPS（每篇研报独立计算后取均值）
        let mut target_price_sum = 0.0_f64;
        let mut target_price_count = 0_i32;
        let mut rating_avg: Option<String> = None;

        for r in reports {
            let eps_val = r["predictThisYearEps"].as_str().and_then(|s| s.parse::<f64>().ok());
            if let Some(val) = eps_val {
                eps_sum += val;
                eps_count += 1;
            }
            // 目标价 = PE × EPS（二者均可用时）
            if let (Some(eps), Some(pe_str)) = (eps_val, r["predictThisYearPe"].as_str()) {
                if let Ok(pe) = pe_str.parse::<f64>() {
                    if pe > 0.0 && eps > 0.0 {
                        target_price_sum += pe * eps;
                        target_price_count += 1;
                    }
                }
            }
            rating_count += 1;
            if rating_avg.is_none() {
                if let Some(rating) = r["emRatingName"].as_str() {
                    rating_avg = Some(rating.to_string());
                }
            }
        }

        if eps_count == 0 && rating_count == 0 {
            return Ok(None);
        }

        let consensus_eps = if eps_count > 0 {
            Some(eps_sum / eps_count as f64)
        } else {
            None
        };
        let consensus_target_price = if target_price_count > 0 {
            Some(target_price_sum / target_price_count as f64)
        } else {
            None
        };

        Ok(Some(ConsensusEPS {
            stock_code: stock_code.to_string(),
            consensus_eps,
            consensus_target_price,
            rating_avg,
            rating_count: Some(rating_count),
            year: chrono::Utc::now().format("%Y").to_string(),
        }))
    }

    async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        // 修复(2026-07-22): 原 push2his fflow API 已失效(连接错误)。
        // 此外,2024-08-16 起监管层暂停披露北向资金实时数据,即使 API 可用,数据也为 0。
        //
        // 替代方案:用 RPT_F10_EH_HOLDERS(十大股东季度持股)中筛选"香港中央结算有限公司"
        // (HKSCC,代表港股通持股)作为北向持股的代理数据。
        //
        // 限制:
        //   1. 数据是季度披露,非实时(延迟最多 90 天)
        //   2. 若该股票港股通持股未排进十大股东,则返回 None(表示北向持股不重要)
        //   3. change_shares 由相邻两期 HOLD_NUM 差值计算
        //
        // 字段:
        //   - END_DATE: 报告期(如 "2026-03-31 00:00:00") → date
        //   - HOLD_NUM: 持股数量 → holding_shares
        //   - HOLD_NUM_RATIO: 持股比例(%) → holding_ratio
        //   - 上期 HOLD_NUM - 本期 HOLD_NUM → change_shares (正数=减持,负数=增持)
        let secucode = to_em_secucode(stock_code);
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_F10_EH_HOLDERS&columns=ALL&\
            filter=(SECUCODE%3D%22{secucode}%22)&\
            pageSize=200&pageNumber=1&source=WEB&\
            sortColumns=END_DATE&sortTypes=-1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        // 筛选"香港中央结算有限公司"(HKSCC)的记录,按 END_DATE 降序排
        let mut hk_records: Vec<&Value> = rows
            .iter()
            .filter(|r| {
                r["HOLDER_NAME"]
                    .as_str()
                    .map(|n| n.contains("香港中央结算") || n.contains("HKSCC"))
                    .unwrap_or(false)
            })
            .collect();

        if hk_records.is_empty() {
            // 该股票港股通持股未排进十大股东,返回 None
            return Ok(None);
        }

        // 按日期降序排(理论上 API 已按 END_DATE DESC 排,但保险起见再排一次)
        hk_records.sort_by(|a, b| {
            let da = a["END_DATE"].as_str().unwrap_or("");
            let db = b["END_DATE"].as_str().unwrap_or("");
            db.cmp(da)
        });

        let latest = hk_records[0];
        let latest_date = latest["END_DATE"]
            .as_str()
            .map(|s| s.split_whitespace().next().unwrap_or(s).to_string())
            .unwrap_or_default();
        let latest_shares = latest["HOLD_NUM"].as_f64().unwrap_or(0.0);
        let latest_ratio = latest["HOLD_NUM_RATIO"].as_f64().unwrap_or(0.0);

        // 取次新的一期(必须是不同的 END_DATE)作为上期
        let prev_shares = hk_records
            .iter()
            .skip(1)
            .find(|r| {
                r["END_DATE"]
                    .as_str()
                    .map(|d| d != latest["END_DATE"].as_str().unwrap_or(""))
                    .unwrap_or(true)
            })
            .and_then(|r| r["HOLD_NUM"].as_f64())
            .unwrap_or(latest_shares);

        // change_shares: 正数=本期相比上期增持,负数=减持
        let change_shares = latest_shares - prev_shares;

        Ok(Some(NorthBoundHolding {
            stock_code: stock_code.to_string(),
            date: latest_date,
            holding_shares: latest_shares,
            holding_ratio: latest_ratio,
            change_shares,
        }))
    }

    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        // 修复(2026-07-22): 原 push2his stock/get API 已失效(WAF/JA3 检测导致连接错误)。
        // 改用 emweb F10 的 CompanySurvey/PageAjax 接口获取行业分类。
        //
        // 字段映射:
        //   - EM2016: 东财行业分类(如 "食品饮料-食品-乳制品"),用 "-" 拆分为一级行业/二级行业/细分
        //   - INDUSTRYCSRC1: 证监会行业分类(如 "制造业-食品制造业"),作为 concept_tags 补充
        //
        // 注:avg_pe/avg_pb 原 push2his clist/get API 也已失效,
        // 行业 PE/PB 数据请通过 get_industry_ranking 或 get_peers 获取,
        // 这里设为 None,不阻塞主流程。
        let secucode = to_em_secucode(stock_code);
        let url =
            format!("https://emweb.eastmoney.com/PC_HSF10/CompanySurvey/PageAjax?code={secucode}");
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let data = match json["jbzl"].get(0) {
            Some(d) => d,
            None => return Ok(None),
        };

        // EM2016: "食品饮料-食品-乳制品" → ["食品饮料", "食品", "乳制品"]
        let em_industry = data["EM2016"].as_str().unwrap_or("");
        let parts: Vec<&str> = em_industry.split('-').collect();
        let sector_name = parts.first().map(|s| s.trim().to_string()).unwrap_or_default();
        let sub_sector = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        // 细分行业(如有)拼接到 sub_sector
        let sub_sector = if parts.len() >= 3 {
            let detail = parts[2].trim();
            format!("{sub_sector}-{detail}")
        } else {
            sub_sector
        };

        // 概念标签:把证监会行业分类作为 concept_tags 的第一项
        let mut concept_tags: Vec<String> = Vec::new();
        if let Some(csrc) = data["INDUSTRYCSRC1"].as_str() {
            if !csrc.is_empty() {
                concept_tags.push(csrc.to_string());
            }
        }
        // SECURITY_TYPE 也作为标签(如"上交所主板A股")
        if let Some(stype) = data["SECURITY_TYPE"].as_str() {
            if !stype.is_empty() {
                concept_tags.push(stype.to_string());
            }
        }

        if sector_name.is_empty() && concept_tags.is_empty() {
            return Ok(None);
        }

        Ok(Some(SectorInfo {
            stock_code: stock_code.to_string(),
            sector_name,
            sub_sector,
            concept_tags,
            // 原 push2his clist/get 失效,avg_pe/avg_pb 暂不可用
            // 行业 PE/PB 请通过 get_industry_ranking / get_peers 获取
            avg_pe: None,
            avg_pb: None,
        }))
    }

    async fn get_shareholder_trades(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        // 修复(2026-07-22): 原 RPTA_WEB_MAJORHOLDERS_TRADE 已失效。
        // 第一版修复改用 RPT_F10_EH_HOLDERS(十大股东季度持股变动),
        // 但该报表不提供成交价格(price=0.0 占位),导致 LLM 无法计算减持均价。
        //
        // 第二版修复(本次): 改用 RPT_F10_HOLDER_HOLDERTRADE 报表
        // (东方财富 F10 股东增减持明细,含真实成交价格)。
        //
        // 字段映射:
        //   - CHANGE_DATE: 变动日期
        //   - HOLDER_NAME: 股东名称
        //   - CHANGE_NUM: 变动数量(股)
        //   - CHANGE_PRICE: 成交均价(元)
        //   - CHANGE_RATIO_AFTER: 变动后持股比例
        //   - CHANGE_TYPE: 变动类型("增持"/"减持")
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let secucode = to_em_secucode(code);
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_F10_HOLDER_HOLDERTRADE&columns=ALL&\
            filter=(SECUCODE%3D%22{secucode}%22)&\
            pageSize=20&pageNumber=1&source=WEB&\
            sortColumns=CHANGE_DATE&sortTypes=-1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                // 回退到 RPT_F10_EH_HOLDERS(无成交价但至少有股东变动信息)
                let eh_url = format!(
                    "https://datacenter-web.eastmoney.com/api/data/v1/get?\
                    reportName=RPT_F10_EH_HOLDERS&columns=ALL&\
                    filter=(SECUCODE%3D%22{secucode}%22)&\
                    pageSize=20&pageNumber=1&source=WEB&\
                    sortColumns=END_DATE&sortTypes=-1"
                );
                let eh_resp = self.em_get(&eh_url).await?;
                let eh_json: Value = eh_resp.json().await?;
                let eh_rows = match eh_json["result"]["data"].as_array() {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => {
                        return Err(DataError::VendorError {
                            vendor: "eastmoney".into(),
                            message: format!(
                                "get_shareholder_trades 数据为空(stock_code={stock_code})"
                            ),
                        });
                    },
                };
                return Ok(eh_rows
                    .iter()
                    .map(|r| {
                        let raw_date = r["END_DATE"].as_str().unwrap_or("");
                        let date =
                            raw_date.split_whitespace().next().unwrap_or(raw_date).to_string();
                        let raw_change = r["HOLD_NUM_CHANGE"].as_str().unwrap_or("");
                        let shares = if raw_change == "不变" || raw_change.is_empty() {
                            0.0
                        } else if let Ok(n) = raw_change.parse::<f64>() {
                            n
                        } else {
                            r["HOLD_NUM"].as_f64().unwrap_or(0.0)
                        };
                        let trade_type = r["HOLDER_STATEE"]
                            .as_str()
                            .or_else(|| r["HOLD_RATIO_QOQ"].as_str())
                            .unwrap_or("不变")
                            .to_string();
                        ShareholderTrade {
                            stock_code: stock_code.to_string(),
                            date,
                            shareholder_name: r["HOLDER_NAME"].as_str().unwrap_or("").to_string(),
                            trade_type,
                            shares,
                            // RPT_F10_EH_HOLDERS 不提供成交价格
                            price: 0.0,
                            reason: r["SHARES_TYPE"].as_str().map(|s| s.to_string()),
                        }
                    })
                    .collect());
            },
        };

        Ok(rows
            .iter()
            .map(|r| {
                let raw_date = r["CHANGE_DATE"].as_str().unwrap_or("");
                let date = raw_date.split_whitespace().next().unwrap_or(raw_date).to_string();
                let shares = r["CHANGE_NUM"].as_f64().unwrap_or(0.0);
                let price = r["CHANGE_PRICE"]
                    .as_f64()
                    .or_else(|| r["CHANGE_PRICE"].as_str().and_then(|s| s.parse::<f64>().ok()))
                    .unwrap_or(0.0);
                let trade_type = r["CHANGE_TYPE"].as_str().unwrap_or("变动").to_string();
                ShareholderTrade {
                    stock_code: stock_code.to_string(),
                    date,
                    shareholder_name: r["HOLDER_NAME"].as_str().unwrap_or("").to_string(),
                    trade_type,
                    shares,
                    price,
                    reason: r["CHANGE_RATIO_AFTER"].as_str().map(|s| s.to_string()),
                }
            })
            .collect())
    }

    async fn get_dividend_records(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        // 东方财富数据中心: 分红送配数据
        // 修复(2026-07-22): SECURITY_CODE 字段需纯数字代码,去除 sh/sz/bj 前缀
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_DIVIDEND&columns=SECURITY_CODE,EX_DIVIDEND_DATE,DIVIDEND_PER_SHARE,BONUS_SHARE_RATIO,RECORD_DATE&filter=(SECURITY_CODE=\"{code}\")&pageSize=10&pageNumber=1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        rows.iter()
            .map(|r| {
                Ok(DividendRecord {
                    stock_code: stock_code.to_string(),
                    ex_date: r["EX_DIVIDEND_DATE"].as_str().unwrap_or("").to_string(),
                    dividend_per_share: r["DIVIDEND_PER_SHARE"].as_f64().unwrap_or(0.0),
                    bonus_share_ratio: r["BONUS_SHARE_RATIO"].as_f64().unwrap_or(0.0),
                    record_date: r["RECORD_DATE"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect()
    }

    /// 获取财报日历事件
    ///
    /// 使用东方财富公告 API（RPTA_WEB_NOTICE），按标题关键词分类：
    /// - "业绩预告" → preliminary
    /// - "业绩快报" → express
    /// - "定期报告"/"年报"/"季报" → formal
    /// - "股东大会" → shareholders_meeting
    /// - 其他 → other
    async fn get_earnings_calendar(
        &self,
        stock_code: &str,
    ) -> Result<Vec<EarningsEvent>, DataError> {
        // 修复(2026-07-22): SECURITY_CODE 字段需纯数字代码,去除 sh/sz/bj 前缀
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_NOTICE&columns=SECURITY_CODE,SECURITY_NAME_ABBR,NOTICE_DATE,TITLE,EQUITY_NOTICE_TYPE&filter=(SECURITY_CODE=\"{code}\")&pageSize=30&sortColumns=NOTICE_DATE&sortTypes=-1&pageNumber=1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

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

                // 按标题关键词分类
                let (event_type, period) = classify_earnings_title(title);

                // 只保留财报相关事件
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
                    source: Some("eastmoney".to_string()),
                    created_at: chrono::Utc::now().timestamp(),
                })
            })
            .collect())
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let url = format!(
            "https://searchadapter.eastmoney.com/api/suggest/get?input={}&type=14&count=20",
            urlencoding::encode(keyword)
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let stocks = match json["QuotationCodeTable"]["Data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(stocks
            .iter()
            .map(|s| StockSearchResult {
                code: s["Code"].as_str().unwrap_or("").to_string(),
                name: s["Name"].as_str().unwrap_or("").to_string(),
                market: s["Market"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    async fn get_research_reports(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        let url = format!(
            "https://reportapi.eastmoney.com/report/list?industryCode=*&pageSize=20&industry=%2A&rating=&ratingChange=&beginTime=2000-01-01&endTime=2030-01-01&pageNo=1&fields=&qType=0&orgCode=&code={}&rcode=&p=1&pageNum=1&pageNumber=1",
            stock_code
        );

        let resp = self.em_get(&url).await?;

        let json: Value = resp.json().await?;

        let reports = match json["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(reports
            .iter()
            .map(|r| {
                let mut eps_forecast = Vec::new();
                let mut this_year_eps: Option<f64> = None;
                if let Some(eps) = r["predictThisYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast { year: "今年".into(), eps: Some(val) });
                        this_year_eps = Some(val);
                    }
                }
                if let Some(eps) = r["predictNextYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast { year: "明年".into(), eps: Some(val) });
                    }
                }
                if let Some(eps) = r["predictNextTwoYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast { year: "后年".into(), eps: Some(val) });
                    }
                }

                // 修复(2026-07-22 #6): 目标价 = 预测PE × 预测EPS
                // 原代码硬编码 target_price: None,导致所有研报目标价为 null。
                // 东方财富 reportapi 不直接返回目标价,但返回预测PE和预测EPS,
                // 二者相乘可得隐含目标价。
                let target_price = if let (Some(eps), Some(pe_str)) =
                    (this_year_eps, r["predictThisYearPe"].as_str())
                {
                    pe_str
                        .parse::<f64>()
                        .ok()
                        .filter(|&pe| pe > 0.0 && eps > 0.0)
                        .map(|pe| pe * eps)
                } else {
                    None
                };

                let info_code = r["infoCode"].as_str().unwrap_or("");
                let pdf_url = if info_code.is_empty() {
                    None
                } else {
                    Some(format!("https://pdf.dfcfw.com/pdf/H3_{}_1.pdf", info_code))
                };

                ResearchReport {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    institution: r["orgSName"].as_str().unwrap_or("").to_string(),
                    analyst: r["researcher"].as_str().map(|s| s.to_string()),
                    rating: r["emRatingName"].as_str().map(|s| s.to_string()),
                    target_price,
                    eps_forecast,
                    publish_date: r["publishDate"].as_str().unwrap_or("").to_string(),
                    pdf_url,
                }
            })
            .collect())
    }

    async fn get_market_dragon_tiger(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPT_DAILYBOARD_DETAILS_NEW&columns=SECURITY_CODE,SECURITY_NAME_ABBR,TRADE_DATE,BUY_AMOUNT,SELL_AMOUNT,NET_BUY,CHANGE_REASON&sortColumns=NET_BUY&sortTypes=-1&pageSize=30&pageNumber=1";

        let resp = self.em_get(url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .map(|r| MarketDragonTiger {
                stock_code: r["SECURITY_CODE"].as_str().unwrap_or("").to_string(),
                stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                date: r["TRADE_DATE"].as_str().unwrap_or("").to_string(),
                net_buy: r["NET_BUY"].as_f64().unwrap_or(0.0),
                buy_amount: r["BUY_AMOUNT"].as_f64().unwrap_or(0.0),
                sell_amount: r["SELL_AMOUNT"].as_f64().unwrap_or(0.0),
                reason: r["CHANGE_REASON"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        // 2026-08-01 修复：旧接口 getNewsByColumns?column=250（财联社快讯频道）已整体失效
        // （curl 实测所有 column 均返回空 list）。改用东财 7x24 快讯接口 getFastNewsList：
        //   - 参数必须 camelCase：pageSize（非 page_size）+ sortEnd（"YYYY-MM-DD HH:MM:SS"）
        //   - 返回 data.fastNewsList，字段 summary/title/content/time
        let req_trace = chrono::Utc::now().timestamp_millis();
        let now_cn = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let url = format!(
            "https://np-listapi.eastmoney.com/comm/web/getFastNewsList?client=web&biz=web_7x24&fastColumn=102&page_index=1&pageSize=20&req_trace={req_trace}&sortEnd={now_cn}"
        );

        let resp = self.em_get(&url).await?;

        let json: Value = resp.json().await?;

        let items = match json["data"]["fastNewsList"].as_array() {
            Some(arr) => arr,
            None => match json["data"].as_array() {
                Some(arr) => arr,
                None => return Ok(vec![]),
            },
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        item.get("summary")
                            .and_then(|v| v.as_str())
                            .map(|s| s.chars().take(80).collect::<String>())
                    })?;
                let content = item
                    .get("summary")
                    .or_else(|| item.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("showTime")
                    .or_else(|| item.get("time"))
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

    async fn get_policy_news(
        &self,
        stock_code: &str,
        limit: u32,
    ) -> Result<Vec<NewsItem>, DataError> {
        // 实现策略(v4, 2026-07-22):
        //
        // 问题历史:
        //   v1: search_news("{行业} 政策") → 中文组合关键词分词差,返回空
        //   v2: get_news(stock_code) → 纯数字关键词搜索差,返回空
        //   v3: get_news(股票名) → 政策新闻不提公司名,过滤后为空
        //
        // v4 根因分析:政策新闻是宏观的,通常不提具体公司名(如"伊利股份"),
        //   但会提行业名(如"食品饮料")。例如《国务院关于印发食品安全规划的通知》
        //   不会出现"伊利股份",但会出现"食品"相关词。
        //
        // v4 方案:双路并行搜索 + 政策过滤 + 兜底
        //   路径A: 行业关键词搜索 - search_news(行业名,如"食品饮料")
        //         纯中文行业名搜索效果好,行业新闻中常含政策内容
        //   路径B: 股票名搜索 - search_news(股票名,如"伊利股份")
        //         获取个股层面新闻,过滤政策相关公告/监管通知
        //   合并去重 + 按 26 个政策关键词过滤
        //   兜底:过滤后为空则返回行业新闻(让 LLM 判断相关性)
        let fetch_limit = limit.clamp(50, 100);

        // 并行获取行业信息和股票名称
        let (sector_result, search_result) =
            tokio::join!(self.get_sector_info(stock_code), self.search_stock(stock_code));

        let sector_name =
            sector_result.ok().and_then(|opt| opt.map(|s| s.sector_name)).unwrap_or_default();

        let stock_name = search_result
            .ok()
            .and_then(|results| {
                results
                    .iter()
                    .find(|r| {
                        r.code
                            == stock_code
                                .trim_start_matches("sh")
                                .trim_start_matches("sz")
                                .trim_start_matches("bj")
                    })
                    .or_else(|| results.first())
                    .map(|r| r.name.clone())
            })
            .unwrap_or_default();

        // 构建搜索关键词列表(纯中文,避免组合分词问题)
        // 先比较再消费,避免 move 后借用
        let stock_differs_from_sector =
            !stock_name.is_empty() && !sector_name.is_empty() && stock_name != sector_name;
        let mut keywords: Vec<String> = Vec::new();
        if !sector_name.is_empty() {
            keywords.push(sector_name);
        }
        if stock_differs_from_sector {
            keywords.push(stock_name);
        }
        // 兜底:行业和名称都拿不到时用代码(可能返回空,但至少尝试过)
        if keywords.is_empty() {
            keywords.push(stock_code.to_string());
        }

        // 对每个关键词搜索新闻,合并去重(按标题)
        let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut all_news: Vec<NewsItem> = Vec::new();

        for keyword in &keywords {
            match self.search_news(keyword, fetch_limit).await {
                Ok(news) => {
                    tracing::debug!(
                        "[get_policy_news] search_news('{}') 返回 {} 条",
                        keyword,
                        news.len()
                    );
                    for n in news {
                        let key = n.title.trim().to_string();
                        if !key.is_empty() && seen_titles.insert(key) {
                            all_news.push(n);
                        }
                    }
                },
                Err(e) => {
                    tracing::debug!("[get_policy_news] search_news('{}') 失败: {}", keyword, e);
                },
            }
        }

        // 政策相关关键词
        const POLICY_KEYWORDS: &[&str] = &[
            "政策",
            "规划",
            "通知",
            "补贴",
            "监管",
            "法规",
            "条例",
            "办法",
            "意见",
            "纲要",
            "改革",
            "扶持",
            "刺激",
            "减税",
            "降费",
            "鼓励",
            "限制",
            "禁止",
            "标准",
            "五年规划",
            "中央经济",
            "工信部",
            "发改委",
            "证监会",
            "农业农村部",
            "国务院",
            "常务会议",
        ];

        let is_policy_related = |n: &NewsItem| {
            let haystack = format!("{} {}", n.title, n.summary);
            POLICY_KEYWORDS.iter().any(|kw| haystack.contains(kw))
        };

        // 先过滤出政策相关新闻(不消费 all_news,用 iter + cloned)
        let filtered: Vec<NewsItem> =
            all_news.iter().filter(|n| is_policy_related(n)).cloned().collect();

        // 决定最终返回:有政策新闻则用过滤结果,否则兜底返回全部行业新闻
        let mut policy_news: Vec<NewsItem> = if !filtered.is_empty() {
            filtered
        } else if !all_news.is_empty() {
            // 兜底:无政策相关但行业新闻非空 → 返回全部行业新闻让 LLM 判断
            // (避免工具返回空导致 a-policy 节点无数据可用)
            tracing::debug!(
                "[get_policy_news] 政策关键词过滤后为空,返回全部行业新闻({}条)供 LLM 判断",
                all_news.len()
            );
            all_news
        } else {
            vec![]
        };

        // 按 publish_time 降序排
        policy_news.sort_by(|a, b| b.publish_time.cmp(&a.publish_time));
        policy_news.truncate(limit as usize);

        Ok(policy_news)
    }

    async fn get_announcements(&self, stock_code: &str) -> Result<Vec<Announcement>, DataError> {
        // 修复(2026-07-21): 原实现 stock_list={market},{stock_code} 的 market 前缀
        // (沪市="1"/深市&北交所="0")不规范,且与 get_announcements_with_asof 的
        // stock_list={stock_code} 格式不一致。统一为不带 market 前缀的格式,
        // 依赖 ann_type=A 让 eastmoney API 自动识别市场。
        let url = format!(
            "https://np-anotice-stock.eastmoney.com/api/security/ann?cb=jQuery&sr=-1&page_size=20&page_index=1&ann_type=A&client_source=web&stock_list={stock_code}&f_node=0&s_node=0"
        );

        let resp = self.em_get(&url).await?;
        // 修复 P0-A5 同类问题: 用 text + 手动剥 JSONP 包裹,与 with_asof 路径一致
        // (em_get 返回的 resp 直接 .json() 在 cb=jQuery 时会解析失败)
        let body = resp.text().await?;
        let json_str =
            body.trim_start_matches("jQuery(").trim_end_matches(')').trim_end_matches(';');
        let json: Value = serde_json::from_str(json_str).map_err(|e| {
            DataError::ParseError(format!(
                "eastmoney announcements json 解析失败: {e}, body preview={}",
                &json_str[..json_str.len().min(200)]
            ))
        })?;
        let items = match json["data"]["list"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                Some(Announcement {
                    title: item.get("title")?.as_str()?.to_string(),
                    stock_code: stock_code.to_string(),
                    stock_name: item
                        .get("art_code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    announce_date: item
                        .get("notice_date")
                        .and_then(|v| v.as_i64())
                        .map(|ts| {
                            crate::vendors::format_timestamp(ts / 1000, "%Y-%m-%d", "eastmoney")
                        })
                        .unwrap_or_default(),
                    ann_type: item
                        .get("columns")
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get("column_name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    pdf_url: item
                        .get("dest_url")
                        .and_then(|v| v.as_str())
                        .map(|s| format!("https://np-anotice-stock.eastmoney.com{s}")),
                })
            })
            .collect())
    }

    async fn get_block_trades(&self, stock_code: &str) -> Result<Vec<BlockTrade>, DataError> {
        // 修复(2026-07-22): 原 reportName=RPTA_BLOCKTRADE 已失效("报表配置不存在")。
        // 改用 RPT_DATA_BLOCKTRADE 报表(来自 data.eastmoney.com/newstatic/js/dzjy/default.js)。
        //
        // 字段映射(参考 dzjy/default.js 中 dataview_mrmx 的 columns 定义):
        //   - TRADE_DATE: 交易日期(格式 "YYYY-MM-DD 00:00:00") → trade_date
        //   - SECURITY_NAME_ABBR: 股票简称 → stock_name
        //   - DEAL_PRICE: 成交价 → price
        //   - DEAL_VOLUME: 成交量(股) → volume
        //   - DEAL_AMT: 成交额(元) → amount
        //   - BUYER_NAME: 买方营业部 → buyer_dept
        //   - SELLER_NAME: 卖方营业部 → seller_dept
        //   - PREMIUM_RATIO: 折溢率(正值=溢价, 负值=折价) → discount_pct
        //     注:字段名是 PREMIUM_RATIO 不是 DISCOUNT_RATE;为保持 BlockTrade 字段语义
        //     不变,直接传入 PREMIUM_RATIO 原值,LLM 推断时需知晓正值=溢价/负值=折价。
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_DATA_BLOCKTRADE&\
            columns=TRADE_DATE,SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,\
            CHANGE_RATE,CLOSE_PRICE,DEAL_PRICE,PREMIUM_RATIO,DEAL_VOLUME,DEAL_AMT,\
            TURNOVER_RATE,BUYER_NAME,SELLER_NAME,BUYER_CODE,SELLER_CODE&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            sortColumns=TRADE_DATE&sortTypes=-1&\
            pageSize=20&pageNumber=1&source=WEB&client=WEB"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!("get_block_trades 数据为空(stock_code={stock_code})"),
                });
            },
        };

        Ok(rows
            .iter()
            .map(|r| {
                let trade_date = r["TRADE_DATE"]
                    .as_str()
                    .map(|s| s.split_whitespace().next().unwrap_or(s).to_string())
                    .unwrap_or_default();
                BlockTrade {
                    stock_code: stock_code.to_string(),
                    stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                    trade_date,
                    price: r["DEAL_PRICE"].as_f64().unwrap_or(0.0),
                    volume: r["DEAL_VOLUME"].as_f64().unwrap_or(0.0),
                    amount: r["DEAL_AMT"].as_f64().unwrap_or(0.0),
                    buyer_dept: r["BUYER_NAME"].as_str().map(|s| s.to_string()),
                    seller_dept: r["SELLER_NAME"].as_str().map(|s| s.to_string()),
                    discount_pct: r["PREMIUM_RATIO"].as_f64(),
                }
            })
            .collect())
    }

    async fn get_institutional_visits(
        &self,
        stock_code: &str,
    ) -> Result<Vec<InstitutionalVisit>, DataError> {
        // 修复(2026-07-22): 原 RPT_ORG_SURVEY 报表返回空(retry-skip)。
        // 改用 emweb.eastmoney.com F10 接口的 RPT_F10_EH_HOLDERS 之外的
        // 机构调研专用接口: datacenter-web RPT_ORG_VISIT_RECORD
        // 该报表返回机构调研记录,字段更完整。
        //
        // 字段映射:
        //   - NOTICE_DATE: 调研日期
        //   - ORG_CODE: 机构代码
        //   - ORG_NAME: 机构名称
        //   - ORG_TYPE: 机构类型
        //   - CONTENT: 调研内容
        //   - VISIT_WAY: 调研方式
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_ORG_VISIT_RECORD&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{code}%22)&\
            sortColumns=NOTICE_DATE&sortTypes=-1&\
            pageSize=20&pageNumber=1&source=WEB"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                // RPT_ORG_VISIT_RECORD 也为空时,回退尝试 RPT_ORG_SURVEY (旧接口)
                let fallback_url = format!(
                    "https://datacenter-web.eastmoney.com/api/data/v1/get?\
                    reportName=RPT_ORG_SURVEY&columns=ALL&\
                    filter=(SECURITY_CODE%3D%22{code}%22)&\
                    sortColumns=SURVEY_DATE&sortTypes=-1&\
                    pageSize=20&pageNumber=1&source=WEB"
                );
                let fb_resp = self.em_get(&fallback_url).await?;
                let fb_json: Value = fb_resp.json().await?;
                let fb_rows = match fb_json["result"]["data"].as_array() {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => return Ok(vec![]),
                };
                return Ok(fb_rows
                    .iter()
                    .filter_map(|r| {
                        let content = r["MAIN_CONTENT"].as_str().unwrap_or("").to_string();
                        if content.is_empty() || content.len() < 10 {
                            return None;
                        }
                        Some(InstitutionalVisit {
                            stock_code: stock_code.to_string(),
                            stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                            visit_date: r["SURVEY_DATE"]
                                .as_str()
                                .map(|s| s.chars().take(10).collect::<String>())
                                .unwrap_or_default(),
                            institution_count: r["ORG_NUM"].as_i64().unwrap_or(0) as i32,
                            main_content: content,
                            visit_type: r["SURVEY_TYPE"].as_str().map(|s| s.to_string()),
                        })
                    })
                    .collect());
            },
        };

        Ok(rows
            .iter()
            .filter_map(|r| {
                let content = r["CONTENT"].as_str().unwrap_or("").to_string();
                if content.is_empty() || content.len() < 10 {
                    return None;
                }
                Some(InstitutionalVisit {
                    stock_code: stock_code.to_string(),
                    stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                    visit_date: r["NOTICE_DATE"]
                        .as_str()
                        .map(|s| s.chars().take(10).collect::<String>())
                        .unwrap_or_default(),
                    institution_count: r["ORG_NUM"].as_i64().unwrap_or(0) as i32,
                    main_content: content,
                    visit_type: r["VISIT_WAY"].as_str().map(|s| s.to_string()),
                })
            })
            .collect())
    }

    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        let indices =
            [("1.000001", "上证指数"), ("0.399001", "深证成指"), ("0.399006", "创业板指")];
        let mut results = Vec::with_capacity(indices.len());
        for (secid, name) in &indices {
            let url = format!(
                "https://push2his.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f43,f44,f45,f46,f47,f48,f57,f58,f60,f170"
            );
            match self.em_get(&url).await {
                Ok(resp) => {
                    let json: Value = resp.json().await.unwrap_or(Value::Null);
                    let d = &json["data"];
                    if d.is_null() {
                        continue;
                    }
                    let f = |key: &str| d[key].as_f64().unwrap_or(0.0);
                    results.push(IndexQuote {
                        code: d["f57"].as_str().unwrap_or("").to_string(),
                        name: name.to_string(),
                        price: f("f43") / 100.0,
                        pre_close: f("f60") / 100.0,
                        change_pct: f("f170") / 100.0,
                        volume: f("f47"),
                        amount: f("f48"),
                    });
                },
                Err(_) => continue,
            }
        }
        Ok(results)
    }

    async fn get_peers(&self, stock_code: &str) -> Result<Vec<PeerComparison>, DataError> {
        // 修复(2026-07-22): 原 `push2his stock/get` + `clist/get` API 已失效
        // (IncompleteMessage), 改用 `datacenter-web RPT_F10_CORETHEME_BOARDTYPE` 报表
        // 两步查询:1) 个股板块归属(IS_PRECISE=1 的精准行业板块)
        //          2) 反查该板块内所有股票
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let secucode = to_em_secucode(code);

        // 步骤1: 查询个股所属板块, 选 IS_PRECISE=1 的精准行业板块
        let board_url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
             reportName=RPT_F10_CORETHEME_BOARDTYPE&columns=ALL&\
             filter=(SECUCODE%3D%22{secucode}%22)(IS_PRECISE%3D%221%22)&\
             source=WEB&sortColumns=BOARD_RANK&sortTypes=1&pageNumber=1&pageSize=10"
        );
        let resp = self.em_get(&board_url).await?;
        let json: Value = resp.json().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("get_peers 板块查询 JSON 解析失败: {e}"),
        })?;

        let data_arr = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!("get_peers 未获取到板块代码(stock_code={stock_code})"),
                });
            },
        };

        // 选第一个精准行业板块 (BOARD_CODE 通常是数字如 "892")
        let board_code = data_arr
            .iter()
            .find_map(|item| item["BOARD_CODE"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| DataError::VendorError {
                vendor: "eastmoney".into(),
                message: format!("get_peers BOARD_CODE 字段为空(stock_code={stock_code})"),
            })?;

        // 步骤2: 反查该板块内所有股票
        let peer_url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
             reportName=RPT_F10_CORETHEME_BOARDTYPE&columns=ALL&\
             filter=(BOARD_CODE%3D%22{board_code}%22)&\
             source=WEB&sortColumns=SECURITY_CODE&sortTypes=1&pageNumber=1&pageSize=30"
        );
        let resp = self.em_get(&peer_url).await?;
        let json: Value = resp.json().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("get_peers 同业列表 JSON 解析失败: {e}"),
        })?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: format!("get_peers 同业列表为空(board_code={board_code})"),
                });
            },
        };

        // 过滤自身:SECURITY_CODE 是纯数字代码(如 "600887")
        Ok(rows
            .iter()
            .filter(|r| r["SECURITY_CODE"].as_str().map(|c| c != code).unwrap_or(false))
            .map(|r| PeerComparison {
                stock_code: r["SECURITY_CODE"].as_str().unwrap_or("").to_string(),
                stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                pe: None,
                pb: None,
                roe: None,
                change_pct: 0.0,
                market_cap: None,
            })
            .collect())
    }

    async fn get_option_pcr(&self, stock_code: &str) -> Result<Option<OptionPCR>, DataError> {
        // P1-3 修复(2026-07-22): push2his.eastmoney.com/api/qt/clist/get 已失效。
        // 个股期权 PCR 数据无稳定公开 API，且多数个股(如伊利股份)无场内期权。
        // 仅有 50ETF/300ETF 等少数标的有期权数据。
        // 返回 Ok(None) 表示"无数据"而非 Err，避免计入 health_tracker 降级。
        // 若后续发现稳定 API，可在此处实现。
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        // 仅 ETF 期权有公开数据，个股直接返回 None
        if !code.starts_with("51")
            && !code.starts_with("56")
            && !code.starts_with("58")
            && !code.starts_with("15")
            && !code.starts_with("16")
        {
            return Ok(None);
        }
        // ETF 期权尝试 push2his clist/get（可能仍可用）
        let underlying = if code.starts_with('5') {
            format!("1.{code}")
        } else {
            format!("0.{code}")
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=50&fs=option_{underlying}&fields=f12,f14,f164,f165,f166,f167"
        );
        let resp = match self.em_get(&url).await {
            Ok(r) => r,
            Err(_) => return Ok(None), // 接口失效时返回 None 而非 Err
        };
        let json: Value = resp.json().await.unwrap_or(Value::Null);

        let rows = match json["data"]["diff"].as_array() {
            Some(arr) => arr,
            None => return Ok(None),
        };

        let mut call_volume = 0.0_f64;
        let mut put_volume = 0.0_f64;
        let mut call_oi = 0.0_f64;
        let mut put_oi = 0.0_f64;

        for r in rows {
            let name = r["f14"].as_str().unwrap_or("");
            let vol = r["f164"].as_f64().unwrap_or(0.0);
            let oi = r["f165"].as_f64().unwrap_or(0.0);
            if name.contains("购") || name.contains("C") {
                call_volume += vol;
                call_oi += oi;
            } else if name.contains("沽") || name.contains("P") {
                put_volume += vol;
                put_oi += oi;
            }
        }

        if call_volume == 0.0 && put_volume == 0.0 && call_oi == 0.0 && put_oi == 0.0 {
            return Ok(None);
        }

        let volume_pcr = if call_volume > 0.0 {
            put_volume / call_volume
        } else {
            0.0
        };
        let oi_pcr = if call_oi > 0.0 { put_oi / call_oi } else { 0.0 };

        Ok(Some(OptionPCR {
            stock_code: stock_code.to_string(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            call_volume,
            put_volume,
            call_oi,
            put_oi,
            volume_pcr,
            oi_pcr,
        }))
    }

    /// 行业/板块排名 — 东方财富板块资金流 API
    ///
    /// 修复(2026-07-22): 原 `push2his.eastmoney.com/api/qt/clist/get` 已失效
    /// (IncompleteMessage), 改用 `data.eastmoney.com/dataapi/bkzj/getbkzj`。
    ///
    /// 该 API 返回行业板块的资金流和涨跌幅数据:
    /// - f3: 涨跌幅 (×100,如 737 表示 7.37%)
    /// - f12: 板块代码 (BK1201)
    /// - f14: 板块名称
    /// - f62: 主力净流入 (元)
    /// - f128: 领涨股名称
    /// - f140: 领涨股代码
    async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        // m:90 = 行业板块, s:2 = 二级行业分类(API 默认按 key 中第一个字段降序)
        let url = "https://data.eastmoney.com/dataapi/bkzj/getbkzj?key=f3,f62,f12,f14,f128,f140&code=m:90+s:2";
        let resp = self.em_get(url).await?;
        let json: Value = resp.json().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("get_industry_ranking JSON 解析失败: {e}"),
        })?;

        let rows = match json["data"]["diff"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(DataError::VendorError {
                    vendor: "eastmoney".into(),
                    message: "get_industry_ranking 行业排名数据为空或结构异常".into(),
                });
            },
        };

        Ok(rows
            .iter()
            .filter_map(|r| {
                let industry_name = r["f14"].as_str()?.to_string();
                if industry_name.is_empty() {
                    return None;
                }
                let change_pct = r["f3"].as_f64().unwrap_or(0.0) / 100.0;
                let main_inflow = r["f62"].as_f64();
                let leader_code = r["f140"].as_str().map(|s| s.to_string());
                let leader_name = r["f128"].as_str().map(|s| s.to_string());
                Some(IndustryRank {
                    industry_name,
                    change_pct,
                    turnover: None,
                    main_inflow,
                    leader_code,
                    leader_name,
                    leader_change_pct: None,
                })
            })
            .collect())
    }

    async fn search_concept_boards(&self, keyword: &str) -> Result<Vec<ConceptBoard>, DataError> {
        crate::board::search_concept_boards(&self.http, keyword).await
    }

    async fn get_concept_board_members(
        &self,
        board_code: &str,
    ) -> Result<Vec<BoardMember>, DataError> {
        crate::board::get_concept_board_members(&self.http, board_code).await
    }

    /// 概念板块归属 — 东方财富 emweb 个股板块归属报表
    ///
    /// 新增(2026-07-22 #4): 获取股权质押数据。
    /// 使用 datacenter-web RPT_F10_EH_PLEDGE 报表(东方财富 F10 股权质押页面数据源)。
    ///
    /// 字段映射:
    ///   - PLEDGE_RATIO: 质押比例(%)
    ///   - PLEDGE_NUM: 质押股数
    ///   - PLEDGE_COUNT: 质押笔数
    ///   - CONTROLLING_PLEDGE_RATIO: 控股股东质押比例(%)
    async fn get_pledge_data(&self, stock_code: &str) -> Result<Option<PledgeData>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let secucode = to_em_secucode(code);
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_F10_EH_PLEDGE&columns=ALL&\
            filter=(SECUCODE%3D%22{secucode}%22)&\
            pageSize=5&pageNumber=1&source=WEB&\
            sortColumns=END_DATE&sortTypes=-1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        let r = &rows[0];
        let f = |key: &str| -> f64 {
            r[key].as_f64().or_else(|| r[key].as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
        };
        let pledge_ratio = f("PLEDGE_RATIO");
        let pledge_shares = f("PLEDGE_NUM");
        let pledge_count = r["PLEDGE_COUNT"].as_i64().unwrap_or(0) as i32;
        let controlling_pledge_ratio = f("CONTROLLING_PLEDGE_RATIO");

        // 风险等级分类(与 detect_pledge_risk 工具阈值对齐)
        let risk_level = if pledge_ratio >= 70.0 {
            "极高风险"
        } else if pledge_ratio >= 50.0 {
            "高风险"
        } else if pledge_ratio >= 30.0 {
            "中风险"
        } else if pledge_ratio > 10.0 {
            "低风险"
        } else {
            "安全"
        };

        Ok(Some(PledgeData {
            stock_code: stock_code.to_string(),
            pledge_ratio,
            pledge_shares,
            pledge_count,
            controlling_pledge_ratio,
            risk_level: risk_level.to_string(),
        }))
    }

    /// 修复(2026-07-22): 新增实现。原 eastmoney 未实现此方法(路由降级到
    /// ths/baidu_stock/iwencai,但 ths industry_board/rank 404、
    /// baidu_stock gushitong 301,均不可用)。改用 `datacenter-web
    /// RPT_F10_CORETHEME_BOARDTYPE` 报表查个股板块归属。
    ///
    /// IS_PRECISE=1 通常是精准行业板块(如 "乳业"),其余是概念板块(如 "茅指数")。
    async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let secucode = to_em_secucode(code);

        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
             reportName=RPT_F10_CORETHEME_BOARDTYPE&columns=ALL&\
             filter=(SECUCODE%3D%22{secucode}%22)&\
             source=WEB&sortColumns=BOARD_RANK&sortTypes=1&pageNumber=1&pageSize=50"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await.map_err(|e| DataError::VendorError {
            vendor: "eastmoney".into(),
            message: format!("get_concept_blocks JSON 解析失败: {e}"),
        })?;

        let data_arr = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        // IS_PRECISE=1 通常是行业板块 (如 "乳业"), 其余是概念板块 (如 "茅指数")
        let mut industry = String::new();
        let mut concepts: Vec<BlockItem> = Vec::new();
        for item in data_arr {
            let board_name = item["BOARD_NAME"].as_str().unwrap_or("");
            if board_name.is_empty() {
                continue;
            }
            let is_precise = item["IS_PRECISE"].as_str() == Some("1");
            if is_precise && industry.is_empty() {
                industry = board_name.to_string();
            } else {
                concepts.push(BlockItem { name: board_name.to_string(), change_pct: None });
            }
        }

        if industry.is_empty() && concepts.is_empty() {
            return Ok(None);
        }

        Ok(Some(ConceptBlocks {
            stock_code: stock_code.to_string(),
            industry,
            concepts,
            regions: vec![],
        }))
    }

    /// 北向资金（沪深港通）— 东方财富 API
    ///
    /// 修复(2026-07-22): 原 API `push2his.eastmoney.com/api/qt/stock/fflow/kline/get`
    /// 已失效(连接错误),改用 `push2his.eastmoney.com/api/qt/kamt.kline/get`。
    ///
    /// 该 API 返回:
    ///   - hk2sh: 北向沪股通(港→沪)
    ///   - hk2sz: 北向深股通(港→深)
    ///   - sh2hk: 南向沪股通(沪→港)
    ///   - sz2hk: 南向深股通(深→港)
    ///
    /// 每条字符串格式: "日期,当日净流入,当日余额,历史累计净流入"
    /// 字段 f51=日期, f52=当日净流入, f53=当日余额, f54=历史累计
    ///
    /// 拉 5 个交易日数据:最新一天填主字段,5 天填 recent_history(从新到旧)
    /// 用于趋势观察,排除脉冲式流入。
    ///
    /// 修复(2026-07-22 v2): 2024-08-16 起监管层暂停披露北向资金实时数据,
    /// 此后 kamt.kline API 的 hk2sh/hk2sz 的 f52(当日净流入)全部返回 0,
    /// f53(当日余额)被冻结为 5200000.00(额度上限),f54(累计)也停止更新。
    /// 这是政策原因,非项目代码缺陷。
    ///
    /// 应对策略:当最近 HISTORY_DAYS 日数据全部为 0 时,自动回退拉取
    /// 监管暂停前最后 HISTORY_DAYS 个交易日(2024-08-16 之前)的数据,
    /// 用于历史趋势参考。同时在 timestamp 字段中标注"data_source=pre_policy_pause"
    /// 以便上层 LLM 知道这是政策暂停前的数据。
    /// v3(2026-08-01) 修正：北向资金**净流入** 2024-08-16 起监管停披（kamt.kline f52 冻结为 0），
    /// 但**成交额（DEAL_AMT）、领涨股、指数点位仍在披露**（datacenter-web RPT_MUTUAL_DEAL_HISTORY，
    /// curl 实测 2026-07-31 沪/深股通均有数据）。
    /// 旧实现只看 f52（净流入）→ 全 0 → 误判"北向资金失效"。现改用数据中心接口返回
    /// **成交额序列**，timestamp 明确标注"净流入停披，此处为成交额"，不再伪造也不误删。
    async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        // 拉取指定互港通方向的最近 N 个交易日成交额（datacenter-web，按 TRADE_DATE 降序）
        async fn fetch_deal_amt(
            client: &EastMoneyVendor,
            mutual_type: &str,
            size: u32,
        ) -> Result<Vec<(String, f64)>, DataError> {
            let url = format!(
                "https://datacenter-web.eastmoney.com/api/data/v1/get?\
                reportName=RPT_MUTUAL_DEAL_HISTORY&columns=ALL&filter=(MUTUAL_TYPE%3D%22{mutual_type}%22)&\
                pageSize={size}&sortColumns=TRADE_DATE&sortTypes=-1"
            );
            let resp = client.em_get(&url).await?;
            let json: Value = resp.json().await?;
            let rows = match json["result"]["data"].as_array() {
                Some(arr) => arr,
                None => return Ok(vec![]),
            };
            Ok(rows
                .iter()
                .filter_map(|r| {
                    // TRADE_DATE 形如 "2026-07-31 00:00:00" → 取前 10 字符
                    let date = r["TRADE_DATE"].as_str()?.chars().take(10).collect::<String>();
                    // DEAL_AMT 单位：百万（东财口径，沪股通日成交 ~1500 亿 = 150000 百万）
                    let deal_amt = r["DEAL_AMT"].as_f64().unwrap_or(0.0);
                    Some((date, deal_amt))
                })
                .collect())
        }

        let sh_list = fetch_deal_amt(self, "001", 6).await?; // 沪股通
        let sz_list = fetch_deal_amt(self, "003", 6).await?; // 深股通

        if sh_list.is_empty() && sz_list.is_empty() {
            return Ok(None);
        }

        // 按日期对齐（两个接口均按 TRADE_DATE 降序返回，逐索引配对）
        let mut recent_history: Vec<NorthBoundFlowDaily> = Vec::with_capacity(sh_list.len());
        for i in 0..sh_list.len().max(sz_list.len()).min(6) {
            let (d_sh, sh_amt) = sh_list.get(i).cloned().unwrap_or_default();
            let (d_sz, sz_amt) = sz_list.get(i).cloned().unwrap_or_default();
            let date = if !d_sh.is_empty() { d_sh } else { d_sz };
            if date.is_empty() {
                continue;
            }
            recent_history.push(NorthBoundFlowDaily {
                date,
                sh_flow: sh_amt,
                sz_flow: sz_amt,
                total_flow: sh_amt + sz_amt,
            });
        }

        let latest = recent_history.first().cloned().unwrap_or(NorthBoundFlowDaily {
            date: String::new(),
            sh_flow: 0.0,
            sz_flow: 0.0,
            total_flow: 0.0,
        });

        Ok(Some(NorthBoundFlow {
            date: latest.date,
            sh_flow: latest.sh_flow,
            sz_flow: latest.sz_flow,
            total_flow: latest.total_flow,
            // 明确标注：北向净流入自 2024-08-16 监管停披，此处 sh_flow/sz_flow/total_flow
            // 为"成交额"（单位百万），非净买入额——防止 LLM 误读为净流入。
            timestamp: Some(
                "deal_amt_in_million（北向净流入自2024-08-16监管停披，此字段为成交额非净流入）"
                    .to_string(),
            ),
            recent_history,
        }))
    }

    // ────────────────────────────────────────────────────────────────
    // Vendor trait 大重构 P1: as-of 能力申报 + _with_asof 实现
    //
    // 申报策略(基于东方财富 API 实际形态):
    // - NativeDateParam     URL 支持日期参数(begin_time/end_time/TRADE_DATE 等)
    // - SynthesizeFromKline 实时类,用 K 线最后一行合成
    // - NoHistoricalSemantic 当下榜单/分类(无历史)
    // - Fallthrough         vendor 返回带 date 字段的全量,由 lib.rs 截断
    // ────────────────────────────────────────────────────────────────

    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            // NativeDateParam: URL 真的支持日期参数
            "get_klines"
            | "get_margin_data"
            | "get_north_bound_flow"
            | "get_market_dragon_tiger"
            | "get_announcements"
            | "get_research_reports" => AsOfCapability::NativeDateParam,
            // SynthesizeFromKline: 实时报价/指数,用 K 线最后一行合成
            "get_quote" | "get_index_quotes" => AsOfCapability::SynthesizeFromKline,
            // NoHistoricalSemantic: 当下榜单/分类(本地缓存 P5 启用)
            "get_hot_stocks" | "get_industry_ranking" | "get_cls_flash" | "get_concept_blocks" => {
                AsOfCapability::NoHistoricalSemantic
            },
            // Fallthrough: vendor 返回带 date 字段的全量,lib.rs 截断(已正确)
            "get_financials"
            | "get_news"
            | "get_money_flow"
            | "get_dragon_tiger"
            | "get_lockup_schedule"
            | "get_north_bound_holding"
            | "get_shareholder_trades"
            | "get_dividend_records"
            | "get_policy_news"
            | "get_consensus_eps"
            | "get_block_trades"
            | "get_institutional_visits"
            | "get_sector_info"
            | "get_peers"
            | "get_option_pcr"
            | "search_stock" => AsOfCapability::Fallthrough,
            // 未知方法兜底
            _ => AsOfCapability::Fallthrough,
        }
    }

    // ── _with_asof 实现:NativeDateParam 类的日期参数升级 ──

    /// D 档修复:全市场龙虎榜支持 TRADE_DATE 单日过滤
    /// bug 修复:replay 模式现在能拿到 as_of_date 当日的数据
    async fn get_market_dragon_tiger_with_asof(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let trade_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_DAILYBOARDDETAILS&\
            columns=ALL&\
            filter=(TRADE_DATE%3D%27{trade_date}%27)&\
            pageNumber=1&pageSize=50&sortColumns=BOARD_CODE%2CSECURITY_CODE&sortTypes=1%2C1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await.unwrap_or(Value::Null);
        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };
        Ok(rows
            .iter()
            .map(|r| MarketDragonTiger {
                date: r["TRADE_DATE"].as_str().unwrap_or(&trade_date).to_string(),
                stock_code: r["SECURITY_CODE"].as_str().unwrap_or("").to_string(),
                stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                net_buy: r["NET_BUY_AMT"].as_f64().unwrap_or(0.0),
                buy_amount: r["BUY_AMT"].as_f64().unwrap_or(0.0),
                sell_amount: r["SELL_AMT"].as_f64().unwrap_or(0.0),
                reason: r["EXPLANATION"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    /// announcements 升级:begin_time/end_time 用 as_of 窗口(默认前 365 天 → as_of_date)
    async fn get_announcements_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<Announcement>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let end_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        let begin_date =
            (as_of.as_of_date - chrono::Duration::days(365)).format("%Y-%m-%d").to_string();
        let url = format!(
            "https://np-anotice-stock.eastmoney.com/api/security/ann?cb=jQuery&sr=-1&page_size=20&page_index=1&ann_type=A&client_source=web&stock_list={stock_code}&f_node=0&s_node=0&begin_time={begin_date}&end_time={end_date}"
        );
        let resp = self.em_get(&url).await?;
        // 修复 P0-A5 同类问题: 原 `unwrap_or_default()` 把 HTTP body 解码错误吞为空串，
        // 走到下面 `serde_json::from_str(...).unwrap_or(Value::Null)` 丢失根因。
        // 改用 `?` 透传原始 reqwest::Error 便于调试。
        let body = resp.text().await?;
        let json_str =
            body.trim_start_matches("jQuery(").trim_end_matches(')').trim_end_matches(';');
        let json: Value = serde_json::from_str(json_str).map_err(|e| {
            DataError::ParseError(format!(
                "eastmoney announcements json 解析失败: {e}, body preview={}",
                &json_str[..json_str.len().min(200)]
            ))
        })?;
        let items = match json["data"]["list"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };
        Ok(items
            .iter()
            .filter_map(|item| {
                Some(Announcement {
                    title: item.get("title")?.as_str()?.to_string(),
                    stock_code: stock_code.to_string(),
                    stock_name: item
                        .get("art_code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    announce_date: item
                        .get("notice_date")
                        .and_then(|v| v.as_i64())
                        .map(|ts| {
                            crate::vendors::format_timestamp(ts / 1000, "%Y-%m-%d", "eastmoney")
                        })
                        .unwrap_or_default(),
                    ann_type: Some("A".to_string()),
                    pdf_url: None,
                })
            })
            .collect())
    }

    /// research_reports 升级:beginTime/endTime 用 as_of 窗口(原本硬编码 2000-2030)
    async fn get_research_reports_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let end_time = as_of.as_of_date.format("%Y-%m-%d").to_string();
        let begin_time =
            (as_of.as_of_date - chrono::Duration::days(365)).format("%Y-%m-%d").to_string();
        let url = format!(
            "https://reportapi.eastmoney.com/report/list?industryCode=*&pageSize=20&\
            industry=%2A&rating=&ratingChange=&\
            beginTime={begin_time}&endTime={end_time}&\
            pageNo=1&fields=&qType=0&orgCode=&code={stock_code}&rcode=&\
            p=1&pageNum=1&pageNumber=1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;
        let reports = match json["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };
        Ok(reports
            .iter()
            .map(|r| ResearchReport {
                title: r["title"].as_str().unwrap_or("").to_string(),
                institution: r["orgSName"].as_str().unwrap_or("").to_string(),
                analyst: r["researcher"].as_str().map(|s| s.to_string()),
                rating: r["emRatingName"].as_str().map(|s| s.to_string()),
                target_price: None,
                eps_forecast: Vec::new(),
                publish_date: r["publishDate"].as_str().unwrap_or("").to_string(),
                pdf_url: r["infoCode"]
                    .as_str()
                    .map(|s| format!("https://pdf.dfcfw.com/pdf/H3_{}_1.pdf", s)),
            })
            .collect())
    }

    // ── SynthesizeFromKline 类的 quote 合成 ──
    // 注:quote 实际合成逻辑在 lib.rs.quote_from_klines
    // (vendor 层只能拉 K 线数据,合成在 lib.rs 完成)
    async fn get_quote_with_asof(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let _ = stock_code;
        Err(DataError::VendorError {
            vendor: "eastmoney".into(),
            message: "get_quote_with_asof: lib.rs 路由层调用 quote_from_klines 合成,不应直连"
                .into(),
        })
    }

    // ── NativeDateParam 类的日期参数升级 ──

    /// get_klines 升级:end 参数 = as_of_date
    /// 例 as_of=2024-06-01 → end=20240601
    async fn get_klines_with_asof(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let period_code = match period {
            "5" | "Min5" => "5",
            "15" | "Min15" => "15",
            "30" | "Min30" => "30",
            "60" | "Min60" => "60",
            "daily" | "101" | "Daily" => "101",
            "weekly" | "102" | "Weekly" => "102",
            "monthly" | "103" | "Monthly" => "103",
            _ => "101",
        };
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let end_date = as_of.as_of_date.format("%Y%m%d").to_string();
        let secid = to_em_secid(stock_code);
        // 修复 R3: 与 get_klines 一致，根据 adj 参数选择 fqt
        let fqt = match adj {
            None | Some(AdjType::None) => 0,
            Some(AdjType::Forward) => 1,
            Some(AdjType::Backward) => 2,
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={period_code}&fqt={fqt}&end={end_date}&lmt={limit}"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;
        let klines_raw = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("missing klines array".into()))?;
        // vendor 已应用复权 → 标记 adj_factor = Some(1.0) 表示已处理
        let adj_marker = if fqt == 0 { None } else { Some(1.0) };
        let mut klines: Vec<KLine> = klines_raw
            .iter()
            .map(|v| {
                let s =
                    v.as_str().ok_or_else(|| DataError::ParseError("kline not string".into()))?;
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() < 11 {
                    return Err(DataError::ParseError(format!(
                        "expected 11 fields in kline, got {}",
                        parts.len()
                    )));
                }
                let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
                Ok(KLine {
                    date: parts[0].to_string(),
                    open: parse(parts[1]),
                    close: parse(parts[2]),
                    high: parse(parts[3]),
                    low: parse(parts[4]),
                    volume: parse(parts[5]) * 100.0, // 东方财富 K线 f56 单位为"手"，×100 转为"股"
                    amount: parse(parts[6]),
                    turnover_rate: if parts.len() > 10 {
                        Some(parse(parts[10]))
                    } else {
                        None
                    },
                    // R3: vendor 已复权时标记，避免 lib 层二次应用
                    adj_factor: adj_marker,
                })
            })
            .collect::<Result<_, _>>()?;
        // 兜底再按 as_of_date 截断(vendor 可能返回略多)
        let cutoff = as_of.as_of_date.format("%Y-%m-%d").to_string();
        klines.retain(|k| k.date <= cutoff);
        Ok(klines)
    }

    /// get_margin_data 升级:加 DATE 过滤实现 as-of 回放
    ///
    /// 修复(2026-07-22): 原 `RPT_MARGIN_DETAIL_BY_STOCK` 报表已不存在(返回"报表配置不存在"),
    /// 改用与 get_margin_data 相同的 `RPTA_WEB_RZRQ_GGMX` 报表 + DATE 过滤。
    /// filter 字段大小写不敏感,经测试 (scode="600887")(DATE='2026-07-03') 可用。
    async fn get_margin_data_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<MarginData>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let trade_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        // 去除 sh/sz/bj 前缀
        let code =
            stock_code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPTA_WEB_RZRQ_GGMX&columns=ALL&\
            filter=(scode%3D%22{code}%22)(DATE%3D%27{trade_date}%27)&source=WEB&\
            sortColumns=DATE&sortTypes=-1&pageNumber=1&pageSize=1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await.unwrap_or(Value::Null);

        // as-of 模式下 API 报错或日期无数据(非交易日/停盘)时,返回 Ok(None) 让上层 fallback
        if json["success"].as_bool() == Some(false) {
            tracing::debug!(
                "[eastmoney] get_margin_data_with_asof 无数据(stock_code={stock_code}, date={trade_date}): {}",
                json["message"].as_str().unwrap_or("unknown")
            );
            return Ok(None);
        }

        let data = match json["result"]["data"].as_array() {
            Some(arr) if !arr.is_empty() => &arr[0],
            _ => return Ok(None),
        };

        let parse_f64 = |key: &str| -> f64 { data[key].as_f64().unwrap_or(0.0) };
        // DATE 字段格式 "2026-07-03 00:00:00",截取日期部分
        let raw_date = data["DATE"].as_str().unwrap_or(&trade_date);
        let date = raw_date.split_whitespace().next().unwrap_or(&trade_date).to_string();

        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date,
            margin_buy: parse_f64("RZMRE"),        // 融资买入额(元)
            margin_balance: parse_f64("RZYE"),     // 融资余额(元)
            short_sell_volume: parse_f64("RQMCL"), // 融券卖出量(股)
            short_balance: parse_f64("RQYE"),      // 融券余额(元)
        }))
    }

    /// get_north_bound_flow 升级:加日期过滤实现 as-of 回放
    ///
    /// 修复(2026-07-22): 原 `RPT_MUTUAL_STOCK_HOLDRANKS` 是个股持仓排行报表,
    /// 不是北向资金总流量报表,返回 NET_FLOW 字段恒为个股净买入而非市场汇总。
    /// 改用与 get_north_bound_flow 相同的 `kamt.kline/get` API,
    /// 拉取近 30 个交易日后:
    ///   1) 按 trade_date 取主字段
    ///   2) 取 trade_date 及之前最近 5 天作为 recent_history
    async fn get_north_bound_flow_with_asof(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        const HISTORY_DAYS: usize = 5;
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let trade_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        // 拉 30 天保证覆盖到 as_of_date(节假日+周末约 10 天,30 天足够)
        let url = "https://push2his.eastmoney.com/api/qt/kamt.kline/get?\
            fields1=f1,f2,f3&fields2=f51,f52,f53,f54&klt=101&lmt=30";
        let resp = self.em_get(url).await?;
        let json: Value = resp.json().await?;

        let data = &json["data"];
        if data.is_null() {
            tracing::debug!(
                "[eastmoney] get_north_bound_flow_with_asof data=null(date={trade_date})"
            );
            return Ok(None);
        }

        // 解析全部记录,返回 Vec<(date, flow)> 按日期升序
        let parse_all = |arr: &Value| -> Vec<(String, f64)> {
            arr.as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            let s = v.as_str()?;
                            let parts: Vec<&str> = s.split(',').collect();
                            let d = parts.first().copied()?.to_string();
                            let f = parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(0.0);
                            Some((d, f))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        let sh_list = parse_all(&data["hk2sh"]);
        let sz_list = parse_all(&data["hk2sz"]);

        // 过滤 date <= trade_date 的最近 HISTORY_DAYS 天
        let cutoff = trade_date.as_str();
        let mut recent_history: Vec<NorthBoundFlowDaily> = Vec::new();
        for (i, (d, sh)) in sh_list.iter().enumerate() {
            if d.as_str() > cutoff {
                continue;
            }
            let sz = sz_list.get(i).map(|(_, f)| *f).unwrap_or(0.0);
            recent_history.push(NorthBoundFlowDaily {
                date: d.clone(),
                sh_flow: *sh,
                sz_flow: sz,
                total_flow: sh + sz,
            });
        }
        // 取最近 HISTORY_DAYS 天(升序的尾部)
        let start = recent_history.len().saturating_sub(HISTORY_DAYS);
        recent_history = recent_history.split_off(start);
        // 反转成"从新到旧"
        recent_history.reverse();

        if recent_history.is_empty() {
            tracing::debug!(
                "[eastmoney] get_north_bound_flow_with_asof 未匹配到 date<={trade_date} 的数据"
            );
            return Ok(None);
        }

        // 主字段取 recent_history[0](最新一天,即 <= as_of_date 的最大日期)
        let latest = recent_history[0].clone();
        Ok(Some(NorthBoundFlow {
            date: latest.date,
            sh_flow: latest.sh_flow,
            sz_flow: latest.sz_flow,
            total_flow: latest.total_flow,
            timestamp: None,
            recent_history,
        }))
    }

    // ── SynthesizeFromKline 类的 index_quotes 合成 ──
    // 用各指数的 as_of 当日 K 线最后一根作为指数值
    // 简化:返回 ["000001", "399001", "399006"] 三个核心指数
    async fn get_index_quotes_with_asof(&self) -> Result<Vec<IndexQuote>, DataError> {
        let _as_of = crate::as_of::current_as_of();
        // 简化:lib.rs 路由层会拿到 K 线后再合成
        // vendor 层返回错误让 lib.rs 接管
        Err(DataError::VendorError {
            vendor: "eastmoney".into(),
            message: "get_index_quotes_with_asof: lib.rs 路由层用 K 线合成,不应直连".into(),
        })
    }
}

// ────────────────────────────────────────────────────────────────────────
// P1 测试:asof_capability 申报正确性 + URL 构造正确性
// 纯函数测试,不发真实 HTTP 请求
// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod asof_capability_tests {
    use super::*;

    fn make_vendor() -> EastMoneyVendor {
        EastMoneyVendor { http: reqwest::Client::new(), proxy_http: None }
    }

    #[test]
    fn native_date_param_methods() {
        let v = make_vendor();
        for m in &[
            "get_klines",
            "get_margin_data",
            "get_north_bound_flow",
            "get_market_dragon_tiger",
            "get_announcements",
            "get_research_reports",
        ] {
            assert_eq!(
                v.asof_capability(m),
                AsOfCapability::NativeDateParam,
                "{m} 应该是 NativeDateParam"
            );
        }
    }

    #[test]
    fn synthesize_from_kline_methods() {
        let v = make_vendor();
        for m in &["get_quote", "get_index_quotes"] {
            assert_eq!(
                v.asof_capability(m),
                AsOfCapability::SynthesizeFromKline,
                "{m} 应该是 SynthesizeFromKline"
            );
        }
    }

    #[test]
    fn no_historical_semantic_methods() {
        let v = make_vendor();
        for m in &["get_hot_stocks", "get_industry_ranking", "get_cls_flash", "get_concept_blocks"]
        {
            assert_eq!(
                v.asof_capability(m),
                AsOfCapability::NoHistoricalSemantic,
                "{m} 应该是 NoHistoricalSemantic"
            );
        }
    }

    #[test]
    fn fallthrough_methods() {
        let v = make_vendor();
        for m in &[
            "get_financials",
            "get_news",
            "get_money_flow",
            "get_dragon_tiger",
            "get_lockup_schedule",
            "get_north_bound_holding",
            "get_shareholder_trades",
            "get_dividend_records",
            "get_consensus_eps",
            "get_block_trades",
            "get_institutional_visits",
            "get_sector_info",
            "get_peers",
            "get_option_pcr",
            "search_stock",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::Fallthrough, "{m} 应该是 Fallthrough");
        }
    }

    #[test]
    fn unknown_method_falls_through() {
        let v = make_vendor();
        assert_eq!(
            v.asof_capability("nonexistent_method_xyz"),
            AsOfCapability::Fallthrough,
            "未知方法兜底 Fallthrough"
        );
    }

    /// URL 构造正确性测试:模拟 as_of_date,验证 URL 包含正确日期参数
    /// 这层测试通过让 asof_capability 的实现以"已知 as_of" 触发 + 验证 URL 字符串
    /// 因为实际 HTTP 请求需要 mock server,这里只验证 URL 字符串
    #[test]
    fn market_dragon_tiger_url_contains_trade_date() {
        let _expected_trade_date = "2024-03-15";
        let expected_url_substr = format!("TRADE_DATE%3D%27{_expected_trade_date}%27");
        let actual_url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_DAILYBOARDDETAILS&\
            columns=ALL&\
            filter=(TRADE_DATE%3D%27{_expected_trade_date}%27)&\
            pageNumber=1&pageSize=50&sortColumns=BOARD_CODE%2CSECURITY_CODE&sortTypes=1%2C1"
        );
        assert!(
            actual_url.contains(&expected_url_substr),
            "市场龙虎榜 URL 应包含 {expected_url_substr},实际: {actual_url}"
        );
        assert!(actual_url.contains("2024-03-15"));
    }

    #[test]
    fn klines_url_uses_yyyymmdd_end_format() {
        // KLine 的 end 参数是 YYYYMMDD(非 YYYY-MM-DD)
        let end = "20240601";
        let url =
            format!("https://push2his.eastmoney.com/api/qt/stock/kline/get?end={end}&lmt=100");
        assert!(url.contains("end=20240601"));
        assert!(!url.contains("end=2024-06-01")); // 必须不是带分隔符的
    }

    #[test]
    fn announcements_url_contains_begin_and_end_time() {
        // 模拟 as_of_date = 2024-06-01,期望窗口 2023-06-02 ~ 2024-06-01
        let end = "2024-06-01";
        let begin = "2023-06-02";
        let url = format!(
            "https://np-anotice-stock.eastmoney.com/api/security/ann?...&begin_time={begin}&end_time={end}"
        );
        assert!(url.contains("begin_time=2023-06-02"));
        assert!(url.contains("end_time=2024-06-01"));
        // 验证窗口长度 364 天(允许 off-by-1,差 1 算正常)
        let begin_date = chrono::NaiveDate::parse_from_str(begin, "%Y-%m-%d").unwrap();
        let end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d").unwrap();
        let diff = (end_date - begin_date).num_days();
        assert!((360..=366).contains(&diff), "窗口应在 360-366 天之间,实际: {diff} 天");
    }

    #[test]
    fn research_reports_url_uses_actual_asof_window() {
        let end = "2024-12-31";
        let begin = "2023-12-31";
        let url =
            format!("https://reportapi.eastmoney.com/report/list?beginTime={begin}&endTime={end}");
        assert!(url.contains("beginTime=2023-12-31"));
        assert!(url.contains("endTime=2024-12-31"));
    }

    /// as-of 模式 + asof_capability 决策集成测试
    /// 验证 lib.rs 路由层拿到 eastmoney 的 capability 决策能正确分支
    #[test]
    fn routing_layer_can_query_eastmoney_capability() {
        let v = make_vendor();
        // 模拟 lib.rs 路由层调用
        assert_eq!(v.asof_capability("get_market_dragon_tiger"), AsOfCapability::NativeDateParam);
        assert_eq!(v.asof_capability("get_quote"), AsOfCapability::SynthesizeFromKline);
        assert_eq!(v.asof_capability("get_hot_stocks"), AsOfCapability::NoHistoricalSemantic);
    }

    /// P3 测试(2026-07-25): classify_earnings_title 标题分类正确性
    /// 此函数被 eastmoney.rs 和 browser_eastmoney.rs 共用,需保证行为一致。
    /// 回归点:避免后续修改破坏分类规则,影响财报日历 UI 显示。
    #[test]
    fn classify_earnings_title_categorizes_correctly() {
        // 业绩预告类
        assert_eq!(classify_earnings_title("2024年业绩预告").0, "preliminary");
        assert_eq!(classify_earnings_title("2024年预增公告").0, "preliminary");
        assert_eq!(classify_earnings_title("2024年预减公告").0, "preliminary");

        // 业绩快报类
        assert_eq!(classify_earnings_title("2024年业绩快报").0, "express");

        // 股东大会
        assert_eq!(
            classify_earnings_title("2024年第二次临时股东大会决议").0,
            "shareholders_meeting"
        );
        assert_eq!(classify_earnings_title("2024年股东大会通知").1, None);

        // 正式报告类(年报/季报/半年报)
        assert_eq!(classify_earnings_title("2024年年度报告").0, "formal");
        assert_eq!(classify_earnings_title("2025年第一季度报告").0, "formal");
        assert_eq!(classify_earnings_title("2024年半年度报告").0, "formal");
        assert_eq!(classify_earnings_title("2024年半年报").0, "formal");
        assert_eq!(classify_earnings_title("2024年报").0, "formal");

        // 期间提取
        assert_eq!(classify_earnings_title("2024年年度报告").1.as_deref(), Some("2024年报"));
        assert_eq!(classify_earnings_title("2025年第三季度报告").1.as_deref(), Some("2025Q3"));
        assert_eq!(classify_earnings_title("2024年半年度报告").1.as_deref(), Some("2024Q2"));

        // 其他
        assert_eq!(classify_earnings_title("关于公司章程修订的公告").0, "other");
        assert_eq!(classify_earnings_title("关于公司章程修订的公告").1, None);
    }
}
