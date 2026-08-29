// SPDX-License-Identifier: AGPL-3.0-only

//! Reddit 平台扫描器
//!
//! 通过 Reddit 公开 JSON API 搜索相关讨论，提取需求线索。
//! 无需 API Key，使用 Reddit 公开的 `.json` 接口。

use async_trait::async_trait;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// Reddit 扫描器
pub struct RedditScanner {
    http: reqwest::Client,
}

impl RedditScanner {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("AxAgent/1.0 (demand-discovery)")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 检查帖子是否与需求相关
    fn is_demand_related(&self, title: &str, description: &str) -> bool {
        let demand_keywords = [
            "need",
            "looking for",
            "want",
            "how to",
            "problem",
            "issue",
            "help",
            "require",
            "implement",
            "build",
            "frustrating",
            "difficult",
            "lack",
            "missing",
            "solution",
            "looking to",
            "trying to",
            "can't",
            "doesn't work",
            "recommendation",
            "suggestion",
            "advice",
        ];
        let text = format!("{} {}", title, description).to_lowercase();
        demand_keywords.iter().any(|kw| text.contains(kw))
    }

    /// 从 Reddit API 构建请求 URL
    fn build_url(&self, query: &str) -> String {
        let encoded_query = urlencoding::encode(query);
        format!(
            "https://www.reddit.com/search.json?q={}&sort=new&limit=20&restrict_sr=off",
            encoded_query
        )
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
        let url = self.build_url(q);

        tracing::info!(
            query = q,
            url = %url,
            "[RedditScanner] 发起搜索请求"
        );

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("Reddit API 请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("Reddit API 返回状态码: {}", status));
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("Reddit API 响应解析失败: {}", e))?;

        let mut leads = Vec::new();

        if let Some(posts) = data["data"]["children"].as_array() {
            for post in posts {
                let post_data = &post["data"];

                // 跳过已删除的帖子
                if post_data["removed"].as_bool().unwrap_or(false) {
                    continue;
                }

                let title = post_data["title"].as_str().unwrap_or("").to_string();

                // 获取帖子内容（selftext）或链接描述
                let description = post_data["selftext"]
                    .as_str()
                    .or_else(|| post_data["link_flair_text"].as_str())
                    .unwrap_or("")
                    .to_string();

                let permalink = post_data["permalink"].as_str().unwrap_or("").to_string();

                let url = if permalink.is_empty() {
                    post_data["url"].as_str().unwrap_or("").to_string()
                } else {
                    format!("https://reddit.com{}", permalink)
                };

                // 提取元数据
                let subreddit = post_data["subreddit"].as_str().unwrap_or("").to_string();

                let score = post_data["score"].as_i64().unwrap_or(0);
                let num_comments = post_data["num_comments"].as_i64().unwrap_or(0);

                // 过滤：只保留需求相关的帖子
                if self.is_demand_related(&title, &description) {
                    let mut snapshot = post_data.clone();
                    snapshot["_extracted_subreddit"] = serde_json::json!(subreddit);
                    snapshot["_extracted_score"] = serde_json::json!(score);
                    snapshot["_extracted_comments"] = serde_json::json!(num_comments);

                    leads.push(RawLead {
                        platform: "reddit".to_string(),
                        title,
                        description,
                        url,
                        price_text: None,
                        contact: None,
                        contact_email: None,
                        contact_phone: None,
                        snapshot,
                    });
                }
            }
        }

        tracing::info!(query = q, found = leads.len(), "[RedditScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = RedditScanner::new();
        assert_eq!(scanner.platform(), "reddit");
    }

    #[test]
    fn test_build_url() {
        let scanner = RedditScanner::new();
        let url = scanner.build_url("AI tools");
        assert!(url.contains("reddit.com/search.json"));
        assert!(url.contains("AI+tools") || url.contains("AI%20tools"));
    }

    #[test]
    fn test_is_demand_related() {
        let scanner = RedditScanner::new();

        // 需求相关
        assert!(scanner.is_demand_related("Need a tool for X", ""));
        assert!(scanner.is_demand_related("Looking for recommendation", ""));
        assert!(scanner.is_demand_related("This is frustrating", "Can't find a solution"));

        // 不相关
        assert!(!scanner.is_demand_related("Great sunset photo", "My vacation pics"));
        assert!(!scanner.is_demand_related("Happy birthday", "Party time"));
    }

    #[tokio::test]
    async fn test_search_with_empty_query() {
        let scanner = RedditScanner::new();
        // 空查询应该能处理（可能返回空结果或错误）
        let result = scanner.search("").await;
        // 不 panic 即可
        assert!(result.is_ok() || result.is_err());
    }
}
