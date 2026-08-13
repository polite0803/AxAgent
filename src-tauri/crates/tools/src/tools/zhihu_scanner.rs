//! 知乎扫描器
//! 通过公开 API 采集知乎上的技术需求和痛点讨论
//! 主要数据源：知乎问答、文章

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// 知乎扫描器
pub struct ZhihuScanner {
    http: reqwest::Client,
    /// API Token（可选）
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl ZhihuScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("ZHIHU_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://www.zhihu.com".to_string() }
    }

    /// 从配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://www.zhihu.com".to_string()),
        }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "%20");
        format!("{}/search?q={}&type=content", self.base_url, encoded_query)
    }

    /// 构建请求头（模拟浏览器访问）
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".parse().unwrap(),
        );
        headers.insert(reqwest::header::REFERER, "https://www.zhihu.com/".parse().unwrap());
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 中文技术痛点关键词
    fn pain_point_keywords() -> Vec<&'static str> {
        vec![
            // AI/机器学习痛点
            "大模型",
            "LLM",
            "RAG",
            "向量数据库",
            "embedding",
            "微调",
            "prompt",
            "agent",
            "智能体",
            "扩散模型",
            "diffusion",
            "transformer",
            // 开发痛点
            "解决",
            "问题",
            "报错",
            "异常",
            "失败",
            "优化",
            "性能",
            "卡顿",
            "慢",
            "配置",
            "部署",
            "环境",
            "依赖",
            // 需求表达
            "怎么",
            "如何",
            "有没有",
            "有没有人",
            "求",
            "推荐",
            "对比",
            "哪个好",
            "怎么选",
            "新手",
            "入门",
            "教程",
            "指南",
        ]
    }

    /// 需求模式识别
    fn extract_demand_signals(text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();
        let mut signals = Vec::new();

        // 检查痛点关键词
        let pain_keywords = Self::pain_point_keywords();
        let matched_keywords: Vec<&str> =
            pain_keywords.iter().filter(|kw| text_lower.contains(*kw)).cloned().collect();

        if !matched_keywords.is_empty() {
            signals.push(format!("pain_points:{}", matched_keywords.join(",")));
        }

        // 检查需求模式
        let demand_patterns = [
            ("demand:how_to", vec!["怎么", "如何", "怎样", "要怎么做"]),
            ("demand:comparison", vec!["对比", "比较", "哪个好", "区别", "vs"]),
            ("demand:recommendation", vec!["推荐", "有什么推荐", "求推荐", "哪家好"]),
            (
                "demand:troubleshooting",
                vec!["报错", "异常", "失败", "解决不了", "求助", "遇到问题"],
            ),
            ("demand:learning", vec!["学习", "入门", "教程", "指南", "有没有资料"]),
            ("demand:implementation", vec!["实现", "代码", "示例", "demo", "有没有人做过"]),
            ("demand:architecture", vec!["架构", "设计模式", "最佳实践", "怎么设计"]),
        ];

        for (tag, patterns) in &demand_patterns {
            if patterns.iter().any(|p| text_lower.contains(p)) {
                signals.push(tag.to_string());
            }
        }

        signals
    }

    /// 判断是否为高价值需求内容
    fn is_valuable_demand(content: &str) -> bool {
        // 至少包含一个需求模式
        let demand_patterns = [
            "怎么",
            "如何",
            "对比",
            "推荐",
            "报错",
            "学习",
            "实现",
            "架构",
            "优化",
            "配置",
            "问题",
            "解决",
            "求助",
            "有没有",
            "哪个",
            "更好",
            "哪个好",
            "优缺点",
            "区别",
            "vs",
            "vs",
        ];
        demand_patterns.iter().any(|p| content.contains(p))
    }

    /// 提取需求摘要
    fn extract_summary(title: &str, excerpt: Option<&str>) -> String {
        if let Some(desc) = excerpt {
            let combined = format!("{} - {}", title, desc);
            if combined.len() > 200 {
                format!("{}...", &combined[..200])
            } else {
                combined
            }
        } else {
            if title.len() > 150 {
                format!("{}...", &title[..150])
            } else {
                title.to_string()
            }
        }
    }
}

