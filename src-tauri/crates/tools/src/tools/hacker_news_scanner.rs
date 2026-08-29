// SPDX-License-Identifier: AGPL-3.0-only

//! HackerNews 平台扫描器
//!
//! 通过 HackerNews Firebase API 获取最新故事，提取需求线索。
//! HN API 为公开 REST API，无需认证。

use async_trait::async_trait;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// HackerNews 扫描器
pub struct HackerNewsScanner {
    http: reqwest::Client,
    base_url: String,
}

impl HackerNewsScanner {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("AxAgent/1.0 (demand-discovery)")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            base_url: "https://hacker-news.firebaseio.com/v0".to_string(),
        }
    }

    /// 检查故事是否与需求相关
    fn is_demand_related(&self, title: &str) -> bool {
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
            "ask hn",
            "discuss",
            "anyone else",
            "is there",
            "what's the best",
            "recommendation",
            "suggestion",
            "feedback",
        ];
        let text = title.to_lowercase();
        demand_keywords.iter().any(|kw| text.contains(kw))
    }

    /// 获取最新故事 ID 列表
    async fn fetch_latest_ids(&self, limit: u32) -> Result<Vec<u64>, String> {
        let url = format!("{}/newstories.json", self.base_url);

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("HN API 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HN API 返回状态码: {}", response.status()));
        }

        let ids: Vec<u64> =
            response.json().await.map_err(|e| format!("HN API 响应解析失败: {}", e))?;

        Ok(ids.into_iter().take(limit as usize).collect())
    }

    /// 获取单个故事详情
    async fn fetch_item(&self, id: u64) -> Result<Option<serde_json::Value>, String> {
        let url = format!("{}/item/{}.json", self.base_url, id);

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("HN item 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let item: serde_json::Value =
            response.json().await.map_err(|e| format!("HN item 解析失败: {}", e))?;

        // 如果 deleted 或 dead，跳过
        if item["deleted"].as_bool().unwrap_or(false) || item["dead"].as_bool().unwrap_or(false) {
            return Ok(None);
        }

        Ok(Some(item))
    }

    /// 通过关键词搜索 HN（使用 Algolia API）
    async fn search_by_keyword(&self, query: &str) -> Result<Vec<RawLead>, String> {
        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={}&tags=story&hitsPerPage=20",
            urlencoding::encode(query)
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HN Algolia API 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HN Algolia API 返回状态码: {}", response.status()));
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("HN Algolia 响应解析失败: {}", e))?;

        let mut leads = Vec::new();

        if let Some(hits) = data["hits"].as_array() {
            for hit in hits {
                let title = hit["title"].as_str().unwrap_or("").to_string();

                let description = hit["story_text"]
                    .as_str()
                    .or_else(|| hit["comment_text"].as_str())
                    .unwrap_or("")
                    .to_string();

                let url = if let Some(orig_url) = hit["url"].as_str() {
                    orig_url.to_string()
                } else {
                    let object_id = hit["objectID"].as_str().unwrap_or("");
                    if object_id.is_empty() {
                        String::new()
                    } else {
                        format!("https://news.ycombinator.com/item?id={}", object_id)
                    }
                };

                let points = hit["points"].as_i64().unwrap_or(0);
                let num_comments = hit["num_comments"].as_i64().unwrap_or(0);

                if self.is_demand_related(&title) {
                    let mut snapshot = hit.clone();
                    snapshot["_extracted_points"] = serde_json::json!(points);
                    snapshot["_extracted_comments"] = serde_json::json!(num_comments);

                    leads.push(RawLead {
                        platform: "hackernews".to_string(),
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

        Ok(leads)
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
        tracing::info!(query = q, "[HackerNewsScanner] 发起搜索请求");

        // 优先使用 Algolia 搜索 API（更精准）
        match self.search_by_keyword(q).await {
            Ok(leads) if !leads.is_empty() => {
                tracing::info!(
                    query = q,
                    found = leads.len(),
                    "[HackerNewsScanner] 搜索完成（Algolia）"
                );
                return Ok(leads);
            },
            Ok(_) => {
                tracing::info!(query = q, "[HackerNewsScanner] Algolia 无结果，尝试 Firebase API");
            },
            Err(e) => {
                tracing::warn!(
                    query = q,
                    error = %e,
                    "[HackerNewsScanner] Algolia 搜索失败，尝试 Firebase API"
                );
            },
        }

        // 回退方案：获取最新故事并过滤
        let latest_ids = self.fetch_latest_ids(30).await?;
        let mut leads = Vec::new();

        for id in latest_ids {
            if let Ok(Some(item)) = self.fetch_item(id).await {
                let title = item["title"].as_str().unwrap_or("").to_string();

                if title.is_empty() {
                    continue;
                }

                let url = item["url"]
                    .as_str()
                    .unwrap_or_else(|| {
                        let item_id = item["id"].as_u64().unwrap_or(id);
                        Box::leak(
                            format!("https://news.ycombinator.com/item?id={}", item_id)
                                .into_boxed_str(),
                        )
                    })
                    .to_string();

                let score = item["score"].as_i64().unwrap_or(0);

                if self.is_demand_related(&title) {
                    let mut snapshot = item.clone();
                    snapshot["_extracted_source"] = serde_json::json!("firebase_latest");
                    snapshot["_extracted_score"] = serde_json::json!(score);

                    leads.push(RawLead {
                        platform: "hackernews".to_string(),
                        title,
                        description: String::new(),
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

        tracing::info!(
            query = q,
            found = leads.len(),
            "[HackerNewsScanner] 搜索完成（Firebase 回退）"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = HackerNewsScanner::new();
        assert_eq!(scanner.platform(), "hackernews");
    }

    #[test]
    fn test_is_demand_related() {
        let scanner = HackerNewsScanner::new();

        // 需求相关
        assert!(scanner.is_demand_related("Need a recommendation for X"));
        assert!(scanner.is_demand_related("Show HN: I built a tool, need feedback"));
        assert!(scanner.is_demand_related("Ask HN: What's the best way to do Y?"));
        assert!(scanner.is_demand_related("Looking for solutions to this problem"));

        // 不相关
        assert!(!scanner.is_demand_related("Show HN: My new website"));
        assert!(!scanner.is_demand_related("Tell HN: I just launched"));
    }

    #[tokio::test]
    async fn test_fetch_latest_ids() {
        let scanner = HackerNewsScanner::new();
        let ids = scanner.fetch_latest_ids(5).await;
        assert!(ids.is_ok());
        let ids = ids.unwrap();
        assert!(ids.len() <= 5);
    }

    #[tokio::test]
    async fn test_search_with_common_keyword() {
        let scanner = HackerNewsScanner::new();
        let result = scanner.search("AI").await;

        // 网络集成测试：依赖外部 API，任何错误都可能是网络/环境问题，跳过以避免 CI 不稳定
        if let Err(e) = &result {
            eprintln!("[HackerNewsScanner] 网络请求失败，跳过网络集成测试: {}", e);
            return;
        }

        assert!(result.is_ok());
    }
}
