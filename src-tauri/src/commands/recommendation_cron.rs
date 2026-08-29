// SPDX-License-Identifier: AGPL-3.0-only

//! 荐股定时任务：配置解析与辅助函数
//!
//! 与 [stock_cron] 不同的是：
//! - task_type = `stock-recommendation`（用于在 [services] 的 cron executor 中路由到荐股 handler）
//! - 配置（periods / min_confidence / top_n）以 JSON 形式写入 `CronJob.prompt`
//! - 不绑定 workflow（不走 work_engine）
//!
//! 注意：原荐股扫描执行引擎委托给 `axagent_analysis_engine::recommender`，
//! 该 crate 已删除。`run_recommendation_cron` 现为存根，始终返回错误。
//! 荐股 CRUD 命令保留（允许增删改查 cron 配置），但任务实际执行会失败。
//!
//! [stock_cron]: crate::commands::stock_analysis::create_stock_cron
//! [services]: crate::init::services::start_cron_scheduler

/// K 线周期（原来自 axagent_analysis_engine::recommender::Period，本地兼容定义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Period {
    Short,
    Mid,
    Long,
}

/// 推荐 cron 配置（写入 `CronJob.prompt`）
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecoCronConfig {
    pub periods: Vec<Period>,
    pub min_confidence: u8,
    pub top_n: usize,
}

impl RecoCronConfig {
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("解析荐股 cron 配置失败: {e}"))
    }
}

// ── Tauri 命令：荐股定时任务 CRUD ──

use axagent_agent_macro::agent_command;
use axagent_runtime_core::{CronJob, CronJobStatus};
use serde::Serialize;
use tauri::State;

use crate::AppState;

/// 与前端 `RecoCronRow` 对齐的响应结构
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoCronJobResponse {
    id: String,
    name: String,
    description: String,
    schedule: String,
    status: String,
    recurring: bool,
    run_count: u32,
    last_run_at: Option<i64>,
    next_run_at: Option<i64>,
    /// 解析后的配置（periods / min_confidence / top_n）
    config: RecoCronConfig,
    /// 上次推送的 picks 数量（从 last_result.output 反序列化）
    last_picks_count: Option<usize>,
}

impl RecoCronJobResponse {
    /// 从 CronJob 构造响应；prompt 解析失败时 config 用默认值
    fn from_job(j: &CronJob) -> Self {
        let config = RecoCronConfig::from_json(&j.prompt).unwrap_or(RecoCronConfig {
            periods: vec![Period::Short],
            min_confidence: 60,
            top_n: 5,
        });
        // last_result.output 是执行结果的 JSON
        let last_picks_count = j
            .last_result
            .as_ref()
            .and_then(|r| r.output.as_deref())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("pushed").and_then(|n| n.as_u64()).map(|n| n as usize));
        Self {
            id: j.id.clone(),
            name: j.name.clone(),
            description: j.description.clone(),
            schedule: j.schedule.clone(),
            status: format!("{:?}", j.status).to_lowercase(),
            recurring: j.recurring,
            run_count: j.run_count,
            last_run_at: j.last_run_at,
            next_run_at: j.next_run_at,
            config,
            last_picks_count,
        }
    }
}

/// 创建荐股定时任务
///
/// - 配置以 JSON 写入 `CronJob.prompt`，由 `run_recommendation_cron` 在执行时解析
/// - task_type = "stock-recommendation"，不绑定 workflow
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "创建荐股定时任务")]
#[tauri::command]
pub async fn create_recommendation_cron(
    state: State<'_, AppState>,
    name: String,
    cron_expression: String,
    periods: Vec<Period>,
    min_confidence: u8,
    top_n: usize,
) -> Result<RecoCronJobResponse, String> {
    if periods.is_empty() {
        return Err("periods 不能为空".to_string());
    }
    if top_n == 0 {
        return Err("top_n 必须大于 0".to_string());
    }
    let config = RecoCronConfig { periods, min_confidence, top_n };
    let prompt =
        serde_json::to_string(&config).map_err(|e| format!("序列化荐股 cron 配置失败: {e}"))?;
    let desc = format!("荐股定时推送 (置信度≥{}%, 前{}只)", min_confidence, top_n);
    let job = CronJob::new(&name, &cron_expression, &prompt, &desc)
        .with_task_type("stock-recommendation");
    let id = state.cron_job_store.add(job).await;
    // 重新读回以拿到完整字段（next_run_at 等）
    let saved =
        state.cron_job_store.get(&id).await.ok_or_else(|| "保存后未找到任务".to_string())?;
    Ok(RecoCronJobResponse::from_job(&saved))
}

/// 列出所有荐股定时任务
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "列出荐股定时任务")]
#[tauri::command]
pub async fn list_recommendation_crons(
    state: State<'_, AppState>,
) -> Result<Vec<RecoCronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("stock-recommendation"))
        .map(RecoCronJobResponse::from_job)
        .collect())
}

/// 启停荐股定时任务
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "启停荐股定时任务")]
#[tauri::command]
pub async fn toggle_recommendation_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除荐股定时任务
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "删除荐股定时任务")]
#[tauri::command]
pub async fn delete_recommendation_cron(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reco_cron_config_from_json() {
        let json = r#"{"periods":["short","mid"],"minConfidence":70,"topN":5}"#;
        let config = RecoCronConfig::from_json(json).unwrap();
        assert_eq!(config.periods.len(), 2);
        assert_eq!(config.min_confidence, 70);
        assert_eq!(config.top_n, 5);
    }

    #[test]
    fn test_reco_cron_config_invalid_json() {
        let result = RecoCronConfig::from_json("invalid");
        assert!(result.is_err());
    }
}
