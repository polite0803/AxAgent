//! 东方财富股吧 (guba.eastmoney.com) 社交舆情数据源
//!
//! 通过东方财富股吧 API 获取个股讨论热度、帖子数、情感倾向。
//! 仅实现 `get_social_sentiment`，其他行情/财务类方法返回空或错误，
//! 由路由层 (`VendorRouting`) 自动降级到其他 vendor。

use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct GubaVendor {
    pub http: reqwest::Client,
}

impl GubaVendor {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// 带反爬头的 GET 请求
    async fn guba_get(&self, url: &str) -> Result<Value, DataError> {
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://guba.eastmoney.com/")
            .header("Accept", "application/json, text/plain, */*")
            .send()
            .await
            .map_err(DataError::from)?;

        if !resp.status().is_success() {
            return Err(DataError::VendorError {
                vendor: "guba".into(),
                message: format!("HTTP {}", resp.status()),
            });
        }

        resp.json().await.map_err(DataError::from)
    }
}

#[async_trait]
impl StockVendor for GubaVendor {
    // ── 行情/财务类方法：股吧不提供，返回空或错误，由路由层降级 ──

    async fn get_quote(&self, _stock_code: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "guba".into(), message: "股吧不提供行情数据".into()
        })
    }

    async fn get_klines(
        &self,
        _stock_code: &str,
        _period: &str,
        _limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        Ok(vec![])
    }

    async fn get_financials(&self, _stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        Ok(vec![])
    }

    async fn get_news(&self, _stock_code: &str, _limit: u32) -> Result<Vec<NewsItem>, DataError> {
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

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    // ── 社交舆情：核心实现 ──

    /// 获取股吧社交舆情数据
    ///
    /// 使用东方财富股吧 API 获取帖子数和热度。
    /// 情感分析基于帖子标题关键词（"涨"/"牛" → 看多，"跌"/"熊" → 看空）。
    async fn get_social_sentiment(
        &self,
        stock_code: &str,
    ) -> Result<Vec<SocialSentiment>, DataError> {
        // 股吧 API：获取个股帖子列表
        // 参数：code=股票代码，ps=每页数量，p=页码
        let url = format!(
            "https://guba.eastmoney.com/interface/GetData.aspx?path=guba/newlist&param=ps%3D20%26code%3D{stock_code}%26p%3D1%26type%3D1"
        );

        let json = self.guba_get(&url).await?;

        // 解析帖子列表
        let posts = json["Data"]["data"]
            .as_array()
            .or_else(|| json["data"].as_array())
            .ok_or_else(|| DataError::VendorError {
                vendor: "guba".into(),
                message: "股吧数据格式异常".into(),
            })?;

        if posts.is_empty() {
            return Ok(vec![SocialSentiment {
                stock_code: stock_code.to_string(),
                stock_name: String::new(),
                platform: "guba".to_string(),
                post_count: 0,
                hot_rank: None,
                sentiment_score: None,
                bull_ratio: None,
                fetched_at: chrono::Utc::now().timestamp(),
            }]);
        }

        // 统计帖子数 + 简单情感分析
        let post_count = posts.len() as u32;
        let mut bull_count = 0u32;
        let mut bear_count = 0u32;

        for post in posts {
            let title =
                post["post_title"].as_str().or_else(|| post["title"].as_str()).unwrap_or("");

            // 基于关键词的情感分析（粗略估计）
            if title.contains("涨")
                || title.contains("牛")
                || title.contains("利好")
                || title.contains("加仓")
            {
                bull_count += 1;
            } else if title.contains("跌")
                || title.contains("熊")
                || title.contains("利空")
                || title.contains("减仓")
            {
                bear_count += 1;
            }
        }

        let total = bull_count + bear_count;
        let bull_ratio = if total > 0 {
            Some(bull_count as f64 / total as f64)
        } else {
            None
        };
        // sentiment_score: -1.0 ~ 1.0，bull_ratio 0.5 → 0.0
        let sentiment_score = bull_ratio.map(|r| (r - 0.5) * 2.0);

        let stock_name = posts
            .first()
            .and_then(|p| p["stock_name"].as_str().or_else(|| p["name"].as_str()))
            .unwrap_or("")
            .to_string();

        Ok(vec![SocialSentiment {
            stock_code: stock_code.to_string(),
            stock_name,
            platform: "guba".to_string(),
            post_count,
            hot_rank: None,
            sentiment_score,
            bull_ratio,
            fetched_at: chrono::Utc::now().timestamp(),
        }])
    }
}
