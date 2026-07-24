//! 证据引用审计溯源 — 决策理由 → 分析师报告 → 原始数据的引用链
//!
//! ## 设计
//!
//! 每次股票分析完成后，从 `decision_json` 和 `blackboard_snapshot` 中提取
//! 证据引用关系，构建可审计的引用链：
//!
//! ```text
//! 决策: 买入 贵州茅台 (置信度 75%)
//!   ├─ 理由1: ROE 持续高于 20%
//!   │   ├─ 来源: fundamentals-analyst
//!   ���   └─ 数据: 2025年报 ROE=22.3% (eastmoney)
//!   ├─ 理由2: 白酒行业景气度上行
//!   │   ├─ 来源: sector-analyst
//!   │   └─ 数据: 行业营收同比+15% (industry_ranking)
//!   └─ 理由3: MA5 金叉 MA20
//!       ├─ 来源: market-analyst
//!       └─ 数据: 5日均价 1850 > 20日均价 1830 (tencent)
//! ```
//!
//! 引用提取算法：将 `decision_reasoning` 文本按句拆分，与各分析师报告的
//! 关键词做 Jaccard 相似度匹配，找到最可能的来源。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单条证据引用
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceCitation {
    /// 决策中的一句话（理由）
    pub claim: String,
    /// 来源分析师 ID（如 "a-fundamentals", "a-technical"）
    pub source_analyst_id: String,
    /// 来源分析师显示名（如 "基本面分析师"）
    pub source_analyst_name: String,
    /// 匹配置信度 (0.0-1.0)
    pub match_confidence: f64,
    /// 分析师原文中匹配的片段
    pub source_snippet: String,
    /// 该理由是否在分析师报告中有数据支撑
    pub has_data_support: bool,
    /// 数据来源描述（如 "2025年报 ROE=22.3%"）
    pub data_source: Option<String>,
}

/// 完整引用报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationReport {
    pub stock_code: String,
    pub stock_name: String,
    pub analysis_date: String,
    pub decision_action: String,
    pub decision_confidence: f64,
    pub decision_reasoning: String,
    /// 所有证据引用（按说服力降序）
    pub citations: Vec<EvidenceCitation>,
    /// 有数据支撑的理由数
    pub supported_claims: usize,
    /// 总理由数
    pub total_claims: usize,
    /// 支撑率
    pub support_rate: f64,
    /// 参与的分析师数量
    pub analyst_count: usize,
}

/// 分析师 ID → 中文名的映射
fn analyst_display_name(id: &str) -> String {
    match id {
        "a-fundamentals" | "fundamentals-analyst" => "基本面分析师".into(),
        "a-technical" | "market-analyst" => "技术面分析师".into(),
        "a-sector" | "sector-analyst" => "行业分析师".into(),
        "a-macro" | "policy-analyst" => "宏观分析师".into(),
        "a-sentiment" | "sentiment-analyst" => "情绪分析师".into(),
        "a-news" | "news-analyst" => "新闻分析师".into(),
        "a-hot-money" | "hot-money-tracker" => "热钱追踪".into(),
        "value-investor" => "价值投资者".into(),
        "research-analyst" => "研报分析师".into(),
        "bull-researcher" | "bull-r2" | "bull-r3" => "多头研究员".into(),
        "bear-researcher" | "bear-r2" | "bear-r3" => "空头研究员".into(),
        "research-manager" | "research-mgr" => "研究经理".into(),
        "trader" => "交易员".into(),
        "catalyst-analyst" => "催化剂分析师".into(),
        "lockup-watcher" => "解禁观察".into(),
        "rule-checker" => "规则检查".into(),
        _ => id.to_string(),
    }
}

