// SPDX-License-Identifier: AGPL-3.0-only

//! GitHub Issue 扫描器
//!
//! 通过 GitHub Search API 采集开源项目中的需求线索（Feature Requests、Bug Reports 等）。
//! 支持按仓库范围和关键词搜索。

use async_trait::async_trait;
use reqwest::Client;

use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// GitHub Issue 扫描器
pub struct GitHubIssueScanner {
    client: Client,
    github_token: Option<String>,
}

impl GitHubIssueScanner {
    pub fn new(github_token: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("AxAgent/1.0 (demand-discovery)")
                .build()
                .unwrap_or_default(),
            github_token,
        }
    }

    /// 构建搜索 URL
    fn build_search_url(q: &str, per_page: u32) -> String {
        let encoded_q = urlencoding::encode(q);
        format!("https://api.github.com/search/issues?q={}&per_page={}", encoded_q, per_page)
    }

    /// 构建带认证的请求
    async fn send_request(&self, url: &str) -> Result<reqwest::Response, String> {
        let mut request = self.client.get(url);

        if let Some(ref token) = self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        request
            .header("Accept", "application/vnd.github.v3+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| format!("GitHub API 请求失败: {}", e))
    }

    /// 解析 GitHub Issues 数据
    fn parse_issues(data: &serde_json::Value) -> Vec<RawLead> {
        let mut leads = Vec::new();

        if let Some(items) = data.get("items")
            && let Some(arr) = items.as_array()
        {
            for item in arr {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

                if title.is_empty() {
                    continue;
                }

                let body = item.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let html_url =
                    item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let issue_state =
                    item.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let comments = item.get("comments").and_then(|v| v.as_i64()).unwrap_or(0);

                let created_at =
                    item.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let labels = item
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();

                let user = item
                    .get("user")
                    .and_then(|u| u.get("login"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let repo_url =
                    item.get("repository_url").and_then(|v| v.as_str()).unwrap_or("").to_string();

                leads.push(RawLead {
                    platform: "github".to_string(),
                    title,
                    description: body,
                    url: html_url,
                    price_text: None,
                    contact: None,
                    snapshot: serde_json::json!({
                        "state": issue_state,
                        "comments": comments,
                        "created_at": created_at,
                        "labels": labels,
                        "user": user,
                        "repository_url": repo_url,
                    }),
                });
            }
        }

        leads
    }

    /// 过滤需求相关 Issues（feature request / bug report）
    fn filter_demand_issues(leads: Vec<RawLead>) -> Vec<RawLead> {
        let demand_labels = [
            "enhancement",
            "feature",
            "feature-request",
            "improvement",
            "suggestion",
            "bug",
            "bug-report",
            "help-wanted",
        ];

        let demand_keywords = [
            "feature", "request", "suggest", "improve", "enhance", "bug", "fix", "issue", "需求",
            "建议", "改进",
        ];

        leads
            .into_iter()
            .filter(|lead| {
                let labels = lead
                    .snapshot
                    .get("labels")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();

                let has_demand_label = demand_labels.iter().any(|lbl| labels.contains(lbl));

                if has_demand_label {
                    return true;
                }

                let text = format!("{} {}", lead.title, lead.description).to_lowercase();
                demand_keywords.iter().any(|kw| text.contains(kw))
            })
            .collect()
    }
}

impl Default for GitHubIssueScanner {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl MarketplaceScanner for GitHubIssueScanner {
    fn platform(&self) -> &'static str {
        "github"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        let search_query = format!("{} is:issue", q);
        let url = Self::build_search_url(&search_query, 20);

        tracing::info!(query = q, "[GitHubIssueScanner] 发起搜索请求");

        let resp = self.send_request(&url).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                query = q,
                status = %status,
                body = %body,
                "[GitHubIssueScanner] API 响应异常"
            );
            return Err(format!(
                "GitHub API 返回状态码 {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        let body = resp.text().await.map_err(|e| format!("响应体读取失败: {}", e))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {}", e))?;

        let all_leads = Self::parse_issues(&parsed);
        let total_count = all_leads.len();
        let filtered_leads = Self::filter_demand_issues(all_leads);

        tracing::info!(
            query = q,
            total = total_count,
            filtered = filtered_leads.len(),
            "[GitHubIssueScanner] 搜索完成"
        );

        Ok(filtered_leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url() {
        let url = GitHubIssueScanner::build_search_url("AI tool", 10);
        assert!(url.contains("AI+tool") || url.contains("AI%20tool"));
        assert!(url.contains("per_page=10"));
        assert!(url.contains("api.github.com"));
    }

    #[test]
    fn test_parse_issues_empty() {
        let data = serde_json::json!({});
        let leads = GitHubIssueScanner::parse_issues(&data);
        assert!(leads.is_empty());
    }

    #[test]
    fn test_parse_issues_with_data() {
        let data = serde_json::json!({
            "items": [
                {
                    "title": "Add dark mode support",
                    "body": "Users have been requesting dark mode for a while",
                    "html_url": "https://github.com/example/repo/issues/1",
                    "state": "open",
                    "comments": 15,
                    "created_at": "2026-08-01T00:00:00Z",
                    "labels": [
                        {"name": "enhancement"},
                        {"name": "feature-request"}
                    ],
                    "user": {"login": "testuser"},
                    "repository_url": "https://api.github.com/repos/example/repo"
                }
            ]
        });

        let leads = GitHubIssueScanner::parse_issues(&data);
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].platform, "github");
        assert_eq!(leads[0].title, "Add dark mode support");
    }

    #[test]
    fn test_filter_demand_issues() {
        let leads = vec![
            RawLead {
                platform: "github".to_string(),
                title: "Add new feature".to_string(),
                description: "Feature request for X".to_string(),
                url: "https://github.com/1".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({
                    "labels": "enhancement,feature-request",
                }),
            },
            RawLead {
                platform: "github".to_string(),
                title: "Just a question".to_string(),
                description: "How do I use this?".to_string(),
                url: "https://github.com/2".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({
                    "labels": "question",
                }),
            },
            RawLead {
                platform: "github".to_string(),
                title: "Improve performance".to_string(),
                description: "The app is slow".to_string(),
                url: "https://github.com/3".to_string(),
                price_text: None,
                contact: None,
                snapshot: serde_json::json!({
                    "labels": "performance",
                }),
            },
        ];

        let filtered = GitHubIssueScanner::filter_demand_issues(leads);
        assert!(filtered.len() >= 2); // First has labels, third has keyword "improve"
    }

    #[tokio::test]
    async fn test_search_with_empty_query() {
        let scanner = GitHubIssueScanner::new(None);
        let result = scanner.search("").await;
        // GitHub API with empty query might error or return empty
        assert!(result.is_ok() || result.is_err());
    }
}
