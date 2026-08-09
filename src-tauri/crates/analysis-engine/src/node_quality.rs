// SPDX-License-Identifier: AGPL-3.0-only

//! 通用节点质量检测规则系统
//!
//! 为不同类型的节点提供统一的质量检测接口：
//! - analyst: 分析师节点（对偶评分结构）
//! - debate: 辩论节点（立场+强度结构）
//! - decision: 决策节点（决策+风险评估结构）
//! - tool: 工具/算法节点（准确性+完整性结构）
//! - valuation: 估值节点（估值区间+合理性结构）
//! - risk: 风险节点（风险指标+阈值合规结构）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点类型枚举（与 entities 保持一致）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Analyst,
    Debate,
    Decision,
    Tool,
    Valuation,
    Risk,
    Other,
}

impl NodeType {
    /// 获取节点类型的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            NodeType::Analyst => "analyst",
            NodeType::Debate => "debate",
            NodeType::Decision => "decision",
            NodeType::Tool => "tool",
            NodeType::Valuation => "valuation",
            NodeType::Risk => "risk",
            NodeType::Other => "other",
        }
    }
}

/// 单条质量检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheck {
    /// 检查类别
    pub category: String,
    /// 检查字段
    pub field: String,
    /// 状态 (pass/warning/issue)
    pub status: String,
    /// 详细说明
    pub detail: String,
}

/// 节点质量检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeQualityResult {
    /// 节点类型
    pub node_type: NodeType,
    /// 节点 ID
    pub node_id: String,
    /// 质量评分 (0-100)
    pub quality_score: i32,
    /// 评分等级
    pub grade: String,
    /// 问题数量
    pub issue_count: i32,
    /// 警告数量
    pub warning_count: i32,
    /// 良好项数量
    pub good_count: i32,
    /// 检查详情
    pub checks: Vec<QualityCheck>,
    /// 通用质量指标 (JSON)
    pub quality_metrics: serde_json::Value,
}

/// 获取节点类型的判断规则
pub fn detect_node_type(node_id: &str) -> NodeType {
    if node_id.starts_with("a-") {
        NodeType::Analyst
    } else if node_id.starts_with("bull-") || node_id.starts_with("bear-") {
        NodeType::Debate
    } else if node_id.starts_with("t-") || node_id.starts_with("u-") {
        NodeType::Tool
    } else if node_id.contains("decision") || node_id.contains("manager") {
        NodeType::Decision
    } else if node_id.contains("valuation") {
        NodeType::Valuation
    } else if node_id.contains("risk") {
        NodeType::Risk
    } else {
        NodeType::Other
    }
}

/// 计算质量等级
fn calc_grade(score: i32) -> String {
    if score >= 90 {
        "A".to_string()
    } else if score >= 80 {
        "B".to_string()
    } else if score >= 70 {
        "C".to_string()
    } else if score >= 60 {
        "D".to_string()
    } else {
        "F".to_string()
    }
}

/// ── 分析师节点质量检测 ──
///
/// 分析师特有的质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalystQualityMetrics {
    pub bull_score: Option<f64>,
    pub bear_score: Option<f64>,
    pub confidence: Option<f64>,
    pub score_consistent: bool,
    pub direction_consistent: bool,
}

