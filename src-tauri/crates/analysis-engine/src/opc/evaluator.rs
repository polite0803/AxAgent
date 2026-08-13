// SPDX-License-Identifier: AGPL-3.0-only

//! 需求价值评估引擎
//!
//! 借鉴 `ai-community-intelligence` 的 Market Gap Detector 和 Traction Scorer 算法，
//! 对采集到的需求线索进行商业价值评估。
//!
//! 评估维度：
//! 1. 痛点强度（Pain Score）：需求描述中的痛点关键词密度
//! 2. 市场缺口（Market Gap）：痛点强度 × 稀缺度（现有方案数的倒数）
//! 3. 商业价值（Commercial Value）：综合评分
//! 4. 需求类型识别（Demand Type）：自动识别需求类别
//! 5. 价格信号（Price Signal）：从描述中提取价格信息

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 需求价值评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandEvaluation {
    pub demand_id: String,
    pub pain_score: f64,
    pub existing_solutions: u32,
    pub market_gap_score: f64,
    pub commercial_value_score: f64,
    pub opportunity_level: String,
    pub confidence: f64,
    pub demand_type: DemandType,
    pub extracted_price_range: Option<PriceRange>,
    pub market_fit_score: f64,
}

/// 需求类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DemandType {
    /// 完全未识别
    Unknown,
    /// 工具/软件类需求
    ToolSoftware,
    /// 内容创作需求
    ContentCreation,
    /// 设计需求
    Design,
    /// 开发需求
    Development,
    /// 运营需求
    Operations,
    /// 营销需求
    Marketing,
    /// 教育/学习需求
    Education,
    /// 企业服务需求
    EnterpriseService,
    /// 外包/兼职需求
    Outsourcing,
    /// 咨询需求
    Consulting,
}

impl DemandType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::ToolSoftware => "tool_software",
            Self::ContentCreation => "content_creation",
            Self::Design => "design",
            Self::Development => "development",
            Self::Operations => "operations",
            Self::Marketing => "marketing",
            Self::Education => "education",
            Self::EnterpriseService => "enterprise_service",
            Self::Outsourcing => "outsourcing",
            Self::Consulting => "consulting",
        }
    }

    /// 获取该类型的平均价格区间（用于估算市场规模）
    pub fn typical_price_range(&self) -> (f64, f64) {
        match self {
            Self::ToolSoftware => (5000.0, 100000.0),
            Self::ContentCreation => (1000.0, 20000.0),
            Self::Design => (2000.0, 30000.0),
            Self::Development => (10000.0, 200000.0),
            Self::Operations => (3000.0, 50000.0),
            Self::Marketing => (5000.0, 100000.0),
            Self::Education => (2000.0, 30000.0),
            Self::EnterpriseService => (50000.0, 500000.0),
            Self::Outsourcing => (5000.0, 50000.0),
            Self::Consulting => (10000.0, 100000.0),
            Self::Unknown => (1000.0, 10000.0),
        }
    }
}

/// 价格区间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceRange {
    pub min: f64,
    pub max: f64,
    pub currency: String,
    pub confidence: f64,
}

impl PriceRange {
    pub fn new(min: f64, max: f64, currency: &str) -> Self {
        Self { min, max, currency: currency.to_string(), confidence: 1.0 }
    }

    pub fn midpoint(&self) -> f64 {
        (self.min + self.max) / 2.0
    }
}

impl DemandEvaluation {
    pub fn opportunity_level(&self) -> &str {
        match self.commercial_value_score {
            v if v >= 80.0 => "very_high",
            v if v >= 60.0 => "high",
            v if v >= 40.0 => "medium",
            _ => "low",
        }
    }
}

/// 需求价值评估配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationConfig {
    /// 痛点分权重（默认 0.30）
    pub pain_weight: f64,
    /// 市场缺口分权重（默认 0.40）
    pub market_gap_weight: f64,
    /// 稀缺度权重（默认 0.30）
    pub scarcity_weight: f64,
    /// 市场契合度权重（默认 0.0，为可选加成）
    pub market_fit_weight: f64,
    /// 最低价值分阈值（用于筛选，默认 50.0）
    pub min_value_threshold: f64,
    /// 是否启用中文痛点分析
    pub enable_chinese_analysis: bool,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            pain_weight: 0.30,
            market_gap_weight: 0.40,
            scarcity_weight: 0.30,
            market_fit_weight: 0.0,
            min_value_threshold: 50.0,
            enable_chinese_analysis: true,
        }
    }
}

