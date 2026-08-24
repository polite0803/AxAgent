// SPDX-License-Identifier: AGPL-3.0-only

//! 通用节点质量反馈命令
//!
//! 支持所有节点类型（分析师/辩论/决策/工具/估值/风险）的质量反馈存储和查询。

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_entities::analyst_feedback;
use axagent_entities::analyst_feedback::Entity as AnalystFeedback;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 通用节点质量反馈的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNodeFeedbackRequest {
    /// 节点类型 (analyst/debate/decision/tool/valuation/risk/other)
    pub node_type: String,
    /// 节点 ID
    pub node_id: String,
    /// 报告 ID
    pub report_id: String,
    /// 股票代码
    pub stock_code: String,
    /// 执行 ID (workflow run id)
    pub execution_id: String,
    /// 质量评分 (0-100)
    pub quality_score: i32,
    /// 评分等级 (A/B/C/D/F)
    pub grade: String,
    /// 检测到的问题数量
    pub issue_count: i32,
    /// 检测到的警告数量
    pub warning_count: i32,
    /// 检测到的良好项数量
    pub good_count: i32,
    /// 详细检查结果 (JSON)
    pub checks_json: String,
    /// 通用质量指标 (JSON) - 存储节点特有的质量指标
    pub quality_metrics_json: String,
}

/// 节点反馈摘要（用于前端列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFeedbackSummary {
    pub id: String,
    pub node_type: String,
    pub node_id: String,
    pub quality_score: i32,
    pub grade: String,
    pub issue_count: i32,
    pub warning_count: i32,
    pub created_at: String,
}

/// 获取节点反馈列表的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNodeFeedbacksRequest {
    /// 节点类型 (可选，用于过滤)
    pub node_type: Option<String>,
    /// 节点 ID
    pub node_id: Option<String>,
    /// 限制数量
    pub limit: Option<u64>,
    /// 仅显示有问题的
    pub only_issues: Option<bool>,
}

/// 保存节点质量反馈（通用版）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "保存节点质量反馈数据，用于自我进化")]
pub async fn save_node_feedback(
    state: State<'_, AppState>,
    req: SaveNodeFeedbackRequest,
) -> Result<String, String> {
    let db = state.harness.db();

    let new_feedback = analyst_feedback::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        node_type: Set(req.node_type),
        node_id: Set(req.node_id),
        report_id: Set(req.report_id),
        stock_code: Set(req.stock_code),
        execution_id: Set(req.execution_id),
        quality_score: Set(req.quality_score),
        grade: Set(req.grade),
        issue_count: Set(req.issue_count),
        warning_count: Set(req.warning_count),
        good_count: Set(req.good_count),
        checks_json: Set(req.checks_json),
        quality_metrics_json: Set(req.quality_metrics_json),
        consumed: Set(false),
        evolution_triggered: Set(false),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
    };

    let result = new_feedback.insert(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(result.id)
}

/// 保存分析师反馈（向后兼容包装）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "保存分析师质量反馈数据（向后兼容）")]
pub async fn save_analyst_feedback(
    state: State<'_, AppState>,
    req: SaveAnalystFeedbackRequest,
) -> Result<String, String> {
    // 将旧格式转换为新格式
    let quality_metrics = serde_json::json!({
        "bull_score": req.bull_score,
        "bear_score": req.bear_score,
        "confidence": req.confidence,
        "score_consistent": req.score_consistent,
        "direction_consistent": req.direction_consistent,
    });

    save_node_feedback(
        state,
        SaveNodeFeedbackRequest {
            node_type: "analyst".to_string(),
            node_id: req.analyst_id,
            report_id: req.report_id,
            stock_code: req.stock_code,
            execution_id: req.execution_id,
            quality_score: req.quality_score,
            grade: req.grade,
            issue_count: req.issue_count,
            warning_count: req.warning_count,
            good_count: req.good_count,
            checks_json: req.checks_json,
            quality_metrics_json: quality_metrics.to_string(),
        },
    )
    .await
}

/// 分析师反馈请求 DTO（向后兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAnalystFeedbackRequest {
    pub analyst_id: String,
    pub report_id: String,
    pub stock_code: String,
    pub execution_id: String,
    pub quality_score: i32,
    pub grade: String,
    pub issue_count: i32,
    pub warning_count: i32,
    pub good_count: i32,
    pub checks_json: String,
    pub bull_score: Option<f64>,
    pub bear_score: Option<f64>,
    pub confidence: Option<f64>,
    pub score_consistent: bool,
    pub direction_consistent: bool,
}