pub fn check_analyst_quality(node_id: &str, parsed: &serde_json::Value) -> NodeQualityResult {
    let mut checks: Vec<QualityCheck> = Vec::new();
    let mut score: i32 = 100;
    let mut issue_count: i32 = 0;
    let mut warning_count: i32 = 0;
    let mut good_count: i32 = 0;

    // 提取字段
    let bull_score = parsed.get("bull_score").and_then(|v| v.as_f64());
    let bear_score = parsed.get("bear_score").and_then(|v| v.as_f64());
    let confidence = parsed.get("confidence").and_then(|v| v.as_f64());
    let verdict = parsed.get("verdict").and_then(|v| v.as_str()).unwrap_or("");

    // 1. 评分自洽性检查
    let score_consistent = match (bull_score, bear_score) {
        (Some(bs), Some(bes)) => {
            let sum = bs + bes;
            if (sum - 100.0).abs() > 10.0 {
                checks.push(QualityCheck {
                    category: "consistency".into(),
                    field: "score_consistency".into(),
                    status: "issue".into(),
                    detail: format!(
                        "评分不自洽: bull_score({}) + bear_score({}) = {} (应约为 100)",
                        bs, bes, sum
                    ),
                });
                score -= 20;
                issue_count += 1;
                false
            } else {
                checks.push(QualityCheck {
                    category: "consistency".into(),
                    field: "score_consistency".into(),
                    status: "pass".into(),
                    detail: format!("评分自洽: bull + bear = {:.0}", sum),
                });
                good_count += 1;
                true
            }
        },
        _ => {
            checks.push(QualityCheck {
                category: "completeness".into(),
                field: "score_fields".into(),
                status: "warning".into(),
                detail: "缺少 bull_score 或 bear_score 字段".into(),
            });
            score -= 10;
            warning_count += 1;
            true
        },
    };

    // 2. 方向一致性检查
    let direction_consistent = match (bull_score, bear_score) {
        (Some(bs), Some(bes)) if !verdict.is_empty() => {
            let is_bullish =
                verdict.contains("看多") || verdict.contains("看涨") || verdict.contains("偏多");
            let is_bearish =
                verdict.contains("看空") || verdict.contains("看跌") || verdict.contains("偏空");

            if is_bullish && bs <= bes {
                checks.push(QualityCheck {
                    category: "consistency".into(),
                    field: "direction_consistency".into(),
                    status: "issue".into(),
                    detail: format!(
                        "逻辑矛盾: verdict='{}' 但 bull_score({}) <= bear_score({})",
                        verdict, bs, bes
                    ),
                });
                score -= 15;
                issue_count += 1;
                false
            } else if is_bearish && bes <= bs {
                checks.push(QualityCheck {
                    category: "consistency".into(),
                    field: "direction_consistency".into(),
                    status: "issue".into(),
                    detail: format!(
                        "逻辑矛盾: verdict='{}' 但 bear_score({}) <= bull_score({})",
                        verdict, bes, bs
                    ),
                });
                score -= 15;
                issue_count += 1;
                false
            } else {
                checks.push(QualityCheck {
                    category: "consistency".into(),
                    field: "direction_consistency".into(),
                    status: "pass".into(),
                    detail: format!("方向一致: verdict='{}' 与评分匹配", verdict),
                });
                good_count += 1;
                true
            }
        },
        _ => true,
    };

    // 3. 置信度检查
    if let Some(conf) = confidence {
        if conf > 100.0 {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "issue".into(),
                detail: format!("置信度越界: {} (超出 0-100)", conf),
            });
            score -= 10;
            issue_count += 1;
        } else if conf < 10.0 {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "warning".into(),
                detail: format!("置信度极低: {}", conf),
            });
            score -= 5;
            warning_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "pass".into(),
                detail: format!("置信度正常: {}", conf),
            });
            good_count += 1;
        }
    }

    // 4. verdict 字段完整性
    if verdict.is_empty() {
        checks.push(QualityCheck {
            category: "completeness".into(),
            field: "verdict".into(),
            status: "warning".into(),
            detail: "缺少 verdict 字段".into(),
        });
        score -= 10;
        warning_count += 1;
    }

    // 构建质量指标
    let metrics = AnalystQualityMetrics {
        bull_score,
        bear_score,
        confidence,
        score_consistent,
        direction_consistent,
    };

    NodeQualityResult {
        node_type: NodeType::Analyst,
        node_id: node_id.to_string(),
        quality_score: score.clamp(0, 100),
        grade: calc_grade(score.clamp(0, 100)),
        issue_count,
        warning_count,
        good_count,
        checks,
        quality_metrics: serde_json::to_value(&metrics).unwrap_or_default(),
    }
}

/// ── 辩论节点质量检测 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateQualityMetrics {
    pub stance: Option<String>,
    pub strength_score: Option<f64>,
    pub confidence: Option<f64>,
    pub logic_consistent: bool,
}