/// 评估需求的商业价值
///
/// # 参数
/// - `demand_id`: 需求唯一标识
/// - `title`: 需求标题
/// - `description`: 需求描述
/// - `known_competitors`: 已知竞品数量（可选，None 时自动估算）
/// - `config`: 评估配置（可选，None 时使用默认配置）
///
/// # 返回
/// `DemandEvaluation` 包含多维度评分和机会等级
pub fn evaluate_demand_value(
    demand_id: &str,
    title: &str,
    description: &str,
    known_competitors: Option<u32>,
) -> DemandEvaluation {
    evaluate_demand_with_config(
        demand_id,
        title,
        description,
        known_competitors,
        &EvaluationConfig::default(),
    )
}

/// 使用自定义配置评估需求
pub fn evaluate_demand_with_config(
    demand_id: &str,
    title: &str,
    description: &str,
    known_competitors: Option<u32>,
    config: &EvaluationConfig,
) -> DemandEvaluation {
    // 1. 计算痛点强度（双语）
    let pain_score = calculate_pain_intensity_bilingual(title, description);

    // 2. 估算现有方案数
    let solutions =
        known_competitors.unwrap_or_else(|| estimate_solution_count_bilingual(title, description));

    // 3. 计算市场缺口分
    let market_gap = calculate_market_gap(pain_score, solutions);

    // 4. 识别需求类型
    let demand_type = identify_demand_type(title, description);

    // 5. 提取价格信号
    let extracted_price = extract_price_range(title, description);

    // 6. 计算市场契合度（基于需求类型的典型价格区间）
    let market_fit = calculate_market_fit(&demand_type, &extracted_price);

    // 7. 计算综合商业价值分
    let scarcity_score = (1.0 / solutions.max(1) as f64) * 100.0;
    let commercial_value = (pain_score * config.pain_weight
        + market_gap * config.market_gap_weight
        + scarcity_score * config.scarcity_weight
        + market_fit * config.market_fit_weight)
        .min(100.0);

    // 8. 判断机会等级
    let opportunity_level = match commercial_value {
        v if v >= 80.0 => "very_high",
        v if v >= 60.0 => "high",
        v if v >= 40.0 => "medium",
        _ => "low",
    }
    .to_string();

    // 9. 计算置信度
    let confidence = calculate_confidence(pain_score, solutions);

    DemandEvaluation {
        demand_id: demand_id.to_string(),
        pain_score,
        existing_solutions: solutions,
        market_gap_score: market_gap,
        commercial_value_score: commercial_value,
        opportunity_level,
        confidence,
        demand_type,
        extracted_price_range: extracted_price,
        market_fit_score: market_fit,
    }
}

/// 计算痛点强度（中英文双语关键词）
fn calculate_pain_intensity_bilingual(title: &str, description: &str) -> f64 {
    let en_keywords = [
        "difficult",
        "hard",
        "struggle",
        "problem",
        "issue",
        "frustrating",
        "impossible",
        "lack",
        "missing",
        "can't",
        "need",
        "urgent",
        "critical",
        "blocked",
        "stuck",
        "painful",
        "slow",
        "expensive",
        "complicated",
        "confusing",
        "annoying",
        "inconvenient",
        "time-consuming",
        "error",
        "bug",
        "crash",
        "fail",
        "broken",
        "doesn't work",
        "not working",
    ];

    let zh_keywords = [
        "困难",
        "麻烦",
        "问题",
        "报错",
        "出错",
        "无法",
        "不能",
        "缺少",
        "不足",
        "需要",
        "紧急",
        "关键",
        "阻塞",
        "卡住",
        "痛点",
        "缓慢",
        "昂贵",
        "复杂",
        "混乱",
        "困惑",
        "烦人",
        "不便",
        "耗时",
        "错误",
        "故障",
        "崩溃",
        "失败",
        "损坏",
        "不正常",
        "不工作",
        "怎么",
        "如何",
        "为什么",
        "对比",
        "哪个",
    ];

    let text = format!("{} {}", title, description).to_lowercase();

    let en_count = en_keywords.iter().filter(|kw| text.contains(*kw)).count() as f64;
    let zh_count = zh_keywords.iter().filter(|kw| text.contains(*kw)).count() as f64;

    let total_keywords = (en_keywords.len() + zh_keywords.len()) as f64;
    let total_hits = en_count + zh_count;

    (total_hits / total_keywords * 100.0).min(100.0)
}