/// 获取节点反馈历史（通用版）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取指定节点的质量反馈历史")]
pub async fn get_node_feedbacks(
    state: State<'_, AppState>,
    req: GetNodeFeedbacksRequest,
) -> Result<Vec<NodeFeedbackSummary>, String> {
    let db = state.harness.db();

    let limit = req.limit.unwrap_or(50);
    let only_issues = req.only_issues.unwrap_or(false);

    let mut query = AnalystFeedback::find();

    if let Some(ref node_type) = req.node_type {
        query = query.filter(analyst_feedback::Column::NodeType.eq(node_type));
    }
    if let Some(ref node_id) = req.node_id {
        query = query.filter(analyst_feedback::Column::NodeId.eq(node_id));
    }

    query = query.order_by_desc(analyst_feedback::Column::CreatedAt).limit(limit);

    if only_issues {
        query = query.filter(analyst_feedback::Column::IssueCount.gt(0));
    }

    let results = query.all(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let summaries: Vec<NodeFeedbackSummary> = results
        .into_iter()
        .map(|m| NodeFeedbackSummary {
            id: m.id,
            node_type: m.node_type,
            node_id: m.node_id,
            quality_score: m.quality_score,
            grade: m.grade,
            issue_count: m.issue_count,
            warning_count: m.warning_count,
            created_at: m.created_at,
        })
        .collect();

    Ok(summaries)
}

/// 获取分析师反馈历史（向后兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAnalystFeedbacksRequest {
    pub analyst_id: String,
    pub limit: Option<u64>,
    pub only_issues: Option<bool>,
}

#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取指定分析师的质量反馈历史（向后兼容）")]
pub async fn get_analyst_feedbacks(
    state: State<'_, AppState>,
    req: GetAnalystFeedbacksRequest,
) -> Result<Vec<NodeFeedbackSummary>, String> {
    get_node_feedbacks(
        state,
        GetNodeFeedbacksRequest {
            node_type: Some("analyst".to_string()),
            node_id: Some(req.analyst_id),
            limit: req.limit,
            only_issues: req.only_issues,
        },
    )
    .await
}

/// 节点反馈统计（用于判断是否需要触发进化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFeedbackStats {
    pub node_type: String,
    pub node_id: String,
    pub total_count: u64,
    pub issue_count_total: u64,
    pub avg_quality_score: f64,
    pub low_score_count: u64,
    pub needs_evolution: bool,
    pub consistency_metrics: std::collections::HashMap<String, f64>,
}

/// 获取节点反馈统计（通用版）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取节点的反馈统计数据，判断是否需要触发进化")]
pub async fn get_node_feedback_stats(
    state: State<'_, AppState>,
    node_type: String,
    node_id: String,
) -> Result<NodeFeedbackStats, String> {
    let db = state.harness.db();

    let all_feedbacks = AnalystFeedback::find()
        .filter(analyst_feedback::Column::NodeType.eq(&node_type))
        .filter(analyst_feedback::Column::NodeId.eq(&node_id))
        .all(db)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let total_count = all_feedbacks.len() as u64;

    if total_count == 0 {
        return Ok(NodeFeedbackStats {
            node_type,
            node_id,
            total_count: 0,
            issue_count_total: 0,
            avg_quality_score: 100.0,
            low_score_count: 0,
            needs_evolution: false,
            consistency_metrics: std::collections::HashMap::new(),
        });
    }

    let issue_count_total = all_feedbacks.iter().map(|f| f.issue_count as u64).sum();
    let avg_quality_score =
        all_feedbacks.iter().map(|f| f.quality_score as f64).sum::<f64>() / total_count as f64;
    let low_score_count = all_feedbacks.iter().filter(|f| f.quality_score < 60).count() as u64;

    // 计算一致性指标
    let mut consistency_metrics = std::collections::HashMap::new();
    let node_type_enum = parse_node_type(&node_type);
    for f in &all_feedbacks {
        if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(&f.quality_metrics_json) {
            let node_metrics = axagent_analysis_engine::node_quality::calc_consistency_metrics(
                &node_type_enum,
                &metrics,
            );
            for (key, value) in node_metrics {
                *consistency_metrics.entry(key).or_insert(0.0) += value;
            }
        }
    }
    // 计算平均值
    let total = total_count as f64;
    for value in consistency_metrics.values_mut() {
        *value /= total;
    }

    // 触发进化的条件：
    // 1. 至少有 3 次反馈
    // 2. 平均分 < 70 或 任一致性率 < 0.7 或 低分之多
    let needs_evolution = total_count >= 3
        && (avg_quality_score < 70.0
            || low_score_count >= 2
            || consistency_metrics.values().any(|v| *v < 0.7));

    Ok(NodeFeedbackStats {
        node_type,
        node_id,
        total_count,
        issue_count_total,
        avg_quality_score,
        low_score_count,
        needs_evolution,
        consistency_metrics,
    })
}

/// 解析节点类型字符串
fn parse_node_type(type_str: &str) -> axagent_analysis_engine::NodeType {
    match type_str {
        "analyst" => axagent_analysis_engine::NodeType::Analyst,
        "debate" => axagent_analysis_engine::NodeType::Debate,
        "decision" => axagent_analysis_engine::NodeType::Decision,
        "tool" | "valuation" | "risk" => axagent_analysis_engine::NodeType::Tool,
        _ => axagent_analysis_engine::NodeType::Other,
    }
}

/// 获取分析师反馈统计（向后兼容）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取分析师的反馈统计数据（向后兼容）")]
pub async fn get_analyst_feedback_stats(
    state: State<'_, AppState>,
    analyst_id: String,
) -> Result<NodeFeedbackStats, String> {
    get_node_feedback_stats(state, "analyst".to_string(), analyst_id).await
}
