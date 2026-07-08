use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct CninfoVendor {
    pub http: reqwest::Client,
}

#[async_trait]
impl StockVendor for CninfoVendor {
    async fn get_quote(&self, _: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "cninfo".into(),
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

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    async fn get_announcements(&self, stock_code: &str) -> Result<Vec<Announcement>, DataError> {
        let org_id = self.resolve_org_id(stock_code).await;
        let stock_param = if let Some(ref oid) = org_id {
            format!("{stock_code},{oid}")
        } else {
            stock_code.to_string()
        };

        let plate = match stock_code.chars().next() {
            Some('6') => "sh",
            Some('8') | Some('4') => "bj",
            _ => "sz",
        };

        let url = "https://www.cninfo.com.cn/new/hisAnnouncement/query";
        let body = format!(
            "pageNum=1&pageSize=20&column={}&tabName=fulltext&plate={}&stock={}&searchkey=&secid=&category=&seDate=2020-01-01~{}&sortName=&sortType=&isHLtitle=true",
            plate,
            plate,
            urlencoding::encode(&stock_param),
            chrono::Utc::now().format("%Y-%m-%d")
        );

        let resp = self
            .http
            .post(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://www.cninfo.com.cn/")
            .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
            .header("X-Requested-With", "XMLHttpRequest")
            .body(body)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        let announcements = match json["announcements"].as_array() {
            Some(arr) => arr,
            None => return Ok(vec![]),
        };

        Ok(announcements
            .iter()
            .filter_map(|item| {
                let title = item["announcementTitle"].as_str()?.to_string();
                let sec_name = item["secName"].as_str().map(|s| s.to_string());
                let ann_date = item["announcementTime"]
                    .as_i64()
                    .map(|ts| {
                        let secs = ts / 1000;
                        chrono::DateTime::from_timestamp(secs, 0)
                            .map(|dt| dt.format("%Y-%m-%d").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let adj_url = item["adjunctUrl"].as_str().unwrap_or("");
                let pdf_url = if adj_url.is_empty() {
                    None
                } else {
                    Some(format!("https://static.cninfo.com.cn/{}", adj_url))
                };
                let ann_type = item["announcementTypeName"].as_str().map(|s| s.to_string());

                Some(Announcement {
                    title,
                    stock_code: stock_code.to_string(),
                    stock_name: sec_name,
                    announce_date: ann_date,
                    ann_type,
                    pdf_url,
                })
            })
            .collect())
    }

    // ── Vendor trait 大重构 P2:cninfo 能力申报 ──
    // cninfo 只有 get_announcements 有真实实现,其他都是 stub 返回 Error/None。
    // get_announcements 返回带 date 字段的全量,lib.rs 已正确 truncate_by_asof → Fallthrough。
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        let _ = method;
        AsOfCapability::Fallthrough
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> CninfoVendor {
        CninfoVendor {
            http: reqwest::Client::new(),
        }
    }

    #[test]
    fn cninfo_all_fallthrough() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_announcements"), AsOfCapability::Fallthrough);
        assert_eq!(v.asof_capability("get_news"), AsOfCapability::Fallthrough);
        assert_eq!(v.asof_capability("nonexistent"), AsOfCapability::Fallthrough);
    }
}

impl CninfoVendor {
    async fn resolve_org_id(&self, stock_code: &str) -> Option<String> {
        let url = format!("https://www.cninfo.com.cn/new/data/szse_stock.json");
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .header("Referer", "https://www.cninfo.com.cn/")
            .send()
            .await
            .ok()?;

        let json: Value = resp.json().await.ok()?;
        let stocks = json["stockList"].as_array()?;

        for stock in stocks {
            let code = stock["code"].as_str().unwrap_or("");
            if code == stock_code {
                return stock["orgId"].as_str().map(|s| s.to_string());
            }
        }
        None
    }
}
