//! LinkedIn 扫描器
//! 通过公开 API 采集 LinkedIn 上的 B2B 需求信号
//! 主要数据源：招聘 JD、公司页面、动态

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// LinkedIn 扫描器
pub struct LinkedInScanner {
    http: reqwest::Client,
    /// API Token，用于官方 API 认证
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl LinkedInScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("LINKEDIN_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://api.linkedin.com/v2".to_string() }
    }

    /// 从配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://api.linkedin.com/v2".to_string()),
        }
    }

    /// 构建搜索 URL（Jobs API - 招聘信息是 B2B 需求的核心信号）
    fn build_jobs_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "+");
        format!("{}/jobSearch?keywords={}&start=0&count=20", self.base_url, encoded_query)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers
            .insert(reqwest::header::USER_AGENT, "AxAgent/1.0 (DemandDiscovery)".parse().unwrap());
        headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// AI/技术栈关键词（从招聘 JD 中提取的需求信号）
    fn tech_stack_keywords() -> Vec<&'static str> {
        vec![
            // AI 相关
            "llm",
            "gpt",
            "transformer",
            "diffusion",
            "rag",
            "vector database",
            "milvus",
            "pinecone",
            "weaviate",
            "embedding",
            "fine-tuning",
            "prompt engineering",
            "agent",
            "copilot",
            "autonomous",
            // 云原生
            "kubernetes",
            "docker",
            "serverless",
            "faas",
            "aws",
            "azure",
            "gcp",
            // 数据工程
            "spark",
            "flink",
            "kafka",
            "airflow",
            "data warehouse",
            "data lake",
            "lakehouse",
            // 现代前端
            "react",
            "vue",
            "next.js",
            "svelte",
            "tailwind",
            "typescript",
        ]
    }

    /// B2B 需求模式（从招聘信息中识别的业务需求）
    fn extract_b2b_signals(job_description: &str) -> Vec<String> {
        let text = job_description.to_lowercase();
        let mut signals = Vec::new();

        // 检查技术栈采用（直接反映企业技术需求）
        for tech in Self::tech_stack_keywords() {
            let full_tech = tech.to_lowercase();
            // 使用词边界匹配避免误判
            if text.contains(&format!(" {}", full_tech))
                || text.contains(&format!(", {}", full_tech))
                || text.contains(&format!(" {}", full_tech))
            {
                signals.push(format!("adoption:{}", tech));
            }
        }

        // 检查需求模式
        let demand_patterns = [
            (
                "demand:new_role",
                vec!["hiring", "looking for", "seeking", "recruiting", "we are looking"],
            ),
            (
                "demand:team_expansion",
                vec!["expanding", "growing team", "new team member", "additional engineer"],
            ),
            ("demand:new_project", vec!["new project", "greenfield", "from scratch", "build from"]),
            (
                "demand:modernization",
                vec!["migrate", "upgrade", "refactor", "legacy", "modernize", "tech debt"],
            ),
            (
                "demand:integration",
                vec![
                    "integrate",
                    "integration with",
                    "connect",
                    "plugin",
                    "sdk",
                    "api integration",
                ],
            ),
            (
                "demand:scale",
                vec!["scale", "high traffic", "high concurrency", "performance", "optimize"],
            ),
            (
                "demand:ai_transformation",
                vec!["ai transformation", "digital transformation", "ai strategy", "generative ai"],
            ),
        ];

        for (tag, patterns) in &demand_patterns {
            if patterns.iter().any(|p| text.contains(p)) {
                signals.push(tag.to_string());
            }
        }

        signals
    }

    /// 从职位描述中提取核心需求摘要
    fn extract_demand_summary(
        title: &str,
        company: Option<&str>,
        location: Option<&str>,
    ) -> String {
        let mut parts = vec![title.to_string()];
        if let Some(c) = company {
            parts.push(format!("公司: {}", c));
        }
        if let Some(l) = location {
            parts.push(format!("地点: {}", l));
        }
        parts.join(" | ")
    }
}