/// 估算现有解决方案数量（中英文双语）
fn estimate_solution_count_bilingual(title: &str, description: &str) -> u32 {
    let en_keywords = [
        "solution",
        "tool",
        "software",
        "platform",
        "service",
        "app",
        "library",
        "framework",
        "plugin",
        "module",
        "system",
        "product",
        "feature",
        "implement",
        "integration",
        "api",
        "sdk",
        "template",
        "boilerplate",
        "starter",
    ];

    let zh_keywords = [
        "方案",
        "工具",
        "软件",
        "平台",
        "服务",
        "应用",
        "库",
        "框架",
        "插件",
        "模块",
        "系统",
        "产品",
        "功能",
        "实现",
        "集成",
        "接口",
        "模板",
        "脚手架",
    ];

    let text = format!("{} {}", title, description).to_lowercase();

    let en_count = en_keywords.iter().filter(|k| text.contains(*k)).count() as u32;
    let zh_count = zh_keywords.iter().filter(|k| text.contains(*k)).count() as u32;

    (en_count + zh_count).max(1)
}

/// 计算市场缺口分
fn calculate_market_gap(pain_score: f64, existing_solutions: u32) -> f64 {
    let scarcity = if existing_solutions == 0 {
        1.0
    } else {
        1.0 / existing_solutions as f64
    };

    // pain_score (0-100) * scarcity (0-1) → 0-100
    (pain_score * scarcity).min(100.0)
}

/// 识别需求类型
fn identify_demand_type(title: &str, description: &str) -> DemandType {
    let text = format!("{} {}", title, description).to_lowercase();

    let type_patterns: Vec<(DemandType, Vec<&str>)> = vec![
        (
            DemandType::Development,
            vec![
                "develop",
                "开发",
                "code",
                "编程",
                "programming",
                "api",
                "后端",
                "frontend",
                "前端",
                "backend",
                "全栈",
                "fullstack",
                "web",
                "网站",
                "app",
                "应用",
                "mobile",
                "ios",
                "android",
                "小程序",
            ],
        ),
        (
            DemandType::Design,
            vec![
                "design",
                "设计",
                "ui",
                "ux",
                "logo",
                "品牌",
                "视觉",
                "graphic",
                "illustration",
                "插画",
                "banner",
                "海报",
                "包装",
                "packaging",
            ],
        ),
        (
            DemandType::ToolSoftware,
            vec![
                "tool",
                "工具",
                "software",
                "软件",
                "saas",
                "dashboard",
                "仪表盘",
                "系统",
                "system",
                "platform",
                "平台",
                "crm",
                "erp",
            ],
        ),
        (
            DemandType::ContentCreation,
            vec![
                "content",
                "内容",
                "article",
                "文章",
                "blog",
                "writing",
                "写作",
                "video",
                "视频",
                "podcast",
                "audio",
                "音频",
                "翻译",
                "translate",
            ],
        ),
        (
            DemandType::Marketing,
            vec![
                "marketing",
                "营销",
                "seo",
                "sem",
                "广告",
                "social",
                "媒体",
                "推广",
                "promotion",
                "lead",
                "客户",
                "funnel",
                "漏斗",
            ],
        ),
        (
            DemandType::EnterpriseService,
            vec![
                "enterprise",
                "企业",
                "company",
                "公司",
                "b2b",
                "business",
                "商业",
                "crm",
                "erp",
                "consulting",
                "咨询",
                "strategy",
                "战略",
            ],
        ),
        (
            DemandType::Outsourcing,
            vec![
                "outsource",
                "外包",
                "freelance",
                "自由职业",
                "contract",
                "合同",
                "part-time",
                "兼职",
                "project",
                "项目",
                "deliver",
                "交付",
            ],
        ),
        (
            DemandType::Education,
            vec![
                "course",
                "课程",
                "training",
                "培训",
                "learn",
                "学习",
                "teach",
                "教学",
                "tutorial",
                "教程",
                "education",
                "教育",
                "school",
                "学校",
            ],
        ),
        (
            DemandType::Operations,
            vec![
                "operation",
                "运营",
                "manage",
                "管理",
                "admin",
                "行政",
                "support",
                "支持",
                "service",
                "service",
                "客服",
                "crm",
            ],
        ),
        (
            DemandType::Consulting,
            vec![
                "consult", "咨询", "advice", "建议", "strategy", "策略", "analysis", "分析",
                "expert", "专家", "advisor", "顾问",
            ],
        ),
    ];

    let mut type_scores: HashMap<DemandType, f64> = HashMap::new();

    for (demand_type, patterns) in &type_patterns {
        let score = patterns.iter().filter(|p| text.contains(*p)).count() as f64;
        if score > 0.0 {
            type_scores.insert(demand_type.clone(), score);
        }
    }

    type_scores
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(t, _)| t)
        .unwrap_or(DemandType::Unknown)
}

