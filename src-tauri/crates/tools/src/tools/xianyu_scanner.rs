//! 闲鱼扫描器
//! 通过公开 API 或轻量级代理采集闲鱼平台的二手需求线索

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// 闲鱼扫描器
pub struct XianyuScanner {
    http: reqwest::Client,
    /// API Token，用于官方 API 认证（可选）
    api_token: Option<String>,
    /// 基础 URL
    base_url: String,
}

impl XianyuScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("XIANYU_API_TOKEN").ok();
        Self { http, api_token, base_url: "https://www.goofish.com".to_string() }
    }

    /// 从配置创建
    pub fn with_config(api_token: Option<String>, base_url: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self {
            http,
            api_token,
            base_url: base_url.unwrap_or_else(|| "https://www.goofish.com".to_string()),
        }
    }

    /// 构建搜索 URL
    fn build_search_url(&self, query: &str) -> String {
        // 简单的 URL 编码：空格转为 +，其他保持原样
        let encoded_query = query.replace(' ', "+");
        format!("{}/search?q={}", self.base_url, encoded_query)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 闲鱼需求分类（二手交易中可能转化为需求的品类）
    fn demand_categories() -> Vec<&'static str> {
        vec![
            // 数码类（可能有定制需求）
            "手机",
            "电脑",
            "相机",
            "无人机",
            "智能设备",
            // 设计类（设计服务需求）
            "设计",
            "定制",
            "Logo",
            "VI设计",
            "包装设计",
            // 服务类（外包/兼职需求）
            "代练",
            "代写",
            "代做",
            "代运营",
            "代设计",
            // 开发类（技术需求）
            "小程序",
            "网站",
            "APP",
            "系统",
            "软件",
            // 创意类
            "手绘",
            "插画",
            "动画",
            "视频剪辑",
        ]
    }

    /// 需求模式识别
    fn extract_demand_patterns(title: &str) -> Vec<String> {
        let text = title.to_lowercase();
        let mut patterns = Vec::new();

        // 求购类关键词
        let buy_patterns = [
            ("demand:want_to_buy", vec!["求购", "求", "要", "想收", "找", "求收"]),
            ("demand:custom_make", vec!["定制", "定做", "定制", "代做", "代开发", "代设计"]),
            ("demand:service_need", vec!["代练", "代写", "代运营", "代办", "代做", "代设计"]),
            ("demand:collaboration", vec!["合作", "合伙", "招合伙人", "招", "聘"]),
            ("demand:swap", vec!["换", "以物换物", "置换"]),
        ];

        for (tag, keywords) in &buy_patterns {
            if keywords.iter().any(|k| text.contains(k)) {
                patterns.push(tag.to_string());
            }
        }

        patterns
    }

    /// 检查是否属于需求相关内容
    fn is_demand_related(title: &str, description: &str) -> bool {
        let full_text = format!("{} {}", title, description).to_lowercase();

        // 首先排除明显的出售/卖出内容
        let sell_patterns = [
            "出售",
            "卖出",
            "转卖",
            "闲置",
            "九成新",
            "全新",
            "包邮",
            "包邮出",
            "低价出",
            "诚心出",
        ];
        if sell_patterns.iter().any(|p| full_text.contains(p)) {
            return false;
        }

        // 检查需求模式
        let demand_patterns = [
            "求购",
            "求",
            "定制",
            "定做",
            "代做",
            "代开发",
            "代设计",
            "代练",
            "代写",
            "代运营",
            "代办",
            "合作",
            "合伙",
            "招合伙人",
            "换",
            "以物换物",
            "置换",
            "设计",
            "开发",
            "制作",
            "实现",
            "帮忙",
            "协助",
            "需要",
        ];

        if demand_patterns.iter().any(|p| full_text.contains(p)) {
            return true;
        }

        // 检查是否包含需求分类关键词
        let categories = Self::demand_categories();
        categories.iter().any(|c| full_text.contains(c))
    }

    /// 提取价格信息
    fn extract_price(text: &str) -> Option<String> {
        // 简化实现：查找常见的价格格式
        // 匹配 "数字元"、"¥数字"、"￥数字" 等格式

        // 尝试匹配 "¥500" 或 "￥500" 格式
        for prefix in ["¥", "￥"] {
            if let Some(start) = text.find(prefix) {
                let after = &text[start + prefix.len()..];
                let end = after
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .map(|pos| start + prefix.len() + pos)
                    .unwrap_or(text.len());
                let price_str = &text[start + prefix.len()..end];
                if !price_str.is_empty() {
                    return Some(format!("{}{}", prefix, price_str));
                }
            }
        }

        // 尝试匹配 "1200元" 或 "500块" 格式
        for suffix in ["元", "块", "毛"] {
            if let Some(end) = text.find(suffix) {
                // 使用字符遍历找到前面的连续数字
                let mut chars = text[..end].chars().rev();
                let mut num_start = end;
                let mut found_digit = false;

                for c in chars.by_ref() {
                    if c.is_ascii_digit() || c == '.' {
                        found_digit = true;
                        // 计算这个字符在原始字符串中的起始位置
                        let char_len = c.len_utf8();
                        num_start -= char_len;
                    } else if found_digit {
                        break;
                    }
                }

                if found_digit && num_start < end {
                    let price_num = &text[num_start..end];
                    return Some(format!("{}{}", price_num, suffix));
                }
            }
        }

        None
    }

    /// 提取核心需求描述
    fn extract_demand_summary(title: &str, description: &str) -> String {
        let combined = if description.is_empty() {
            title.to_string()
        } else {
            format!("{} - {}", title, description)
        };

        if combined.len() > 150 {
            format!("{}...", &combined[..150])
        } else {
            combined
        }
    }
}

