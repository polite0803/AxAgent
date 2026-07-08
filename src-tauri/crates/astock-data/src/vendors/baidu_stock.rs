use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct BaiduStockVendor {
    pub http: reqwest::Client,
}

fn to_baidu_code(stock_code: &str) -> String {
    let prefix = if stock_code.starts_with('6') || stock_code.starts_with('9') {
        "sh"
    } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
        "bj"
    } else {
        "sz"
    };
    format!("{prefix}{stock_code}")
}

fn val_to_f64(v: &Value) -> Option<f64> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
}

impl BaiduStockVendor {
    fn build_url(&self, resource_id: u32, code: &str, extra: &str) -> String {
        format!(
            "https://gushitong.baidu.com/opendata?openapi=1&dspName=iphone&tn=tangram&client=app&resource_id={resource_id}&code={code}{extra}"
        )
    }

    async fn fetch_json(&self, url: &str) -> Result<Value, DataError> {
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://gushitong.baidu.com/")
            .send()
            .await?;
        crate::check_response_429(&resp, "baidu_stock")?;
        let json: Value = resp.json().await?;
        Ok(json)
    }
}

#[async_trait]
impl StockVendor for BaiduStockVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5352, &code, "");
        let json = self.fetch_json(&url).await?;

        let result = &json["Result"];
        if result.is_null() {
            return Err(DataError::VendorError {
                vendor: "baidu_stock".into(),
                message: "no quote data".into(),
            });
        }

        let price = val_to_f64(&result["price"]).unwrap_or(0.0);
        let open = val_to_f64(&result["open"]).unwrap_or(0.0);
        let high = val_to_f64(&result["high"]).unwrap_or(0.0);
        let low = val_to_f64(&result["low"]).unwrap_or(0.0);
        let _close = val_to_f64(&result["close"])
            .or(val_to_f64(&result["yestclose"]))
            .unwrap_or(0.0);
        let volume = val_to_f64(&result["volume"]).unwrap_or(0.0);
        let amount = val_to_f64(&result["amount"]).unwrap_or(0.0);
        let change_pct = val_to_f64(&result["changepercent"]).unwrap_or(0.0);
        let turnover_rate = val_to_f64(&result["turnoverratio"]).unwrap_or(0.0);

        Ok(StockQuote {
            code: stock_code.to_string(),
            name: result["name"].as_str().unwrap_or("").to_string(),
            price,
            pre_close: 0.0,
            open,
            high,
            low,
            volume,
            amount,
            change_pct,
            turnover_rate,
            pe: val_to_f64(&result["pe"]),
            pb: val_to_f64(&result["pb"]),
            total_mv: val_to_f64(&result["totalmktcap"]),
            circulating_mv: val_to_f64(&result["circulatingmktcap"]),
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: result["time"].as_str().unwrap_or("").to_string(),
        })
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let code = to_baidu_code(stock_code);
        let ktype = match period {
            "daily" | "101" | "Daily" | "8" => "day",
            "weekly" | "102" | "Weekly" | "9" => "week",
            "monthly" | "103" | "Monthly" | "10" => "month",
            _ => "day",
        };
        let url = self.build_url(5353, &code, &format!("&type={ktype}&count={limit}"));
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => match json["Result"].as_array() {
                Some(arr) => arr,
                None => return Ok(vec![]),
            },
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let date = item.get("date")?.as_str()?.to_string();
                let open = val_to_f64(item.get("open")?)?;
                let close = val_to_f64(item.get("close")?)?;
                let high = val_to_f64(item.get("high")?)?;
                let low = val_to_f64(item.get("low")?)?;
                let volume = val_to_f64(item.get("volume")?).unwrap_or(0.0);
                let amount = val_to_f64(item.get("amount")?).unwrap_or(0.0);
                let turnover_rate = item.get("turnoverratio").and_then(val_to_f64);

                Some(KLine {
                    date,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    amount,
                    turnover_rate,
                    // P1-4: vendor 默认不复权
                    adj_factor: None,
                })
            })
            .collect())
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5354, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| FinancialReport {
                stock_code: stock_code.to_string(),
                report_date: item["reportDate"].as_str().unwrap_or("").to_string(),
                revenue: val_to_f64(&item["totalOperateIncome"]),
                net_profit: val_to_f64(&item["parentNetprofit"]),
                eps: val_to_f64(&item["basicEps"]),
                bps: val_to_f64(&item["bps"]),
                roe: val_to_f64(&item["weightavgRoe"]),
                debt_ratio: val_to_f64(&item["debtAssetRatio"]),
                gross_margin: val_to_f64(&item["grossProfitRatio"]),
                net_margin: val_to_f64(&item["netprofitMargin"]),
                revenue_yoy: val_to_f64(&item["totalOperateIncomeYoy"]),
                profit_yoy: val_to_f64(&item["parentNetprofitYoy"]),
                total_assets: None,
                operating_cash_flow: None,
                capital_expenditure: None,
                free_cash_flow: None,
                current_ratio: None,
                quick_ratio: None,
            })
            .collect())
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5366, &code, &format!("&count={limit}"));
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
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
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("百度股市通")
                    .to_string();
                let url = item
                    .get("url")
                    .or_else(|| item.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let publish_time = item
                    .get("ptime")
                    .or_else(|| item.get("publishTime"))
                    .or_else(|| item.get("time"))
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

    async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5356, &code, "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        Ok(Some(MoneyFlow {
            date: data["date"].as_str().unwrap_or("").to_string(),
            main_net_inflow: val_to_f64(&data["mainNetInflow"]).unwrap_or(0.0),
            super_large_net: val_to_f64(&data["superLargeNet"]).unwrap_or(0.0),
            large_net: val_to_f64(&data["largeNet"]).unwrap_or(0.0),
            medium_net: val_to_f64(&data["mediumNet"]).unwrap_or(0.0),
            small_net: val_to_f64(&data["smallNet"]).unwrap_or(0.0),
        }))
    }

    async fn get_dragon_tiger(&self, stock_code: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5360, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| DragonTigerEntry {
                stock_code: stock_code.to_string(),
                date: item["date"].as_str().unwrap_or("").to_string(),
                dept_name: item["deptName"].as_str().unwrap_or("").to_string(),
                buy_amount: val_to_f64(&item["buyAmount"]).unwrap_or(0.0),
                sell_amount: val_to_f64(&item["sellAmount"]).unwrap_or(0.0),
                net_amount: val_to_f64(&item["netAmount"]).unwrap_or(0.0),
                reason: item["reason"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    async fn get_lockup_schedule(
        &self,
        stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5362, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| LockupSchedule {
                stock_code: stock_code.to_string(),
                stock_name: item["stockName"].as_str().unwrap_or("").to_string(),
                unlock_date: item["unlockDate"].as_str().unwrap_or("").to_string(),
                unlock_shares: val_to_f64(&item["unlockShares"]).unwrap_or(0.0),
                unlock_ratio: val_to_f64(&item["unlockRatio"]).unwrap_or(0.0),
                shareholder: item["shareholder"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5363, &code, "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        Ok(Some(MarginData {
            stock_code: stock_code.to_string(),
            date: data["date"].as_str().unwrap_or("").to_string(),
            margin_buy: val_to_f64(&data["marginBuy"]).unwrap_or(0.0),
            margin_balance: val_to_f64(&data["marginBalance"]).unwrap_or(0.0),
            short_sell_volume: val_to_f64(&data["shortSellVolume"]).unwrap_or(0.0),
            short_balance: val_to_f64(&data["shortBalance"]).unwrap_or(0.0),
        }))
    }

    async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5364, &code, "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        Ok(Some(NorthBoundHolding {
            stock_code: stock_code.to_string(),
            date: data["date"].as_str().unwrap_or("").to_string(),
            holding_shares: val_to_f64(&data["holdingShares"]).unwrap_or(0.0),
            holding_ratio: val_to_f64(&data["holdingRatio"]).unwrap_or(0.0),
            change_shares: val_to_f64(&data["changeShares"]).unwrap_or(0.0),
        }))
    }

    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5359, &code, "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        let sector_name = data["industry"].as_str().unwrap_or("").to_string();
        let concept_tags: Vec<String> = data["concepts"]
            .as_str()
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        if sector_name.is_empty() && concept_tags.is_empty() {
            return Ok(None);
        }

        Ok(Some(SectorInfo {
            stock_code: stock_code.to_string(),
            sector_name,
            sub_sector: data["subIndustry"].as_str().unwrap_or("").to_string(),
            concept_tags,
            avg_pe: None,
            avg_pb: None,
        }))
    }

    async fn get_shareholder_trades(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5367, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| ShareholderTrade {
                stock_code: stock_code.to_string(),
                date: item["date"].as_str().unwrap_or("").to_string(),
                shareholder_name: item["shareholderName"].as_str().unwrap_or("").to_string(),
                trade_type: item["tradeType"].as_str().unwrap_or("").to_string(),
                shares: val_to_f64(&item["shares"]).unwrap_or(0.0),
                price: val_to_f64(&item["price"]).unwrap_or(0.0),
                reason: item["reason"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    async fn get_dividend_records(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5358, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| DividendRecord {
                stock_code: stock_code.to_string(),
                ex_date: item["exDate"].as_str().unwrap_or("").to_string(),
                dividend_per_share: val_to_f64(&item["dividendPerShare"]).unwrap_or(0.0),
                bonus_share_ratio: val_to_f64(&item["bonusShareRatio"]).unwrap_or(0.0),
                record_date: item["recordDate"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let url = self.build_url(5351, "", &format!("&query={}", urlencoding::encode(keyword)));
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let code = item.get("code")?.as_str()?.to_string();
                let name = item.get("name")?.as_str()?.to_string();
                let market = item
                    .get("market")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(StockSearchResult { code, name, market })
            })
            .collect())
    }

    async fn get_research_reports(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5365, &code, "");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .map(|item| ResearchReport {
                title: item["title"].as_str().unwrap_or("").to_string(),
                institution: item["institution"].as_str().unwrap_or("").to_string(),
                analyst: item["analyst"].as_str().map(|s| s.to_string()),
                rating: item["rating"].as_str().map(|s| s.to_string()),
                target_price: val_to_f64(&item["targetPrice"]),
                eps_forecast: vec![],
                publish_date: item["publishDate"].as_str().unwrap_or("").to_string(),
                pdf_url: item["pdfUrl"].as_str().map(|s| s.to_string()),
            })
            .collect())
    }

    async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        let code = to_baidu_code(stock_code);
        let url = self.build_url(5359, &code, "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        let industry = data["industry"].as_str().unwrap_or("").to_string();
        let concepts: Vec<BlockItem> = data["conceptList"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(BlockItem {
                            name: item.get("name")?.as_str()?.to_string(),
                            change_pct: item.get("changePct").and_then(val_to_f64),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

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
        let url = self.build_url(5359, "", "&type=hot");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let code = item.get("code")?.as_str()?.to_string();
                let name = item.get("name")?.as_str()?.to_string();
                let change_pct = item
                    .get("changePct")
                    .or_else(|| item.get("changepercent"))
                    .and_then(val_to_f64)
                    .unwrap_or(0.0);
                let turnover_rate = item.get("turnoverRatio").and_then(val_to_f64);
                let reason_tags = item
                    .get("reasonTags")
                    .and_then(|v| {
                        v.as_str()
                            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                    })
                    .unwrap_or_default();
                let sector = item
                    .get("industry")
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
        let url = self.build_url(5359, "", "&type=ranking");
        let json = self.fetch_json(&url).await?;

        let items = match json["Result"]["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(items
            .iter()
            .filter_map(|item| {
                let industry_name = item
                    .get("industryName")
                    .or_else(|| item.get("name"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let change_pct = item
                    .get("changePct")
                    .or_else(|| item.get("changepercent"))
                    .and_then(val_to_f64)
                    .unwrap_or(0.0);
                let turnover = item
                    .get("turnover")
                    .or_else(|| item.get("amount"))
                    .and_then(val_to_f64);
                let leader_code = item
                    .get("leaderCode")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                let leader_name = item
                    .get("leaderName")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                let leader_change_pct = item.get("leaderChangePct").and_then(val_to_f64);

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
        let url = self.build_url(5364, "", "");
        let json = self.fetch_json(&url).await?;

        let data = &json["Result"];
        if data.is_null() {
            return Ok(None);
        }

        let sh_flow = val_to_f64(&data["shFlow"]).unwrap_or(0.0);
        let sz_flow = val_to_f64(&data["szFlow"]).unwrap_or(0.0);

        Ok(Some(NorthBoundFlow {
            date: data["date"].as_str().unwrap_or("").to_string(),
            sh_flow,
            sz_flow,
            total_flow: sh_flow + sz_flow,
            timestamp: data["time"].as_str().map(|s| s.to_string()),
        }))
    }

    // ── Vendor trait 大重构 P2:baidu_stock 能力申报 ──
    // 百度财经 API 形态:
    // - quote:实时快照,用 K 线合成(SynthesizeFromKline)
    // - klines:原生支持日期范围(NativeDateParam)
    // - 其他方法:返回带 date 字段的全量,lib.rs truncate_by_asof 正确截断(Fallthrough)
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "get_quote" => AsOfCapability::SynthesizeFromKline,
            "get_klines" => AsOfCapability::NativeDateParam,
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> BaiduStockVendor {
        BaiduStockVendor {
            http: reqwest::Client::new(),
        }
    }

    #[test]
    fn baidu_asof_capability_quote_is_synthesize() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_quote"), AsOfCapability::SynthesizeFromKline);
    }

    #[test]
    fn baidu_asof_capability_klines_is_native() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_klines"), AsOfCapability::NativeDateParam);
    }

    #[test]
    fn baidu_asof_capability_others_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_financials",
            "get_news",
            "get_money_flow",
            "get_dragon_tiger",
            "get_lockup_schedule",
            "get_margin_data",
            "get_north_bound_holding",
            "get_sector_info",
            "get_shareholder_trades",
            "get_dividend_records",
            "search_stock",
            "get_research_reports",
            "get_concept_blocks",
            "get_hot_stocks",
            "get_industry_ranking",
            "get_north_bound_flow",
        ] {
            assert_eq!(
                v.asof_capability(m),
                AsOfCapability::Fallthrough,
                "baidu.{m} 应为 Fallthrough(lib.rs truncate 正确)"
            );
        }
    }
}
