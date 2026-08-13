//! CSDN/掘金扫描器
//! 通过公开 API 采集国内开发者社区的技术需求和趋势
//! 主要数据源：CSDN 博客、掘金文章

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// 开发者社区类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevCommunity {
    Csdn,
    Juejin,
}

/// CSDN/掘金扫描器
pub struct CsdnScanner {
    http: reqwest::Client,
    /// 目标社区
    community: DevCommunity,
    /// 基础 URL
    base_url: String,
}

impl CsdnScanner {
    pub fn new(community: DevCommunity) -> Self {
        let http = reqwest::Client::new();
        let base_url = match community {
            DevCommunity::Csdn => "https://so.csdn.net".to_string(),
            DevCommunity::Juejin => "https://api.juejin.cn".to_string(),
        };
        Self { http, community, base_url }
    }

    /// 创建 CSDN 扫描器
    pub fn csdn() -> Self {
        Self::new(DevCommunity::Csdn)
    }

    /// 创建掘金扫描器
    pub fn juejin() -> Self {
        Self::new(DevCommunity::Juejin)
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "%20");
        match self.community {
            DevCommunity::Csdn => {
                format!(
                    "{}/api/v1/search?q={}&t=all&p=1&s=0&tm=0&lv=-1&ft=0&l=&u=&ct=-1&pnt=-1&ry=-1&ss=-1&dct=-1&vt=-1&bnt=-1&ewt=-1&fst=0&ra=21",
                    self.base_url, encoded_query
                )
            },
            DevCommunity::Juejin => {
                format!("{}/search_api/v1/search?keyword={}&limit=20", self.base_url, encoded_query)
            },
        }
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".parse().unwrap(),
        );
        headers
            .insert(reqwest::header::ACCEPT, "application/json, text/plain, */*".parse().unwrap());

        match self.community {
            DevCommunity::Csdn => {
                headers.insert(reqwest::header::REFERER, "https://so.csdn.net/".parse().unwrap());
                headers.insert("X-Ca-Timestamp", "1".parse().unwrap());
            },
            DevCommunity::Juejin => {
                headers.insert(reqwest::header::REFERER, "https://juejin.cn/".parse().unwrap());
            },
        }

        headers
    }

    /// 技术趋势关键词（从文章标题中提取的趋势信号）
    fn trend_keywords() -> Vec<&'static str> {
        vec![
            // AI 技术趋势
            "大模型",
            "LLM",
            "GPT",
            "RAG",
            "向量数据库",
            "Agent",
            "智能体",
            "Diffusion",
            "扩散模型",
            "微调",
            "Fine-tuning",
            "Embedding",
            // 框架/工具趋势
            "LangChain",
            "LlamaIndex",
            "Dify",
            "PyTorch",
            "TensorFlow",
            "JAX",
            // 工程实践趋势
            "容器",
            "Kubernetes",
            "微服务",
            "Serverless",
            "DevOps",
            "CI/CD",
            "云原生",
            // 前端趋势
            "React 19",
            "Vue 3",
            "Svelte 5",
            "Tailwind",
            "Vite",
            "Turbopack",
        ]
    }

    /// 需求模式识别
    fn extract_demand_signals(title: &str, content: &str) -> Vec<String> {
        let text = format!("{} {}", title, content).to_lowercase();
        let mut signals = Vec::new();

        // 检查技术趋势
        for trend in Self::trend_keywords() {
            if text.contains(&trend.to_lowercase().to_string()) {
                signals.push(format!("trend:{}", trend));
            }
        }

        // 检查需求模式（文章类型预示需求）
        let demand_patterns = [
            ("demand:tutorial", vec!["教程", "入门", "实战", "指南", "手把手", "快速上手"]),
            ("demand:comparison", vec!["对比", "比较", "优缺点", "vs", "哪个好", "选型"]),
            ("demand:troubleshooting", vec!["踩坑", "避坑", "问题", "错误", "异常", "解决", "bug"]),
            ("demand:architecture", vec!["架构", "设计", "模式", "最佳实践", "设计模式"]),
            ("demand:optimization", vec!["优化", "性能", "提速", "调优", "实战优化"]),
            ("demand:migration", vec!["迁移", "升级", "改造", "重构", "迭代"]),
            ("demand:integration", vec!["集成", "整合", "对接", "接入", "插件"]),
            ("demand:new_tech", vec!["新特性", "新版本", "发布", "更新", "升级到"]),
        ];

        for (tag, patterns) in &demand_patterns {
            if patterns.iter().any(|p| text.contains(p)) {
                signals.push(tag.to_string());
            }
        }

        signals
    }

    /// 判断文章是否为高价值需求信号
    fn is_valuable_signal(title: &str) -> bool {
        let high_value_patterns = [
            "大模型",
            "LLM",
            "AI",
            "人工智能",
            "实战",
            "教程",
            "项目",
            "案例",
            "对比",
            "选型",
            "推荐",
            "问题",
            "解决",
            "踩坑",
            "架构",
            "设计",
            "优化",
        ];
        high_value_patterns.iter().any(|p| title.contains(p))
    }

    /// 提取文章摘要
    fn extract_summary(title: &str, description: Option<&str>) -> String {
        if let Some(desc) = description {
            let combined = if title.contains(desc) {
                title.to_string()
            } else {
                format!("{} - {}", title, desc)
            };
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

    /// 获取平台名称
    fn platform_name(&self) -> &'static str {
        match self.community {
            DevCommunity::Csdn => "csdn",
            DevCommunity::Juejin => "juejin",
        }
    }
}

