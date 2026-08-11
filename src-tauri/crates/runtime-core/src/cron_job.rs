// SPDX-License-Identifier: AGPL-3.0-only

//! 统一定时任务数据模型 — 合并 CronJob + ScheduledTaskService。
//!
//! CronJob + CronJobStore — 供 runtime/cron 调度器、tools/cron.rs 工具、
//! 和 src/commands/ Tauri 命令共用。

use chrono::Timelike;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 确保 cron_jobs 表存在（幂等）
async fn ensure_table(db: &DatabaseConnection) {
    let _ = db
        .execute_unprepared(
            "CREATE TABLE IF NOT EXISTS cron_jobs (\
             id TEXT PRIMARY KEY NOT NULL, \
             data TEXT NOT NULL)",
        )
        .await;
}

/// 确保 cron_job_history 表存在（幂等）
async fn ensure_history_table(db: &DatabaseConnection) {
    let _ = db
        .execute_unprepared(
            "CREATE TABLE IF NOT EXISTS cron_job_history (\
             id TEXT PRIMARY KEY NOT NULL, \
             task_id TEXT NOT NULL, \
             started_at INTEGER NOT NULL, \
             completed_at INTEGER, \
             success INTEGER NOT NULL DEFAULT 0, \
             output TEXT, \
             error TEXT, \
             duration_ms INTEGER NOT NULL DEFAULT 0)",
        )
        .await;
}

// ── CronJob 最大合集 ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Cron 表达式 (如 "0 9 * * *") 或原始调度字符串
    pub schedule: String,
    /// 任务指令 / prompt
    pub prompt: String,
    /// 关联的工作流 ID
    pub workflow_id: Option<String>,
    /// 任务类型标签 (用于模板查找)
    pub task_type: Option<String>,
    /// 消息平台
    pub platform: Option<String>,
    /// 启用的工具集
    pub enabled_toolsets: Option<Vec<String>>,
    /// 三态状态
    pub status: CronJobStatus,
    /// 是否循环 (false = 一次性)
    pub recurring: bool,
    /// 执行次数
    pub run_count: u32,
    /// 上次执行时间 (epoch millis)
    pub last_run_at: Option<i64>,
    /// 上次执行结果
    pub last_result: Option<TaskRunResult>,
    /// 下次执行时间 (epoch millis)
    pub next_run_at: Option<i64>,
    /// 重试/超时配置
    pub config: TaskConfig,
    /// 创建/更新时间
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CronJobStatus {
    Active,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub timeout_seconds: u32,
    pub retry_on_failure: bool,
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
    pub notification_enabled: bool,
    pub run_on_startup: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay_seconds: 60,
            notification_enabled: false,
            run_on_startup: false,
        }
    }
}

