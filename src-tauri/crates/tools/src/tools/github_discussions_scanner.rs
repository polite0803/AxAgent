//! GitHub Discussions 扫描器
//! 通过 GitHub Search API 采集 Discussions 中的需求线索

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;
use std::collections::HashSet;

/// GitHub Discussions 扫描器
pub struct GitHubDiscussionsScanner {
    http: reqwest::Client,
    github_token: Option<String>,
}

impl GitHubDiscussionsScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let github_token = std::env::var("GITHUB_TOKEN").ok();
        Self { http, github_token }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "+");
        format!(
            "https://api.github.com/search/issues?q={}+type:discussion&sort=reactions&order=desc&per_page=30",
            encoded_query
        )
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::ACCEPT, "application/vnd.github.v3+json".parse().unwrap());
        if let Some(ref token) = self.github_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 需求相关标签
    fn demand_labels() -> Vec<&'static str> {
        vec![
            "enhancement",
            "feature",
            "feature-request",
            "new-feature",
            "improvement",
            "suggestion",
            "request",
        ]
    }

    /// 检查是否为需求相关讨论
    fn is_demand_discussion(title: &str, labels: &[String]) -> bool {
        let demand_keywords = [
            "feature request",
            "feature",
            "enhancement",
            "new feature",
            "would be nice",
            "integrate",
            "integration",
            "connect",
        ];
        let title_lower = title.to_lowercase();
        let has_demand_keyword = demand_keywords.iter().any(|kw| title_lower.contains(kw));

        let demand_labels_set: HashSet<&str> = Self::demand_labels().into_iter().collect();
        let has_demand_label =
            labels.iter().any(|label| demand_labels_set.contains(label.to_lowercase().as_str()));

        has_demand_keyword || has_demand_label
    }

    /// 从讨论中提取需求描述
    fn extract_demand_description(
        title: &str,
        body: Option<&str>,
        comments: u64,
    ) -> Option<String> {
        let body_text = body.unwrap_or("").to_string();
        let full_text = format!("{}\n{}", title, body_text);
        let text_lower = full_text.to_lowercase();

        let demand_patterns = [
            (
                "feature_request",
                vec![
                    "feature request",
                    "new feature",
                    "would it be possible",
                    "could you add",
                    "is there a way",
                ],
            ),
            ("how_to", vec!["how do i", "how to", "how can i", "how does", "how would"]),
            (
                "enhancement",
                vec![
                    "enhancement",
                    "improvement",
                    "better",
                    "optimize",
                    "faster",
                    "more efficient",
                ],
            ),
            (
                "integration",
                vec!["integrate", "integration", "connect", "plugin", "extension", "adapter"],
            ),
            ("ui_ux", vec!["ui", "ux", "interface", "design", "layout", "theme", "dark mode"]),
            ("api", vec!["api", "rest", "graphql", "endpoint", "webhook", "sdk", "library"]),
            (
                "performance",
                vec!["performance", "slow", "fast", "speed", "cache", "optimize", "memory"],
            ),
            (
                "use_case",
                vec!["use case", "use cases", "scenario", "workflow", "pipeline", "automation"],
            ),
        ];

        let mut detected_categories = Vec::new();
        for (category, patterns) in &demand_patterns {
            if patterns.iter().any(|p| text_lower.contains(p)) {
                detected_categories.push(category.to_string());
            }
        }

        if !detected_categories.is_empty() || comments > 0 {
            Some(format!("[{}] {} ({} comments)", detected_categories.join(", "), title, comments))
        } else {
            None
        }
    }
}

impl Default for GitHubDiscussionsScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for GitHubDiscussionsScanner {
    fn platform(&self) -> &'static str {
        "github_discussion"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[GitHubDiscussionsScanner] 发起搜索请求");

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("GitHub Discussions API 请求失败: {}", e))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("GitHub API 速率限制，请稍后重试或配置 GITHUB_TOKEN".to_string());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "GitHub API 返回状态码 {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("GitHub API 响应解析失败: {}", e))?;

        let mut leads = Vec::new();

        if let Some(items) = data["items"].as_array() {
            for item in items {
                let title = item["title"].as_str().unwrap_or("").to_string();
                let body = item["body"].as_str().map(|s| s.to_string());
                let html_url = item["html_url"].as_str().unwrap_or("").to_string();
                let comments = item["comments"].as_i64().unwrap_or(0) as u64;
                let reactions = item["reactions"]["total_count"].as_i64().unwrap_or(0);

                let labels: Vec<String> = item["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                if Self::is_demand_discussion(&title, &labels)
                    && let Some(desc) =
                        Self::extract_demand_description(&title, body.as_deref(), comments)
                {
                    let mut snapshot = item.clone();
                    snapshot["_extracted_demand"] = serde_json::json!(desc);
                    snapshot["_extracted_reactions"] = serde_json::json!(reactions);
                    snapshot["_extracted_source"] = serde_json::json!("github_discussion");

                    leads.push(RawLead {
                        platform: "github_discussion".to_string(),
                        title,
                        description: desc,
                        url: html_url,
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
            total = data["total_count"].as_i64().unwrap_or(0),
            filtered = leads.len(),
            "[GitHubDiscussionsScanner] 搜索完成"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = GitHubDiscussionsScanner::new();
        assert_eq!(scanner.platform(), "github_discussion");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = GitHubDiscussionsScanner::new();
        let url = scanner.build_search_url("rust async");
        assert!(url.contains("api.github.com/search/issues"));
        assert!(url.contains("type:discussion"));
    }

    #[test]
    fn test_is_demand_discussion() {
        // 包含需求关键词
        assert!(GitHubDiscussionsScanner::is_demand_discussion(
            "Feature Request: Add dark mode support",
            &[]
        ));

        // 包含需求标签
        assert!(GitHubDiscussionsScanner::is_demand_discussion(
            "How to use the API",
            &["enhancement".to_string()]
        ));

        // 普通讨论
        assert!(!GitHubDiscussionsScanner::is_demand_discussion(
            "How was your weekend?",
            &["question".to_string()]
        ));

        // 包含 feature request
        assert!(GitHubDiscussionsScanner::is_demand_discussion(
            "Is there a way to integrate with Slack?",
            &[]
        ));
    }

    #[test]
    fn test_extract_demand_description() {
        // Feature request
        let desc = GitHubDiscussionsScanner::extract_demand_description(
            "Feature Request: Add dark mode support",
            Some("Users want a dark theme option"),
            15,
        );
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("feature_request"));

        // How-to
        let desc = GitHubDiscussionsScanner::extract_demand_description(
            "How to connect to database?",
            Some("Need help with PostgreSQL integration"),
            8,
        );
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("how_to"));

        // 简单讨论无回复
        let desc = GitHubDiscussionsScanner::extract_demand_description("Test discussion", None, 0);
        assert!(desc.is_none());
    }

    #[tokio::test]
    async fn test_search_without_token() {
        let scanner = GitHubDiscussionsScanner::new();
        let result = scanner.search("async programming").await;
        // 允许成功或失败（取决于网络环境）
        let _ = result;
    }
}