pub fn check_debate_quality(node_id: &str, parsed: &serde_json::Value) -> NodeQualityResult {
    let mut checks: Vec<QualityCheck> = Vec::new();
    let mut score: i32 = 100;
    let mut issue_count: i32 = 0;
    let mut warning_count: i32 = 0;
    let mut good_count: i32 = 0;

    let stance = parsed.get("stance").and_then(|v| v.as_str());
    let strength = parsed.get("strength_score").and_then(|v| v.as_f64());
    let confidence = parsed.get("confidence").and_then(|v| v.as_f64());

    // 1. stance 字段检查
    if let Some(s) = stance {
        let valid_stances = ["bullish", "bearish", "neutral", "看涨", "看跌", "中性"];
        if !valid_stances.iter().any(|v| s.contains(v)) {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "stance".into(),
                status: "warning".into(),
                detail: format!("stance 值异常: '{}'", s),
            });
            score -= 10;
            warning_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "stance".into(),
                status: "pass".into(),
                detail: format!("stance 有效: '{}'", s),
            });
            good_count += 1;
        }
    } else {
        checks.push(QualityCheck {
            category: "completeness".into(),
            field: "stance".into(),
            status: "issue".into(),
            detail: "缺少 stance 字段".into(),
        });
        score -= 20;
        issue_count += 1;
    }

    // 2. strength_score 范围检查
    if let Some(s) = strength {
        if !(0.0..=100.0).contains(&s) {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "strength_score".into(),
                status: "issue".into(),
                detail: format!("strength_score 越界: {} (应为 0-100)", s),
            });
            score -= 15;
            issue_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "strength_score".into(),
                status: "pass".into(),
                detail: format!("strength_score 正常: {}", s),
            });
            good_count += 1;
        }
    }

    // 3. confidence 检查
    if let Some(conf) = confidence {
        if conf > 100.0 {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "issue".into(),
                detail: format!("置信度越界: {}", conf),
            });
            score -= 10;
            issue_count += 1;
        } else if conf < 20.0 {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "warning".into(),
                detail: format!("置信度过低: {}", conf),
            });
            score -= 5;
            warning_count += 1;
        }
    }

    let metrics = DebateQualityMetrics {
        stance: stance.map(|s| s.to_string()),
        strength_score: strength,
        confidence,
        logic_consistent: issue_count == 0,
    };

    NodeQualityResult {
        node_type: NodeType::Debate,
        node_id: node_id.to_string(),
        quality_score: score.clamp(0, 100),
        grade: calc_grade(score.clamp(0, 100)),
        issue_count,
        warning_count,
        good_count,
        checks,
        quality_metrics: serde_json::to_value(&metrics).unwrap_or_default(),
    }
}

/// ── 决策节点质量检测 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionQualityMetrics {
    pub action: Option<String>,
    pub confidence: Option<f64>,
    pub risk_assessed: bool,
    pub criteria_met: bool,
}

