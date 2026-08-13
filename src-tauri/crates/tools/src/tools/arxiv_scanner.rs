//! ArXiv 扫描器
//! 通过 ArXiv API 采集研究论文中的技术趋势和需求信号

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// ArXiv 扫描器
pub struct ArxivScanner {
    http: reqwest::Client,
}

impl ArxivScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        Self { http }
    }

    /// 构建搜索 URL
    fn build_search_url(query: &str, max_results: u32) -> String {
        let encoded_query = query.replace(' ', "+AND+");
        format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}&sortBy=submittedDate&sortOrder=descending",
            encoded_query, max_results
        )
    }

    /// ArXiv 分类映射
    fn category_map() -> Vec<(&'static str, &'static str)> {
        vec![
            ("cs.AI", "人工智能"),
            ("cs.LG", "机器学习"),
            ("cs.CL", "自然语言处理"),
            ("cs.CV", "计算机视觉"),
            ("cs.SE", "软件工程"),
            ("cs.DC", "分布式计算"),
            ("stat.ML", "统计机器学习"),
            ("q-fin", "量化金融"),
            ("cs.CE", "计算工程"),
            ("eess.SP", "信号处理"),
        ]
    }

    /// 检查论文是否为需求相关
    fn is_demand_paper(title: &str, abstract_text: &str, categories: &[String]) -> bool {
        let demand_keywords = [
            "framework",
            "library",
            "tool",
            "system",
            "implementation",
            "architecture",
            "design",
            "integration",
            "api",
            "protocol",
            "optimization",
            "performance",
            "efficiency",
            "real-world",
            "production",
            "deployment",
            "financial",
            "trading",
            "market",
            "stock",
            "application",
            "app",
            "platform",
            "benchmark",
            "evaluation",
            "comparison",
        ];

        let full_text = format!("{} {}", title, abstract_text).to_lowercase();
        let has_demand_keyword = demand_keywords.iter().any(|kw| full_text.contains(kw));

        let demand_categories: std::collections::HashSet<&str> =
            Self::category_map().iter().map(|(cat, _)| *cat).collect();
        let has_demand_category = categories.iter().any(|c| demand_categories.contains(c.as_str()));

        has_demand_keyword || has_demand_category
    }

    /// 从论文中提取技术需求描述
    fn extract_tech_trend(
        title: &str,
        abstract_text: &str,
        categories: &[String],
    ) -> Option<String> {
        let abstract_lower = abstract_text.to_lowercase();

        let tech_patterns = [
            ("llm", vec!["large language model", "llm", "transformer", "gpt", "bert"]),
            ("multi_modal", vec!["multi-modal", "multimodal", "cross-modal"]),
            ("agent", vec!["agent", "agents", "autonomous", "multi-agent"]),
            ("rag", vec!["retrieval-augmented", "rag", "retrieval augmented"]),
            ("fine_tuning", vec!["fine-tuning", "fine tuning", "parameter-efficient"]),
            ("quantization", vec!["quantization", "quantized", "int8", "int4"]),
            ("distributed", vec!["distributed", "parallel", "scaling", "large-scale"]),
            ("real_time", vec!["real-time", "real time", "streaming", "online"]),
            ("financial", vec!["financial", "finance", "trading", "stock market", "investment"]),
            ("code", vec!["code generation", "code completion", "program synthesis"]),
            ("edge", vec!["edge", "on-device", "mobile", "embedded", "IoT"]),
        ];

        let mut detected_trends = Vec::new();
        for (trend, patterns) in &tech_patterns {
            if patterns.iter().any(|p| abstract_lower.contains(p)) {
                detected_trends.push(trend.to_string());
            }
        }

        if !detected_trends.is_empty() {
            Some(format!(
                "[{}] {}",
                detected_trends.join(", "),
                title.chars().take(120).collect::<String>()
            ))
        } else if categories.len() >= 2 {
            Some(format!(
                "[cross-disciplinary] {} ({})",
                title.chars().take(120).collect::<String>(),
                categories.join(", ")
            ))
        } else {
            None
        }
    }

    /// 解析 Atom XML 响应
    fn parse_atom_response(body: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut entries = Vec::new();

        // 简单 XML 解析（不依赖 xml 库）
        let entries_str = body.split("<entry>").collect::<Vec<&str>>();
        for entry in entries_str.iter().skip(1) {
            let title = Self::extract_xml_text(entry, "title");
            let summary = Self::extract_xml_text(entry, "summary");
            let id = Self::extract_xml_text(entry, "id");
            let published = Self::extract_xml_text(entry, "published");
            let updated = Self::extract_xml_text(entry, "updated");

            // 提取分类
            let categories: Vec<String> = {
                let mut cats = Vec::new();
                let mut offset = 0;
                while let Some(start) = entry[offset..].find("<category") {
                    let start = offset + start;
                    if let Some(end) = entry[start..].find('>') {
                        let cat_str = &entry[start..start + end + 1];
                        if let Some(term_start) = cat_str.find("term=\"") {
                            let term_start = term_start + 6;
                            if let Some(term_end) = cat_str[term_start..].find('"') {
                                let term = &cat_str[term_start..term_start + term_end];
                                cats.push(term.to_string());
                            }
                        }
                        offset = start + end + 1;
                    } else {
                        break;
                    }
                }
                cats
            };

            // 提取链接
            let pdf_link = {
                let mut link = String::new();
                if let Some(pdf_start) = entry.find("title=\"pdf\"")
                    && let Some(href_rel) = entry[pdf_start..].find("href=\"")
                {
                    let href_start = pdf_start + href_rel + 6;
                    if let Some(href_end) = entry[href_start..].find('"') {
                        link = entry[href_start..href_start + href_end].to_string();
                    }
                }
                link
            };

            entries.push(serde_json::json!({
                "title": title,
                "summary": summary,
                "id": id,
                "published": published,
                "updated": updated,
                "categories": categories,
                "pdf_link": pdf_link,
            }));
        }

        Ok(entries)
    }

    /// 提取 XML 标签文本
    fn extract_xml_text(xml: &str, tag: &str) -> String {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);
        if let Some(start) = xml.find(&open_tag) {
            let start = start + open_tag.len();
            if let Some(end) = xml[start..].find(close_tag.as_str()) {
                let text = &xml[start..start + end];
                return text.trim().replace('\n', " ").replace("  ", " ");
            }
        }
        String::new()
    }
}

