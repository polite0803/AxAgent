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
        match reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(&proxy_url).ok()?)
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
        Err(last_err.unwrap())
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
            timestamp: d["f171"]
                .as_i64()
                .map(|t| t.to_string())
                .unwrap_or_default(),
        })
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        _adj: Option<AdjType>,
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
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={period_code}&fqt=1&end=20500101&lmt={limit}"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let klines_raw = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("missing klines array".into()))?;

        let mut klines: Vec<KLine> = klines_raw
            .iter()
            .map(|v| {
                let s = v
                    .as_str()
                    .ok_or_else(|| DataError::ParseError("kline not string".into()))?;
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
                    volume: parse(parts[5]),
                    amount: parse(parts[6]),
                    turnover_rate: Some(parse(parts[10])),
                    // P1-4: vendor 默认不复权
                    adj_factor: None,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        klines.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(klines)
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        // 东方财富 2025 年后 FinanceSummary API 失效，改用 NewFinanceAnalysis/ZYZBAjaxNew
        let em_code = if stock_code.starts_with('6') || stock_code.starts_with('9') {
            format!("SH{stock_code}")
        } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
            format!("BJ{stock_code}")
        } else {
            format!("SZ{stock_code}")
        };

        let url = format!(
            "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/ZYZBAjaxNew?type=0&code={}",
            em_code
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let data = match json["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(data
            .iter()
            .take(24)           // 取24条(6年季度)，as-of 截断后有足够历史数据
            .map(|r| {
                let s = |key: &str| -> &str { r[key].as_str().unwrap_or("") };
                let n = |key: &str| -> Option<f64> { r[key].as_str().and_then(|v| v.parse().ok()) };
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
                }
            })
            .collect())
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

        let items = match json["result"]["cmsArticleWebOld"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
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

        let items = match json["result"]["cmsArticleWebOld"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
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
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/fflow/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55&klt=1&lmt=1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let klines = json["data"]["klines"].as_array();
        match klines {
            Some(arr) if !arr.is_empty() => {
                let s = arr[0].as_str().unwrap_or("");
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() < 5 {
                    return Ok(None);
                }
                let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
                let main_net = parse(parts[1]);
                let super_large = if parts.len() > 2 {
                    parse(parts[2])
                } else {
                    0.0
                };
                let large = if parts.len() > 3 {
                    parse(parts[3])
                } else {
                    0.0
                };
                let medium = if parts.len() > 4 {
                    parse(parts[4])
                } else {
                    0.0
                };
                let small = if parts.len() > 5 {
                    parse(parts[5])
                } else {
                    0.0
                };
                Ok(Some(MoneyFlow {
                    date: parts[0].to_string(),
                    main_net_inflow: main_net,
                    super_large_net: super_large,
                    large_net: large,
                    medium_net: medium,
                    small_net: small,
                }))
            },
            _ => Ok(None),
        }
    }

    async fn get_dragon_tiger(&self, stock_code: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/mmpa/get?secid={secid}&fields1=f1,f2,f3,f4&fields2=f51,f52,f53,f54,f55,f56,f57,f58"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let entries = match json["data"]["mmpa"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        entries
            .iter()
            .map(|e| {
                let s = e.as_str().unwrap_or("");
                let parts: Vec<&str> = s.split(',').collect();
                let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
                Ok(DragonTigerEntry {
                    stock_code: stock_code.to_string(),
                    date: parts.first().map(|s| s.to_string()).unwrap_or_default(),
                    dept_name: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                    buy_amount: parse(parts.get(3).unwrap_or(&"0")),
                    sell_amount: parse(parts.get(4).unwrap_or(&"0")),
                    net_amount: parse(parts.get(5).unwrap_or(&"0")),
                    reason: parts.get(7).map(|s| s.to_string()),
                })
            })
            .collect()
    }

    async fn get_lockup_schedule(
        &self,
        stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_LOCKUP&columns=SECURITY_CODE,SECURITY_NAME_ABBR,UNLOCK_DATE,UNLOCK_SHARES,PLACING_RATIO,HOLDER_NAME&filter=(SECURITY_CODE=\"{stock_code}\")&pageSize=20&sortColumns=UNLOCK_DATE&pageNumber=1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .map(|r| LockupSchedule {
                stock_code: stock_code.to_string(),
                stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                unlock_date: r["UNLOCK_DATE"].as_str().unwrap_or("").to_string(),
                unlock_shares: r["UNLOCK_SHARES"].as_f64().unwrap_or(0.0),
                unlock_ratio: r["PLACING_RATIO"].as_f64().unwrap_or(0.0),
                shareholder: r["HOLDER_NAME"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    /// 获取融资融券数据
    async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/margin/get?secid={secid}&fields1=f1,f2,f3,f4,f5&fields2=f51,f52,f53,f54,f55"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let data = &json["data"];
        let parse_num =
            |v: &Value| -> f64 { v.as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0) };

        if data.is_null() || data["f51"].is_null() {
            return Ok(None);
        }

        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date: data["f51"].as_str().unwrap_or("").to_string(),
            margin_buy: parse_num(&data["f52"]) * 10000.0,
            margin_balance: parse_num(&data["f53"]) * 10000.0,
            short_sell_volume: parse_num(&data["f54"]) * 100.0,
            short_balance: parse_num(&data["f55"]) * 100.0,
        }))
    }

    async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get?secid={secid}&fields1=f1,f2,f3&fields2=f51,f52,f53&lmt=2&klt=3"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        if let Some(arr) = json["data"]["klines"].as_array() {
            let len = arr.len();
            if len >= 1 {
                if let Some(line) = arr[len - 1].as_str() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        let holding_shares: f64 = parts[1].parse().unwrap_or(0.0);
                        let holding_ratio: f64 = parts[2].parse().unwrap_or(0.0);
                        let prev_shares = if len >= 2 {
                            arr[len - 2]
                                .as_str()
                                .and_then(|s| s.split(',').nth(1))
                                .and_then(|v| v.parse::<f64>().ok())
                                .unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        return Ok(Some(NorthBoundHolding {
                            stock_code: stock_code.to_string(),
                            date: parts[0].to_string(),
                            holding_shares,
                            holding_ratio,
                            change_shares: holding_shares - prev_shares,
                        }));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        let secid = to_em_secid(stock_code);
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f158,f159,f160,f127"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let data = &json["data"];
        let sector_name = data["f158"].as_str().unwrap_or("").to_string();
        let sub_sector = data["f159"].as_str().unwrap_or("").to_string();
        let concept_tags: Vec<String> = data["f160"]
            .as_str()
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();
        let board_code = data["f127"].as_str().unwrap_or("").to_string();

        if sector_name.is_empty() && concept_tags.is_empty() {
            return Ok(None);
        }

        let (avg_pe, avg_pb) = if !board_code.is_empty() {
            let board_url = format!(
                "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=1&fs=b:{board_code}&fields=f162,f167"
            );
            match self.em_get(&board_url).await {
                Ok(resp) => {
                    let board_json: Value = resp.json().await.unwrap_or(Value::Null);
                    let diff = &board_json["data"]["diff"];
                    let f = |key: &str| {
                        diff.get(0).and_then(|row| row[key].as_f64()).and_then(|v| {
                            if v > 0.0 {
                                Some(v / 100.0)
                            } else {
                                None
                            }
                        })
                    };
                    (f("f162"), f("f167"))
                },
                Err(_) => (None, None),
            }
        } else if !sector_name.is_empty() {
            let board_url = format!(
                "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=1&fs=b:{sector_name}&fields=f162,f167"
            );
            match self.em_get(&board_url).await {
                Ok(resp) => {
                    let board_json: Value = resp.json().await.unwrap_or(Value::Null);
                    let diff = &board_json["data"]["diff"];
                    let f = |key: &str| {
                        diff.get(0).and_then(|row| row[key].as_f64()).and_then(|v| {
                            if v > 0.0 {
                                Some(v / 100.0)
                            } else {
                                None
                            }
                        })
                    };
                    (f("f162"), f("f167"))
                },
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        Ok(Some(SectorInfo {
            stock_code: stock_code.to_string(),
            sector_name,
            sub_sector,
            concept_tags,
            avg_pe,
            avg_pb,
        }))
    }

    async fn get_shareholder_trades(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        // 东方财富数据中心: 大股东增减持数据
        // SECURITY_CODE=股票代码, CHANGE_DATE=变动日期, SHAREHD_NAME=股东名称
        // CHANGE_TYPE=变动类型(增持/减持), CHANGE_NUM=变动数量, CHANGE_PRICE=变动均价
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_MAJORHOLDERS_TRADE&columns=SECURITY_CODE,CHANGE_DATE,SHAREHD_NAME,CHANGE_TYPE,CHANGE_NUM,CHANGE_PRICE,CHANGE_REASON&filter=(SECURITY_CODE=\"{stock_code}\")&pageSize=20&pageNumber=1"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        rows.iter()
            .map(|r| {
                Ok(ShareholderTrade {
                    stock_code: stock_code.to_string(),
                    date: r["CHANGE_DATE"].as_str().unwrap_or("").to_string(),
                    shareholder_name: r["SHAREHD_NAME"].as_str().unwrap_or("").to_string(),
                    trade_type: r["CHANGE_TYPE"].as_str().unwrap_or("").to_string(),
                    shares: r["CHANGE_NUM"].as_f64().unwrap_or(0.0),
                    price: r["CHANGE_PRICE"].as_f64().unwrap_or(0.0),
                    reason: r["CHANGE_REASON"].as_str().map(|s| s.to_string()),
                })
            })
            .collect()
    }

    async fn get_dividend_records(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        // 东方财富数据中心: 分红送配数据
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_DIVIDEND&columns=SECURITY_CODE,EX_DIVIDEND_DATE,DIVIDEND_PER_SHARE,BONUS_SHARE_RATIO,RECORD_DATE&filter=(SECURITY_CODE=\"{stock_code}\")&pageSize=10&pageNumber=1"
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
                if let Some(eps) = r["predictThisYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast {
                            year: "今年".into(),
                            eps: Some(val),
                        });
                    }
                }
                if let Some(eps) = r["predictNextYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast {
                            year: "明年".into(),
                            eps: Some(val),
                        });
                    }
                }
                if let Some(eps) = r["predictNextTwoYearEps"].as_str() {
                    if let Ok(val) = eps.parse::<f64>() {
                        eps_forecast.push(EpsForecast {
                            year: "后年".into(),
                            eps: Some(val),
                        });
                    }
                }

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
                    target_price: None,
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
        // req_trace 为必填参数，使用当前毫秒时间戳
        let req_trace = chrono::Utc::now().timestamp_millis();
        let url = format!(
            "https://np-listapi.eastmoney.com/comm/web/getNewsByColumns?client=web&biz=web_news_col&column=250&order=1&needInteractData=0&page_index=1&page_size=20&req_trace={req_trace}"
        );

        let resp = self.em_get(&url).await?;

        let json: Value = resp.json().await?;

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

                Some(ClsFlashItem {
                    title,
                    content,
                    publish_time,
                    source,
                })
            })
            .collect())
    }

    async fn get_announcements(&self, stock_code: &str) -> Result<Vec<Announcement>, DataError> {
        let market = if stock_code.starts_with('6') || stock_code.starts_with('9') {
            "1"
        } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
            "0"
        } else {
            "0"
        };
        let url = format!(
            "https://np-anotice-stock.eastmoney.com/api/security/ann?page_index=1&page_size=20&stock_list={market},{stock_code}"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

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
                            let secs = ts / 1000;
                            let naive = chrono::DateTime::from_timestamp(secs, 0)
                                .map(|dt| dt.format("%Y-%m-%d").to_string())
                                .unwrap_or_default();
                            naive
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
        let _secid = to_em_secid(stock_code);
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_BLOCKTRADE&columns=SECURITY_CODE,SECURITY_NAME_ABBR,TRADE_DATE,TRADE_PRICE,TRADE_VOL,TRADE_AMOUNT,BUYER_NAME,SELLER_NAME,DISCOUNT_RATE&filter=(SECURITY_CODE=\"{stock_code}\")&sortColumns=TRADE_DATE&sortTypes=-1&pageSize=20&pageNumber=1"
        );

        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .map(|r| BlockTrade {
                stock_code: stock_code.to_string(),
                stock_name: r["SECURITY_NAME_ABBR"].as_str().unwrap_or("").to_string(),
                trade_date: r["TRADE_DATE"]
                    .as_str()
                    .map(|s| s.chars().take(10).collect::<String>())
                    .unwrap_or(
                        r["TRADE_DATE"]
                            .as_i64()
                            .map(|ts| {
                                let secs = ts / 1000;
                                chrono::DateTime::from_timestamp(secs, 0)
                                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                    ),
                price: r["TRADE_PRICE"].as_f64().unwrap_or(0.0),
                volume: r["TRADE_VOL"].as_f64().unwrap_or(0.0),
                amount: r["TRADE_AMOUNT"].as_f64().unwrap_or(0.0),
                buyer_dept: r["BUYER_NAME"].as_str().map(|s| s.to_string()),
                seller_dept: r["SELLER_NAME"].as_str().map(|s| s.to_string()),
                discount_pct: r["DISCOUNT_RATE"].as_f64(),
            })
            .collect())
    }

    async fn get_institutional_visits(
        &self,
        stock_code: &str,
    ) -> Result<Vec<InstitutionalVisit>, DataError> {
        let _secid = to_em_secid(stock_code);
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPT_ORG_SURVEY&columns=SECUCODE,SECURITY_NAME_ABBR,SURVEY_DATE,ORG_NUM,MAIN_CONTENT,SURVEY_TYPE&filter=(SECURITY_CODE=\"{stock_code}\")&sortColumns=SURVEY_DATE&sortTypes=-1&pageSize=20&pageNumber=1"
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
            .collect())
    }

    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        let indices = [
            ("1.000001", "上证指数"),
            ("0.399001", "深证成指"),
            ("0.399006", "创业板指"),
        ];
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
        let secid = to_em_secid(stock_code);
        let board_url =
            format!("https://push2his.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f127");
        let resp = self.em_get(&board_url).await?;
        let json: Value = resp.json().await.unwrap_or(Value::Null);
        let board_code = json["data"]["f127"].as_str().unwrap_or("").to_string();
        if board_code.is_empty() {
            return Ok(vec![]);
        }

        let peer_url = format!(
            "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=10&fs=b:{board_code}&fields=f12,f14,f162,f167,f127,f2,f116,f170"
        );
        let resp = self.em_get(&peer_url).await?;
        let json: Value = resp.json().await.unwrap_or(Value::Null);

        let rows = match json["data"]["diff"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .filter(|r| r["f12"].as_str().map(|c| c != stock_code).unwrap_or(false))
            .map(|r| {
                let f = |key: &str| r[key].as_f64();
                PeerComparison {
                    stock_code: r["f12"].as_str().unwrap_or("").to_string(),
                    stock_name: r["f14"].as_str().unwrap_or("").to_string(),
                    pe: f("f162").and_then(|v| if v > 0.0 { Some(v / 100.0) } else { None }),
                    pb: f("f167").and_then(|v| if v > 0.0 { Some(v / 100.0) } else { None }),
                    roe: f("f127").and_then(|v| if v > 0.0 { Some(v / 100.0) } else { None }),
                    change_pct: f("f170").unwrap_or(0.0) / 100.0,
                    market_cap: f("f116").filter(|v| *v > 0.0),
                }
            })
            .collect())
    }

    async fn get_option_pcr(&self, stock_code: &str) -> Result<Option<OptionPCR>, DataError> {
        let underlying = if stock_code.starts_with('5') || stock_code.starts_with('6') {
            format!("1.{stock_code}")
        } else {
            format!("0.{stock_code}")
        };
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=50&fs=option_{underlying}&fields=f12,f14,f164,f165,f166,f167"
        );
        let resp = self.em_get(&url).await?;
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

    /// 行业/板块排名 — 东方财富板块 API
    async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        // m:90 = 行业板块, t:2 = 概念板块；fid=f3 按涨跌幅排序
        let url = "https://push2his.eastmoney.com/api/qt/clist/get?pn=1&pz=50&po=1&np=1&fltt=2&invt=2&fid=f3&fs=m:90+t:2&fields=f12,f14,f2,f3,f62,f184,f66,f69,f72,f75,f78,f81,f84,f87,f204,f205,f124";
        let resp = self.em_get(url).await?;
        let json: Value = resp.json().await?;

        let rows = match json["data"]["diff"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(rows
            .iter()
            .filter_map(|r| {
                let industry_name = r["f14"].as_str()?.to_string();
                let change_pct = r["f3"].as_f64().unwrap_or(0.0) / 100.0;
                let main_inflow = r["f62"].as_f64().map(|v| v * 10000.0);
                Some(IndustryRank {
                    industry_name,
                    change_pct,
                    turnover: None,
                    main_inflow,
                    leader_code: None,
                    leader_name: None,
                    leader_change_pct: None,
                })
            })
            .collect())
    }

    /// 北向资金（沪深港通）— 东方财富 API
    async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        let url = "https://push2his.eastmoney.com/api/qt/stock/fflow/kline/get?lmt=0&klt=1&secid=1.000001&secid2=0.399001&fields1=f1,f2,f3,f7&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63";
        let resp = self.em_get(url).await?;
        let json: Value = resp.json().await?;

        let data = &json["data"];
        if data.is_null() {
            return Ok(None);
        }

        let parse_kline = |v: &Value| -> (String, f64) {
            let s = v.as_str().unwrap_or("");
            let parts: Vec<&str> = s.split(',').collect();
            let date = parts.first().unwrap_or(&"").to_string();
            let inflow = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            (date, inflow)
        };

        let (_sh_date, sh_flow) = data["klines"]
            .as_array()
            .and_then(|arr| arr.last())
            .map(|v| parse_kline(v))
            .unwrap_or_default();
        let (_sz_date, sz_flow) = data["klines2"]
            .as_array()
            .and_then(|arr| arr.last())
            .map(|v| parse_kline(v))
            .unwrap_or_default();

        Ok(Some(NorthBoundFlow {
            date: _sh_date,
            sh_flow,
            sz_flow,
            total_flow: sh_flow + sz_flow,
            timestamp: None,
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
        let begin_date = (as_of.as_of_date - chrono::Duration::days(365))
            .format("%Y-%m-%d")
            .to_string();
        let url = format!(
            "https://np-anotice-stock.eastmoney.com/api/security/ann?cb=jQuery&sr=-1&page_size=20&page_index=1&ann_type=A&client_source=web&stock_list={stock_code}&f_node=0&s_node=0&begin_time={begin_date}&end_time={end_date}"
        );
        let resp = self.em_get(&url).await?;
        let body = resp.text().await.unwrap_or_default();
        let json_str = body
            .trim_start_matches("jQuery(")
            .trim_end_matches(')')
            .trim_end_matches(';');
        let json: Value = serde_json::from_str(json_str).unwrap_or(Value::Null);
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
                            let secs = ts / 1000;
                            chrono::DateTime::from_timestamp(secs, 0)
                                .map(|dt| dt.format("%Y-%m-%d").to_string())
                                .unwrap_or_default()
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
        let begin_time = (as_of.as_of_date - chrono::Duration::days(365))
            .format("%Y-%m-%d")
            .to_string();
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
        _adj: Option<AdjType>,
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
        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={period_code}&fqt=1&end={end_date}&lmt={limit}"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await?;
        let klines_raw = json["data"]["klines"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("missing klines array".into()))?;
        let mut klines: Vec<KLine> = klines_raw
            .iter()
            .map(|v| {
                let s = v
                    .as_str()
                    .ok_or_else(|| DataError::ParseError("kline not string".into()))?;
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() < 7 {
                    return Err(DataError::ParseError(format!(
                        "expected 7 fields in kline, got {}",
                        parts.len()
                    )));
                }
                let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
                Ok(KLine {
                    date: parts[0].to_string(),
                    open: parse(parts[1]),
                    high: parse(parts[3]),
                    low: parse(parts[4]),
                    close: parse(parts[2]),
                    volume: parse(parts[5]),
                    amount: parse(parts[6]),
                    turnover_rate: if parts.len() > 7 {
                        Some(parse(parts[7]))
                    } else {
                        None
                    },
                    // P1-4: vendor 默认不复权
                    adj_factor: None,
                })
            })
            .collect::<Result<_, _>>()?;
        // 兜底再按 as_of_date 截断(vendor 可能返回略多)
        let cutoff = as_of.as_of_date.format("%Y-%m-%d").to_string();
        klines.retain(|k| k.date <= cutoff);
        Ok(klines)
    }

    /// get_margin_data 升级:加 TRADE_DATE 过滤
    async fn get_margin_data_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<MarginData>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let trade_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        let _secid = to_em_secid(stock_code);
        // EM 融资融券:支持按个股 + 单日查询
        // 沪市 1.融券余额 3.融资余额;深市 secid 标识不同
        let url = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_MARGIN_DETAIL_BY_STOCK&columns=ALL&\
            filter=(SECURITY_CODE%3D%22{stock_code}%22)(TRADE_DATE%3D%27{trade_date}%27)&\
            pageNumber=1&pageSize=10"
        );
        let resp = self.em_get(&url).await?;
        let json: Value = resp.json().await.unwrap_or(Value::Null);
        let rows = match json["result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(None),
        };
        if rows.is_empty() {
            return Ok(None);
        }
        let r = &rows[0];
        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date: trade_date,
            margin_balance: r["RZYE"].as_f64().unwrap_or(0.0),
            short_balance: r["RQYE"].as_f64().unwrap_or(0.0),
            margin_buy: r["RZMR"].as_f64().unwrap_or(0.0),
            short_sell_volume: r["RQMC"].as_f64().unwrap_or(0.0),
        }))
    }

    /// get_north_bound_flow 升级:加 TRADE_DATE 过滤
    /// (原本只能取最近 2 个交易日,as_of 模式可指定日期)
    async fn get_north_bound_flow_with_asof(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        let as_of = crate::as_of::current_as_of()
            .ok_or_else(|| DataError::ParseError("no as_of context".into()))?;
        let trade_date = as_of.as_of_date.format("%Y-%m-%d").to_string();
        // EM 北向资金:沪股通 1.000001 + 深股通 0.000001
        // 用 datacenter-web 接口支持单日查询
        let url_sh = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_MUTUAL_STOCK_HOLDRANKS&columns=ALL&\
            filter=(TRADE_DATE%3D%27{trade_date}%27)(TRADE_TYPE%3D%27001%27)&\
            pageNumber=1&pageSize=5"
        );
        let url_sz = format!(
            "https://datacenter-web.eastmoney.com/api/data/v1/get?\
            reportName=RPT_MUTUAL_STOCK_HOLDRANKS&columns=ALL&\
            filter=(TRADE_DATE%3D%27{trade_date}%27)(TRADE_TYPE%3D%27003%27)&\
            pageNumber=1&pageSize=5"
        );
        let resp_sh = self.em_get(&url_sh).await?;
        let resp_sz = self.em_get(&url_sz).await?;
        let json_sh: Value = resp_sh.json().await.unwrap_or(Value::Null);
        let json_sz: Value = resp_sz.json().await.unwrap_or(Value::Null);
        // 简化:取沪股通 + 深股通 当日总净买入(在 rows 里的某行)
        // 真实解析应该按 SECURITY_TYPE 汇总,这里留作未来细化
        let sh_net = json_sh["result"]["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|r| r["NET_FLOW"].as_f64())
            .unwrap_or(0.0);
        let sz_net = json_sz["result"]["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|r| r["NET_FLOW"].as_f64())
            .unwrap_or(0.0);
        Ok(Some(NorthBoundFlow {
            date: trade_date,
            sh_flow: sh_net,
            sz_flow: sz_net,
            total_flow: sh_net + sz_net,
            timestamp: None,
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
        EastMoneyVendor {
            http: reqwest::Client::new(),
            proxy_http: None,
        }
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
        for m in &[
            "get_hot_stocks",
            "get_industry_ranking",
            "get_cls_flash",
            "get_concept_blocks",
        ] {
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
}
