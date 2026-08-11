// SPDX-License-Identifier: AGPL-3.0-only

//! 同步调度器
//!
//! 管理同步任务的调度、重试和队列，
//! 支持定期同步、触发同步和同步优先级管理。

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use axagent_harness::device_sync::{SyncEngine, SyncResult, SyncType};
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::history_store::HistoryStore;
use crate::manager::DeviceStore;
use crate::permission_checker::PermissionChecker;
use crate::sync_engine::SyncEngineImpl;

/// 同步任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPriority {
    /// 低优先级（后台同步）
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级（用户触发）
    High,
    /// 紧急（立即执行）
    Urgent,
}

impl SyncPriority {
    fn weight(&self) -> u8 {
        match self {
            SyncPriority::Low => 0,
            SyncPriority::Normal => 1,
            SyncPriority::High => 2,
            SyncPriority::Urgent => 3,
        }
    }
}

/// 同步任务
#[derive(Debug, Clone)]
pub struct SyncTask {
    /// 任务 ID
    pub id: String,
    /// 目标设备 ID
    pub target_device_id: String,
    /// 同步类型
    pub sync_type: SyncType,
    /// 优先级
    pub priority: SyncPriority,
    /// 创建时间
    pub created_at: Instant,
    /// 最大重试次数
    pub max_retries: u32,
    /// 当前重试次数
    pub retry_count: u32,
    /// 最后执行结果
    pub last_result: Option<SyncResult>,
}

impl SyncTask {
    pub fn new(target_device_id: String, sync_type: SyncType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target_device_id,
            sync_type,
            priority: SyncPriority::Normal,
            created_at: Instant::now(),
            max_retries: 3,
            retry_count: 0,
            last_result: None,
        }
    }

    pub fn with_priority(mut self, priority: SyncPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 同步调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 执行间隔（毫秒）
    pub interval_ms: u64,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务队列最大长度
    pub max_queue_size: usize,
    /// 失败任务重试间隔（毫秒）
    pub retry_interval_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5000,
            max_concurrent_tasks: 2,
            max_queue_size: 100,
            retry_interval_ms: 30_000,
        }
    }
}

/// 同步调度器
pub struct SyncScheduler {
    config: SchedulerConfig,
    queue: Arc<RwLock<VecDeque<SyncTask>>>,
    running: Arc<RwLock<bool>>,
    sync_engine: Arc<SyncEngineImpl>,
    device_store: Arc<DeviceStore>,
    history_store: Arc<HistoryStore>,
    permission_checker: PermissionChecker,
}

impl SyncScheduler {
    pub fn new(
        config: SchedulerConfig,
        sync_engine: Arc<SyncEngineImpl>,
        device_store: Arc<DeviceStore>,
        history_store: Arc<HistoryStore>,
        permission_checker: PermissionChecker,
    ) -> Self {
        Self {
            config,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            running: Arc::new(RwLock::new(false)),
            sync_engine,
            device_store,
            history_store,
            permission_checker,
        }
    }

    /// 添加同步任务到队列
    pub async fn add_task(&self, task: SyncTask) -> Result<(), String> {
        let mut queue = self.queue.write().await;

        if queue.len() >= self.config.max_queue_size {
            // 队列已满，尝试移除最低优先级的任务
            if let Some(lowest_idx) =
                queue.iter().enumerate().min_by_key(|(_, t)| t.priority.weight()).map(|(i, _)| i)
            {
                queue.remove(lowest_idx);
            } else {
                return Err("同步任务队列已满".to_string());
            }
        }

        queue.push_back(task);
        Ok(())
    }

    /// 添加高优先级任务
    pub async fn add_urgent_task(
        &self,
        target_device_id: String,
        sync_type: SyncType,
    ) -> Result<(), String> {
        let task = SyncTask::new(target_device_id, sync_type)
            .with_priority(SyncPriority::Urgent)
            .with_max_retries(1);
        self.add_task(task).await
    }

    /// 触发调度执行
    pub async fn trigger_execute(&self) -> Result<Vec<SyncResult>, String> {
        let is_running = *self.running.read().await;
        if is_running {
            return Err("调度器正在运行".to_string());
        }

        *self.running.write().await = true;

        let results = self.execute_pending_tasks().await;

        *self.running.write().await = false;

        Ok(results)
    }

