//! Upwork 扫描器
//! 通过公开 API 采集 Upwork 平台的国际外包需求线索
//! 与猪八戒形成国内外外包市场的互补

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// Upwork 扫描器
pub struct UpworkScanner {
    http: reqwest::Client,
    /// API Token
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl UpworkScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("UPWORK_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://www.upwork.com".to_string() }
    }

    /// 从配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://www.upwork.com".to_string()),
        }
    }

    /// 构建搜索 URL（Jobs API - 公开 API）
    fn build_jobs_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "%20");
        format!(
            "{}/nx/api/search/jobs?q={}&sort=recency&t=0&amount=1000-99999&duration=4-12&profiles=1",
            self.base_url, encoded_query
        )
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap(),
        );
        headers
            .insert(reqwest::header::ACCEPT, "application/json, text/plain, */*".parse().unwrap());
        headers.insert(
            reqwest::header::REFERER,
            "https://www.upwork.com/nx/search/jobs/".parse().unwrap(),
        );
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 技术/技能分类（Upwork 上的高需求技能）
    fn skill_categories() -> Vec<&'static str> {
        vec![
            // AI/机器学习
            "machine learning",
            "deep learning",
            "nlp",
            "computer vision",
            "llm",
            "gpt",
            "ai agent",
            "rag",
            "langchain",
            // 全栈开发
            "full stack",
            "react",
            "vue",
            "next.js",
            "node.js",
            "python",
            "django",
            "flask",
            "fastapi",
            // 云/DevOps
            "aws",
            "azure",
            "gcp",
            "docker",
            "kubernetes",
            "devops",
            "ci/cd",
            "terraform",
            // 移动端
            "ios",
            "android",
            "react native",
            "flutter",
            "swift",
            "kotlin",
            // 设计
            "ui/ux design",
            "web design",
            "mobile app design",
            "logo design",
            "brand identity",
            // 数据
            "data analysis",
            "data science",
            "tableau",
            "power bi",
            "sql",
            "python for data",
        ]
    }

    /// 外包需求模式识别
    fn extract_hiring_signals(job_title: &str, description: &str) -> Vec<String> {
        let text = format!("{} {}", job_title, description).to_lowercase();
        let mut signals = Vec::new();

        // 检查技能需求（直接反映市场需求）
        for skill in Self::skill_categories() {
            if text.contains(skill) {
                signals.push(format!("skill_demand:{}", skill));
            }
        }

        // 检查项目类型
        let project_patterns = [
            ("project:web_app", vec!["web application", "website", "web app", "saas", "dashboard"]),
            ("project:mobile_app", vec!["mobile app", "ios app", "android app", "ios application"]),
            (
                "project:api_development",
                vec!["api", "rest api", "graphql", "backend", "microservices"],
            ),
            (
                "project:ai_implementation",
                vec!["ai", "machine learning", "llm", "ai agent", "chatbot"],
            ),
            ("project:design", vec!["design", "ui design", "ux design", "logo", "branding"]),
            ("project:data", vec!["data analysis", "data science", "dashboard", "reporting", "bi"]),
            (
                "project:integration",
                vec!["integration", "api integration", "third party", "plugin", "extension"],
            ),
            ("project:migration", vec!["migration", "upgrade", "refactor", "rewrite", "convert"]),
        ];

        for (tag, patterns) in &project_patterns {
            if patterns.iter().any(|p| text.contains(p)) {
                signals.push(tag.to_string());
            }
        }

        // 检查合同类型（时薪 vs 固定价格）
        if text.contains("hourly") || text.contains("per hour") || text.contains("/hr") {
            signals.push("contract:hourly".to_string());
        }
        if text.contains("fixed price")
            || text.contains("fixed-price")
            || text.contains("project-based")
        {
            signals.push("contract:fixed".to_string());
        }

        signals
    }

    /// 从薪资范围提取价格信号
    fn extract_price_range(text: &str) -> Option<(f64, f64)> {
        let text_lower = text.to_lowercase();

        // 匹配 "$500-$1000" 或 "$20/hr" 等格式
        let price_patterns = [
            // 固定价格范围
            ("$500-$1,000", "fixed_range"),
            ("$1,000-$5,000", "fixed_range"),
            ("$5,000-$10,000", "fixed_range"),
            // 时薪
            ("$20/hr", "hourly_rate"),
            ("$50/hr", "hourly_rate"),
            ("$100/hr", "hourly_rate"),
        ];

        for (pattern, _) in &price_patterns {
            if text_lower.contains(&pattern.to_lowercase()) {
                // 返回估算值
                match *pattern {
                    "$500-$1,000" => return Some((500.0, 1000.0)),
                    "$1,000-$5,000" => return Some((1000.0, 5000.0)),
                    "$5,000-$10,000" => return Some((5000.0, 10000.0)),
                    "$20/hr" => return Some((20.0, 40.0)),
                    "$50/hr" => return Some((50.0, 100.0)),
                    "$100/hr" => return Some((100.0, 200.0)),
                    _ => {},
                }
            }
        }

        None
    }

    /// 判断是否为高价值外包需求
    fn is_valuable_hiring(job_title: &str) -> bool {
        let text_lower = job_title.to_lowercase();

        // 首先排除明显的低价值职位
        let low_value_patterns = [
            "data entry",
            "clerk",
            "virtual assistant",
            "customer service",
            "call center",
            "telemarketing",
            "sales representative",
            "receptionist",
            "administrative",
            "office assistant",
            "cleaning",
            "maintenance",
            "labor",
        ];
        if low_value_patterns.iter().any(|p| text_lower.contains(p)) {
            return false;
        }

        let high_value_skills = [
            "ai",
            "llm",
            "machine learning",
            "python",
            "react",
            "vue",
            "full stack",
            "mobile app",
            "aws",
            "kubernetes",
            "devops",
            "design",
            "ui",
            "ux",
            "data science",
            "analytics engineer",
        ];
        high_value_skills.iter().any(|s| text_lower.contains(s))
    }

    /// 提取职位摘要
    fn extract_summary(title: &str, client: Option<&str>, price_range: Option<&str>) -> String {
        let mut parts = vec![title.to_string()];
        if let Some(c) = client {
            parts.push(format!("客户: {}", c));
        }
        if let Some(p) = price_range {
            parts.push(format!("预算: {}", p));
        }
        parts.join(" | ")
    }
}