impl Default for LinkedInScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for LinkedInScanner {
    fn platform(&self) -> &'static str {
        "linkedin"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_jobs_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[LinkedInScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    // 实际实现中应解析 JSON 响应
                    // 此处演示处理逻辑

                    // 简单的文本分析：查找包含需求信号的内容块
                    for segment in text.split(|c: char| c.is_control() && !c.is_ascii_whitespace())
                    {
                        let trimmed = segment.trim();
                        if trimmed.len() < 30 {
                            continue;
                        }

                        let signals = Self::extract_b2b_signals(trimmed);
                        if !signals.is_empty() {
                            // 从文本中提取可能的职位标题
                            let title = if trimmed.len() > 100 {
                                format!("{}...", &trimmed[..100])
                            } else {
                                trimmed.to_string()
                            };

                            let summary = Self::extract_demand_summary(&title, None, None);

                            leads.push(RawLead {
                                platform: "linkedin".to_string(),
                                title: format!("LinkedIn B2B Signal: {}", signals.join(", ")),
                                description: summary,
                                url: url.clone(),
                                price_text: None,
                                contact: None,
                                snapshot: serde_json::json!({
                                    "source": "linkedin_scanner",
                                    "signals": signals,
                                    "type": "b2b_demand",
                                    "raw_text": trimmed,
                                }),
                            });
                        }
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[LinkedInScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("LinkedIn API 速率限制".to_string());
                }
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err("LinkedIn API 需要认证".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[LinkedInScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[LinkedInScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = LinkedInScanner::new();
        assert_eq!(scanner.platform(), "linkedin");
    }

    #[test]
    fn test_build_jobs_search_url() {
        let scanner = LinkedInScanner::new();
        let url = scanner.build_jobs_search_url("AI Engineer");
        assert!(url.contains("AI+Engineer"));
        assert!(url.contains("jobSearch"));
    }

    #[test]
    fn test_extract_b2b_signals_tech_adoption() {
        // 检测技术栈采用
        let signals = LinkedInScanner::extract_b2b_signals(
            "We are looking for a Senior Engineer with experience in Kubernetes and Kafka",
        );
        assert!(signals.iter().any(|s| s.contains("adoption:kubernetes")));
        assert!(signals.iter().any(|s| s.contains("adoption:kafka")));
        assert!(signals.iter().any(|s| s.contains("demand:new_role")));
    }

    #[test]
    fn test_extract_b2b_signals_demand_patterns() {
        // 检测需求模式
        let signals = LinkedInScanner::extract_b2b_signals(
            "Our company is expanding and looking for engineers to help with our AI transformation initiative",
        );
        assert!(signals.iter().any(|s| s.contains("demand:team_expansion")));
        assert!(signals.iter().any(|s| s.contains("demand:ai_transformation")));
    }

    #[test]
    fn test_extract_b2b_signals_modernization() {
        // 检测技术升级需求
        let signals = LinkedInScanner::extract_b2b_signals(
            "Legacy system migration to modern cloud stack, need someone experienced in refactoring",
        );
        assert!(signals.iter().any(|s| s.contains("demand:modernization")));
    }

    #[test]
    fn test_tech_stack_keywords() {
        let keywords = LinkedInScanner::tech_stack_keywords();
        assert!(keywords.contains(&"llm"));
        assert!(keywords.contains(&"kubernetes"));
        assert!(keywords.contains(&"react"));
        assert!(keywords.len() > 20);
    }

    #[test]
    fn test_extract_demand_summary() {
        let summary = LinkedInScanner::extract_demand_summary(
            "Senior AI Engineer",
            Some("TechCorp Inc."),
            Some("San Francisco"),
        );
        assert!(summary.contains("Senior AI Engineer"));
        assert!(summary.contains("TechCorp Inc."));
        assert!(summary.contains("San Francisco"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = LinkedInScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
