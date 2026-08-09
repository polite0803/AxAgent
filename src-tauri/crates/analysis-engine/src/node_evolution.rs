// SPDX-License-Identifier: AGPL-3.0-only

//! 通用节点自我进化引擎
//!
//! 基于历史反馈数据，自动分析节点的"常见病"并进化其 Prompt/配置。
//! 支持的节点类型：分析师、辩论、决策、工具、估值、风险
//!
//! 核心逻辑：
//! 1. 拉取指定节点类型+节点ID的历史反馈
//! 2. 分析反馈中的共性问题（类型相关的质量指标）
//! 3. 当问题频率超过阈值时，自动生成并应用优化建议

use crate::node_quality::{calc_consistency_metrics, NodeType};
use axagent_entities::analyst_feedback::{Entity as AnalystFeedback, Model as FeedbackModel};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点进化状态（通用版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEvolutionStatus {
    pub node_type: String,
    pub node_id: String,
    pub total_feedbacks: u64,
    pub issue_rate: f64,
    pub consistency_metrics: HashMap<String, f64>,
    pub last_evolution_time: Option<String>,
    pub evolution_count: u32,
    pub status: String,
    pub suggestions: Vec<String>,
}

/// 执行节点进化（基于反馈历史的 Prompt/配置优化）
pub async fn evolve_node(
    db: &DatabaseConnection,
    node_type: &NodeType,
    node_id: &str,
) -> Result<NodeEvolutionStatus, String> {
    let type_str = node_type.as_str();

    // 1. 拉取历史反馈
    let feedbacks = AnalystFeedback::find()
        .filter(axagent_entities::analyst_feedback::Column::NodeType.eq(type_str))
        .filter(axagent_entities::analyst_feedback::Column::NodeId.eq(node_id))
        .order_by_desc(axagent_entities::analyst_feedback::Column::CreatedAt)
        .limit(100)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let total = feedbacks.len() as u64;

    if total == 0 {
        let suggestions = vec!["暂无反馈数据，建议先运行几次分析".to_string()];

        return Ok(NodeEvolutionStatus {
            node_type: type_str.to_string(),
            node_id: node_id.to_string(),
            total_feedbacks: 0,
            issue_rate: 0.0,
            consistency_metrics: HashMap::new(),
            last_evolution_time: None,
            evolution_count: 0,
            status: "no_data".to_string(),
            suggestions,
        });
    }

    // 2. 计算统计指标
    let total_issues: u64 = feedbacks.iter().map(|f| f.issue_count as u64).sum();
    let avg_quality: f64 =
        feedbacks.iter().map(|f| f.quality_score as f64).sum::<f64>() / total as f64;

    // 3. 计算一致性指标（按节点类型）
    let mut consistency_metrics: HashMap<String, f64> = HashMap::new();
    for f in &feedbacks {
        if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(&f.quality_metrics_json) {
            let metrics_for_node = calc_consistency_metrics(node_type, &metrics);
            for (key, value) in metrics_for_node {
                *consistency_metrics.entry(key).or_insert(0.0) += value;
            }
        }
    }
    // 计算平均值
    for value in consistency_metrics.values_mut() {
        *value /= total as f64;
    }

    // 4. 分析问题模式
    let mut problem_types: HashMap<String, u64> = HashMap::new();
    for f in &feedbacks {
        if f.issue_count > 0 {
            if let Ok(checks) = serde_json::from_str::<Vec<serde_json::Value>>(&f.checks_json) {
                for check in &checks {
                    if check.get("status").and_then(|s| s.as_str()) == Some("issue") {
                        let field =
                            check.get("field").and_then(|v| v.as_str()).unwrap_or("unknown");
                        *problem_types.entry(field.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // 5. 生成建议（按节点类型）
    let mut suggestions = Vec::new();
    let mut should_evolve = false;

    // 5.1 质量分过低
    if avg_quality < 70.0 {
        suggestions
            .push(format!("平均质量分偏低 ({:.1})，建议检查 Prompt/配置的指令清晰度", avg_quality));
        should_evolve = true;
    }

    // 5.2 节点类型特有的一致性建议
    match node_type {
        NodeType::Analyst => {
            if let Some(score_rate) = consistency_metrics.get("score_consistency_rate") {
                if *score_rate < 0.7 {
                    suggestions.push(format!(
                        "评分自洽性过低 ({:.1}%)，建议强化 'bull_score + bear_score 必须等于 100' 的约束",
                        score_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
            if let Some(dir_rate) = consistency_metrics.get("direction_consistency_rate") {
                if *dir_rate < 0.7 {
                    suggestions.push(format!(
                        "方向一致性过低 ({:.1}%)，建议增加 verdict 与评分方向匹配的自检步骤",
                        dir_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
        },
        NodeType::Debate => {
            if let Some(logic_rate) = consistency_metrics.get("logic_consistency_rate") {
                if *logic_rate < 0.7 {
                    suggestions.push(format!(
                        "逻辑一致性过低 ({:.1}%)，建议强化立场与论据的逻辑约束",
                        logic_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
        },
        NodeType::Decision => {
            if let Some(criteria_rate) = consistency_metrics.get("criteria_met_rate") {
                if *criteria_rate < 0.7 {
                    suggestions.push(format!(
                        "决策标准达标率过低 ({:.1}%)，建议明确决策所需的评估标准",
                        criteria_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
            if let Some(risk_rate) = consistency_metrics.get("risk_assessment_rate") {
                if *risk_rate < 0.7 {
                    suggestions.push(format!(
                        "风险评估覆盖率过低 ({:.1}%)，建议在决策流程中强制包含风险评估",
                        risk_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
        },
        NodeType::Tool | NodeType::Valuation | NodeType::Risk => {
            if let Some(completeness_rate) = consistency_metrics.get("completeness_rate") {
                if *completeness_rate < 0.7 {
                    suggestions.push(format!(
                        "输出完整性过低 ({:.1}%)，建议检查工具的数据获取逻辑",
                        completeness_rate * 100.0
                    ));
                    should_evolve = true;
                }
            }
        },
        _ => {},
    }

    // 5.3 高频问题字段
    let mut frequent_issues: Vec<(&String, &u64)> = problem_types.iter().collect();
    frequent_issues.sort_by(|a, b| b.1.cmp(a.1));
    for (field, count) in &frequent_issues {
        if **count > total / 3 {
            suggestions.push(format!(
                "字段 '{}' 在 {} 次反馈中出现 {} 次问题，建议优化该字段的生成逻辑",
                field, total, count
            ));
            should_evolve = true;
        }
    }

    // 6. 构造进化状态
    let status = if should_evolve {
        "needs_attention".to_string()
    } else if total >= 5 {
        // 检查所有一致性指标是否都良好
        let all_good = consistency_metrics.values().all(|v| *v >= 0.9);
        if all_good && avg_quality >= 80.0 {
            "healthy".to_string()
        } else {
            "collecting_data".to_string()
        }
    } else {
        "collecting_data".to_string()
    };

    Ok(NodeEvolutionStatus {
        node_type: type_str.to_string(),
        node_id: node_id.to_string(),
        total_feedbacks: total,
        issue_rate: total_issues as f64 / total as f64,
        consistency_metrics,
        last_evolution_time: None,
        evolution_count: 0,
        status,
        suggestions,
    })
}

/// 获取节点的进化状态（不触发进化）
pub async fn get_node_evolution_status(
    db: &DatabaseConnection,
    node_type: &NodeType,
    node_id: &str,
) -> Result<NodeEvolutionStatus, String> {
    let status = evolve_node(db, node_type, node_id).await?;
    Ok(status)
}

/// 兼容性函数：evolve_analyst（保留向后兼容）
pub async fn evolve_analyst(
    db: &DatabaseConnection,
    analyst_id: &str,
) -> Result<NodeEvolutionStatus, String> {
    evolve_node(db, &NodeType::Analyst, analyst_id).await
}

/// 兼容性函数：get_analyst_evolution_status（保留向后兼容）
pub async fn get_analyst_evolution_status(
    db: &DatabaseConnection,
    analyst_id: &str,
) -> Result<NodeEvolutionStatus, String> {
    get_node_evolution_status(db, &NodeType::Analyst, analyst_id).await
}

/// 应用 Prompt 修正规则（通用版）
///
/// 根据节点类型和反馈分析结果，生成附加到 Prompt 末尾的修正指令。
pub fn generate_prompt_corrections(node_type: &NodeType, feedback: &FeedbackModel) -> Vec<String> {
    let mut corrections = Vec::new();

    if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(&feedback.quality_metrics_json) {
        match node_type {
            NodeType::Analyst => {
                if let Some(m) = metrics.as_object() {
                    let score_consistent =
                        m.get("score_consistent").and_then(|v| v.as_bool()).unwrap_or(true);
                    let direction_consistent =
                        m.get("direction_consistent").and_then(|v| v.as_bool()).unwrap_or(true);

                    if !score_consistent {
                        corrections.push(
                            "【质量强制约束】请确保 bull_score + bear_score 的总和必须严格等于 100。在输出前务必自检。".to_string()
                        );
                    }
                    if !direction_consistent {
                        corrections.push(
                            "【逻辑强制约束】请确保 verdict（看多/看空）必须与 bull_score/bear_score 的大小关系一致。".to_string()
                        );
                    }
                }
            },
            NodeType::Debate => {
                if let Some(m) = metrics.as_object() {
                    let logic_consistent =
                        m.get("logic_consistent").and_then(|v| v.as_bool()).unwrap_or(true);

                    if !logic_consistent {
                        corrections.push(
                            "【逻辑强制约束】请确保你的立场（stance）与论据链保持逻辑一致性，论据必须支持声明。".to_string()
                        );
                    }
                }
            },
            NodeType::Decision => {
                if let Some(m) = metrics.as_object() {
                    let risk_assessed =
                        m.get("risk_assessed").and_then(|v| v.as_bool()).unwrap_or(false);
                    let criteria_met =
                        m.get("criteria_met").and_then(|v| v.as_bool()).unwrap_or(false);

                    if !risk_assessed {
                        corrections.push(
                            "【风险强制约束】在做出任何决策前，必须完成风险评估并明确列出主要风险因素。".to_string()
                        );
                    }
                    if !criteria_met {
                        corrections.push(
                            "【标准强制约束】决策必须明确说明符合的评估标准和满足的条件。"
                                .to_string(),
                        );
                    }
                }
            },
            NodeType::Tool | NodeType::Valuation | NodeType::Risk => {
                if let Some(m) = metrics.as_object() {
                    let completeness =
                        m.get("completeness").and_then(|v| v.as_bool()).unwrap_or(true);

                    if !completeness {
                        corrections.push(
                            "【完整性强制约束】请确保输出包含所有必需的字段，不允许返回空结果或部分结果。".to_string()
                        );
                    }
                }
            },
            _ => {},
        }
    }

    corrections
}
