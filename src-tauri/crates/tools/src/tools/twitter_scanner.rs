//! Twitter 扫描器
//! 通过公开 API 或轻量级代理采集 Twitter/X 上的 AI 相关讨论和趋势

use async_trait::async_trait;
use super::marketplace_scanner::{MarketplaceScanner, RawLead};

/// Twitter 扫描器
pub struct TwitterScanner {
    http: reqwest::Client,
    /// 可选的 Bearer Token，用于官方 API
    api_token: Option<String>,
    /// 基础 URL，默认为 Twitter 官方搜索 API
    base_url: String,
}

impl TwitterScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("TWITTER_BEARER_TOKEN").ok();
        // 为了演示，这里使用一个公开的 Nitter 实例作为备用数据源
        // 实际生产中应优先使用官方 API
        let base_url = "https://nitter.net".to_string(); 
        Self { http, api_token, base_url }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str, _max_results: u32) -> String {
        let encoded_query = query.replace(' ', "+");
        // 这里演示如何使用 Nitter 搜索。实际应用中应调用 Twitter v2 API
        format!(
            "{}/search?f=tweets&q={}&since=&until=&near=",
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
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// AI/技术趋势关键词
    fn trend_keywords() -> Vec<&'static str> {
        vec![
            "llm", "gpt", "ai agent", "agentic", "rag",
            "vector database", "embedding", "fine-tuning",
            "opensource alternative", "how to integrate",
            "struggling with", "looking for", "need help",
        ]
    }

    /// 检查推文是否包含趋势关键词或需求信号
    fn extract_signals(tweet_text: &str) -> Option<Vec<String>> {
        let text_lower = tweet_text.to_lowercase();
        let mut detected = Vec::new();

        // 技术趋势
        for kw in Self::trend_keywords() {
            if text_lower.contains(kw) {
                detected.push(format!("trend:{}", kw));
            }
        }

        // 需求模式
        let demand_patterns = [
            ("demand:problem", vec!["how to", "how do i", "trying to", "struggling", "issue with", "bug in", "doesn't work", "not working"]),
            ("demand:integration", vec!["integrate", "integration with", "connect to", "works with", "supports", "plugin for", "sdk for"]),
            ("demand:feature_request", vec!["would love", "would be great if", "is there a way", "any plans", "feature request", "need a", "looking for a"]),
            ("demand:comparison", vec!["vs", "versus", "compare", "better than", "alternative to", "why use", "which is best"]),
        ];

        for (tag, patterns) in &demand_patterns {
            if patterns.iter().any(|p| text_lower.contains(p)) {
                detected.push(tag.to_string());
            }
        }

        if detected.is_empty() {
            None
        } else {
            Some(detected)
        }
    }

    /// 从推文中提取核心需求描述
    fn extract_summary(tweet_text: &str) -> String {
        let text = tweet_text.replace('\n', " ").trim().to_string();
        if text.len() > 150 {
            format!("{}...", &text[..150])
        } else {
            text
        }
    }
}

impl Default for TwitterScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for TwitterScanner {
    fn platform(&self) -> &'static str {
        "twitter"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q, 20);
        let headers = self.build_headers();

        tracing::info!(
            query = q,
            "[TwitterScanner] 发起搜索请求"
        );

        // 注意：在真实生产环境中，这里应该使用 Twitter v2 API (`https://api.twitter.com/2/tweets/search/recent`)
        // 并解析 JSON 响应。当前的实现演示了处理逻辑，针对 Nitter HTML 解析或 fallback 逻辑。
        // 为了保证在无网络或非官方环境下也能正常测试，我们先尝试请求，失败则返回空结果。
        
        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                // 在真实场景中，这里应解析 `resp.json::<TwitterResponse>().await` 并提取 tweets
                // 为了保持代码的健壮性，我们假设可能解析失败
                if let Ok(text) = resp.text().await {
                    // 简单的 HTML/JSON 解析示例：查找可能包含需求信号的文本块
                    // 这只是一个占位逻辑，实际应由专门的解析器处理
                    for line in text.lines() {
                        if let Some(signals) = Self::extract_signals(line) {
                            let summary = Self::extract_summary(line);
                            let url = self.base_url.clone(); // 实际上应提取 tweet URL

                            leads.push(RawLead {
                                platform: "twitter".to_string(),
                                title: format!("Twitter Signal: {}", signals.join(", ")),
                                description: summary,
                                url,
                                price_text: None,
                                contact: None,
                                snapshot: serde_json::json!({
                                    "source": "twitter_scanner",
                                    "signals": signals,
                                    "raw_text": line,
                                }),
                            });
                        }
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[TwitterScanner] 请求失败");
                // 如果是速率限制，返回错误
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("Twitter API 速率限制".to_string());
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "[TwitterScanner] 网络请求异常，返回空结果");
                // 网络错误时不中断流程，返回空结果
            }
        }

        tracing::info!(
            query = q,
            filtered = leads.len(),
            "[TwitterScanner] 搜索完成"
        );

        Ok(leads)
    }
}

// 预留的 Twitter API 响应结构体
// structs are reserved for future use with official API

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = TwitterScanner::new();
        assert_eq!(scanner.platform(), "twitter");
    }

    #[test]
    fn test_extract_signals_demand() {
        // 包含需求信号
        let signals = TwitterScanner::extract_signals("How to integrate this LLM with my existing API?");
        assert!(signals.is_some());
        let sigs = signals.unwrap();
        assert!(sigs.iter().any(|s| s.contains("integration")));
    }

    #[test]
    fn test_extract_signals_trend() {
        // 包含技术趋势
        let signals = TwitterScanner::extract_signals("Just tried the new RAG approach with vector databases, game changer!");
        assert!(signals.is_some());
        assert!(signals.unwrap().iter().any(|s| s.contains("vector database")));
    }

    #[test]
    fn test_extract_signals_noise() {
        // 无关内容
        let signals = TwitterScanner::extract_signals("Had a great lunch today.");
        assert!(signals.is_none());
    }

    #[test]
    fn test_build_search_url() {
        let scanner = TwitterScanner::new();
        let url = scanner.build_search_url("ai agent", 10);
        assert!(url.contains("ai+agent"));
        assert!(url.contains("search"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = TwitterScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