    /// 启动后台调度循环
    pub async fn start(&self) {
        let running = self.running.clone();
        let queue = self.queue.clone();
        let sync_engine = self.sync_engine.clone();
        let device_store = self.device_store.clone();
        let history_store = self.history_store.clone();
        let permission_checker = self.permission_checker.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(config.interval_ms)).await;

                let is_running = *running.read().await;
                if is_running {
                    continue;
                }

                *running.write().await = true;

                Self::execute_tasks_cycle(
                    &queue,
                    &sync_engine,
                    &device_store,
                    &history_store,
                    &permission_checker,
                    &config,
                )
                .await;

                *running.write().await = false;
            }
        });
    }

    /// 停止调度器
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// 获取队列中的任务数
    pub async fn queue_size(&self) -> usize {
        self.queue.read().await.len()
    }

    /// 获取所有待处理任务
    pub async fn get_pending_tasks(&self) -> Vec<SyncTask> {
        self.queue.read().await.iter().cloned().collect()
    }

    /// 清除所有待处理任务
    pub async fn clear_queue(&self) {
        self.queue.write().await.clear();
    }

    /// 移除指定任务
    pub async fn remove_task(&self, task_id: &str) -> bool {
        let mut queue = self.queue.write().await;
        let len_before = queue.len();
        queue.retain(|t| t.id != task_id);
        queue.len() < len_before
    }

    /// 获取调度器状态
    pub async fn get_status(&self) -> SchedulerStatus {
        let queue = self.queue.read().await;
        SchedulerStatus {
            is_running: *self.running.read().await,
            pending_tasks: queue.len(),
            urgent_tasks: queue.iter().filter(|t| t.priority == SyncPriority::Urgent).count(),
            failed_tasks: queue.iter().filter(|t| !t.can_retry()).count(),
        }
    }

    // ─── 内部方法 ────────────────────────────────────────────────────────

    async fn execute_pending_tasks(&self) -> Vec<SyncResult> {
        Self::execute_tasks_cycle(
            &self.queue,
            &self.sync_engine,
            &self.device_store,
            &self.history_store,
            &self.permission_checker,
            &self.config,
        )
        .await
    }

    async fn execute_tasks_cycle(
        queue: &Arc<RwLock<VecDeque<SyncTask>>>,
        sync_engine: &Arc<SyncEngineImpl>,
        _device_store: &Arc<DeviceStore>,
        history_store: &Arc<HistoryStore>,
        permission_checker: &PermissionChecker,
        config: &SchedulerConfig,
    ) -> Vec<SyncResult> {
        let mut results = Vec::new();
        let mut tasks_to_requeue = Vec::new();
        let mut completed_ids = Vec::new();

        // 获取待执行任务（按优先级排序）
        let pending: Vec<SyncTask> = {
            let queue_lock = queue.read().await;
            let mut tasks: Vec<_> = queue_lock.iter().cloned().collect();
            tasks.sort_by_key(|b| std::cmp::Reverse(b.priority.weight()));
            tasks.truncate(config.max_concurrent_tasks);
            tasks
        };

        for mut task in pending {
            // 检查设备权限
            let permission_result = permission_checker.check_pull(&task.target_device_id).await;

            if !permission_result.is_allowed() {
                history_store
                    .log_audit(
                        axagent_harness::device_sync::AuditAction::SyncFailed,
                        "sync",
                        &task.id,
                        "scheduler",
                        false,
                        Some(format!("权限检查失败: {:?}", permission_result)),
                        None,
                    )
                    .await;
                completed_ids.push(task.id.clone());
                continue;
            }

            // 执行同步
            let result = match task.sync_type {
                SyncType::Full => sync_engine.full_sync(&task.target_device_id).await,
                SyncType::Incremental | SyncType::Manual | SyncType::Scheduled => {
                    sync_engine.incremental_sync(&task.target_device_id).await
                },
            };

            match &result {
                Ok(sync_result) => {
                    history_store
                        .log_audit(
                            if sync_result.success {
                                axagent_harness::device_sync::AuditAction::SyncCompleted
                            } else {
                                axagent_harness::device_sync::AuditAction::SyncFailed
                            },
                            "sync",
                            &task.id,
                            "scheduler",
                            sync_result.success,
                            serde_json::to_string(sync_result).ok(),
                            sync_result.error_message.as_ref().map(|s| s.to_string()),
                        )
                        .await;

                    if sync_result.success {
                        completed_ids.push(task.id.clone());
                    } else {
                        task.increment_retry();
                        if task.can_retry() {
                            tasks_to_requeue.push(task);
                        } else {
                            completed_ids.push(task.id.clone());
                        }
                    }
                },
                Err(e) => {
                    history_store
                        .log_audit(
                            axagent_harness::device_sync::AuditAction::SyncFailed,
                            "sync",
                            &task.id,
                            "scheduler",
                            false,
                            None,
                            Some(e.clone()),
                        )
                        .await;

                    task.increment_retry();
                    if task.can_retry() {
                        tasks_to_requeue.push(task);
                    } else {
                        completed_ids.push(task.id.clone());
                    }
                },
            }

            if let Ok(r) = result {
                results.push(r);
            }
        }

        // 更新队列
        {
            let mut queue_lock = queue.write().await;
            // 移除已完成的任务
            queue_lock.retain(|t| !completed_ids.contains(&t.id));
            // 重新加入需要重试的任务
            for task in tasks_to_requeue {
                queue_lock.push_back(task);
            }
        }

        results
    }
}