/// 从文本中提取价格区间
fn extract_price_range(title: &str, description: &str) -> Option<PriceRange> {
    let text = format!("{} {}", title, description);

    // 尝试匹配中文价格格式
    let cn_patterns = vec![
        // "xxx-yyy元" 或 "xxx元-yyy元"
        r"(\d+)\s*-?\s*(\d+)?\s*元",
        // "¥xxx-yyy"
        r"[¥￥]\s*(\d+)\s*-?\s*(\d+)?",
        // "xxx-yyy RMB"
        r"(\d+)\s*-?\s*(\d+)?\s*(?:RMB|CNY|人民币)",
    ];

    // 尝试匹配英文价格格式
    let en_patterns = vec![
        // "$xxx-yyy" or "xxx-yyy dollars"
        r"\$\s*(\d+[\d,]*)\s*-?\s*(\d+[\d,]*)?",
        // "xxx-yyy USD"
        r"(\d+[\d,]*)\s*-?\s*(\d+[\d,]*)\s*(?:USD|dollars?)",
        // "budget: xxx-yyy"
        r"(?:budget|price|cost)\s*[:：]?\s*\$?\s*(\d+[\d,]*)\s*-?\s*(\d+[\d,]*)?",
    ];

    // 先尝试中文模式
    for pattern in &cn_patterns {
        if let Some(range) = try_parse_price(&text, pattern, "CNY") {
            return Some(range);
        }
    }

    // 再尝试英文模式
    for pattern in &en_patterns {
        if let Some(range) = try_parse_price(&text, pattern, "USD") {
            return Some(range);
        }
    }

    None
}

fn try_parse_price(text: &str, pattern: &str, currency: &str) -> Option<PriceRange> {
    let regex = regex::Regex::new(pattern).ok()?;
    let caps = regex.captures(text)?;

    let min_str = caps.get(1)?.as_str().replace(',', "");
    let min = min_str.parse::<f64>().ok()?;

    let max = caps
        .get(2)
        .and_then(|m| m.as_str().replace(',', "").parse::<f64>().ok())
        .unwrap_or(min * 1.5); // 如果没有上限，假设是下限的1.5倍

    if min <= 0.0 {
        return None;
    }

    Some(PriceRange::new(min, max, currency))
}

/// 计算市场契合度
fn calculate_market_fit(demand_type: &DemandType, price: &Option<PriceRange>) -> f64 {
    // 如果没有价格信息，给中性分
    let price = match price {
        Some(p) => p,
        None => return 50.0,
    };

    let (typical_min, typical_max) = demand_type.typical_price_range();
    let price_mid = price.midpoint();

    // 如果价格在典型区间内，给高分
    if price_mid >= typical_min && price_mid <= typical_max {
        return 80.0;
    }

    // 如果价格偏低，可能是好机会（蓝海市场）
    if price_mid < typical_min * 0.5 {
        return 70.0;
    }

    // 如果价格偏高，可能是高端市场
    if price_mid > typical_max * 2.0 {
        return 60.0;
    }

    // 其他情况给中性分
    50.0
}

/// 计算置信度
fn calculate_confidence(pain_score: f64, solutions: u32) -> f64 {
    let mut confidence: f64 = 0.5;

    if pain_score > 50.0 {
        confidence += 0.15;
    }
    if pain_score > 70.0 {
        confidence += 0.1;
    }

    if solutions <= 2 {
        confidence += 0.15;
    } else if solutions <= 5 {
        confidence += 0.05;
    }

    if solutions > 10 {
        confidence -= 0.2;
    }
    if solutions > 20 {
        confidence -= 0.1;
    }

    confidence.clamp(0.0, 1.0)
}

