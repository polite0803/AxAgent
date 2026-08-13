//! Dribbble 扫描器
//! 通过公开 API 采集 Dribbble 上的设计需求和服务信号
//! 设计服务需求是判断创意产业需求的重要指标

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// Dribbble 扫描器
pub struct DribbbleScanner {
    http: reqwest::Client,
    /// API Token
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl DribbbleScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("DRIBBBLE_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://api.dribbble.com/v1".to_string() }
    }

    /// 从配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://api.dribbble.com/v1".to_string()),
        }
    }

    /// 构建搜索 URL（Shots API）
    fn build_shots_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "+");
        format!("{}/shots?list=recent&tags={}&per_page=20", self.base_url, encoded_query)
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

    /// 设计领域关键词
    fn design_tags() -> Vec<&'static str> {
        vec![
            // UI/UX 设计
            "ui",
            "ux",
            "user interface",
            "user experience",
            "web design",
            "app design",
            "mobile app",
            // 品牌设计
            "logo",
            "branding",
            "brand identity",
            "visual identity",
            "logo design",
            "brand design",
            // 视觉设计
            "graphic design",
            "visual design",
            "poster",
            "brochure",
            "flyer",
            "business card",
            // 数字产品
            "dashboard",
            "admin panel",
            "landing page",
            "ecommerce",
            "shopify",
            "wordpress",
            // 动效设计
            "animation",
            "motion design",
            "3d",
            "illustration",
            "icon set",
            "iconography",
            // AI 设计
            "ai design",
            "generative design",
            "ai art",
            "midjourney",
            "stable diffusion",
        ]
    }

    /// 服务需求信号（从 Dribbble 动态中识别的需求）
    fn extract_design_signals(title: &str, description: Option<&str>) -> Vec<String> {
        let text = format!("{} {}", title, description.unwrap_or("")).to_lowercase();
        let mut signals = Vec::new();

        // 检查设计标签
        for tag in Self::design_tags() {
            if text.contains(tag) {
                signals.push(format!("design_tag:{}", tag));
            }
        }

        // 检查服务需求模式
        let service_patterns = [
            (
                "demand:design_service",
                vec![
                    "design service",
                    "design agency",
                    "freelance",
                    "hire designer",
                    "looking for designer",
                ],
            ),
            (
                "demand:ui_ux_design",
                vec!["ui design", "ux design", "ui/ux", "product design", "interface design"],
            ),
            (
                "demand:branding",
                vec![
                    "logo design",
                    "brand identity",
                    "branding",
                    "brand design",
                    "visual identity",
                ],
            ),
            (
                "demand:web_design",
                vec!["web design", "website design", "landing page", "webflow", "figma to code"],
            ),
            (
                "demand:mobile_design",
                vec!["app design", "mobile app", "ios design", "android design", "mobile ui"],
            ),
            (
                "demand:illustration",
                vec!["illustration", "icon design", "icon set", "vector", "custom illustration"],
            ),
            (
                "demand:3d_design",
                vec!["3d", "motion design", "animation", "3d render", "3d illustration"],
            ),
            (
                "demand:ai_design",
                vec!["ai design", "generative", "midjourney", "stable diffusion", "ai art"],
            ),
        ];

        for (tag, patterns) in &service_patterns {
            if patterns.iter().any(|p| text.contains(p)) {
                signals.push(tag.to_string());
            }
        }

        signals
    }

    /// 判断是否为高价值设计需求
    fn is_valuable_design(title: &str) -> bool {
        let high_value_patterns = [
            "设计",
            "design",
            "ui",
            "ux",
            "logo",
            "branding",
            "网站",
            "app",
            "移动端",
            "图标",
            "插画",
            "3d",
            "动画",
            "品牌",
            "视觉",
        ];
        let text_lower = title.to_lowercase();
        high_value_patterns.iter().any(|p| text_lower.contains(p))
    }

    /// 提取设计需求摘要
    fn extract_summary(title: &str, designer: Option<&str>, tags: Option<&str>) -> String {
        let mut parts = vec![title.to_string()];
        if let Some(d) = designer {
            parts.push(format!("设计师: {}", d));
        }
        if let Some(t) = tags {
            parts.push(format!("标签: {}", t));
        }
        parts.join(" | ")
    }
}

impl Default for DribbbleScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for DribbbleScanner {
    fn platform(&self) -> &'static str {
        "dribbble"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_shots_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[DribbbleScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    // 实际实现中应解析 JSON 响应
                    // 此处提供文本分析逻辑

                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.len() < 15 {
                            continue;
                        }

                        // 检查是否为高价值设计内容
                        if !Self::is_valuable_design(trimmed) {
                            continue;
                        }

                        let signals = Self::extract_design_signals(trimmed, None);
                        if signals.is_empty() {
                            continue;
                        }

                        let title = if trimmed.len() > 80 {
                            format!("{}...", &trimmed[..80])
                        } else {
                            trimmed.to_string()
                        };

                        let summary = Self::extract_summary(&title, None, None);

                        leads.push(RawLead {
                            platform: "dribbble".to_string(),
                            title: format!("Dribbble 设计信号: {}", signals.join(", ")),
                            description: summary,
                            url: url.clone(),
                            price_text: None,
                            contact: None,
                            contact_email: None,
                            contact_phone: None,
                            snapshot: serde_json::json!({
                                "source": "dribbble_scanner",
                                "signals": signals,
                                "type": "design_demand",
                                "raw_text": trimmed,
                            }),
                        });
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[DribbbleScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("Dribbble API 速率限制".to_string());
                }
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err("Dribbble API 需要认证".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[DribbbleScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[DribbbleScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = DribbbleScanner::new();
        assert_eq!(scanner.platform(), "dribbble");
    }

    #[test]
    fn test_build_shots_search_url() {
        let scanner = DribbbleScanner::new();
        let url = scanner.build_shots_search_url("ui design");
        assert!(url.contains("ui+design"));
        assert!(url.contains("shots"));
    }

    #[test]
    fn test_extract_design_signals() {
        // 包含多个设计信号
        let signals = DribbbleScanner::extract_design_signals(
            "Modern SaaS Dashboard UI Design",
            Some("Clean admin panel design"),
        );
        assert!(!signals.is_empty());
        assert!(signals.iter().any(|s| s.contains("design_tag:ui")));
        assert!(signals.iter().any(|s| s.contains("demand:ui_ux_design")));
    }

    #[test]
    fn test_is_valuable_design() {
        // 有价值的设计内容
        assert!(DribbbleScanner::is_valuable_design("UI Design for Mobile App"));
        assert!(DribbbleScanner::is_valuable_design("Brand Identity Logo"));
        assert!(DribbbleScanner::is_valuable_design("3D Illustration"));

        // 低价值内容
        assert!(!DribbbleScanner::is_valuable_design("Hello World"));
        assert!(!DribbbleScanner::is_valuable_design("测试内容"));
    }

    #[test]
    fn test_design_tags() {
        let tags = DribbbleScanner::design_tags();
        assert!(tags.contains(&"ui"));
        assert!(tags.contains(&"logo"));
        assert!(tags.contains(&"branding"));
        assert!(tags.len() > 30);
    }

    #[test]
    fn test_extract_summary() {
        let summary = DribbbleScanner::extract_summary(
            "Modern Dashboard UI",
            Some("DesignStudio"),
            Some("ui, dashboard, admin"),
        );
        assert!(summary.contains("Modern Dashboard UI"));
        assert!(summary.contains("DesignStudio"));
        assert!(summary.contains("ui, dashboard, admin"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = DribbbleScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