/// 调度器状态
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerStatus {
    pub is_running: bool,
    pub pending_tasks: usize,
    pub urgent_tasks: usize,
    pub failed_tasks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission_store::PermissionStore;
    use crate::sync_engine::ChangeLogStore;

    async fn setup_scheduler() -> SyncScheduler {
        let change_log = Arc::new(ChangeLogStore::new());
        let device_store = Arc::new(DeviceStore::new());
        let history_store = Arc::new(HistoryStore::new());
        let sync_engine = Arc::new(SyncEngineImpl::new(
            change_log,
            device_store.clone(),
            history_store.clone(),
            "test-device".to_string(),
        ));
        let permission_store = Arc::new(RwLock::new(PermissionStore::new()));
        let permission_checker = PermissionChecker::new(permission_store.clone());

        // 初始化设备权限
        {
            let store = permission_store.write().await;
            store
                .init_device_permissions("device-1", axagent_harness::device_sync::TrustLevel::Full)
                .await;
        }

        SyncScheduler::new(
            SchedulerConfig::default(),
            sync_engine,
            device_store,
            history_store,
            permission_checker,
        )
    }

    #[tokio::test]
    async fn test_add_task() {
        let scheduler = setup_scheduler().await;

        let task = SyncTask::new("device-1".to_string(), SyncType::Incremental);
        scheduler.add_task(task).await.expect("测试：异步操作应成功");

        assert_eq!(scheduler.queue_size().await, 1);
    }

    #[tokio::test]
    async fn test_add_urgent_task() {
        let scheduler = setup_scheduler().await;

        scheduler
            .add_urgent_task("device-1".to_string(), SyncType::Incremental)
            .await
            .expect("测试：异步操作应成功");

        let status = scheduler.get_status().await;
        assert_eq!(status.urgent_tasks, 1);
    }

    #[tokio::test]
    async fn test_execute_tasks() {
        let scheduler = setup_scheduler().await;

        scheduler
            .add_task(SyncTask::new("device-1".to_string(), SyncType::Incremental))
            .await
            .expect("测试应成功");

        let results = scheduler.trigger_execute().await.expect("测试：异步操作应成功");
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let scheduler = setup_scheduler().await;

        // 添加不同优先级的任务
        scheduler
            .add_task(
                SyncTask::new("device-1".to_string(), SyncType::Incremental)
                    .with_priority(SyncPriority::Low),
            )
            .await
            .expect("测试应成功");
        scheduler
            .add_urgent_task("device-1".to_string(), SyncType::Full)
            .await
            .expect("测试：异步操作应成功");
        scheduler
            .add_task(
                SyncTask::new("device-1".to_string(), SyncType::Incremental)
                    .with_priority(SyncPriority::Normal),
            )
            .await
            .expect("测试应成功");

        let pending = scheduler.get_pending_tasks().await;
        assert_eq!(pending.len(), 3);

        // 执行调度器，紧急任务应该先执行
        let _ = scheduler.trigger_execute().await;
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let scheduler = setup_scheduler().await;

        scheduler
            .add_task(SyncTask::new("device-1".to_string(), SyncType::Incremental))
            .await
            .expect("测试应成功");
        scheduler.clear_queue().await;
        assert_eq!(scheduler.queue_size().await, 0);
    }
}