impl CronJob {
    pub fn new(name: &str, schedule: &str, prompt: &str, description: &str) -> Self {
        let now = now_millis();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            schedule: schedule.to_string(),
            prompt: prompt.to_string(),
            workflow_id: None,
            task_type: None,
            platform: None,
            enabled_toolsets: None,
            status: CronJobStatus::Active,
            recurring: true,
            run_count: 0,
            last_run_at: None,
            last_result: None,
            next_run_at: None,
            config: TaskConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_platform(mut self, platform: &str) -> Self {
        self.platform = Some(platform.to_string());
        self
    }

    pub fn with_toolsets(mut self, toolsets: Vec<String>) -> Self {
        self.enabled_toolsets = Some(toolsets);
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: String) -> Self {
        self.workflow_id = Some(workflow_id);
        self
    }

    pub fn with_task_type(mut self, task_type: &str) -> Self {
        self.task_type = Some(task_type.to_string());
        self
    }

    pub fn is_active(&self) -> bool {
        self.status == CronJobStatus::Active
    }
}

/// 根据 cron 表达式和当前时间戳（毫秒）计算下次执行时间。
/// 返回 None 表示无法计算（非循环任务或无效表达式）。
fn calculate_next_run(schedule: &str, now_ms: i64) -> Option<i64> {
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    let now_sec = now_ms / 1000;

    // 处理 */N 间隔模式（仅分钟字段）
    if let Some(step) = parts[0].strip_prefix("*/")
        && let Ok(interval) = step.parse::<i64>()
    {
        let next = now_sec + interval * 60;
        return Some(next * 1000);
    }

    // 处理 */N 间隔模式（仅小时字段，分钟为 0 时）
    if parts[0] == "0"
        && let Some(step) = parts[1].strip_prefix("*/")
        && let Ok(interval) = step.parse::<i64>()
    {
        let next = now_sec + interval * 3600;
        return Some(next * 1000);
    }

    // 对于具体时间点（如 "0 9 * * *"），计算下一次触发时间
    // 使用简单的 UTC 时间计算
    if let (Ok(minute), Ok(hour)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
        let target_sec = hour * 3600 + minute * 60;
        let day_sec = 86400;
        let current_day_sec = {
            let dt = chrono::DateTime::from_timestamp(now_sec, 0).unwrap_or_else(|| {
                chrono::DateTime::from_timestamp(0, 0).expect("Cron：Unix epoch 0 应始终有效")
            });
            (dt.hour() as i64) * 3600 + (dt.minute() as i64) * 60
        };

        let offset = if target_sec > current_day_sec {
            target_sec - current_day_sec
        } else {
            day_sec - current_day_sec + target_sec
        };
        return Some((now_sec + offset) * 1000);
    }

    // 兜底：30 秒后
    Some(now_ms + 30_000)
}

// ── CronJobStore ──────────────────────────────────────────────

pub struct CronJobStore {
    jobs: Arc<RwLock<Vec<CronJob>>>,
    db: Arc<DatabaseConnection>,
}

impl CronJobStore {
    /// 纯内存模式（测试/降级用），不含 DB 持久化。
    pub fn new_ephemeral() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(Vec::new())),
            db: Arc::new(DatabaseConnection::default()),
        }
    }

    /// 构造 CronJobStore 并自动从 DB 恢复已持久化的任务。
    ///
    /// 恢复的任务会重新计算 `next_run_at`：设为 0 使其在下次调度时立即触发，
    /// 避免因重启导致错过的任务被无限推后。
    pub async fn new(db: Arc<DatabaseConnection>) -> Self {
        ensure_table(&db).await;
        ensure_history_table(&db).await;

        let jobs = Self::load_from_db(&db).await;

        let count = jobs.len();
        if count > 0 {
            info!("[CronJobStore] 从 DB 恢复了 {count} 个定时任务");
        }

        Self { jobs: Arc::new(RwLock::new(jobs)), db }
    }

    /// 从 DB 加载全部任务，恢复时重置 next_run_at 为 0（立即触发）。
    async fn load_from_db(db: &DatabaseConnection) -> Vec<CronJob> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT data FROM cron_jobs ORDER BY json_extract(data, '$.created_at') ASC",
            [],
        );
        let rows = db.query_all_raw(stmt).await;

        let Ok(rows) = rows else {
            return Vec::new();
        };

        let now = now_millis();
        rows.into_iter()
            .filter_map(|row| {
                let json_str = row.try_get_by_index::<String>(0).ok()?;
                let mut job: CronJob = serde_json::from_str(&json_str).ok()?;
                // 重启后重置 next_run_at：活跃任务立即触发，暂停/禁用保持不变
                if job.is_active() {
                    job.next_run_at = Some(0);
                }
                job.updated_at = now;
                Some(job)
            })
            .collect()
    }

    pub async fn add(&self, job: CronJob) -> String {
        let id = job.id.clone();
        // 写入 DB（参数化查询，避免 SQL 注入）
        if let Ok(json) = serde_json::to_string(&job) {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT OR REPLACE INTO cron_jobs (id, data) VALUES (?, ?)",
                [id.clone().into(), json.into()],
            );
            let _ = self.db.execute_raw(stmt).await;
        }
        // 写入内存
        let mut jobs = self.jobs.write().await;
        jobs.push(job);
        id
    }

    pub async fn remove(&self, id: &str) -> bool {
        // 删除 DB 记录（参数化查询）
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "DELETE FROM cron_jobs WHERE id = ?",
            [id.into()],
        );
        let _ = self.db.execute_raw(stmt).await;
        // 删除内存
        let mut jobs = self.jobs.write().await;
        let len = jobs.len();
        jobs.retain(|j| j.id != id);
        jobs.len() < len
    }

    pub async fn get(&self, id: &str) -> Option<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().find(|j| j.id == id).cloned()
    }

    pub async fn update(&self, id: &str, updater: impl FnOnce(&mut CronJob)) -> bool {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            updater(job);
            job.updated_at = now_millis();
            // 同步写入 DB（参数化查询）
            if let Ok(json) = serde_json::to_string(job) {
                let stmt = Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "INSERT OR REPLACE INTO cron_jobs (id, data) VALUES (?, ?)",
                    [job.id.clone().into(), json.into()],
                );
                let _ = self.db.execute_raw(stmt).await;
            }
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<CronJob> {
        self.jobs.read().await.clone()
    }

    pub async fn list_active(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().filter(|j| j.is_active()).cloned().collect()
    }

    pub async fn list_due(&self) -> Vec<CronJob> {
        let now = now_millis();
        let jobs = self.jobs.read().await;
        jobs.iter()
            .filter(|j| j.is_active() && j.next_run_at.is_none_or(|next| now >= next))
            .cloned()
            .collect()
    }

    pub async fn set_status(&self, id: &str, status: CronJobStatus) -> bool {
        self.update(id, |job| {
            job.status = status;
        })
        .await
    }

    pub async fn record_run(&self, id: &str, result: TaskRunResult) -> bool {
        let now = now_millis();
        let updated = self
            .update(id, |job| {
                job.last_run_at = Some(now);
                job.run_count += 1;
                job.last_result = Some(result.clone());
                job.next_run_at = calculate_next_run(&job.schedule, now);
            })
            .await;

        // 同时保存到执行历史表
        if updated {
            let history_id = uuid::Uuid::new_v4().to_string();
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO cron_job_history (id, task_id, started_at, completed_at, success, output, error, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    history_id.into(),
                    id.to_string().into(),
                    result.executed_at.into(),
                    (result.executed_at + result.duration_ms as i64).into(),
                    (result.success as i32).into(),
                    result.output.clone().unwrap_or_default().into(),
                    result.error.clone().unwrap_or_default().into(),
                    (result.duration_ms as i64).into(),
                ],
            );
            let _ = self.db.execute_raw(stmt).await;
        }

        updated
    }

    pub async fn count(&self) -> usize {
        self.jobs.read().await.len()
    }

    /// 批量加载任务（用于从 DB 恢复）
    pub async fn load_batch(&self, jobs: Vec<CronJob>) {
        let mut store = self.jobs.write().await;
        *store = jobs;
    }

    /// 从 DB 重新加载所有任务（刷新内存状态）
    pub async fn reload_from_db(&self) -> usize {
        let jobs = Self::load_from_db(&self.db).await;
        let count = jobs.len();
        let mut store = self.jobs.write().await;
        *store = jobs;
        count
    }

    /// 查询指定任务的执行历史（最近 50 条，倒序）
    pub async fn get_execution_history(&self, task_id: &str) -> Vec<ExecutionRecord> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, task_id, started_at, completed_at, success, output, error, duration_ms \
             FROM cron_job_history WHERE task_id = ? ORDER BY started_at DESC LIMIT 50",
            [task_id.into()],
        );
        let rows = self.db.query_all_raw(stmt).await;
        let Ok(rows) = rows else {
            return Vec::new();
        };
        rows.into_iter()
            .filter_map(|row| {
                Some(ExecutionRecord {
                    id: row.try_get_by_index::<String>(0).ok()?,
                    task_id: row.try_get_by_index::<String>(1).ok()?,
                    started_at: row.try_get_by_index::<i64>(2).ok()?,
                    completed_at: row.try_get_by_index::<i64>(3).ok(),
                    success: row.try_get_by_index::<i32>(4).ok()? != 0,
                    output: row.try_get_by_index::<String>(5).ok(),
                    error: row.try_get_by_index::<String>(6).ok(),
                    duration_ms: row.try_get_by_index::<i64>(7).ok()?,
                })
            })
            .collect()
    }
}

