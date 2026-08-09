// SPDX-License-Identifier: AGPL-3.0-only

//! 分析师自我进化引擎（向后兼容包装）
//!
//! 此模块保留原有的分析师专用接口，但内部实现委托给通用节点进化引擎。
//! 新代码应直接使用 crate::node_evolution 中的通用接口。

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// 分析师进化状态（向后兼容结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalystEvolutionStatus {
    pub analyst_id: String,
    pub total_feedbacks: u64,
    pub issue_rate: f64,
    pub score_consistency_rate: f64,
    pub direction_consistency_rate: f64,
    pub last_evolution_time: Option<String>,
    pub evolution_count: u32,
    pub status: String, // "healthy" | "needs_attention" | "evolving"
    pub suggestions: Vec<String>,
}

/// 执行分析师进化（基于反馈历史的 Prompt 优化）
///
/// 向后兼容包装，内部委托给通用节点进化引擎
pub async fn evolve_analyst(
    db: &DatabaseConnection,
    analyst_id: &str,
) -> Result<AnalystEvolutionStatus, String> {
    // 使用通用节点进化引擎
    let status =
        crate::node_evolution::evolve_node(db, &crate::node_quality::NodeType::Analyst, analyst_id)
            .await?;

    // 转换为向后兼容的结构
    let score_consistency_rate = extract_metric_rate(&status.node_id, "score_consistent");
    let direction_consistency_rate = extract_metric_rate(&status.node_id, "direction_consistent");

    Ok(AnalystEvolutionStatus {
        analyst_id: status.node_id,
        total_feedbacks: status.total_feedbacks,
        issue_rate: status.issue_rate,
        score_consistency_rate,
        direction_consistency_rate,
        last_evolution_time: status.last_evolution_time,
        evolution_count: status.evolution_count,
        status: status.status,
        suggestions: status.suggestions,
    })
}

/// 获取分析师的进化状态（不触发进化）
pub async fn get_analyst_evolution_status(
    db: &DatabaseConnection,
    analyst_id: &str,
) -> Result<AnalystEvolutionStatus, String> {
    let status = crate::node_evolution::get_node_evolution_status(
        db,
        &crate::node_quality::NodeType::Analyst,
        analyst_id,
    )
    .await?;

    let score_consistency_rate = extract_metric_rate(&status.node_id, "score_consistent");
    let direction_consistency_rate = extract_metric_rate(&status.node_id, "direction_consistent");

    Ok(AnalystEvolutionStatus {
        analyst_id: status.node_id,
        total_feedbacks: status.total_feedbacks,
        issue_rate: status.issue_rate,
        score_consistency_rate,
        direction_consistency_rate,
        last_evolution_time: status.last_evolution_time,
        evolution_count: status.evolution_count,
        status: status.status,
        suggestions: status.suggestions,
    })
}

/// 从通用节点反馈中提取指标达标率（简化实现）
fn extract_metric_rate(_node_id: &str, _metric_name: &str) -> f64 {
    // 简化：实际实现应查询数据库获取历史反馈的指标达标率
    // 此处返回默认值，实际值由通用引擎计算
    1.0
}

/// 应用 Prompt 修正规则
///
/// 根据反馈分析结果，生成附加到 Prompt 末尾的修正指令。
/// 这些指令会在下次分析时自动应用，形成"自我进化"的闭环。
///
/// 注意：此函数已改为基于通用质量指标生成建议
pub fn generate_prompt_corrections(quality_metrics_json: &str) -> Vec<String> {
    let mut corrections = Vec::new();

    if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(quality_metrics_json) {
        // 检查评分自洽性
        if let Some(score_consistent) = metrics.get("score_consistent").and_then(|v| v.as_bool()) {
            if !score_consistent {
                corrections.push(
                    "【质量强制约束】请确保 bull_score + bear_score 的总和必须严格等于 100。在输出前务必自检。".to_string()
                );
            }
        }

        // 检查方向一致性
        if let Some(direction_consistent) =
            metrics.get("direction_consistent").and_then(|v| v.as_bool())
        {
            if !direction_consistent {
                corrections.push(
                    "【逻辑强制约束】请确保 verdict（看多/看空）必须与 bull_score/bear_score 的大小关系一致。看多时分数必须 bull_score > bear_score，反之亦然。".to_string()
                );
            }
        }

        // 检查置信度范围
        if let Some(confidence) = metrics.get("confidence").and_then(|v| v.as_f64()) {
            if !(0.0..=100.0).contains(&confidence) {
                corrections.push("【范围约束】confidence 必须在 0-100 范围内。".to_string());
            }
        }
    }

    corrections
}
