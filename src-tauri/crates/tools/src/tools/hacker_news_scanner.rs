// SPDX-License-Identifier: AGPL-3.0-only

//! HackerNews 扫描器
//!
//! 通过 HN Search API（Algolia）采集技术趋势和需求线索。
//! 支持按热度、时间排序。

use async_trait::async_trait;
use reqwest::Client;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// HackerNews 扫描器
pub struct HackerNewsScanner {
    client: Client,
    _base_url: String,
}

impl HackerNewsScanner {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("AxAgent/1.0 (demand-discovery)")
                .build()
                .unwrap_or_default(),
            _base_url: "https://hn.algolia.com/api/v1/search".to_string(),
        }
    }

    /// 构建搜索 URL
    fn build_search_url(q: &str, tags: &str, hits_per_page: u32) -> String {
        let encoded_q = urlencoding::encode(q);
        let encoded_tags = urlencoding::encode(tags);
        format!(
            "{}?query={}&tags={}&hitsPerPage={}",
            Self::get_base_url(),
            encoded_q,
            encoded_tags,
            hits_per_page
        )
    }

    fn get_base_url() -> &'static str {
        "https://hn.algolia.com/api/v1/search"
    }

    /// 解析 HN 帖子数据
    fn parse_hits(data: &serde_json::Value) -> Vec<RawLead> {
        let mut leads = Vec::new();

        if let Some(hits) = data.get("hits")
            && let Some(arr) = hits.as_array()
        {
            for hit in arr {
                let title = hit.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

                if title.is_empty() {
                    continue;
                }

                let url = hit
                    .get("url")
                    .and_then(|v| v.as_str())
                    .or_else(|| hit.get("story_url").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();

                let story_text = hit
                    .get("story_text")
                    .and_then(|v| v.as_str())
                    .or_else(|| hit.get("comment_text").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();

                let description = if !story_text.is_empty() {
                    story_text
                } else {
                    String::new()
                };

                let points = hit.get("points").and_then(|v| v.as_i64()).unwrap_or(0);

                let num_comments = hit.get("num_comments").and_then(|v| v.as_i64()).unwrap_or(0);

                let object_id =
                    hit.get("objectID").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let hn_url = if object_id.is_empty() {
                    String::new()
                } else {
                    format!("https://news.ycombinator.com/item?id={}", object_id)
                };

                let full_url = if url.is_empty() {
                    hn_url.clone()
                } else {
                    url.clone()
                };

                let created_at =
                    hit.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let story_tags = hit
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(","))
                    .unwrap_or_default();

                leads.push(RawLead {
                    platform: "hackernews".to_string(),
                    title,
                    description,
                    url: full_url,
                    price_text: None,
                    contact: None,
                    snapshot: serde_json::json!({
                        "points": points,
                        "num_comments": num_comments,
                        "object_id": object_id,
                        "created_at": created_at,
                        "hn_url": hn_url,
                        "tags": story_tags,
                    }),
                });
            }
        }

        leads
    }

    /// 过滤高热度帖子（Points > 50 或 Comments > 10）
    fn filter_high_impact(leads: Vec<RawLead>) -> Vec<RawLead> {
        leads
            .into_iter()
            .filter(|lead| {
                let points = lead.snapshot.get("points").and_then(|v| v.as_i64()).unwrap_or(0);
                let comments =
                    lead.snapshot.get("num_comments").and_then(|v| v.as_i64()).unwrap_or(0);
                points > 50 || comments > 10
            })
            .collect()
    }
}

impl Default for HackerNewsScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for HackerNewsScanner {
    fn platform(&self) -> &'static str {
        "hackernews"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        let url = Self::build_search_url(q, "story", 20);

        tracing::info!(query = q, "[HackerNewsScanner] 发起搜索请求");

        let resp =
            self.client.get(&url).send().await.map_err(|e| format!("HN API 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(format!("HN API 返回状态码: {}", status));
        }

        let body = resp.text().await.map_err(|e| format!("响应体读取失败: {}", e))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {}", e))?;

        let all_leads = Self::parse_hits(&parsed);
        let total_count = all_leads.len();
        let filtered_leads = Self::filter_high_impact(all_leads);

        tracing::info!(
            query = q,
            total = total_count,
            filtered = filtered_leads.len(),
            "[HackerNewsScanner] 搜索完成"
        );

        Ok(filtered_leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url() {
        let url = HackerNewsScanner::build_search_url("AI startup", "story", 15);
        assert!(url.contains("AI+startup") || url.contains("AI%20startup"));
        assert!(url.contains("tags=story"));
        assert!(url.contains("hitsPerPage=15"));
    }

    #[test]
    fn test_parse_hits_empty() {
        let data = serde_json::json!({});
        let leads = HackerNewsScanner::parse_hits(&data);
        assert!(leads.is_empty());
    }

    #[test]
    fn test_parse_hits_with_data() {
        let data = serde_json::json!({
            "hits": [
                {
                    "title": "Show HN: My Startup",
                    "url": "https://mystartup.com",
                    "story_text": "Building a startup in AI space",
                    "points": 150,
                    "num_comments": 45,
                    "objectID": "12345678",
                    "created_at": "2026-08-13T10:00:00Z",
                    "tags": ["story", "show_hn"]
                }
            ]
        });

        let leads = HackerNewsScanner::parse_hits(&data);
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].platform, "hackernews");
        assert_eq!(leads[0].title, "Show HN: My Startup");
    }

    #[test]
    fn test_filter_high_impact() {
        let leads = vec![
            RawLead {
                platform: "hackernews".to_string(),
                title: "High Impact Post".to_string(),
                description: "Very popular".to_string(),
                url: "https://hn/1".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({
                    "points": 200,
                    "num_comments": 50,
                }),
            },
            RawLead {
                platform: "hackernews".to_string(),
                title: "Low Impact Post".to_string(),
                description: "Not popular".to_string(),
                url: "https://hn/2".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({
                    "points": 5,
                    "num_comments": 2,
                }),
            },
        ];

        let filtered = HackerNewsScanner::filter_high_impact(leads);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "High Impact Post");
    }
}
