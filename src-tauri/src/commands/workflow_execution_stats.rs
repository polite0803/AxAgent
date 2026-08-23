// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流执行统计命令层 —— 把 dao 层包装为 Tauri 命令。
//!
//! 用途：
//! - `record_workflow_execution` — 工作流执行完成后记录效果数据
//! - `get_workflow_stats_by_template` — 查询某模板最近的执行记录
//! - `get_workflow_stats_by_mission` — 按 mission_hash 聚合查询
//! - `get_template_effect_summary` — 模板效果聚合统计（成功率/平均延迟/平均 token）

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use axagent_agent_macro::agent_command;
use axagent_dao::repo::workflow_execution_stats as db_repo;
use axagent_dao::repo::workflow_execution_stats::TemplateEffectSummary;
use axagent_harness::repo_dtos::WorkflowExecutionStatsDto;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordExecutionInput {
    pub mission_hash: Option<String>,
    pub template_id: Option<String>,
    pub execution_id: Option<String>,
    pub status: String,
    pub total_time_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub error_message: Option<String>,
    pub user_rating: Option<f64>,
}

/// 记录一次工作流执行的效果数据。
///
/// 通常在工作流执行完成（成功/失败/取消）后由前端调用，把执行结果落库。
/// `id` 由调用方生成（建议用 uuid），避免 dao 层重复生成。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "记录工作流执行效果数据")]
#[tauri::command]
pub async fn record_workflow_execution(
    state: State<'_, AppState>,
    input: RecordExecutionInput,
) -> Result<WorkflowExecutionStatsDto, String> {
    // user_rating 边界校验：必须在 [0.0, 5.0] 区间（5 星制）
    let user_rating = input.user_rating.map(|r| r.clamp(0.0, 5.0));

    // negative token/time 防御：负值视为 0
    let total_time_ms = input.total_time_ms.max(0);
    let input_tokens = input.input_tokens.max(0);
    let output_tokens = input.output_tokens.max(0);

    let id = uuid::Uuid::new_v4().to_string();
    let db = state.harness.db();

    db_repo::record_execution(
        db,
        &id,
        input.mission_hash.as_deref(),
        input.template_id.as_deref(),
        input.execution_id.as_deref(),
        &input.status,
        total_time_ms,
        input_tokens,
        output_tokens,
        input.error_message.as_deref(),
        user_rating,
    )
    .await
    .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 查询某模板最近的执行记录（按 created_at 倒序）。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "查询模板执行记录")]
#[tauri::command]
pub async fn get_workflow_stats_by_template(
    state: State<'_, AppState>,
    template_id: String,
    limit: Option<u64>,
) -> Result<Vec<WorkflowExecutionStatsDto>, String> {
    let db = state.harness.db();
    // limit 上限 1000，防止恶意大查询
    let limit = limit.unwrap_or(50).min(1000);
    db_repo::get_stats_by_template(db, &template_id, limit)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 按 mission_hash 查询执行记录（按 created_at 倒序）。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "按mission查询执行记录")]
#[tauri::command]
pub async fn get_workflow_stats_by_mission(
    state: State<'_, AppState>,
    mission_hash: String,
    limit: Option<u64>,
) -> Result<Vec<WorkflowExecutionStatsDto>, String> {
    let db = state.harness.db();
    let limit = limit.unwrap_or(50).min(1000);
    db_repo::get_stats_by_mission(db, &mission_hash, limit)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 模板效果聚合统计（成功率/平均延迟/平均 token/平均评分）。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取模板效果汇总")]
#[tauri::command]
pub async fn get_template_effect_summary(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<TemplateEffectSummary, String> {
    let db = state.harness.db();
    db_repo::get_template_effect_summary(db, &template_id)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}