pub fn check_decision_quality(node_id: &str, parsed: &serde_json::Value) -> NodeQualityResult {
    let mut checks: Vec<QualityCheck> = Vec::new();
    let mut score: i32 = 100;
    let mut issue_count: i32 = 0;
    let mut warning_count: i32 = 0;
    let mut good_count: i32 = 0;

    let action = parsed.get("action").and_then(|v| v.as_str());
    let confidence = parsed.get("confidence").and_then(|v| v.as_f64());
    let risk_assessed = parsed.get("risk_assessed").and_then(|v| v.as_bool()).unwrap_or(false);

    // 1. action 检查
    if let Some(a) = action {
        let valid_actions = ["buy", "sell", "hold", "add", "reduce", "买入", "卖出", "持有"];
        if !valid_actions.iter().any(|v| a.contains(v)) {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "action".into(),
                status: "warning".into(),
                detail: format!("action 值异常: '{}'", a),
            });
            score -= 10;
            warning_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "action".into(),
                status: "pass".into(),
                detail: format!("action 有效: '{}'", a),
            });
            good_count += 1;
        }
    } else {
        checks.push(QualityCheck {
            category: "completeness".into(),
            field: "action".into(),
            status: "issue".into(),
            detail: "缺少 action 字段".into(),
        });
        score -= 25;
        issue_count += 1;
    }

    // 2. 风险评估检查
    if !risk_assessed {
        checks.push(QualityCheck {
            category: "risk".into(),
            field: "risk_assessed".into(),
            status: "warning".into(),
            detail: "未进行风险评估".into(),
        });
        score -= 15;
        warning_count += 1;
    } else {
        checks.push(QualityCheck {
            category: "risk".into(),
            field: "risk_assessed".into(),
            status: "pass".into(),
            detail: "已完成风险评估".into(),
        });
        good_count += 1;
    }

    // 3. confidence 检查
    if let Some(conf) = confidence {
        if !(0.0..=100.0).contains(&conf) {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "issue".into(),
                detail: format!("置信度越界: {}", conf),
            });
            score -= 10;
            issue_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "range".into(),
                field: "confidence".into(),
                status: "pass".into(),
                detail: format!("置信度正常: {}", conf),
            });
            good_count += 1;
        }
    }

    let metrics = DecisionQualityMetrics {
        action: action.map(|a| a.to_string()),
        confidence,
        risk_assessed,
        criteria_met: issue_count == 0 && warning_count == 0,
    };

    NodeQualityResult {
        node_type: NodeType::Decision,
        node_id: node_id.to_string(),
        quality_score: score.clamp(0, 100),
        grade: calc_grade(score.clamp(0, 100)),
        issue_count,
        warning_count,
        good_count,
        checks,
        quality_metrics: serde_json::to_value(&metrics).unwrap_or_default(),
    }
}

/// ── 工具/算法节点质量检测 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolQualityMetrics {
    pub accuracy: Option<f64>,
    pub completeness: bool,
    pub credibility: Option<String>,
    pub data_freshness: Option<String>,
}

pub fn check_tool_quality(node_id: &str, parsed: &serde_json::Value) -> NodeQualityResult {
    let mut checks: Vec<QualityCheck> = Vec::new();
    let mut score: i32 = 100;
    let mut issue_count: i32 = 0;
    let mut warning_count: i32 = 0;
    let mut good_count: i32 = 0;

    let credibility = parsed.get("credibility").and_then(|v| v.as_str());
    let data_freshness = parsed.get("data_freshness").and_then(|v| v.as_str());
    let warnings = parsed.get("warnings").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0);

    // 1. credibility 检查
    if let Some(c) = credibility {
        let valid_levels = ["high", "medium", "low", "delayed", "stale"];
        if !valid_levels.contains(&c) {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "credibility".into(),
                status: "warning".into(),
                detail: format!("credibility 值异常: '{}'", c),
            });
            score -= 10;
            warning_count += 1;
        } else if c == "low" || c == "delayed" || c == "stale" {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "credibility".into(),
                status: "warning".into(),
                detail: format!("数据可信度低: '{}'", c),
            });
            score -= 20;
            warning_count += 1;
        } else {
            checks.push(QualityCheck {
                category: "validity".into(),
                field: "credibility".into(),
                status: "pass".into(),
                detail: format!("数据可信度高: '{}'", c),
            });
            good_count += 1;
        }
    }

    // 2. warnings 检查
    if warnings > 0 {
        checks.push(QualityCheck {
            category: "warnings".into(),
            field: "warnings".into(),
            status: if warnings > 3 { "issue" } else { "warning" }.into(),
            detail: format!("存在 {} 个警告", warnings),
        });
        score -= ((warnings * 5) as i32).min(30);
        if warnings > 3 {
            issue_count += 1;
        } else {
            warning_count += 1;
        }
    }

    // 3. data_freshness 检查
    if let Some(df) = data_freshness {
        checks.push(QualityCheck {
            category: "freshness".into(),
            field: "data_freshness".into(),
            status: "pass".into(),
            detail: format!("数据时效性: '{}'", df),
        });
        good_count += 1;
    }

    let completeness = parsed.as_object().map(|m| !m.is_empty()).unwrap_or(false);

    let metrics = ToolQualityMetrics {
        accuracy: Some(score as f64 / 100.0),
        completeness,
        credibility: credibility.map(|c| c.to_string()),
        data_freshness: data_freshness.map(|d| d.to_string()),
    };

    NodeQualityResult {
        node_type: NodeType::Tool,
        node_id: node_id.to_string(),
        quality_score: score.clamp(0, 100),
        grade: calc_grade(score.clamp(0, 100)),
        issue_count,
        warning_count,
        good_count,
        checks,
        quality_metrics: serde_json::to_value(&metrics).unwrap_or_default(),
    }
}