/// 批量评估需求
pub fn batch_evaluate(demands: &[(String, String, String)]) -> Vec<DemandEvaluation> {
    demands.iter().map(|(id, title, desc)| evaluate_demand_value(id, title, desc, None)).collect()
}

/// 使用配置批量评估需求
pub fn batch_evaluate_with_config(
    demands: &[(String, String, String)],
    config: &EvaluationConfig,
) -> Vec<DemandEvaluation> {
    demands
        .iter()
        .map(|(id, title, desc)| evaluate_demand_with_config(id, title, desc, None, config))
        .collect()
}

/// 筛选高价值需求
pub fn filter_high_value(
    evaluations: &[DemandEvaluation],
    min_score: f64,
) -> Vec<&DemandEvaluation> {
    evaluations.iter().filter(|e| e.commercial_value_score >= min_score).collect()
}

/// 按价值分排序需求
pub fn sort_by_value(evaluations: &mut [DemandEvaluation]) {
    evaluations.sort_by(|a, b| {
        b.commercial_value_score
            .partial_cmp(&a.commercial_value_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// 按需求类型分组
pub fn group_by_type(evaluations: &[DemandEvaluation]) -> HashMap<String, Vec<&DemandEvaluation>> {
    let mut groups: HashMap<String, Vec<&DemandEvaluation>> = HashMap::new();

    for eval in evaluations {
        let key = eval.demand_type.as_str().to_string();
        groups.entry(key).or_default().push(eval);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_demand_value_basic() {
        let result = evaluate_demand_value(
            "test-1",
            "Need a tool for managing invoices",
            "Current process is very slow and error-prone. It takes 3 hours per week.",
            None,
        );

        assert_eq!(result.demand_id, "test-1");
        assert!(result.pain_score >= 0.0 && result.pain_score <= 100.0);
        assert!(result.market_gap_score >= 0.0 && result.market_gap_score <= 100.0);
        assert!(result.commercial_value_score >= 0.0 && result.commercial_value_score <= 100.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_evaluate_chinese_demand() {
        let result = evaluate_demand_value(
            "cn-1",
            "需要一个发票管理工具",
            "当前流程非常繁琐，容易出错，每周要花3小时处理发票。",
            None,
        );

        assert_eq!(result.demand_id, "cn-1");
        assert!(result.pain_score >= 0.0, "中文痛点分析应生效");
        assert!(result.commercial_value_score >= 0.0);
    }

    #[test]
    fn test_evaluate_high_pain() {
        let result = evaluate_demand_value(
            "high-pain",
            "URGENT: System crashes every day, critical business impacted",
            "This is extremely frustrating and painful. We lose money every hour. Impossible to work with. The system is broken, has errors, blocked and stuck. Very slow and expensive. The bug causes critical failures. We need urgent help because the problem is critical and difficult.",
            None,
        );

        assert!(result.pain_score > 20.0, "高痛点需求应有较高的痛点分");
    }

    #[test]
    fn test_evaluate_low_pain() {
        let result = evaluate_demand_value(
            "low-pain",
            "Nice to have a dark mode",
            "It would be cool to have dark mode, but not critical.",
            None,
        );

        assert!(result.pain_score < 30.0, "低痛点需求应有较低的痛点分");
    }

    #[test]
    fn test_evaluate_with_known_competitors() {
        let result = evaluate_demand_value(
            "known-competitors",
            "Build a project management tool",
            "Need a new PM tool",
            Some(10),
        );

        assert_eq!(result.existing_solutions, 10);
        assert!(result.market_gap_score < 50.0, "竞争激烈时市场缺口应较低");
    }

    #[test]
    fn test_demand_type_identification() {
        let dev_result = evaluate_demand_value(
            "dev-1",
            "需要一个Web应用开发",
            "开发一个React前端应用，需要API后端支持",
            None,
        );
        assert_eq!(dev_result.demand_type, DemandType::Development);

        let design_result = evaluate_demand_value(
            "design-1",
            "Logo设计需求",
            "需要一个专业的UI设计师来完成品牌视觉设计",
            None,
        );
        assert_eq!(design_result.demand_type, DemandType::Design);
    }

    #[test]
    fn test_price_extraction() {
        let result = evaluate_demand_value(
            "price-1",
            "企业官网建设",
            "预算8000-15000元，需要5-8个页面",
            None,
        );

        assert!(result.extracted_price_range.is_some(), "应提取到价格区间");
        let price = result.extracted_price_range.unwrap();
        assert!(price.min >= 8000.0, "最低价应大于等于8000");
        assert!(price.max >= 15000.0, "最高价应大于等于15000");
    }

    #[test]
    fn test_price_extraction_usd() {
        let result = evaluate_demand_value(
            "price-usd",
            "Need a dashboard",
            "Budget: $5000-10000 for a SaaS dashboard",
            None,
        );

        assert!(result.extracted_price_range.is_some(), "应提取到USD价格");
    }

    #[test]
    fn test_opportunity_levels() {
        let evaluation = DemandEvaluation {
            demand_id: "test".to_string(),
            pain_score: 80.0,
            existing_solutions: 1,
            market_gap_score: 90.0,
            commercial_value_score: 85.0,
            opportunity_level: "very_high".to_string(),
            confidence: 0.9,
            demand_type: DemandType::ToolSoftware,
            extracted_price_range: None,
            market_fit_score: 80.0,
        };

        assert_eq!(evaluation.opportunity_level(), "very_high");

        let evaluation = DemandEvaluation { commercial_value_score: 65.0, ..evaluation.clone() };
        assert_eq!(evaluation.opportunity_level(), "high");

        let evaluation = DemandEvaluation { commercial_value_score: 45.0, ..evaluation.clone() };
        assert_eq!(evaluation.opportunity_level(), "medium");

        let evaluation = DemandEvaluation { commercial_value_score: 20.0, ..evaluation.clone() };
        assert_eq!(evaluation.opportunity_level(), "low");
    }

    #[test]
    fn test_batch_evaluate() {
        let demands = vec![
            ("id-1".to_string(), "Test demand 1".to_string(), "Description 1".to_string()),
            ("id-2".to_string(), "Test demand 2".to_string(), "Description 2".to_string()),
        ];

        let results = batch_evaluate(&demands);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].demand_id, "id-1");
        assert_eq!(results[1].demand_id, "id-2");
    }

    #[test]
    fn test_filter_high_value() {
        let evaluations = vec![
            DemandEvaluation {
                demand_id: "low".to_string(),
                commercial_value_score: 30.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "high".to_string(),
                commercial_value_score: 70.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "medium".to_string(),
                commercial_value_score: 55.0,
                ..Default::default()
            },
        ];

        let high_value = filter_high_value(&evaluations, 50.0);
        assert_eq!(high_value.len(), 2);
        assert_eq!(high_value[0].demand_id, "high");
        assert_eq!(high_value[1].demand_id, "medium");
    }

    #[test]
    fn test_sort_by_value() {
        let mut evaluations = vec![
            DemandEvaluation {
                demand_id: "low".to_string(),
                commercial_value_score: 30.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "high".to_string(),
                commercial_value_score: 90.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "medium".to_string(),
                commercial_value_score: 60.0,
                ..Default::default()
            },
        ];

        sort_by_value(&mut evaluations);
        assert_eq!(evaluations[0].demand_id, "high");
        assert_eq!(evaluations[1].demand_id, "medium");
        assert_eq!(evaluations[2].demand_id, "low");
    }

    #[test]
    fn test_group_by_type() {
        let evaluations = vec![
            DemandEvaluation {
                demand_id: "dev-1".to_string(),
                demand_type: DemandType::Development,
                commercial_value_score: 70.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "design-1".to_string(),
                demand_type: DemandType::Design,
                commercial_value_score: 80.0,
                ..Default::default()
            },
            DemandEvaluation {
                demand_id: "dev-2".to_string(),
                demand_type: DemandType::Development,
                commercial_value_score: 60.0,
                ..Default::default()
            },
        ];

        let groups = group_by_type(&evaluations);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("development").unwrap().len(), 2);
        assert_eq!(groups.get("design").unwrap().len(), 1);
    }

    #[test]
    fn test_calculate_pain_intensity_empty() {
        let score = calculate_pain_intensity_bilingual("", "");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_calculate_market_gap_zero_solutions() {
        let gap = calculate_market_gap(80.0, 0);
        assert!(gap >= 80.0, "无竞争时市场缺口应较大");
    }

    #[test]
    fn test_calculate_market_gap_many_solutions() {
        let gap = calculate_market_gap(80.0, 20);
        assert!(gap < 10.0, "竞争激烈时市场缺口应很小");
    }

    #[test]
    fn test_confidence_range() {
        for pain in [0.0, 30.0, 50.0, 80.0, 100.0] {
            for solutions in [0, 1, 5, 10, 20, 50] {
                let conf = calculate_confidence(pain, solutions);
                assert!(
                    (0.0..=1.0).contains(&conf),
                    "置信度 {} 超出范围 (pain={}, solutions={})",
                    conf,
                    pain,
                    solutions
                );
            }
        }
    }

    #[test]
    fn test_evaluation_config_weights() {
        let config = EvaluationConfig {
            pain_weight: 0.5,
            market_gap_weight: 0.3,
            scarcity_weight: 0.2,
            ..Default::default()
        };

        let result = evaluate_demand_with_config(
            "config-test",
            "High pain demand",
            "Very urgent and difficult problem",
            None,
            &config,
        );

        assert_eq!(result.demand_id, "config-test");
        assert!(result.commercial_value_score >= 0.0);
    }

    impl Default for DemandEvaluation {
        fn default() -> Self {
            Self {
                demand_id: String::new(),
                pain_score: 0.0,
                existing_solutions: 0,
                market_gap_score: 0.0,
                commercial_value_score: 0.0,
                opportunity_level: "low".to_string(),
                confidence: 0.0,
                demand_type: DemandType::Unknown,
                extracted_price_range: None,
                market_fit_score: 0.0,
            }
        }
    }

    #[test]
    fn test_demand_type_as_str() {
        assert_eq!(DemandType::Unknown.as_str(), "unknown");
        assert_eq!(DemandType::ToolSoftware.as_str(), "tool_software");
        assert_eq!(DemandType::ContentCreation.as_str(), "content_creation");
        assert_eq!(DemandType::Design.as_str(), "design");
        assert_eq!(DemandType::Development.as_str(), "development");
        assert_eq!(DemandType::Operations.as_str(), "operations");
        assert_eq!(DemandType::Marketing.as_str(), "marketing");
        assert_eq!(DemandType::Education.as_str(), "education");
        assert_eq!(DemandType::EnterpriseService.as_str(), "enterprise_service");
        assert_eq!(DemandType::Outsourcing.as_str(), "outsourcing");
        assert_eq!(DemandType::Consulting.as_str(), "consulting");
    }

    #[test]
    fn test_demand_type_typical_price_range() {
        let (min, max) = DemandType::ToolSoftware.typical_price_range();
        assert!(min > 0.0);
        assert!(max > min);

        let (min, max) = DemandType::Unknown.typical_price_range();
        assert_eq!(min, 1000.0);
        assert_eq!(max, 10000.0);
    }

    #[test]
    fn test_price_extraction_no_price() {
        let result = evaluate_demand_value(
            "no-price",
            "简单需求",
            "这是一个没有任何价格信息的需求描述",
            None,
        );
        assert!(result.extracted_price_range.is_none());
    }

    #[test]
    fn test_evaluation_consistency() {
        // evaluate_demand_value 应等价于 evaluate_demand_with_config + 默认配置
        let result1 = evaluate_demand_value("consistency", "测试需求", "测试描述", None);
        let result2 = evaluate_demand_with_config(
            "consistency",
            "测试需求",
            "测试描述",
            None,
            &EvaluationConfig::default(),
        );

        assert_eq!(result1.pain_score, result2.pain_score);
        assert_eq!(result1.market_gap_score, result2.market_gap_score);
        assert_eq!(result1.commercial_value_score, result2.commercial_value_score);
        assert_eq!(result1.demand_type, result2.demand_type);
    }

    #[test]
    fn test_batch_evaluate_empty() {
        let results = batch_evaluate(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_high_value_empty() {
        let high_value = filter_high_value(&[], 50.0);
        assert!(high_value.is_empty());
    }

    #[test]
    fn test_sort_by_value_empty() {
        let mut evaluations: Vec<DemandEvaluation> = vec![];
        sort_by_value(&mut evaluations);
        assert!(evaluations.is_empty());
    }

    #[test]
    fn test_group_by_type_empty() {
        let groups = group_by_type(&[]);
        assert!(groups.is_empty());
    }
}