impl Default for CsdnScanner {
    fn default() -> Self {
        Self::csdn()
    }
}

#[async_trait]
impl MarketplaceScanner for CsdnScanner {
    fn platform(&self) -> &'static str {
        self.platform_name()
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q);
        let headers = self.build_headers();
        let platform = self.platform_name();

        tracing::info!(
            query = q,
            community = ?self.community,
            "[CsdnScanner] 发起搜索请求"
        );

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    // 解析响应
                    // 实际实现中应解析 JSON 响应并提取文章列表
                    // 此处提供文本分析逻辑

                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.len() < 15 {
                            continue;
                        }

                        // 检查是否为高价值信号
                        if !Self::is_valuable_signal(trimmed) {
                            continue;
                        }

                        let signals = Self::extract_demand_signals(trimmed, "");
                        if signals.is_empty() {
                            continue;
                        }

                        let title = if trimmed.len() > 80 {
                            format!("{}...", &trimmed[..80])
                        } else {
                            trimmed.to_string()
                        };

                        let summary = Self::extract_summary(&title, None);

                        leads.push(RawLead {
                            platform: platform.to_string(),
                            title: format!("{} 技术信号: {}", platform, signals.join(", ")),
                            description: summary,
                            url: url.clone(),
                            price_text: None,
                            contact: None,
                            contact_email: None,
                            contact_phone: None,
                            snapshot: serde_json::json!({
                                "source": "dev_community_scanner",
                                "community": platform,
                                "signals": signals,
                                "type": "tech_trend",
                                "raw_text": trimmed,
                            }),
                        });
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(
                    status = status.as_u16(),
                    community = platform,
                    "[CsdnScanner] 请求失败"
                );
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(format!("{} API 速率限制", platform));
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, community = platform, "[CsdnScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(
            query = q,
            filtered = leads.len(),
            community = platform,
            "[CsdnScanner] 搜索完成"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_csdn() {
        let scanner = CsdnScanner::csdn();
        assert_eq!(scanner.platform(), "csdn");
    }

    #[test]
    fn test_platform_juejin() {
        let scanner = CsdnScanner::juejin();
        assert_eq!(scanner.platform(), "juejin");
    }

    #[test]
    fn test_build_search_url_csdn() {
        let scanner = CsdnScanner::csdn();
        let url = scanner.build_search_url("大模型开发");
        assert!(url.contains("大模型"));
        assert!(url.contains("search"));
    }

    #[test]
    fn test_build_search_url_juejin() {
        let scanner = CsdnScanner::juejin();
        let url = scanner.build_search_url("RAG 实战");
        assert!(url.contains("RAG"));
        assert!(url.contains("search"));
    }

    #[test]
    fn test_extract_demand_signals() {
        // 包含多个需求信号
        let signals = CsdnScanner::extract_demand_signals(
            "大模型 RAG 实战教程",
            "详细介绍如何从零搭建 RAG 系统",
        );
        assert!(!signals.is_empty());
        assert!(signals.iter().any(|s| s.contains("trend:大模型")));
        assert!(signals.iter().any(|s| s.contains("trend:RAG")));
        assert!(signals.iter().any(|s| s.contains("demand:tutorial")));
    }

    #[test]
    fn test_is_valuable_signal() {
        // 有价值的信号
        assert!(CsdnScanner::is_valuable_signal("大模型 RAG 实战"));
        assert!(CsdnScanner::is_valuable_signal("LLM 应用对比与选型"));
        assert!(CsdnScanner::is_valuable_signal("Kubernetes 架构设计最佳实践"));

        // 低价值信号
        assert!(!CsdnScanner::is_valuable_signal("闲聊几句"));
        assert!(!CsdnScanner::is_valuable_signal("今日心情"));
    }

    #[test]
    fn test_trend_keywords() {
        let keywords = CsdnScanner::trend_keywords();
        assert!(keywords.contains(&"大模型"));
        assert!(keywords.contains(&"LangChain"));
        assert!(keywords.contains(&"Kubernetes"));
        assert!(keywords.len() > 30);
    }

    #[test]
    fn test_extract_summary() {
        let summary = CsdnScanner::extract_summary("大模型 RAG 实战教程", None);
        assert_eq!(summary, "大模型 RAG 实战教程");

        let summary = CsdnScanner::extract_summary("标题", Some("这是一段关于大模型的描述"));
        assert!(summary.contains("标题"));
        assert!(summary.contains("大模型"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = CsdnScanner::csdn();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
