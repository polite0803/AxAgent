// SPDX-License-Identifier: AGPL-3.0-only

//! GitHub Issue 扫描器
//!
//! 通过 GitHub Search API 搜索开源项目中的 Issue，
//! 从 Bug 报告和 Feature Request 中提取需求线索。

use async_trait::async_trait;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// GitHub Issue 扫描器
pub struct GitHubIssueScanner {
    http: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl GitHubIssueScanner {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("AxInvest/1.0 (demand-discovery)")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            base_url: "https://api.github.com".to_string(),
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    /// 带 Token 的构造
    pub fn with_token(token: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("AxInvest/1.0 (demand-discovery)")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            base_url: "https://api.github.com".to_string(),
            token: Some(token),
        }
    }

    /// 检查 Issue 是否为需求相关
    fn is_demand_issue(&self, labels: &[String], title: &str) -> bool {
        let demand_labels = [
            "enhancement",
            "feature",
            "feature-request",
            "wishlist",
            "improvement",
            "suggestion",
            "help wanted",
            "good first issue",
            "documentation",
            "question",
        ];

        let title_demand_keywords = [
            "feature",
            "add",
            "support",
            "implement",
            "suggest",
            "improve",
            "enhance",
            "request",
            "wish",
            "looking for",
            "how to",
            "question",
            "help",
        ];

        // 检查标签
        let has_demand_label = labels
            .iter()
            .any(|label| demand_labels.iter().any(|dl| label.to_lowercase().contains(dl)));
        // 检查标题关键词
        let title_lower = title.to_lowercase();
        let has_demand_keyword = title_demand_keywords.iter().any(|kw| title_lower.contains(kw));

        has_demand_label || has_demand_keyword
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        // 使用 GitHub Search API 搜索 Issue
        // 搜索范围：公开仓库中的 Issue 和 PR
        let search_query = format!("{} is:issue is:open", query);
        format!(
            "{}/search/issues?q={}&per_page=20&sort=created&order=desc",
            self.base_url,
            urlencoding::encode(&search_query)
        )
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());
        if let Some(ref token) = self.token {
            headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
        }
        headers
    }
}

impl Default for GitHubIssueScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for GitHubIssueScanner {
    fn platform(&self) -> &'static str {
        "github_issue"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        let url = self.build_search_url(q);
        let headers = self.build_headers();

        tracing::info!(
            query = q,
            url = %url,
            has_token = self.token.is_some(),
            "[GitHubIssueScanner] 发起搜索请求"
        );

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("GitHub API 请求失败: {}", e))?;

        // 检查速率限制
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

                let body = item["body"].as_str().unwrap_or("").to_string();

                let html_url = item["html_url"].as_str().unwrap_or("").to_string();

                // 提取标签
                let labels: Vec<String> = item["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                // 提取仓库信息
                let repo_url = item["repository_url"].as_str().unwrap_or("").to_string();

                // 提取统计信息
                let comments = item["comments"].as_i64().unwrap_or(0);
                let created_at = item["created_at"].as_str().unwrap_or("").to_string();
                let state = item["state"].as_str().unwrap_or("open").to_string();

                // 过滤：只保留需求相关的 Issue
                if self.is_demand_issue(&labels, &title) {
                    let mut snapshot = item.clone();
                    snapshot["_extracted_labels"] = serde_json::json!(labels);
                    snapshot["_extracted_repo"] = serde_json::json!(repo_url);
                    snapshot["_extracted_comments"] = serde_json::json!(comments);
                    snapshot["_extracted_created_at"] = serde_json::json!(created_at);
                    snapshot["_extracted_state"] = serde_json::json!(state);

                    leads.push(RawLead {
                        platform: "github_issue".to_string(),
                        title,
                        description: body,
                        url: html_url,
                        price_text: None,
                        contact: None,
                        snapshot,
                    });
                }
            }
        }

        tracing::info!(query = q, found = leads.len(), "[GitHubIssueScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = GitHubIssueScanner::new();
        assert_eq!(scanner.platform(), "github_issue");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = GitHubIssueScanner::new();
        let url = scanner.build_search_url("AI tools");
        assert!(url.contains("api.github.com/search/issues"));
        // URL 编码可能使用 + 或 %20
        assert!(url.contains("AI+tools") || url.contains("AI%20tools"));
        assert!(url.contains("is%3Aissue"));
    }

    #[test]
    fn test_is_demand_issue() {
        let scanner = GitHubIssueScanner::new();

        // 有需求标签
        assert!(scanner.is_demand_issue(&["enhancement".to_string()], "Add dark mode support"));
        assert!(scanner.is_demand_issue(&["feature-request".to_string()], "New feature"));
        assert!(scanner.is_demand_issue(&["help wanted".to_string()], "Need help with setup"));

        // 标题包含需求关键词
        assert!(scanner.is_demand_issue(&[], "Feature: Add export to PDF"));
        assert!(scanner.is_demand_issue(&[], "How to implement authentication?"));

        // 不相关
        assert!(!scanner.is_demand_issue(
            &["bug".to_string(), "duplicate".to_string()],
            "Fix null pointer exception"
        ));
    }

    #[test]
    fn test_build_headers() {
        let scanner = GitHubIssueScanner::new();
        let headers = scanner.build_headers();
        assert!(headers.contains_key("Accept"));
        // 无 token 时不应有 Authorization
        assert!(!headers.contains_key("Authorization"));

        let scanner_with_token = GitHubIssueScanner::with_token("test_token".to_string());
        let headers = scanner_with_token.build_headers();
        assert!(headers.contains_key("Authorization"));
    }

    #[tokio::test]
    async fn test_search_without_token() {
        let scanner = GitHubIssueScanner::new();
        let result = scanner.search("rust async").await;
        // 应该成功（公开 API 无需 token，但可能有速率限制）
        if result.is_err() {
            let err = result.unwrap_err();
            // 速率限制或网络错误是可接受的
            assert!(err.contains("速率限制") || err.contains("失败") || err.contains("错误"));
        }
    }
}