/// 从决策理由和黑板快照中提取证据引用
///
/// - `decision_reasoning`: 决策中的理由文本（由 trader 节点生成）
/// - `blackboard_snapshot`: 工作流结束时保存的黑板快照 JSON 字符串
///
/// 返回按匹配置信度降序排列的引用列表。
pub fn extract_citations(
    decision_reasoning: &str,
    blackboard_snapshot: &str,
) -> CitationReport {
    let mut citations = Vec::new();

    // 1. 解析黑板快照
    let bb: HashMap<String, serde_json::Value> =
        serde_json::from_str(blackboard_snapshot).unwrap_or_default();

    // 2. 提取所有分析师报告（report.* 前缀的键）
    let mut analyst_reports: Vec<(String, String)> = Vec::new();
    for (key, value) in &bb {
        if key.starts_with("report.") {
            let analyst_id = key.strip_prefix("report.").unwrap_or(key).to_string();
            let text = match value {
                serde_json::Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            analyst_reports.push((analyst_id, text));
        }
    }

    // 3. 按句拆分理由文本
    let claims: Vec<&str> = decision_reasoning
        .split(&['。', '！', '？', '\n'][..])
        .map(|s| s.trim())
        .filter(|s| s.len() > 6)
        .collect();

    let mut supported = 0usize;

    for claim in &claims {
        let claim_chars: std::collections::HashSet<char> = claim.chars().collect();
        let claim_len = claim_chars.len() as f64;

        let mut best_match: Option<(String, f64, String)> = None;

        for (analyst_id, report_text) in &analyst_reports {
            // 用 Jaccard 相似度做简单匹配
            let report_chars: std::collections::HashSet<char> =
                report_text.chars().take(2000).collect();
            let intersection = claim_chars.intersection(&report_chars).count() as f64;
            let union = claim_chars.union(&report_chars).count() as f64;
            let jaccard = if union > 0.0 { intersection / union } else { 0.0 };

            // 也检查是否包含相同的财务数字模式
            let number_overlap = extract_numbers(claim)
                .iter()
                .filter(|n| report_text.contains(&n.to_string()))
                .count();

            let score = jaccard * 0.7 + (number_overlap as f64 / claim_len.max(1.0)) * 30.0;

            if score > 0.15 && (best_match.is_none() || score > best_match.as_ref().unwrap().1) {
                // 取匹配段（前后 60 字）
                let snippet = extract_snippet(report_text, claim, 60);
                best_match = Some((analyst_id.clone(), score, snippet));
            }
        }

        if let Some((analyst_id, score, snippet)) = best_match {
            let has_data = extract_numbers(claim).iter().any(|n| {
                analyst_reports.iter().any(|(_, text)| text.contains(&n.to_string()))
            });
            if has_data {
                supported += 1;
            }
            citations.push(EvidenceCitation {
                claim: claim.to_string(),
                source_analyst_id: analyst_id.clone(),
                source_analyst_name: analyst_display_name(&analyst_id),
                match_confidence: (score * 100.0).round() / 100.0,
                source_snippet: snippet,
                has_data_support: has_data,
                data_source: if has_data {
                    Some("分析师报告包含匹配数据".into())
                } else {
                    None
                },
            });
        } else {
            // 无匹配 → 标记为"无来源"
            citations.push(EvidenceCitation {
                claim: claim.to_string(),
                source_analyst_id: "unknown".into(),
                source_analyst_name: "未识别来源".into(),
                match_confidence: 0.0,
                source_snippet: String::new(),
                has_data_support: false,
                data_source: None,
            });
        }
    }

    // 按置信度降序
    citations.sort_by(|a, b| b.match_confidence.partial_cmp(&a.match_confidence).unwrap_or(std::cmp::Ordering::Equal));

    let total = citations.len();
    let support_rate = if total > 0 { supported as f64 / total as f64 } else { 0.0 };

    // 分析师数量（在 citations 被 move 前计算）
    let analyst_ids: Vec<String> = citations.iter().map(|c| c.source_analyst_id.clone()).collect();
    let unique_count: std::collections::HashSet<&str> =
        analyst_ids.iter().map(|s| s.as_str()).collect();

    CitationReport {
        stock_code: String::new(),
        stock_name: String::new(),
        analysis_date: String::new(),
        decision_action: String::new(),
        decision_confidence: 0.0,
        decision_reasoning: decision_reasoning.to_string(),
        citations,
        supported_claims: supported,
        total_claims: total,
        support_rate,
        analyst_count: unique_count.len(),
    }
}

/// 从文本中提取数字
fn extract_numbers(text: &str) -> Vec<f64> {
    let mut nums = Vec::new();
    let mut current = String::new();
    let mut has_dot = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if ch == '.' && !has_dot && !current.is_empty() {
            current.push(ch);
            has_dot = true;
        } else {
            if !current.is_empty() {
                if let Ok(n) = current.parse::<f64>() {
                    nums.push(n);
                }
                current.clear();
                has_dot = false;
            }
        }
    }
    if !current.is_empty() {
        if let Ok(n) = current.parse::<f64>() {
            nums.push(n);
        }
    }
    nums
}

