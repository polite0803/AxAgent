//! Stack Overflow 扫描器
//! 通过 Stack Overflow API 采集技术痛点和需求线索

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// Stack Overflow 扫描器
pub struct StackOverflowScanner {
    http: reqwest::Client,
    api_key: Option<String>,
}

impl StackOverflowScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_key = std::env::var("SO_API_KEY").ok();
        Self { http, api_key }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str, tags: &[String]) -> String {
        let query_encoded = query.replace(' ', "%20");
        let tags_encoded = tags.join("%20");
        let mut url = format!(
            "https://api.stackexchange.com/2.3/search?order=desc&sort=votes&site=stackoverflow&q={}&tagged={}",
            query_encoded, tags_encoded
        );
        if let Some(ref key) = self.api_key {
            url.push_str(&format!("&key={}", key));
        }
        url
    }

    /// 需求相关标签
    fn demand_tags() -> Vec<&'static str> {
        vec![
            "api",
            "rest",
            "graphql",
            "integration",
            "automation",
            "workflow",
            "pipeline",
            "data",
            "analytics",
            "reporting",
            "trading",
            "finance",
            "stock",
            "machine-learning",
            "ai",
            "nlp",
            "performance",
            "optimization",
            "scalability",
            "security",
            "authentication",
            "authorization",
        ]
    }

    /// 检查问题是否为需求相关
    fn is_demand_question(title: &str, tags: &[String]) -> bool {
        let demand_keywords = [
            "how to",
            "how do i",
            "how can",
            "how does",
            "possible to",
            "able to",
            "support",
            "implement",
            "integration",
            "integrate",
            "connect",
            "communicate",
            "api",
            "rest",
            "graphql",
            "webhook",
            "trading",
            "stock",
            "market",
            "finance",
            "problem",
            "issue",
            "error",
            "bug",
            "fix",
            "performance",
            "slow",
            "fast",
            "optimize",
            "recommended",
            "best practice",
            "approach",
        ];

        let title_lower = title.to_lowercase();
        let has_demand_keyword = demand_keywords.iter().any(|kw| title_lower.contains(kw));

        let demand_tags_set: std::collections::HashSet<&str> =
            Self::demand_tags().into_iter().collect();
        let has_demand_tag =
            tags.iter().any(|t| demand_tags_set.contains(t.to_lowercase().as_str()));

        has_demand_keyword || has_demand_tag
    }

    /// 从问题中提取需求描述
    fn extract_demand_description(
        title: &str,
        body: &str,
        answer_count: u64,
        view_count: u64,
    ) -> Option<String> {
        let body_lower = body.to_lowercase();

        let demand_patterns = [
            (
                "integration",
                vec![
                    "integrate",
                    "integration",
                    "connect",
                    "communicate",
                    "api",
                    "rest",
                    "graphql",
                ],
            ),
            (
                "performance",
                vec!["performance", "slow", "fast", "speed", "optimize", "cache", "memory"],
            ),
            (
                "data_processing",
                vec!["data", "process", "transform", "parse", "extract", "analyze"],
            ),
            (
                "trading_finance",
                vec!["trading", "stock", "market", "finance", "investment", "portfolio"],
            ),
            (
                "ai_ml",
                vec!["machine learning", "deep learning", "ai", "model", "nlp", "prediction"],
            ),
            (
                "deployment",
                vec!["deploy", "deployment", "docker", "kubernetes", "ci", "cd", "pipeline"],
            ),
            (
                "security",
                vec!["security", "authentication", "authorization", "token", "jwt", "oauth"],
            ),
            ("ui_ux", vec!["ui", "ux", "interface", "design", "component", "responsive"]),
        ];

        let mut detected_categories = Vec::new();
        for (category, patterns) in &demand_patterns {
            if patterns.iter().any(|p| body_lower.contains(p)) {
                detected_categories.push(category.to_string());
            }
        }

        if !detected_categories.is_empty() || answer_count > 5 || view_count > 100 {
            Some(format!(
                "[{}] {} ({} answers, {} views)",
                detected_categories.join(", "),
                title.chars().take(100).collect::<String>(),
                answer_count,
                view_count
            ))
        } else {
            None
        }
    }
}

