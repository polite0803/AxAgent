use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct IwencaiVendor {
    pub http: reqwest::Client,
    pub api_key: String,
}

fn val_to_f64(v: &Value) -> Option<f64> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
}

impl IwencaiVendor {
    async fn query(&self, question: &str, perpage: u32, page: u32) -> Result<Value, DataError> {
        if self.api_key.is_empty() {
            return Err(DataError::VendorError {
                vendor: "iwencai".into(),
                message: "api_key not configured".into(),
            });
        }

        let body = serde_json::json!({
            "question": question,
            "perpage": perpage,
            "page": page,
            "secondary_intent": "stock",
            "source": "Ths_iwencai_Xuangu",
            "version": "2.0",
            "add_info": "{\"urp\":{\"scene\":1,\"company\":1,\"business\":1},\"contentType\":\"json\",\"searchInfo\":true}"
        });

        let resp = self
            .http
            .post("https://openapi.iwencai.com/v1/comprehensive/search")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .json(&body)
            .send()
            .await?;

        let json: Value = resp.json().await?;

        if json["status_code"].as_i64() != Some(0) {
            let msg = json["status_msg"].as_str().unwrap_or("unknown error");
            return Err(DataError::VendorError {
                vendor: "iwencai".into(),
                message: msg.to_string(),
            });
        }

        Ok(json)
    }

    fn extract_code_list(&self, json: &Value) -> Vec<Value> {
        let answer = match json["data"]["answer"].as_array() {
            Some(arr) => arr,
            None => return vec![],
        };

        for item in answer {
            if item["type"].as_str() == Some("comp_table") {
                if let Some(components) = item["txt"]["components"].as_array() {
                    for comp in components {
                        if comp["type"].as_str() == Some("table") {
                            if let Some(code_list) = comp["data"]["code_list"].as_array() {
                                return code_list.clone();
                            }
                        }
                    }
                }
            }
        }

        vec![]
    }
}