impl Default for XianyuScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for XianyuScanner {
    fn platform(&self) -> &'static str {
        "xianyu"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.build_search_url(q);
        let headers = self.build_headers();

        tracing::info!(query = q, "[XianyuScanner] 发起搜索请求");

        let response = self.http.get(&url).headers(headers).send().await;

        let mut leads = Vec::new();

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.text().await {
                    // 闲鱼页面通常是 JavaScript 渲染，需要特殊处理
                    // 这里提供基础的文本分析逻辑

                    // 按行分割，查找包含需求信号的文本块
                    for segment in body.split(|c: char| c.is_control() && c != ' ' && c != '\n') {
                        let trimmed = segment.trim();
                        if trimmed.len() < 10 {
                            continue;
                        }

                        // 检查是否为需求相关内容
                        if Self::is_demand_related(trimmed, "") {
                            let patterns = Self::extract_demand_patterns(trimmed);
                            let price = Self::extract_price(trimmed);
                            let summary = Self::extract_demand_summary(trimmed, "");

                            leads.push(RawLead {
                                platform: "xianyu".to_string(),
                                title: if trimmed.len() > 80 {
                                    format!("{}...", &trimmed[..80])
                                } else {
                                    trimmed.to_string()
                                },
                                description: summary,
                                url: url.clone(),
                                price_text: price,
                                contact: None,
                                contact_email: None,
                                contact_phone: None,
                                snapshot: serde_json::json!({
                                    "source": "xianyu_scanner",
                                    "patterns": patterns,
                                    "raw_text": trimmed,
                                    "type": "second_hand_demand",
                                }),
                            });
                        }
                    }
                }
            },
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(status = status.as_u16(), "[XianyuScanner] 请求失败");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err("闲鱼 API 速率限制".to_string());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "[XianyuScanner] 网络请求异常，返回空结果");
            },
        }

        tracing::info!(query = q, filtered = leads.len(), "[XianyuScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = XianyuScanner::new();
        assert_eq!(scanner.platform(), "xianyu");
    }

    #[test]
    fn test_build_search_url() {
        let scanner = XianyuScanner::new();
        let url = scanner.build_search_url("设计定制");
        // 中文直接保留，空格转为 +
        assert!(url.contains("设计定制"));
        assert!(url.contains("search"));

        let url_with_space = scanner.build_search_url("设计 定制");
        assert!(url_with_space.contains("设计+定制"));
    }

    #[test]
    fn test_is_demand_related() {
        // 需求相关内容
        assert!(XianyuScanner::is_demand_related("求购一台二手相机", ""));
        assert!(XianyuScanner::is_demand_related("定制Logo设计", ""));
        assert!(XianyuScanner::is_demand_related("代做小程序", ""));
        assert!(XianyuScanner::is_demand_related("找合伙人合作", ""));

        // 不相关内容
        assert!(!XianyuScanner::is_demand_related("出售一台手机", ""));
        assert!(!XianyuScanner::is_demand_related("九成新，包邮", ""));
    }

    #[test]
    fn test_extract_demand_patterns() {
        let patterns = XianyuScanner::extract_demand_patterns("求购一台高端相机");
        assert!(patterns.iter().any(|p| p.contains("want_to_buy")));

        let patterns = XianyuScanner::extract_demand_patterns("代开发一个电商网站");
        assert!(patterns.iter().any(|p| p.contains("custom_make")));
    }

    #[test]
    fn test_extract_price() {
        assert_eq!(XianyuScanner::extract_price("售价1200元"), Some("1200元".to_string()));
        assert_eq!(XianyuScanner::extract_price("价格：¥500"), Some("¥500".to_string()));
        assert!(XianyuScanner::extract_price("无价格信息").is_none());
    }

    #[test]
    fn test_extract_demand_summary() {
        let summary = XianyuScanner::extract_demand_summary("短标题", "");
        assert_eq!(summary, "短标题");

        let long_title = "这是一个非常长的标题".repeat(20);
        let summary = XianyuScanner::extract_demand_summary(&long_title, "");
        assert!(summary.len() <= 153); // 150 + "..."
        assert!(summary.ends_with("..."));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let scanner = XianyuScanner::new();
        let result = scanner.search("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