impl Default for UpworkScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for UpworkScanner {
    fn platform(&self) -> &'static str {
        "upwork"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_jobs_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[UpworkScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    // 实际实现中应解析 JSON 响应并提取职位列表
                    // 此处提供文本分析逻辑

                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.len() < 20 {
                            continue;
                        }

                        // 检查是否为高价值外包需求
                        if !Self::is_valuable_hiring(trimmed) {
                            continue;
                        }

                        let signals = Self::extract_hiring_signals(trimmed, "");
                        if signals.is_empty() {
                            continue;
                        }

                        // 提取价格信息
                        let price_range = Self::extract_price_range(trimmed);
                        let price_text =
                            price_range.map(|(min, max)| format!("${:.0}-${:.0}", min, max));

                        let title = if trimmed.len() > 100 {
                            format!("{}...", &trimmed[..100])
                        } else {
                            trimmed.to_string()
                        };

                        let summary = Self::extract_summary(&title, None, price_text.as_deref());

                        leads.push(RawLead {
                            platform: "upwork".to_string(),
                            title: format!("Upwork 外包需求: {}", signals.join(", ")),
                            description: summary,
                            url: url.clone(),
                            price_text,
                            contact: None,
                            snapshot: serde_json::json!({
                                "source": "upwork_scanner",
                                "signals": signals,
                                "type": "outsourcing_demand",
                                "raw_text": trimmed,
                            }),
                        });
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[UpworkScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("Upwork API 速率限制".to_string());
                }
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err("Upwork API 需要认证".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[UpworkScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[UpworkScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = UpworkScanner::new();
        assert_eq!(scanner.platform(), "upwork");
    }

    #[test]
    fn test_build_jobs_search_url() {
        let scanner = UpworkScanner::new();
        let url = scanner.build_jobs_search_url("AI Developer");
        assert!(url.contains("AI%20Developer"));
        assert!(url.contains("jobs"));
    }

    #[test]
    fn test_extract_hiring_signals() {
        // 包含多个需求信号
        let signals = UpworkScanner::extract_hiring_signals(
            "Looking for Python Developer for AI Project",
            "Need someone experienced in LLMs and RAG",
        );
        assert!(!signals.is_empty());
        assert!(signals.iter().any(|s| s.contains("skill_demand:python")));
        assert!(signals.iter().any(|s| s.contains("skill_demand:llm")));
        assert!(signals.iter().any(|s| s.contains("project:ai_implementation")));
    }

    #[test]
    fn test_is_valuable_hiring() {
        // 高价值需求
        assert!(UpworkScanner::is_valuable_hiring("AI Developer for LLM Project"));
        assert!(UpworkScanner::is_valuable_hiring("Full Stack React Developer"));
        assert!(UpworkScanner::is_valuable_hiring("AWS DevOps Engineer"));

        // 低价值需求
        assert!(!UpworkScanner::is_valuable_hiring("Data Entry Clerk"));
        assert!(!UpworkScanner::is_valuable_hiring("Virtual Assistant"));
    }

    #[test]
    fn test_skill_categories() {
        let skills = UpworkScanner::skill_categories();
        assert!(skills.contains(&"llm"));
        assert!(skills.contains(&"react"));
        assert!(skills.contains(&"aws"));
        assert!(skills.len() > 30);
    }

    #[test]
    fn test_extract_summary() {
        let summary = UpworkScanner::extract_summary(
            "AI Developer Needed",
            Some("TechCorp Inc."),
            Some("$5,000-$10,000"),
        );
        assert!(summary.contains("AI Developer Needed"));
        assert!(summary.contains("TechCorp Inc."));
        assert!(summary.contains("$5,000-$10,000"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = UpworkScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
