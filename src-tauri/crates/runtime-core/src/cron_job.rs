//! 统一定时任务数据模型 — 合并 CronJob + ScheduledTaskService。
//!
//! CronJob + CronJobStore — 供 runtime/cron 调度器、tools/cron.rs 工具、
//! 和 src/commands/ Tauri 命令共用。

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

        let jobs = Self::load_from_db(&db).await;

        let count = jobs.len();
        if count > 0 {
            info!("[CronJobStore] 从 DB 恢复了 {count} 个定时任务");
        }

        Self {
            jobs: Arc::new(RwLock::new(jobs)),
            db,
        }
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
        // 写入 DB
        if let Ok(json) = serde_json::to_string(&job) {
            let safe_id = id.replace('\'', "''");
            let safe_json = json.replace('\'', "''");
            let _ = self
                .db
                .execute_unprepared(&format!(
                    "INSERT OR REPLACE INTO cron_jobs (id, data) VALUES ('{safe_id}', '{safe_json}')",
                ))
                .await;
        }
        // 写入内存
        let mut jobs = self.jobs.write().await;
        jobs.push(job);
        id
    }

    pub async fn remove(&self, id: &str) -> bool {
        // 删除 DB 记录
        let safe_id = id.replace('\'', "''");
        let _ = self
            .db
            .execute_unprepared(&format!("DELETE FROM cron_jobs WHERE id = '{safe_id}'"))
            .await;
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
            // 同步写入 DB
            if let Ok(json) = serde_json::to_string(job) {
                let safe_id = job.id.replace('\'', "''");
                let safe_json = json.replace('\'', "''");
                let _ = self
                    .db
                    .execute_unprepared(&format!(
                        "INSERT OR REPLACE INTO cron_jobs (id, data) VALUES ('{safe_id}', '{safe_json}')",
                    ))
                    .await;
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
        self.update(id, |job| {
            job.last_run_at = Some(now);
            job.run_count += 1;
            job.last_result = Some(result);
        })
        .await
    }

    pub async fn count(&self) -> usize {
        self.jobs.read().await.len()
    }

    /// 批量加载任务（用于从 DB 恢复）
    pub async fn load_batch(&self, jobs: Vec<CronJob>) {
        let mut store = self.jobs.write().await;
        *store = jobs;
    }
}
