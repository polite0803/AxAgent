// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流执行统计 repository —— 记录每次工作流执行的效果数据。
//!
//! 用于驱动效果导向的工作流优化（区别于失败驱动的 replan）。
//! 通过 `mission_hash` 聚合相同任务的不同执行，统计成功率/平均延迟/平均 token 成本，
//! 提供 `get_stats_by_template` / `get_stats_by_mission` 查询接口。

use sea_orm::*;

use axagent_entities::workflow_execution_stats;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::repo_dtos::WorkflowExecutionStatsDto;
use axagent_harness::util_fns::now_ts;

fn stats_from_entity(m: workflow_execution_stats::Model) -> WorkflowExecutionStatsDto {
    WorkflowExecutionStatsDto {
        id: m.id,
        mission_hash: m.mission_hash,
        template_id: m.template_id,
        execution_id: m.execution_id,
        status: m.status,
        total_time_ms: m.total_time_ms,
        input_tokens: m.input_tokens,
        output_tokens: m.output_tokens,
        error_message: m.error_message,
        user_rating: m.user_rating,
        created_at: m.created_at,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn record_execution(
    db: &DatabaseConnection,
    id: &str,
    mission_hash: Option<&str>,
    template_id: Option<&str>,
    execution_id: Option<&str>,
    status: &str,
    total_time_ms: i64,
    input_tokens: i64,
    output_tokens: i64,
    error_message: Option<&str>,
    user_rating: Option<f64>,
) -> Result<WorkflowExecutionStatsDto> {
    let now = now_ts();
    let am = workflow_execution_stats::ActiveModel {
        id: Set(id.to_string()),
        mission_hash: Set(mission_hash.map(|s| s.to_string())),
        template_id: Set(template_id.map(|s| s.to_string())),
        execution_id: Set(execution_id.map(|s| s.to_string())),
        status: Set(status.to_string()),
        total_time_ms: Set(total_time_ms),
        input_tokens: Set(input_tokens),
        output_tokens: Set(output_tokens),
        error_message: Set(error_message.map(|s| s.to_string())),
        user_rating: Set(user_rating),
        created_at: Set(now),
    };

    am.insert(db).await.map_err(AxAgentError::from)?;

    Ok(WorkflowExecutionStatsDto {
        id: id.to_string(),
        mission_hash: mission_hash.map(|s| s.to_string()),
        template_id: template_id.map(|s| s.to_string()),
        execution_id: execution_id.map(|s| s.to_string()),
        status: status.to_string(),
        total_time_ms,
        input_tokens,
        output_tokens,
        error_message: error_message.map(|s| s.to_string()),
        user_rating,
        created_at: now,
    })
}

pub async fn get_stats_by_template(
    db: &DatabaseConnection,
    template_id: &str,
    limit: u64,
) -> Result<Vec<WorkflowExecutionStatsDto>> {
    let rows = workflow_execution_stats::Entity::find()
        .filter(workflow_execution_stats::Column::TemplateId.eq(template_id))
        .order_by_desc(workflow_execution_stats::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(stats_from_entity).collect())
}

pub async fn get_stats_by_mission(
    db: &DatabaseConnection,
    mission_hash: &str,
    limit: u64,
) -> Result<Vec<WorkflowExecutionStatsDto>> {
    let rows = workflow_execution_stats::Entity::find()
        .filter(workflow_execution_stats::Column::MissionHash.eq(mission_hash))
        .order_by_desc(workflow_execution_stats::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(stats_from_entity).collect())
}

/// 模板效果聚合统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateEffectSummary {
    pub template_id: String,
    pub total_executions: u64,
    pub success_count: u64,
    pub failed_count: u64,
    pub success_rate: f64,
    pub avg_total_time_ms: f64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_user_rating: Option<f64>,
}

pub async fn get_template_effect_summary(
    db: &DatabaseConnection,
    template_id: &str,
) -> Result<TemplateEffectSummary> {
    let rows = workflow_execution_stats::Entity::find()
        .filter(workflow_execution_stats::Column::TemplateId.eq(template_id))
        .all(db)
        .await?;

    let total = rows.len() as u64;
    if total == 0 {
        return Ok(TemplateEffectSummary {
            template_id: template_id.to_string(),
            total_executions: 0,
            success_count: 0,
            failed_count: 0,
            success_rate: 0.0,
            avg_total_time_ms: 0.0,
            avg_input_tokens: 0.0,
            avg_output_tokens: 0.0,
            avg_user_rating: None,
        });
    }

    let success_count = rows.iter().filter(|r| r.status == "success").count() as u64;
    let failed_count = total - success_count;
    let success_rate = success_count as f64 / total as f64;
    let avg_total_time_ms = rows.iter().map(|r| r.total_time_ms as f64).sum::<f64>() / total as f64;
    let avg_input_tokens = rows.iter().map(|r| r.input_tokens as f64).sum::<f64>() / total as f64;
    let avg_output_tokens = rows.iter().map(|r| r.output_tokens as f64).sum::<f64>() / total as f64;

    let ratings: Vec<f64> = rows.iter().filter_map(|r| r.user_rating).collect();
    let avg_user_rating = if ratings.is_empty() {
        None
    } else {
        Some(ratings.iter().sum::<f64>() / ratings.len() as f64)
    };

    Ok(TemplateEffectSummary {
        template_id: template_id.to_string(),
        total_executions: total,
        success_count,
        failed_count,
        success_rate,
        avg_total_time_ms,
        avg_input_tokens,
        avg_output_tokens,
        avg_user_rating,
    })
}