impl Default for ZhihuScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for ZhihuScanner {
    fn platform(&self) -> &'static str {
        "zhihu"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[ZhihuScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.text().await {
                    // 知乎页面通常需要 JavaScript 渲染
                    // 此处提供基础的文本分析逻辑

                    // 按行分割，查找包含需求信号的文本块
                    for line in body.lines() {
                        let trimmed = line.trim();
                        if trimmed.len() < 15 {
                            continue;
                        }

                        // 检查是否为高价值需求内容
                        if !Self::is_valuable_demand(trimmed) {
                            continue;
                        }

                        let signals = Self::extract_demand_signals(trimmed);
                        if signals.is_empty() {
                            continue;
                        }

                        // 提取标题（假设前部分是标题）
                        let title = if trimmed.len() > 80 {
                            format!("{}...", &trimmed[..80])
                        } else {
                            trimmed.to_string()
                        };

                        let summary = Self::extract_summary(&title, None);

                        leads.push(RawLead {
                            platform: "zhihu".to_string(),
                            title: format!("知乎需求信号: {}", signals.join(", ")),
                            description: summary,
                            url: url.clone(),
                            price_text: None,
                            contact: None,
                            contact_email: None,
                            contact_phone: None,
                            snapshot: serde_json::json!({
                                "source": "zhihu_scanner",
                                "signals": signals,
                                "type": "tech_demand",
                                "raw_text": trimmed,
                            }),
                        });
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[ZhihuScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("知乎 API 速率限制".to_string());
                }
                if status == reqwest::StatusCode::FORBIDDEN {
                    return Err("知乎 API 需要认证或被限制".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[ZhihuScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[ZhihuScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = ZhihuScanner::new();
        assert_eq!(scanner.platform(), "zhihu");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = ZhihuScanner::new();
        let url = scanner.build_search_url("大模型应用开发");
        assert!(url.contains("大模型"));
        assert!(url.contains("search"));
    }

    #[test]
    fn test_extract_demand_signals() {
        // 包含多个需求信号
        let signals =
            ZhihuScanner::extract_demand_signals("大模型 RAG 怎么实现？有没有推荐的向量数据库？");
        assert!(!signals.is_empty());
        assert!(signals.iter().any(|s| s.contains("demand:how_to")));
        assert!(signals.iter().any(|s| s.contains("demand:recommendation")));
        assert!(signals.iter().any(|s| s.contains("pain_points:")));
    }

    #[test]
    fn test_is_valuable_demand() {
        // 有价值的需求内容
        assert!(ZhihuScanner::is_valuable_demand("请问大模型怎么部署？"));
        assert!(ZhihuScanner::is_valuable_demand("RAG 和 fine-tuning 哪个更好？"));
        assert!(ZhihuScanner::is_valuable_demand("求推荐一个好用的向量数据库"));

        // 无价值内容
        assert!(!ZhihuScanner::is_valuable_demand("今天天气不错"));
        assert!(!ZhihuScanner::is_valuable_demand("大模型")); // 太短太泛
    }

    #[test]
    fn test_pain_point_keywords() {
        let keywords = ZhihuScanner::pain_point_keywords();
        assert!(keywords.contains(&"大模型"));
        assert!(keywords.contains(&"RAG"));
        assert!(keywords.contains(&"怎么"));
        assert!(keywords.len() > 30);
    }

    #[test]
    fn test_extract_summary() {
        let summary = ZhihuScanner::extract_summary("如何学习大模型开发", None);
        assert_eq!(summary, "如何学习大模型开发");

        let summary = ZhihuScanner::extract_summary("标题", Some("这是一段描述内容"));
        assert!(summary.contains("标题"));
        assert!(summary.contains("这是一段描述内容"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = ZhihuScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
