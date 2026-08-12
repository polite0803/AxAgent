//! 猪八戒扫描器
//! 通过官方 API 或网页抓取采集猪八戒平台的外包需求线索

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// 猪八戒扫描器
pub struct ZhubajieScanner {
    http: reqwest::Client,
    /// API Token，用于官方 API 认证
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl ZhubajieScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("ZHUBAJIE_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://www.zhubajie.com".to_string() }
    }

    /// 从环境变量或配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://www.zhubajie.com".to_string()),
        }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        let encoded_query = query.replace(' ', "+");
        format!("{}/task/search/?keyword={}", self.base_url, encoded_query)
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

    /// 猪八戒任务分类关键词（外包需求类型）
    fn demand_categories() -> Vec<&'static str> {
        vec![
            "网站建设",
            "APP开发",
            "小程序",
            "微信开发",
            "H5开发",
            "UI设计",
            "LOGO设计",
            "VI设计",
            "平面设计",
            "视频制作",
            "文案策划",
            "翻译",
            "数据处理",
            "运营推广",
            "营销推广",
            "ERP",
            "CRM",
            "MES",
            "系统开发",
            "软件定制",
        ]
    }

    /// 价格范围转换
    fn parse_price_range(price_text: &str) -> Option<(f64, f64)> {
        // 解析 "1000-5000元" 或 "5000元以上" 等格式
        let cleaned = price_text.replace(['元', ','], "").trim().to_string();

        if let Some((min, max)) = cleaned.split_once('-') {
            let min = min.trim().parse::<f64>().ok()?;
            let max = max.trim().parse::<f64>().ok()?;
            Some((min, max))
        } else if let Some(min) = cleaned.strip_suffix("以上") {
            let min = min.trim().parse::<f64>().ok()?;
            Some((min, min * 2.0)) // 估算上限
        } else if let Some(min) = cleaned.strip_suffix("以下") {
            let max = min.trim().parse::<f64>().ok()?;
            Some((0.0, max))
        } else {
            cleaned.parse::<f64>().ok().map(|val| (val, val))
        }
    }

    /// 需求相关性判断
    fn is_demand_related(title: &str, category: Option<&str>) -> bool {
        let text = title.to_lowercase();

        // 检查是否包含需求类型关键词
        if let Some(cat) = category
            && Self::demand_categories().iter().any(|c| cat.contains(c))
        {
            return true;
        }

        // 检查标题中的需求关键词
        let demand_keywords = [
            "求", "找", "需要", "招", "聘", "外包", "兼职", "开发", "设计", "制作", "实现", "定制",
            "帮忙", "协助", "合作", "项目", "任务",
        ];

        demand_keywords.iter().any(|kw| text.contains(kw))
    }
}

impl Default for ZhubajieScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for ZhubajieScanner {
    fn platform(&self) -> &'static str {
        "zhubajie"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[ZhubajieScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                // 在真实实现中，这里应解析响应并提取任务列表
                // 当前为框架实现，返回空结果
                if let Ok(body) = resp.text().await {
                    // 简单的 HTML 解析示例：查找任务卡片
                    // 实际应使用专门的解析器或 API
                    for line in body.lines() {
                        let trimmed = line.trim();
                        if Self::is_demand_related(trimmed, None) && trimmed.len() > 20 {
                            // 提取价格信息
                            let price_text = Self::parse_price_range(trimmed).map(|(min, max)| {
                                if min == max {
                                    format!("{:.0}元", min)
                                } else {
                                    format!("{:.0}-{:.0}元", min, max)
                                }
                            });

                            // 提取需求线索
                            let title = if trimmed.len() > 80 {
                                format!("{}...", &trimmed[..80])
                            } else {
                                trimmed.to_string()
                            };

                            leads.push(RawLead {
                                platform: "zhubajie".to_string(),
                                title,
                                description: trimmed.to_string(),
                                url: url.clone(),
                                price_text,
                                contact: None,
                                snapshot: serde_json::json!({
                                    "source": "zhubajie_scanner",
                                    "type": "outsourcing",
                                    "raw_text": trimmed,
                                }),
                            });
                        }
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[ZhubajieScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("猪八戒 API 速率限制".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[ZhubajieScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[ZhubajieScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = ZhubajieScanner::new();
        assert_eq!(scanner.platform(), "zhubajie");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = ZhubajieScanner::new();
        let url = scanner.build_search_url("小程序开发");
        // 中文直接保留
        assert!(url.contains("小程序开发"));
        assert!(url.contains("search"));

        let url_with_space = scanner.build_search_url("小程序 开发");
        assert!(url_with_space.contains("小程序+开发"));
    }

    #[test]
    fn test_is_demand_related() {
        // 应该识别为需求
        assert!(ZhubajieScanner::is_demand_related("急需一个小程序开发，预算5000", None));
        assert!(ZhubajieScanner::is_demand_related("寻找UI设计师合作", None));

        // 通过分类识别
        assert!(ZhubajieScanner::is_demand_related("具体需求", Some("APP开发")));

        // 不相关的内容
        assert!(!ZhubajieScanner::is_demand_related("今天天气不错", None));
        assert!(!ZhubajieScanner::is_demand_related("分享一个技术文章", None));
    }

    #[test]
    fn test_parse_price_range() {
        assert_eq!(ZhubajieScanner::parse_price_range("1000-5000元"), Some((1000.0, 5000.0)));
        assert_eq!(ZhubajieScanner::parse_price_range("5000元以上"), Some((5000.0, 10000.0)));
        assert_eq!(ZhubajieScanner::parse_price_range("1000元以下"), Some((0.0, 1000.0)));
        assert_eq!(ZhubajieScanner::parse_price_range("3000元"), Some((3000.0, 3000.0)));
        assert_eq!(ZhubajieScanner::parse_price_range("无效价格"), None);
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = ZhubajieScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