/// 在原文中定位匹配文本段
fn extract_snippet(text: &str, query: &str, context_chars: usize) -> String {
    if let Some(pos) = text.find(&query[..query.len().min(20)]) {
        let start = pos.saturating_sub(context_chars);
        let end = (pos + query.len() + context_chars).min(text.len());
        let snippet = &text[start..end];
        if start > 0 {
            format!("...{}...", snippet)
        } else {
            format!("{}...", snippet)
        }
    } else {
        // 用重叠词定位
        let words: Vec<&str> = query.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 1)
            .collect();
        for w in words {
            if let Some(pos) = text.find(w) {
                let start = pos.saturating_sub(context_chars);
                let end = (pos + context_chars * 2).min(text.len());
                let snippet = &text[start..end];
                return if start > 0 {
                    format!("...{}...", snippet)
                } else {
                    format!("{}...", snippet)
                };
            }
        }
        String::new()
    }
}

/// 将 CitationsReport 渲染为可读的 Markdown 文本
pub fn citations_to_markdown(report: &CitationReport) -> String {
    let mut md = String::new();
    md.push_str("## 证据引用审计\n\n");
    md.push_str(&format!("**决策**: {} (置信度 {:.0}%)\n\n", report.decision_action, report.decision_confidence));
    md.push_str(&format!("**数据支撑率**: {:.0}% ({}/{})\n\n", report.support_rate * 100.0, report.supported_claims, report.total_claims));
    md.push_str(&format!("**参与分析师**: {} 个\n\n", report.analyst_count));

    for (i, citation) in report.citations.iter().enumerate() {
        let confidence_bar = match (citation.match_confidence * 10.0) as usize {
            0..=2 => "🟡",
            3..=6 => "🟢",
            _ => "🔵",
        };
        md.push_str(&format!(
            "{}. {}\n",
            i + 1,
            citation.claim
        ));
        md.push_str(&format!(
            "   {} 来源: {} (匹配度 {:.0}%)\n",
            confidence_bar,
            citation.source_analyst_name,
            citation.match_confidence * 100.0
        ));
        if citation.has_data_support {
            md.push_str("   📊 有数据支撑\n");
        }
        if !citation.source_snippet.is_empty() {
            md.push_str(&format!("   > {}\n", citation.source_snippet));
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_numbers() {
        let nums = extract_numbers("ROE 22.3%, 营收 150亿, 增长 5.2%");
        assert!(!nums.is_empty());
        assert!(nums.contains(&22.3));
        assert!(nums.contains(&5.2));
    }

    #[test]
    fn test_empty_input() {
        let report = extract_citations("", "{}");
        assert_eq!(report.total_claims, 0);
        assert_eq!(report.analyst_count, 0);
    }

    #[test]
    fn test_basic_extraction() {
        let reasoning = "公司ROE持续高于20%,基本面优秀。行业景气度向上。";
        let snapshot = r#"{
            "report.a-fundamentals": "该股ROE连续3年超过20%，盈利能力突出",
            "report.a-sector": "白酒行业整体营收增长15%，景气度较高"
        }"#;
        let report = extract_citations(reasoning, snapshot);
        assert!(report.total_claims > 0);
        // 至少有一个理由匹配上了
        let matched = report.citations.iter().filter(|c| c.match_confidence > 0.0).count();
        assert!(matched > 0, "应有至少一个理由匹配到分析师报告");
    }
}
