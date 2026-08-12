// SPDX-License-Identifier: AGPL-3.0-only

//! 能力匹配引擎
//!
//! 为需求线索匹配系统能力（工具/技能/MCP/工作流），
//! 采用多维度加权评分算法，输出匹配度和推荐工作流。

use serde::{Deserialize, Serialize};

use super::capability::CapabilityEntry;

// ── 匹配结果 ──────────────────────────────────────────────────

/// 单个能力的匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub capability_id: String,
    pub capability_name: String,
    pub capability_source: String,
    pub capability_type: String,
    pub score: f64,
    pub matched_keywords: Vec<String>,
    pub match_reasons: Vec<String>,
}

/// 整体匹配报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchReport {
    pub lead_id: String,
    pub total_score: f64,
    pub confidence: f64,
    pub matched_capabilities: Vec<MatchResult>,
    pub recommended_workflow_id: Option<String>,
    pub recommended_workflow_name: Option<String>,
    pub capability_gaps: Vec<String>,
    pub has_sufficient_capability: bool,
}

// ── 关键词同义词映射 ──────────────────────────────────────────

/// 领域关键词 → 标准化标签映射
///
/// 用于识别需求所属的技术领域，提升匹配精度。
const DOMAIN_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "web_development",
        &["网站", "官网", "网页", "web", "网站建设", "网站开发", "响应式", "前端", "后端", "全栈"],
    ),
    (
        "ui_design",
        &[
            "设计",
            "UI",
            "UI设计",
            "界面",
            "视觉",
            "LOGO",
            "logo",
            "VI",
            "品牌",
            "平面设计",
            "UI/UX",
        ],
    ),
    (
        "mobile_app",
        &["APP", "app", "移动", "iOS", "Android", "安卓", "小程序", "微信小程序", "原生APP"],
    ),
    ("mini_program", &["小程序", "微信小程序", "支付宝小程序", "小程序开发"]),
    ("data_analysis", &["数据", "数据分析", "BI", "报表", "统计", "可视化", "大数据"]),
    ("ai_ml", &["AI", "ai", "人工智能", "机器学习", "深度学习", "LLM", "大模型", "NLP"]),
    ("ecommerce", &["电商", "商城", "购物", "订单", "支付", "商城系统"]),
    ("crm", &["CRM", "crm", "客户管理", "销售", "客户关系"]),
    ("content_mgmt", &["CMS", "cms", "内容管理", "博客", "新闻系统"]),
    ("integration", &["集成", "API", "api", "对接", "第三方", "系统集成"]),
    ("automation", &["自动化", "RPA", "流程自动化", "工作流", "审批"]),
    ("marketing", &["营销", "推广", "SEO", "seo", "SEM", "信息流", "广告投放"]),
    ("branding", &["品牌", "VI", "logo", "标志", "品牌设计", "CI"]),
    ("video", &["视频", "剪辑", "动画", "3D", "宣传片", "短视频"]),
    ("writing", &["文案", "写作", "内容创作", "软文", "策划案"]),
];

/// 预算区间（元）
const BUDGET_RANGES: &[(&str, f64, f64)] = &[
    ("micro", 0.0, 1000.0),
    ("small", 1000.0, 10000.0),
    ("medium", 10000.0, 50000.0),
    ("large", 50000.0, 200000.0),
    ("enterprise", 200000.0, f64::INFINITY),
];

// ── 匹配引擎 ──────────────────────────────────────────────────

/// 能力匹配引擎
pub struct CapabilityMatcher;

impl CapabilityMatcher {
    /// 为需求线索匹配系统能力
    ///
    /// # 参数
    /// - `lead_id`: 需求线索 ID
    /// - `title`: 需求标题
    /// - `description`: 需求描述
    /// - `budget_min/max`: 预算区间（可选）
    /// - `capabilities`: 系统能力清单
    pub fn match_capabilities(
        lead_id: &str,
        title: &str,
        description: &str,
        budget_min: Option<f64>,
        budget_max: Option<f64>,
        capabilities: &[CapabilityEntry],
    ) -> MatchReport {
        let title_lower = title.to_lowercase();
        let description_lower = description.to_lowercase();

        // 1. 识别需求领域标签
        let detected_domains = Self::detect_domains(&title_lower, &description_lower);

        // 2. 确定预算区间
        let budget_level = Self::classify_budget(budget_min, budget_max);

        // 3. 对每个能力进行多维度评分
        let mut results: Vec<MatchResult> = Vec::new();

        for cap in capabilities {
            let score_info = Self::score_capability(
                cap,
                &title_lower,
                &description_lower,
                &detected_domains,
                budget_level,
            );

            if score_info.score > 0.0 {
                results.push(MatchResult {
                    capability_id: cap.id.clone(),
                    capability_name: cap.name.clone(),
                    capability_source: cap.source.as_str().to_string(),
                    capability_type: cap.capability_type.clone(),
                    score: score_info.score,
                    matched_keywords: score_info.matched_keywords,
                    match_reasons: score_info.reasons,
                });
            }
        }

        // 4. 排序并取 Top N
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(10);

        // 5. 计算整体置信度
        let confidence = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
        };

