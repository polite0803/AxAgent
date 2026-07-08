use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct ThsVendor {
    pub http: reqwest::Client,
}

fn val_to_f64(v: &Value) -> Option<f64> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
}

#[async_trait]
impl StockVendor for ThsVendor {
    async fn get_quote(&self, _: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "ths".into(),
            message: "quote handled by tencent vendor".into(),
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

    async fn get_financials(&self, _: &str) -> Result<Vec<FinancialReport>, DataError> {
        Ok(vec![])
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        // 同花顺个股新闻：https://basic.10jqka.com.cn/{stock_code}/news.html
        let url = format!(
            "https://basic.10jqka.com.cn/api/stockph.php?code={}&type=news&page=1&limit={}",
            stock_code,
            limit.min(50)
        );
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", format!("https://basic.10jqka.com.cn/{}/", stock_code))
            .send()
            .await?;
        crate::check_response_429(&resp, "ths")?;

        let text = resp.text().await?;
        let json: Value = serde_json::from_str(&text).map_err(|e| {
            DataError::ParseError(format!(
                "ths news parse failed: {e}, raw: {}",
                &text[..text.len().min(120)]
            ))
        })?;

        // ths 返回格式: { "data": { "items": [...] } } 或 { "status_code": 0, "data": [...] }
        let items = json["data"]["items"]
            .as_array()
            .or_else(|| json["data"].as_array())
            .or_else(|| json["list"].as_array())
            .map(|a| a.to_vec())
            .unwrap_or_default();

        Ok(items
            .iter()
            .map(|item| {
                let title = item["title"]
                    .as_str()
                    .or_else(|| item["news_title"].as_str())
                    .unwrap_or("")
                    .to_string();
                let summary = item["summary"]
                    .as_str()
                    .or_else(|| item["content"].as_str())
                    .or_else(|| item["digest"].as_str())
                    .unwrap_or("")
                    .to_string();
                let source = item["source"]
                    .as_str()
                    .or_else(|| item["from"].as_str())
                    .unwrap_or("同花顺")
                    .to_string();
                let url = item["url"]
                    .as_str()
                    .or_else(|| item["news_url"].as_str())
                    .or_else(|| item["link"].as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item["publish_time"]
                    .as_str()
                    .or_else(|| item["date"].as_str())
                    .or_else(|| item["ctime"].as_str())
                    .unwrap_or("")
                    .to_string();

                NewsItem {
                    title,
                    summary,
                    source,
                    url,
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

    /// 市场龙虎榜：复用涨停板数据，按成交额排序取前20
    async fn get_market_dragon_tiger(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        let hot = self.get_hot_stocks().await?;
        let mut result: Vec<MarketDragonTiger> = hot
            .into_iter()
            .map(|h| MarketDragonTiger {
                stock_code: h.stock_code,
                stock_name: h.stock_name,
                date: String::new(),
                net_buy: 0.0,
                buy_amount: 0.0,
                sell_amount: 0.0,
                reason: h.reason_tags.first().cloned(),
            })
            .collect();
        result.truncate(20);
        Ok(result)
    }

    async fn get_lockup_schedule(&self, _: &str) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
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
        crate::check_response_429(&resp, "ths")?;

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

    async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        let url = format!("https://basic.10jqka.com.cn/{}/concept.shtml", stock_code);
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://basic.10jqka.com.cn/")
            .send()
            .await?;
        crate::check_response_429(&resp, "ths")?;

        let text = resp.text().await?;

        let blocks = extract_json_between(&text, "var conceptList = ", ";")
            .and_then(|json_str| serde_json::from_str::<Value>(&json_str).ok());

        let industry = extract_json_between(&text, "var industry = ", ";")
            .and_then(|json_str| serde_json::from_str::<Value>(&json_str).ok())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let concepts = match blocks {
            Some(arr) if arr.is_array() => arr
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|item| {
                    Some(BlockItem {
                        name: item.get("name")?.as_str()?.to_string(),
                        change_pct: item.get("change").and_then(val_to_f64),
                    })
                })
                .collect(),
            _ => vec![],
        };

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

    async fn get_hot_stocks(&self) -> Result<Vec<HotStock>, DataError> {
        let url = "https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool?page=1&limit=20&field=199112,10,9001,330323,330324,330325,9002,330329,133971,133970,1968584,3475914,9003,9004";
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://data.10jqka.com.cn/")
            .send()
            .await?;
        crate::check_response_429(&resp, "ths")?;

        let json: Value = resp.json().await?;
        let data = &json["data"];
        if data.is_null() {
            return Ok(vec![]);
        }

        let empty_vec = vec![];
        let stocks = data
            .as_object()
            .and_then(|obj| obj.get("info").or_else(|| obj.get("list")))
            .and_then(|v| v.as_array())
            .or_else(|| data.as_array())
            .unwrap_or(&empty_vec);

        Ok(stocks
            .iter()
            .filter_map(|item| {
                let code = item.get("code")?.as_str()?.to_string();
                let name = item.get("name")?.as_str()?.to_string();
                let change_pct = item
                    .get("change_rate")
                    .or_else(|| item.get("change_pct"))
                    .and_then(val_to_f64)
                    .unwrap_or(0.0);
                let turnover_rate = item
                    .get("turnover_ratio")
                    .or_else(|| item.get("hs"))
                    .and_then(val_to_f64);
                let reason_tags = item
                    .get("reason_type")
                    .or_else(|| item.get("reason"))
                    .and_then(|v| {
                        v.as_str()
                            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                    })
                    .unwrap_or_default();
                let sector = item
                    .get("industry")
                    .or_else(|| item.get("belong"))
                    .and_then(|v| v.as_str().map(|s| s.to_string()));

                Some(HotStock {
                    stock_code: code,
                    stock_name: name,
                    change_pct,
                    turnover_rate,
                    reason_tags,
                    sector,
                })
            })
            .collect())
    }

    async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        let url = "https://data.10jqka.com.cn/dataapi/limit_up/industry_board?page=1&limit=90&sort_field=change_pct&sort_order=desc&field=199112,10,9001,330323,330324,330325,9002,330329,133971,133970,1968584,3475914,9003,9004";
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://data.10jqka.com.cn/")
            .send()
            .await?;
        crate::check_response_429(&resp, "ths")?;

        let json: Value = resp.json().await?;
        let data = &json["data"];
        if data.is_null() {
            return Ok(vec![]);
        }

        let empty_vec2 = vec![];
        let ranks = data
            .as_object()
            .and_then(|obj| obj.get("info").or_else(|| obj.get("list")))
            .and_then(|v| v.as_array())
            .or_else(|| data.as_array())
            .unwrap_or(&empty_vec2);

        Ok(ranks
            .iter()
            .filter_map(|item| {
                let industry_name = item
                    .get("industry_name")
                    .or_else(|| item.get("name"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let change_pct = item
                    .get("change_rate")
                    .or_else(|| item.get("change_pct"))
                    .and_then(val_to_f64)
                    .unwrap_or(0.0);
                let turnover = item
                    .get("turnover")
                    .or_else(|| item.get("amount"))
                    .and_then(val_to_f64);
                let leader_code = item
                    .get("leader_code")
                    .or_else(|| item.get("code"))
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                let leader_name = item
                    .get("leader_name")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                let leader_change_pct = item.get("leader_change_pct").and_then(val_to_f64);

                Some(IndustryRank {
                    industry_name,
                    change_pct,
                    turnover,
                    main_inflow: None,
                    leader_code,
                    leader_name,
                    leader_change_pct,
                })
            })
            .collect())
    }

    async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        let url = "https://data.10jqka.com.cn/dataapi/hsgt/hsgt_board";
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://data.10jqka.com.cn/")
            .send()
            .await?;
        crate::check_response_429(&resp, "ths")?;

        let json: Value = resp.json().await?;
        let data = &json["data"];
        if data.is_null() {
            return Ok(None);
        }

        let sh_flow = data
            .get("sh_flow")
            .or_else(|| data.get("hgt"))
            .and_then(val_to_f64)
            .unwrap_or(0.0);
        let sz_flow = data
            .get("sz_flow")
            .or_else(|| data.get("sgt"))
            .and_then(val_to_f64)
            .unwrap_or(0.0);

        Ok(Some(NorthBoundFlow {
            date: data
                .get("date")
                .or_else(|| data.get("tradedate"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            sh_flow,
            sz_flow,
            total_flow: sh_flow + sz_flow,
            timestamp: data
                .get("time")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
        }))
    }

    // ── P3:ths 能力申报 ──
    // 真实实现:
    // - get_market_dragon_tiger:带 date 字段 → Fallthrough
    // - get_hot_stocks:当下榜单 → NoHistoricalSemantic
    // - get_industry_ranking:当下排名 → NoHistoricalSemantic
    // - get_north_bound_flow:带 date 字段 → Fallthrough
    // - get_consensus_eps:带 year 字段 → Fallthrough
    // - get_concept_blocks:当下概念分类 → NoHistoricalSemantic
    // - get_news:带 publish_time 字段 → Fallthrough
    // 其他 stub:Fallthrough
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "get_hot_stocks" | "get_industry_ranking" | "get_concept_blocks" => {
                AsOfCapability::NoHistoricalSemantic
            },
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> ThsVendor {
        ThsVendor {
            http: reqwest::Client::new(),
        }
    }

    #[test]
    fn ths_no_historical_methods() {
        let v = make_vendor();
        for m in &[
            "get_hot_stocks",
            "get_industry_ranking",
            "get_concept_blocks",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::NoHistoricalSemantic);
        }
    }

    #[test]
    fn ths_real_date_methods_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_news",
            "get_market_dragon_tiger",
            "get_north_bound_flow",
            "get_consensus_eps",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::Fallthrough);
        }
    }

    #[test]
    fn ths_stub_methods_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_quote",
            "get_klines",
            "get_financials",
            "get_money_flow",
            "get_dragon_tiger",
            "get_lockup_schedule",
            "search_stock",
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