/// ── 通用质量检测入口 ──
pub fn check_node_quality(
    node_id: &str,
    node_type: &NodeType,
    parsed: &serde_json::Value,
) -> NodeQualityResult {
    match node_type {
        NodeType::Analyst => check_analyst_quality(node_id, parsed),
        NodeType::Debate => check_debate_quality(node_id, parsed),
        NodeType::Decision => check_decision_quality(node_id, parsed),
        NodeType::Tool | NodeType::Valuation | NodeType::Risk => {
            check_tool_quality(node_id, parsed)
        },
        NodeType::Other => {
            // 通用检测
            let checks: Vec<QualityCheck> = vec![QualityCheck {
                category: "generic".into(),
                field: "output_format".into(),
                status: "pass".into(),
                detail: "输出格式有效".into(),
            }];
            NodeQualityResult {
                node_type: NodeType::Other,
                node_id: node_id.to_string(),
                quality_score: 100,
                grade: "A".to_string(),
                issue_count: 0,
                warning_count: 0,
                good_count: 1,
                checks,
                quality_metrics: parsed.clone(),
            }
        },
    }
}

/// 计算节点类型的一致性指标（用于进化引擎）
pub fn calc_consistency_metrics(
    node_type: &NodeType,
    metrics: &serde_json::Value,
) -> HashMap<String, f64> {
    let mut result = HashMap::new();

    match node_type {
        NodeType::Analyst => {
            if let Some(m) = metrics.as_object() {
                let score_consistent =
                    m.get("score_consistent").and_then(|v| v.as_bool()).unwrap_or(true);
                let direction_consistent =
                    m.get("direction_consistent").and_then(|v| v.as_bool()).unwrap_or(true);
                result.insert(
                    "score_consistency_rate".into(),
                    if score_consistent { 1.0 } else { 0.0 },
                );
                result.insert(
                    "direction_consistency_rate".into(),
                    if direction_consistent { 1.0 } else { 0.0 },
                );
            }
        },
        NodeType::Debate => {
            if let Some(m) = metrics.as_object() {
                let logic_consistent =
                    m.get("logic_consistent").and_then(|v| v.as_bool()).unwrap_or(true);
                result.insert(
                    "logic_consistency_rate".into(),
                    if logic_consistent { 1.0 } else { 0.0 },
                );
            }
        },
        NodeType::Decision => {
            if let Some(m) = metrics.as_object() {
                let criteria_met = m.get("criteria_met").and_then(|v| v.as_bool()).unwrap_or(true);
                let risk_assessed =
                    m.get("risk_assessed").and_then(|v| v.as_bool()).unwrap_or(false);
                result.insert("criteria_met_rate".into(), if criteria_met { 1.0 } else { 0.0 });
                result.insert("risk_assessment_rate".into(), if risk_assessed { 1.0 } else { 0.0 });
            }
        },
        NodeType::Tool | NodeType::Valuation | NodeType::Risk => {
            if let Some(m) = metrics.as_object() {
                let completeness = m.get("completeness").and_then(|v| v.as_bool()).unwrap_or(true);
                result.insert("completeness_rate".into(), if completeness { 1.0 } else { 0.0 });
            }
        },
        _ => {},
    }

    result
}