impl Default for StackOverflowScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for StackOverflowScanner {
    fn platform(&self) -> &'static str {
        "stackoverflow"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let default_tags = vec![
            "api".to_string(),
            "integration".to_string(),
            "automation".to_string(),
            "data".to_string(),
        ];
        let url = self.build_search_url(q, &default_tags);

        tracing::info!(query = q, "[StackOverflowScanner] 发起搜索请求");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("StackOverflow API 请求失败: {}", e))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("StackOverflow API 速率限制".to_string());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "StackOverflow API 返回状态码 {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("StackOverflow API 响应解析失败: {}", e))?;

        let mut leads = Vec::new();

        if let Some(items) = data["items"].as_array() {
            for item in items {
                let title = item["title"].as_str().unwrap_or("").to_string();
                let body = item["body"].as_str().unwrap_or("").to_string();
                let link = item["link"].as_str().unwrap_or("").to_string();
                let answer_count = item["answer_count"].as_i64().unwrap_or(0) as u64;
                let view_count = item["view_count"].as_i64().unwrap_or(0) as u64;
                let score = item["score"].as_i64().unwrap_or(0);

                let tags: Vec<String> = item["tags"]
                    .as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect()
                    })
                    .unwrap_or_default();

                if Self::is_demand_question(&title, &tags)
                    && let Some(desc) =
                        Self::extract_demand_description(&title, &body, answer_count, view_count)
                {
                    let mut snapshot = item.clone();
                    snapshot["_extracted_demand"] = serde_json::json!(desc);
                    snapshot["_extracted_score"] = serde_json::json!(score);
                    snapshot["_extracted_source"] = serde_json::json!("stackoverflow");

                    leads.push(RawLead {
                        platform: "stackoverflow".to_string(),
                        title,
                        description: desc,
                        url: link,
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
            total = data["total"].as_i64().unwrap_or(0),
            filtered = leads.len(),
            "[StackOverflowScanner] 搜索完成"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = StackOverflowScanner::new();
        assert_eq!(scanner.platform(), "stackoverflow");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = StackOverflowScanner::new();
        let url = scanner.build_search_url("async programming", &["api".to_string()]);
        assert!(url.contains("api.stackexchange.com"));
        assert!(url.contains("stackoverflow"));
    }

    #[test]
    fn test_is_demand_question() {
        // 包含需求关键词
        assert!(StackOverflowScanner::is_demand_question(
            "How to integrate API with database?",
            &[]
        ));

        // 包含需求标签
        assert!(StackOverflowScanner::is_demand_question("Simple question", &["api".to_string()]));

        // 不相关
        assert!(!StackOverflowScanner::is_demand_question("What is 2 + 2?", &["math".to_string()]));

        // 性能相关
        assert!(StackOverflowScanner::is_demand_question(
            "How to optimize slow query?",
            &["performance".to_string()]
        ));
    }

    #[test]
    fn test_extract_demand_description() {
        // 性能问题
        let desc = StackOverflowScanner::extract_demand_description(
            "How to speed up my API?",
            "My REST API is very slow, takes 5 seconds to respond. Need to optimize performance.",
            10,
            500,
        );
        assert!(desc.is_some());
        let desc_text = desc.unwrap();
        assert!(desc_text.contains("performance"));

        // 集成问题
        let desc = StackOverflowScanner::extract_demand_description(
            "How to connect to external API?",
            "Need to integrate with a third-party API. Looking for best approach.",
            5,
            200,
        );
        assert!(desc.is_some());
        let desc_text = desc.unwrap();
        assert!(desc_text.contains("integration"));

        // 简单问题
        let desc = StackOverflowScanner::extract_demand_description(
            "Simple question",
            "Just a basic question",
            0,
            10,
        );
        assert!(desc.is_none());
    }

    #[tokio::test]
    async fn test_search_without_key() {
        let scanner = StackOverflowScanner::new();
        let result = scanner.search("API integration").await;
        // 允许成功或失败（取决于网络环境）
        let _ = result;
    }
}