/// 执行历史记录（供前端查询）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
}

// ── Harness trait 实现 ──

impl From<axagent_harness::tool_service::CronJobData> for CronJob {
    fn from(data: axagent_harness::tool_service::CronJobData) -> Self {
        let now = now_millis();
        Self {
            id: data.name.clone(),
            name: data.name,
            description: data.description,
            schedule: data.schedule,
            prompt: data.prompt,
            workflow_id: None,
            task_type: None,
            platform: None,
            enabled_toolsets: None,
            status: if data.is_active {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
            recurring: true,
            run_count: data.run_count,
            last_run_at: None,
            last_result: None,
            next_run_at: None,
            config: TaskConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<&CronJob> for axagent_harness::tool_service::CronJobData {
    fn from(job: &CronJob) -> Self {
        Self {
            name: job.name.clone(),
            schedule: job.schedule.clone(),
            prompt: job.prompt.clone(),
            description: job.description.clone(),
            is_active: job.is_active(),
            run_count: job.run_count,
        }
    }
}

#[async_trait::async_trait]
impl axagent_harness::tool_service::CronJobStore for CronJobStore {
    async fn add(&self, job: axagent_harness::tool_service::CronJobData) -> String {
        let cron_job: CronJob = job.into();
        CronJobStore::add(self, cron_job).await
    }

    async fn remove(&self, id: &str) -> bool {
        CronJobStore::remove(self, id).await
    }

    async fn get(&self, id: &str) -> Option<axagent_harness::tool_service::CronJobData> {
        CronJobStore::get(self, id)
            .await
            .map(|job| axagent_harness::tool_service::CronJobData::from(&job))
    }

    async fn list(&self) -> Vec<axagent_harness::tool_service::CronJobData> {
        CronJobStore::list(self)
            .await
            .into_iter()
            .map(|job| axagent_harness::tool_service::CronJobData::from(&job))
            .collect()
    }

    async fn count(&self) -> usize {
        CronJobStore::count(self).await
    }
}