#[async_trait]
impl StockVendor for IwencaiVendor {
    async fn get_quote(&self, _: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "iwencai".into(),
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

    async fn get_news(&self, _: &str, _: u32) -> Result<Vec<NewsItem>, DataError> {
        Ok(vec![])
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
        let question = format!("{keyword} 股票");
        let json = self.query(&question, 20, 1).await?;
        let code_list = self.extract_code_list(&json);

        Ok(code_list
            .iter()
            .filter_map(|item| {
                let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let market = item
                    .get("market_code")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.ends_with(".SZ") {
                            "深圳"
                        } else if s.ends_with(".SH") {
                            "上海"
                        } else {
                            ""
                        }
                    })
                    .unwrap_or("")
                    .to_string();

                if code.is_empty() {
                    return None;
                }

                Some(StockSearchResult {
                    code: code.to_string(),
                    name: name.to_string(),
                    market,
                })
            })
            .collect())
    }

    async fn get_consensus_eps(&self, stock_code: &str) -> Result<Option<ConsensusEPS>, DataError> {
        let question = format!("{stock_code} 一致预期EPS 机构预测");
        let json = self.query(&question, 10, 1).await?;
        let code_list = self.extract_code_list(&json);

        if code_list.is_empty() {
            return Ok(None);
        }

        let item = &code_list[0];
        let consensus_eps = item
            .get("predict_eps")
            .or_else(|| item.get("consensus_eps"))
            .or_else(|| item.get("预测EPS"))
            .and_then(val_to_f64);
        let rating_count = item
            .get("rating_count")
            .or_else(|| item.get("预测机构数"))
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .or_else(|| v.as_i64().map(|i| i as i32))
            });

        Ok(Some(ConsensusEPS {
            stock_code: stock_code.to_string(),
            consensus_eps,
            consensus_target_price: item
                .get("target_price")
                .or_else(|| item.get("目标价"))
                .and_then(val_to_f64),
            rating_avg: item
                .get("rating_avg")
                .or_else(|| item.get("评级"))
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            rating_count,
            year: chrono::Utc::now().format("%Y").to_string(),
        }))
    }

    async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        let question = format!("{stock_code} 所属板块 概念");
        let json = self.query(&question, 50, 1).await?;
        let code_list = self.extract_code_list(&json);

        if code_list.is_empty() {
            return Ok(None);
        }

        let item = &code_list[0];
        let industry = item
            .get("industry")
            .or_else(|| item.get("行业"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let concepts: Vec<BlockItem> = item
            .get("concept")
            .or_else(|| item.get("概念"))
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|t| BlockItem {
                        name: t.trim().to_string(),
                        change_pct: None,
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
        let question = "今日涨幅前20的股票";
        let json = self.query(question, 20, 1).await?;
        let code_list = self.extract_code_list(&json);

        Ok(code_list
            .iter()
            .filter_map(|item| {
                let code = item.get("code")?.as_str()?.to_string();
                let name = item.get("name")?.as_str()?.to_string();
                let change_pct = item
                    .get("change_pct")
                    .or_else(|| item.get("涨跌幅"))
                    .and_then(val_to_f64)
                    .unwrap_or(0.0);
                let turnover_rate = item
                    .get("turnover_ratio")
                    .or_else(|| item.get("换手率"))
                    .and_then(val_to_f64);
                let sector = item
                    .get("industry")
                    .or_else(|| item.get("行业"))
                    .and_then(|v| v.as_str().map(|s| s.to_string()));

                Some(HotStock {
                    stock_code: code,
                    stock_name: name,
                    change_pct,
                    turnover_rate,
                    reason_tags: vec![],
                    sector,
                })
            })
            .collect())
    }

    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        let question = format!("{stock_code} 所属行业 概念板块");
        let json = self.query(&question, 10, 1).await?;
        let code_list = self.extract_code_list(&json);

        if code_list.is_empty() {
            return Ok(None);
        }

        let item = &code_list[0];
        let sector_name = item
            .get("industry")
            .or_else(|| item.get("行业"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let concept_tags: Vec<String> = item
            .get("concept")
            .or_else(|| item.get("概念"))
            .and_then(|v| v.as_str())
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        if sector_name.is_empty() && concept_tags.is_empty() {
            return Ok(None);
        }

        Ok(Some(SectorInfo {
            stock_code: stock_code.to_string(),
            sector_name,
            sub_sector: item
                .get("sub_industry")
                .or_else(|| item.get("二级行业"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            concept_tags,
            avg_pe: None,
            avg_pb: None,
        }))
    }

    // ── P3:iwencai 能力申报 ──
    // 真实实现:
    // - search_stock:搜索是当下语义 → NoHistoricalSemantic
    // - get_consensus_eps:带 year 字段 → Fallthrough
    // - get_concept_blocks:当下概念分类 → NoHistoricalSemantic
    // - get_hot_stocks:当下热门榜单 → NoHistoricalSemantic
    // - get_sector_info:当下行业分类 → NoHistoricalSemantic
    // 其他 stub:Fallthrough
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "search_stock" | "get_concept_blocks" | "get_hot_stocks" | "get_sector_info" => {
                AsOfCapability::NoHistoricalSemantic
            },
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> IwencaiVendor {
        IwencaiVendor {
            http: reqwest::Client::new(),
            api_key: "test".into(),
        }
    }

    #[test]
    fn iwencai_no_historical_methods() {
        let v = make_vendor();
        for m in &[
            "search_stock",
            "get_concept_blocks",
            "get_hot_stocks",
            "get_sector_info",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::NoHistoricalSemantic);
        }
    }

    #[test]
    fn iwencai_real_date_methods_are_fallthrough() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_consensus_eps"), AsOfCapability::Fallthrough);
    }

    #[test]
    fn iwencai_stub_methods_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_quote",
            "get_klines",
            "get_financials",
            "get_news",
            "get_money_flow",
            "get_dragon_tiger",
            "get_lockup_schedule",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::Fallthrough);
        }
    }
}
