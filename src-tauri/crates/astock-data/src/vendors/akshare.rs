use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct AkshareVendor {
    pub http: reqwest::Client,
}

fn val_to_f64(v: &Value) -> Option<f64> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
}

#[async_trait]
impl StockVendor for AkshareVendor {
    async fn get_quote(&self, _: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "akshare".into(),
            message: "quote handled by tencent/mootdx vendor".into(),
        })
    }

    async fn get_klines(
        &self,
        _: &str,
        _: &str,
        _: u32,
        _: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        Ok(vec![])
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        let url = format!(
            "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/ZYZBAjaxNew?type=0&code={}",
            if stock_code.starts_with('6') || stock_code.starts_with('9') {
                format!("SH{}", stock_code)
            } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
                format!("BJ{}", stock_code)
            } else {
                format!("SZ{}", stock_code)
            }
        );

        let resp = self
            .http
            .get(&url)
            .header("Referer", "https://emweb.securities.eastmoney.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

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
                    "pageSize": limit,
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
            .await?;

        let text = resp.text().await?;

        let json_str = text
            .trim()
            .trim_start_matches("jQuery(")
            .trim_start_matches("jQuery")
            .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim_end_matches(';');

        let json: Value = serde_json::from_str(json_str)
            .map_err(|e| DataError::ParseError(format!("jsonp parse failed: {e}")))?;

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
                let url = item
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
                    url,
                    publish_time,
                    sentiment_score: None,
                })
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

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        let url =
            "https://www.cls.cn/nodeapi/updateTelegraphList?app=CailianpressWeb&os=web&sv=7.7.5";

        let resp = self
            .http
            .get(url)
            .header("Referer", "https://www.cls.cn/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        let json: Value = resp.json().await?;

        let items = match json["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let title = item
                    .get("title")
                    .or_else(|| item.get("brief"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let content = item
                    .get("content")
                    .or_else(|| item.get("desc"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("ctime")
                    .or_else(|| item.get("publish_time"))
                    .or_else(|| item.get("created_at"))
                    .and_then(|v| {
                        v.as_i64()
                            .map(|ts| {
                                chrono::DateTime::from_timestamp(ts, 0)
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_default()
                            })
                            .or_else(|| v.as_str().map(|s| s.to_string()))
                    })
                    .unwrap_or_default();
                let source = item
                    .get("source")
                    .or_else(|| item.get("author"))
                    .and_then(|v| v.as_str().map(|s| s.to_string()));

                Some(ClsFlashItem {
                    title,
                    content,
                    publish_time,
                    source,
                })
            })
            .collect())
    }

    async fn get_consensus_eps(&self, stock_code: &str) -> Result<Option<ConsensusEPS>, DataError> {
        let url = format!("https://basic.10jqka.com.cn/{}/worth/", stock_code);
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://basic.10jqka.com.cn/")
            .send()
            .await?;

        let text = resp.text().await?;

        let eps = extract_json_between(&text, "var forecastData = ", ";")
            .and_then(|json_str| serde_json::from_str::<Value>(&json_str).ok());

        let eps_data = match eps {
            Some(v) => v,
            None => return Ok(None),
        };

        let items = match eps_data.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok(None),
        };

        let latest = &items[0];
        let year = latest
            .get("year")
            .and_then(|v| v.as_str().or_else(|| v.as_i64().map(|_| "")))
            .unwrap_or("")
            .to_string();
        let consensus_eps = latest.get("avg").and_then(val_to_f64);
        let rating_count = latest.get("num").and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<i32>().ok())
                .or_else(|| v.as_i64().map(|i| i as i32))
        });

        if consensus_eps.is_none() && rating_count.is_none() {
            return Ok(None);
        }

        Ok(Some(ConsensusEPS {
            stock_code: stock_code.to_string(),
            consensus_eps,
            consensus_target_price: None,
            rating_avg: None,
            rating_count,
            year,
        }))
    }

    // ── Vendor trait 大重构 P2:akshare 能力申报 ──
    // akshare 实现:financials/news/cls_flash/consensus_eps
    // - cls_flash:财联社快讯,当下语义无历史(NoHistoricalSemantic)
    // - 其他:带 date 字段,lib.rs 已正确 truncate(Fallthrough)
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "get_cls_flash" => AsOfCapability::NoHistoricalSemantic,
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> AkshareVendor {
        AkshareVendor {
            http: reqwest::Client::new(),
        }
    }

    #[test]
    fn akshare_cls_flash_is_no_historical() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_cls_flash"), AsOfCapability::NoHistoricalSemantic);
    }

    #[test]
    fn akshare_others_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_financials",
            "get_news",
            "get_consensus_eps",
            "get_quote",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::Fallthrough);
        }
    }
}

fn extract_json_between(text: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = text.find(start)?;
    let json_start = start_idx + start.len();
    let json_end = text[json_start..].find(end)?;
    Some(text[json_start..json_start + json_end].to_string())
}
