// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 需求发现定时任务种子化
//!
//! 创建默认的需求发现定时任务配置：
//! - 每日凌晨 2:00 执行全平台需求扫描
//! - 每周一 9:00 执行周度需求汇总

#[cfg(test)]
use axagent_runtime_core::CronJobStatus;
use axagent_runtime_core::cron_job::now_millis;
use axagent_runtime_core::{CronJob, CronJobStore};
use std::sync::Arc;

const DEMAND_DISCOVERY_JOB_ID: &str = "opc-demand-discovery-daily";
const DEMAND_WEEKLY_JOB_ID: &str = "opc-demand-discovery-weekly";

/// 种子化需求发现定时任务到 CronJobStore
pub async fn seed_demand_discovery_crons(store: &Arc<CronJobStore>) -> Result<(), String> {
    // 1. 每日需求扫描任务（凌晨 2:00 UTC+8 = 18:00 UTC 前一天）
    // Cron 表达式: 0 2 * * * (每天凌晨 2:00)
    let mut daily_job = CronJob::new(
        "OPC 需求发现 - 每日扫描",
        "0 2 * * *",
        "执行全平台需求扫描，收集 Reddit/HackerNews/GitHub/猪八戒/闲鱼的最新需求线索",
        "每日凌晨 2:00 自动扫描各平台需求，评估商业价值",
    )
    .with_workflow_id("opc-demand-discovery".to_string())
    .with_task_type("opc-demand-discovery")
    .with_platform("all")
    .with_toolsets(vec!["opc_scanner".to_string(), "opc_evaluator".to_string()]);
    daily_job.id = DEMAND_DISCOVERY_JOB_ID.to_string();

    // 2. 周度需求汇总任务（每周一 9:00）
    // Cron 表达式: 0 9 * * 1 (每周一 9:00)
    let mut weekly_job = CronJob::new(
        "OPC 需求发现 - 周度汇总",
        "0 9 * * 1",
        "汇总本周需求线索，生成周度需求分析报告",
        "每周一汇总上周的需求发现结果，生成分析报告",
    )
    .with_workflow_id("opc-demand-discovery".to_string())
    .with_task_type("opc-demand-discovery")
    .with_platform("all")
    .with_toolsets(vec![
        "opc_scanner".to_string(),
        "opc_evaluator".to_string(),
        "opc_analysis".to_string(),
    ]);
    weekly_job.id = DEMAND_WEEKLY_JOB_ID.to_string();

    // 检查并添加/更新任务
    upsert_cron_job(store, daily_job).await?;
    upsert_cron_job(store, weekly_job).await?;

    tracing::info!("[opc-cron] 需求发现定时任务种子化完成");
    Ok(())
}

/// 插入或更新 CronJob（基于 ID）
async fn upsert_cron_job(store: &Arc<CronJobStore>, job: CronJob) -> Result<(), String> {
    let existing = store.get(&job.id).await;

    match existing {
        Some(_existing_job) => {
            // 更新现有任务的配置，但保留执行历史
            let job_id = job.id.clone();
            let updated = store
                .update(&job_id, |existing| {
                    existing.name = job.name;
                    existing.description = job.description;
                    existing.schedule = job.schedule;
                    existing.prompt = job.prompt;
                    existing.workflow_id = job.workflow_id;
                    existing.task_type = job.task_type;
                    existing.platform = job.platform;
                    existing.enabled_toolsets = job.enabled_toolsets;
                    existing.config = job.config;
                    existing.delivery = job.delivery;
                    existing.updated_at = now_millis();
                })
                .await;

            if updated {
                tracing::debug!("[opc-cron] 更新现有任务: {}", job.id);
            }
        },
        None => {
            // 添加新任务
            store.add(job).await;
            tracing::debug!("[opc-cron] 创建新任务");
        },
    }

    Ok(())
}

/// 获取默认的需求发现 Cron 表达式选项
#[allow(dead_code)]
pub fn get_default_cron_options() -> Vec<CronOption> {
    vec![
        CronOption {
            label: "每日凌晨 2:00".to_string(),
            schedule: "0 2 * * *".to_string(),
            description: "每天凌晨执行一次全平台扫描".to_string(),
        },
        CronOption {
            label: "每日早上 9:00".to_string(),
            schedule: "0 9 * * *".to_string(),
            description: "每天早上执行一次扫描".to_string(),
        },
        CronOption {
            label: "每周一 9:00".to_string(),
            schedule: "0 9 * * 1".to_string(),
            description: "每周一执行周度汇总".to_string(),
        },
        CronOption {
            label: "每小时".to_string(),
            schedule: "0 * * * *".to_string(),
            description: "每小时执行一次（测试用）".to_string(),
        },
    ]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct CronOption {
    pub label: String,
    pub schedule: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_seed_demand_discovery_crons() {
        let store = Arc::new(CronJobStore::new_ephemeral());

        let result = seed_demand_discovery_crons(&store).await;
        assert!(result.is_ok());

        // 验证任务已创建
        let jobs = store.list().await;
        let demand_jobs: Vec<_> = jobs
            .iter()
            .filter(|j| j.task_type.as_deref() == Some("opc-demand-discovery"))
            .collect();

        assert_eq!(demand_jobs.len(), 2);

        // 验证每日任务
        let daily = store.get(DEMAND_DISCOVERY_JOB_ID).await;
        assert!(daily.is_some());
        let daily = daily.unwrap();
        assert_eq!(daily.schedule, "0 2 * * *");
        assert_eq!(daily.workflow_id.as_deref(), Some("opc-demand-discovery"));
        assert_eq!(daily.status, CronJobStatus::Active);

        // 验证周度任务
        let weekly = store.get(DEMAND_WEEKLY_JOB_ID).await;
        assert!(weekly.is_some());
        let weekly = weekly.unwrap();
        assert_eq!(weekly.schedule, "0 9 * * 1");
    }

    #[tokio::test]
    async fn test_seed_idempotent() {
        let store = Arc::new(CronJobStore::new_ephemeral());

        // 执行两次，应幂等
        seed_demand_discovery_crons(&store).await.unwrap();
        seed_demand_discovery_crons(&store).await.unwrap();

        let jobs = store.list().await;
        let demand_jobs: Vec<_> = jobs
            .iter()
            .filter(|j| j.task_type.as_deref() == Some("opc-demand-discovery"))
            .collect();

        // 应该还是 2 个任务（幂等更新）
        assert_eq!(demand_jobs.len(), 2);
    }

    #[test]
    fn test_get_default_cron_options() {
        let options = get_default_cron_options();
        assert_eq!(options.len(), 4);
        assert!(options.iter().any(|o| o.schedule == "0 2 * * *"));
        assert!(options.iter().any(|o| o.schedule == "0 9 * * 1"));
    }
}