        // 6. 推荐工作流（优先匹配 workflow 类型的能力）
        let recommended = results.iter().find(|r| r.capability_type == "workflow");
        let (wf_id, wf_name) = match recommended {
            Some(r) => (Some(r.capability_id.clone()), Some(r.capability_name.clone())),
            None => (None, None),
        };

        // 7. 识别能力缺口
        let gaps = Self::identify_gaps(&detected_domains, &results);

        MatchReport {
            lead_id: lead_id.to_string(),
            total_score: confidence,
            confidence,
            matched_capabilities: results,
            recommended_workflow_id: wf_id,
            recommended_workflow_name: wf_name,
            capability_gaps: gaps,
            has_sufficient_capability: confidence >= 0.4,
        }
    }

    /// 识别需求所属的技术领域
    fn detect_domains(title: &str, description: &str) -> Vec<String> {
        let text = format!("{} {}", title, description);
        let mut domains: Vec<String> = Vec::new();

        for (domain, keywords) in DOMAIN_KEYWORDS {
            for kw in *keywords {
                if text.contains(&kw.to_lowercase()) {
                    domains.push(domain.to_string());
                    break;
                }
            }
        }

        domains
    }

    /// 分类预算区间
    fn classify_budget(budget_min: Option<f64>, budget_max: Option<f64>) -> &'static str {
        let avg = match (budget_min, budget_max) {
            (Some(min), Some(max)) => (min + max) / 2.0,
            (Some(min), None) => min,
            (None, Some(max)) => max,
            (None, None) => return "unknown",
        };

        for (label, min, max) in BUDGET_RANGES {
            if avg >= *min && avg < *max {
                return label;
            }
        }
        "unknown"
    }

    /// 对单个能力进行多维度评分
    fn score_capability(
        cap: &CapabilityEntry,
        title: &str,
        description: &str,
        detected_domains: &[String],
        budget_level: &str,
    ) -> ScoreInfo {
        let mut score: f64 = 0.0;
        let mut matched_keywords: Vec<String> = Vec::new();
        let mut reasons: Vec<String> = Vec::new();

        let cap_text = format!(
            "{} {} {} {}",
            cap.name,
            cap.description,
            cap.capability_type,
            serde_json::to_string(&cap.metadata).unwrap_or_default()
        )
        .to_lowercase();

        // 维度1：标题关键词匹配（权重 0.5）
        let title_words: Vec<&str> = title.split_whitespace().collect();
        for word in &title_words {
            if word.len() >= 2 && cap_text.contains(word) {
                score += 0.05;
                matched_keywords.push(word.to_string());
            }
        }

        // 维度2：描述关键词匹配（权重 0.3）
        let desc_words: Vec<&str> = description.split_whitespace().collect();
        for word in &desc_words {
            if word.len() >= 2 && cap_text.contains(word) {
                score += 0.02;
                matched_keywords.push(word.to_string());
            }
        }

        // 维度3：能力类型匹配（权重 0.15）
        let cap_type = cap.capability_type.as_str();
        if cap_type == "workflow" && !detected_domains.is_empty() {
            // 工作流类型能力匹配到领域时额外加分
            score += 0.1;
            reasons.push("工作流模板匹配".to_string());
        }

        // 维度4：来源权重（workflow > skill > mcp_tool > tool）
        let source_score = match cap.source {
            super::capability::CapabilitySource::Workflow => 0.05,
            super::capability::CapabilitySource::Skill => 0.03,
            super::capability::CapabilitySource::McpTool => 0.02,
            super::capability::CapabilitySource::Tool => 0.01,
        };
        score += source_score;

        // 维度5：预算匹配调整
        if budget_level == "large" || budget_level == "enterprise" {
            // 高预算需求倾向于完整工作流方案
            if cap_type == "workflow" {
                score += 0.1;
            }
        }

        // 归一化到 0-1
        score = score.min(1.0);

        if score > 0.0 {
            reasons.push(format!("关键词匹配: {} 个", matched_keywords.len()));
        }

        ScoreInfo {
            score,
            matched_keywords: matched_keywords.into_iter().take(5).collect(),
            reasons,
        }
    }

    /// 识别能力缺口
    fn identify_gaps(detected_domains: &[String], matched: &[MatchResult]) -> Vec<String> {
        let mut gaps: Vec<String> = Vec::new();

        // 如果检测到需求领域但没有匹配到工作流能力，记录缺口
        for domain in detected_domains {
            let has_workflow =
                matched.iter().any(|m| m.capability_type == "workflow" && m.score >= 0.3);

            if !has_workflow {
                gaps.push(format!("缺少【{}】领域的工作流模板", domain));
            }
        }

        gaps
    }
}

// ── 内部辅助 ──────────────────────────────────────────────────

struct ScoreInfo {
    score: f64,
    matched_keywords: Vec<String>,
    reasons: Vec<String>,
}
