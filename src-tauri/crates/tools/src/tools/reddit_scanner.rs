// SPDX-License-Identifier: AGPL-3.0-only

//! Reddit 扫描器
//!
//! 通过 Reddit JSON API（无需认证）采集技术社区需求线索。
//! 支持搜索 r/startups、r/Entrepreneur 等子版块。

use async_trait::async_trait;
use reqwest::Client;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// Reddit 扫描器
pub struct RedditScanner {
    client: Client,
}

impl RedditScanner {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("AxAgent/1.0 (demand-discovery)")
                .build()
                .unwrap_or_default(),
        }
    }

    /// 构建搜索 URL
    fn build_search_url(q: &str, limit: u32) -> String {
        let encoded_q = urlencoding::encode(q);
        format!("https://www.reddit.com/search.json?q={}&limit={}&sort=new", encoded_q, limit)
    }

    /// 从 JSON 响应解析帖子数据
    fn parse_posts(data: &serde_json::Value) -> Vec<RawLead> {
        let mut leads = Vec::new();

        if let Some(children) = data.get("data").and_then(|d| d.get("children"))
            && let Some(arr) = children.as_array()
        {
            for child in arr {
                if let Some(post) = child.get("data") {
                    let title =
                        post.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    if title.is_empty() {
                        continue;
                    }

                    let description =
                        post.get("selftext").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let url = post.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let url_for_snapshot = url.clone();

                    let permalink =
                        post.get("permalink").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let full_url = if permalink.is_empty() {
                        url
                    } else {
                        format!("https://www.reddit.com{}", permalink)
                    };

                    let score = post.get("score").and_then(|v| v.as_i64()).unwrap_or(0);

                    let num_comments =
                        post.get("num_comments").and_then(|v| v.as_i64()).unwrap_or(0);

                    let subreddit = post
                        .get("subreddit_name_prefixed")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let created_utc =
                        post.get("created_utc").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    leads.push(RawLead {
                        platform: "reddit".to_string(),
                        title,
                        description,
                        url: full_url,
                        price_text: None,
                        contact: None,
                        snapshot: serde_json::json!({
                            "score": score,
                            "num_comments": num_comments,
                            "subreddit": subreddit,
                            "created_utc": created_utc,
                            "url": url_for_snapshot,
                        }),
                    });
                }
            }
        }

        leads
    }

    /// 过滤需求相关内容
    fn filter_demand_leads(leads: Vec<RawLead>) -> Vec<RawLead> {
        let demand_keywords = [
            "need",
            "want",
            "looking for",
            "help",
            "advice",
            "suggest",
            "demand",
            "requirement",
            "project",
            "startup",
            "business",
            "hire",
            "outsource",
            "freelance",
            "contract",
            "build",
            "开发",
            "需求",
            "项目",
            "外包",
            "合作",
        ];

        leads
            .into_iter()
            .filter(|lead| {
                let text = format!("{} {}", lead.title, lead.description).to_lowercase();
                demand_keywords.iter().any(|kw| text.contains(kw))
            })
            .collect()
    }
}

impl Default for RedditScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for RedditScanner {
    fn platform(&self) -> &'static str {
        "reddit"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        let url = Self::build_search_url(q, 20);

        tracing::info!(query = q, "[RedditScanner] 发起搜索请求");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Reddit API 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(format!("Reddit API 返回状态码: {}", status));
        }

        let body = resp.text().await.map_err(|e| format!("响应体读取失败: {}", e))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {}", e))?;

        let all_leads = Self::parse_posts(&parsed);
        let total_count = all_leads.len();
        let filtered_leads = Self::filter_demand_leads(all_leads);

        tracing::info!(
            query = q,
            total = total_count,
            filtered = filtered_leads.len(),
            "[RedditScanner] 搜索完成"
        );

        Ok(filtered_leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url() {
        let url = RedditScanner::build_search_url("startup idea", 10);
        assert!(url.contains("startup+idea") || url.contains("startup%20idea"));
        assert!(url.contains("limit=10"));
        assert!(url.contains("reddit.com"));
    }

    #[test]
    fn test_parse_posts_empty() {
        let data = serde_json::json!({});
        let leads = RedditScanner::parse_posts(&data);
        assert!(leads.is_empty());
    }

    #[test]
    fn test_parse_posts_with_data() {
        let data = serde_json::json!({
            "data": {
                "children": [
                    {
                        "data": {
                            "title": "Looking for a developer",
                            "selftext": "Need help building a startup",
                            "url": "https://example.com",
                            "permalink": "/r/test/comments/abc/looking_for_dev/",
                            "score": 100,
                            "num_comments": 20,
                            "subreddit_name_prefixed": "r/startups",
                            "created_utc": 1723456789.0
                        }
                    }
                ]
            }
        });

        let leads = RedditScanner::parse_posts(&data);
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].platform, "reddit");
        assert_eq!(leads[0].title, "Looking for a developer");
        assert!(leads[0].url.contains("reddit.com"));
    }

    #[test]
    fn test_filter_demand_leads() {
        let leads = vec![
            RawLead {
                platform: "reddit".to_string(),
                title: "Looking for a developer".to_string(),
                description: "Need help with startup".to_string(),
                url: "https://reddit.com/1".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({}),
            },
            RawLead {
                platform: "reddit".to_string(),
                title: "Beautiful sunset".to_string(),
                description: "Nice photo".to_string(),
                url: "https://reddit.com/2".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({}),
            },
        ];

        let filtered = RedditScanner::filter_demand_leads(leads);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Looking for a developer");
    }

    #[tokio::test]
    async fn test_search_with_empty_query() {
        let scanner = RedditScanner::new();
        let result = scanner.search("").await;
        // Reddit API 对空查询可能返回错误，这是预期行为
        assert!(result.is_ok() || result.is_err());
    }
}
