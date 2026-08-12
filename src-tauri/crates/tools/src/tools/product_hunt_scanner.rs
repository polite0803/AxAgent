//! ProductHunt 扫描器
//! 通过 ProductHunt API 采集产品发布和需求反馈

use async_trait::async_trait;
use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// ProductHunt 扫描器
pub struct ProductHuntScanner {
    http: reqwest::Client,
    access_token: Option<String>,
}

impl ProductHuntScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let access_token = std::env::var("PH_ACCESS_TOKEN").ok();
        Self { http, access_token }
    }

    /// 构建 GraphQL 查询
    fn build_graphql_query(query: &str) -> String {
        format!(
            r#"{{
                posts(first: 30, query: "{}", order: RANKING) {{
                    edges {{
                        node {{
                            name
                            tagline
                            description
                            url
                            votesCount
                            commentsCount
                            createdAt
                            topics {{
                                edges {{
                                    node {{
                                        name
                                    }}
                                }}
                            }}
                            comments {{
                                edges {{
                                    node {{
                                        body
                                        createdAt
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}"#,
            query
        )
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if let Some(ref token) = self.access_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 需求相关话题标签
    fn demand_topics() -> Vec<&'static str> {
        vec![
            "developer-tools", "api", "integration", "productivity",
            "automation", "ai", "machine-learning", "data",
            "finance", "trading", "investment", "stock-market",
            "cryptocurrency", "blockchain", "analytics",
        ]
    }

    /// 检查产品是否与需求相关
    fn is_demand_related(
        name: &str,
        tagline: &str,
        description: &str,
        topics: &[String],
    ) -> bool {
        let demand_keywords = [
            "api", "integration", "plugin", "extension", "sdk",
            "automation", "workflow", "pipeline", "orchestration",
            "developer", "programming", "coding", "debugging",
            "stock", "trading", "finance", "investment", "market",
            "data", "analytics", "reporting", "dashboard",
            "machine learning", "nlp",
        ];

        let full_text = format!("{} {} {}", name, tagline, description).to_lowercase();
        let has_demand_keyword = demand_keywords.iter().any(|kw| full_text.contains(kw));

        let demand_topics_set: std::collections::HashSet<&str> = Self::demand_topics().into_iter().collect();
        let has_demand_topic = topics
            .iter()
            .any(|t| demand_topics_set.contains(t.to_lowercase().as_str()));

        has_demand_keyword || has_demand_topic
    }

    /// 从评论中提取需求信号
    fn extract_demand_from_comments(
        comments: &[String],
    ) -> Vec<String> {
        let demand_patterns = [
            ("feature_request", vec!["feature request", "would love", "would be great", "it would be nice", "please add", "wish there was", "hope you'll add"]),
            ("integration", vec!["integrate", "integration with", "connect to", "works with", "compatible with"]),
            ("improvement", vec!["improve", "better", "faster", "more reliable", "could be", "needs improvement"]),
            ("use_case", vec!["use case", "i need", "i want", "looking for", "use it for", "would use"]),
            ("issue", vec!["issue", "bug", "problem", "doesn't work", "not working", "broken", "error"]),
        ];

        let mut demands = Vec::new();

        for comment in comments {
            let comment_lower = comment.to_lowercase();
            for (category, patterns) in &demand_patterns {
                if patterns.iter().any(|p| comment_lower.contains(p)) {
                    demands.push(format!("[{}] {}", category, comment.chars().take(100).collect::<String>()));
                    break;
                }
            }
        }

        demands
    }
}

impl Default for ProductHuntScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for ProductHuntScanner {
    fn platform(&self) -> &'static str {
        "producthunt"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            query = q,
            "[ProductHuntScanner] 开始搜索"
        );

        // 如果没有 access token，使用公开 API 或跳过
        if self.access_token.is_none() {
            tracing::warn!("[ProductHuntScanner] 未配置 PH_ACCESS_TOKEN，跳过搜索");
            return Ok(Vec::new());
        }

        let graphql_query = Self::build_graphql_query(q);
        let variables = serde_json::json!({});

        let url = "https://api.producthunt.com/v2/api/graphql";
        let headers = self.build_headers();

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(&serde_json::json!({
                "query": graphql_query,
                "variables": variables,
            }))
            .send()
            .await
            .map_err(|e| format!("ProductHunt API 请求失败: {}", e))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("ProductHunt API 速率限制".to_string());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "ProductHunt API 返回状态码 {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("ProductHunt API 响应解析失败: {}", e))?;

        let mut leads = Vec::new();

        if let Some(edges) = data["data"]["posts"]["edges"].as_array() {
            for edge in edges {
                if let Some(node) = edge["node"].as_object() {
                    let name = node["name"].as_str().unwrap_or("").to_string();
                    let tagline = node["tagline"].as_str().unwrap_or("").to_string();
                    let description = node["description"].as_str().unwrap_or("").to_string();
                    let url = node["url"].as_str().unwrap_or("").to_string();
                    let votes = node["votesCount"].as_i64().unwrap_or(0);
                    let comment_count = node["commentsCount"].as_i64().unwrap_or(0);

                    let topics: Vec<String> = node["topics"]["edges"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|e| e["node"]["name"].as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let comments: Vec<String> = node["comments"]["edges"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|e| e["node"]["body"].as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    if Self::is_demand_related(&name, &tagline, &description, &topics) {
                        let demand_comments = Self::extract_demand_from_comments(&comments);

                        let snapshot = serde_json::json!({
                            "name": name,
                            "tagline": tagline,
                            "description": description,
                            "votesCount": votes,
                            "commentsCount": comment_count,
                            "topics": topics,
                            "demand_comments": demand_comments,
                            "_extracted_source": "producthunt",
                        });

                        let description_text = if !demand_comments.is_empty() {
                            demand_comments.join("\n")
                        } else {
                            format!("{} - {} ({} votes, {} comments)", name, tagline, votes, comment_count)
                        };

                        leads.push(RawLead {
                            platform: "producthunt".to_string(),
                            title: name,
                            description: description_text,
                            url,
                            price_text: None,
                            contact: None,
                            snapshot,
                        });
                    }
                }
            }
        }

        tracing::info!(
            query = q,
            filtered = leads.len(),
            "[ProductHuntScanner] 搜索完成"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = ProductHuntScanner::new();
        assert_eq!(scanner.platform(), "producthunt");
    }

    #[test]
    fn test_is_demand_related() {
        // API 工具
        assert!(ProductHuntScanner::is_demand_related(
            "API Builder",
            "Build APIs in minutes",
            "A tool for building REST APIs",
            &["api".to_string(), "developer-tools".to_string()]
        ));

        // 金融相关
        assert!(ProductHuntScanner::is_demand_related(
            "StockPilot",
            "AI-powered stock analysis",
            "Analyze stocks with machine learning",
            &["finance".to_string(), "trading".to_string()]
        ));

        // 不相关
        assert!(!ProductHuntScanner::is_demand_related(
            "Cat Photos",
            "Daily cat photos",
            "A website for cat lovers",
            &["entertainment".to_string()]
        ));

        // 关键词匹配
        assert!(ProductHuntScanner::is_demand_related(
            "DevTools",
            "The ultimate developer toolkit",
            "Automates your workflow with AI",
            &[]
        ));
    }

    #[test]
    fn test_extract_demand_from_comments() {
        let comments = vec![
            "Would love a dark mode feature".to_string(),
            "Great product!".to_string(),
            "Please add integration with Slack".to_string(),
            "It would be nice if it supported OAuth".to_string(),
        ];

        let demands = ProductHuntScanner::extract_demand_from_comments(&comments);
        assert!(demands.len() >= 3);
        assert!(demands.iter().any(|d| d.contains("feature_request")));
        assert!(demands.iter().any(|d| d.contains("integration")));
    }

    #[test]
    fn test_graphql_query() {
        let query = ProductHuntScanner::build_graphql_query("API tools");
        assert!(query.contains("posts"));
        assert!(query.contains("votesCount"));
        assert!(query.contains("commentsCount"));
    }

    #[tokio::test]
    async fn test_search_without_token() {
        let scanner = ProductHuntScanner::new();
        let result = scanner.search("API tools").await;
        assert!(result.is_ok());
        // 无 token 时应返回空列表
        assert!(result.unwrap().is_empty());
    }
}