impl Default for ArxivScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for ArxivScanner {
    fn platform(&self) -> &'static str {
        "arxiv"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let url = Self::build_search_url(q, 30);

        tracing::info!(query = q, "[ArxivScanner] 发起搜索请求");

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("ArXiv API 请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("ArXiv API 返回状态码 {}", status));
        }

        let body = response.text().await.map_err(|e| format!("ArXiv 响应读取失败: {}", e))?;

        let papers = Self::parse_atom_response(&body)?;

        let mut leads = Vec::new();

        for paper in &papers {
            let title = paper["title"].as_str().unwrap_or("").to_string();
            let summary = paper["summary"].as_str().unwrap_or("").to_string();
            let id = paper["id"].as_str().unwrap_or("").to_string();
            let pdf_link = paper["pdf_link"].as_str().unwrap_or("").to_string();
            let categories: Vec<String> = paper["categories"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|c| c.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            if Self::is_demand_paper(&title, &summary, &categories)
                && let Some(trend_desc) = Self::extract_tech_trend(&title, &summary, &categories)
            {
                let url = if !pdf_link.is_empty() {
                    pdf_link
                } else {
                    id.clone()
                };

                let mut snapshot = paper.clone();
                snapshot["_extracted_trend"] = serde_json::json!(trend_desc);
                snapshot["_extracted_source"] = serde_json::json!("arxiv");

                leads.push(RawLead {
                    platform: "arxiv".to_string(),
                    title,
                    description: trend_desc,
                    url,
                    price_text: None,
                    contact: None,
                    contact_email: None,
                    contact_phone: None,
                    snapshot,
                });
            }
        }

        tracing::info!(
            query = q,
            total = papers.len(),
            filtered = leads.len(),
            "[ArxivScanner] 搜索完成"
        );

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = ArxivScanner::new();
        assert_eq!(scanner.platform(), "arxiv");
    }

    #[test]
    fn test_build_search_url() {
        let url = ArxivScanner::build_search_url("machine learning", 10);
        assert!(url.contains("export.arxiv.org"));
        assert!(url.contains("machine"));
        assert!(url.contains("10"));
    }

    #[test]
    fn test_is_demand_paper() {
        // 包含框架关键词
        assert!(ArxivScanner::is_demand_paper(
            "A Framework for Real-time ML",
            "We propose a framework for real-time machine learning systems.",
            &["cs.LG".to_string()]
        ));

        // 包含金融分类
        assert!(ArxivScanner::is_demand_paper(
            "Deep Learning for Stock Prediction",
            "Using neural networks for financial time series prediction.",
            &["q-fin".to_string()]
        ));

        // 不相关
        assert!(!ArxivScanner::is_demand_paper(
            "A Study of Abstract Algebra",
            "We study the properties of abstract algebraic structures.",
            &["math.AG".to_string()]
        ));
    }

    #[test]
    fn test_extract_tech_trend() {
        // LLM 趋势
        let trend = ArxivScanner::extract_tech_trend(
            "Efficient Training of Large Language Models",
            "We propose a novel approach for training large language models efficiently.",
            &["cs.CL".to_string()],
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("llm"));

        // Agent 趋势
        let trend = ArxivScanner::extract_tech_trend(
            "Multi-Agent Systems for Code Generation",
            "We introduce autonomous agents that can generate code.",
            &["cs.AI".to_string()],
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("agent"));

        // 金融趋势
        let trend = ArxivScanner::extract_tech_trend(
            "Deep Learning for Financial Trading",
            "Applying deep learning to financial trading strategies.",
            &["q-fin".to_string()],
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("financial"));
    }

    #[test]
    fn test_parse_atom_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Test Paper Title</title>
    <summary>This is a test abstract about machine learning framework.</summary>
    <id>http://arxiv.org/abs/2301.00001v1</id>
    <published>2023-01-01T00:00:00Z</published>
    <updated>2023-01-01T00:00:00Z</updated>
    <category term="cs.LG" scheme="http://arxiv.org/schemas/atom"/>
    <link title="pdf" href="http://arxiv.org/pdf/2301.00001v1" rel="alternate" type="application/pdf"/>
  </entry>
</feed>"#;

        let papers = ArxivScanner::parse_atom_response(xml).unwrap();
        assert_eq!(papers.len(), 1);

        let paper = &papers[0];
        assert!(paper["title"].as_str().unwrap().contains("Test Paper Title"));
        assert!(!paper["categories"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_search_with_common_keyword() {
        let scanner = ArxivScanner::new();
        let result = scanner.search("machine learning framework").await;
        assert!(result.is_ok());
    }
}
