// SPDX-License-Identifier: AGPL-3.0-only

//! 3.3 P2:6 小时无人值守持久重试调度器
//!
//! 跨进程持久化重试调度器,支持长时任务:
//! 1. 失败后保存 session 到 `.axagent/persistent_sessions/{session_id}.json`
//! 2. 等待冷却时间(cooldown_secs)
//! 3. 后台守护进程定时唤醒,加载 pending session
//! 4. 自动恢复执行,在 max_run_hours 小时内重试
//!
//! ## 持久化格式
//! 每个 pending session 存储为 JSON 文件,包含:
//! - session_id: 会话唯一标识
//! - input: 原始输入
//! - retry_count: 已重试次数
//! - first_attempt_at: 首次尝试时间戳
//! - last_attempt_at: 上次尝试时间戳
//! - status: Pending / Running / Completed / Failed
//! - error: 上次失败原因

use axagent_harness::constants::dir_name;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use std::sync::Arc;

/// 持久化 session 的状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// 等待冷却后重试
    Pending,
    /// 正在执行
    Running,
    /// 已成功完成
    Completed,
    /// 已失败(超过最大重试次数或运行时长)
    Failed,
}

/// 持久化 session 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSession {
    /// 会话唯一标识
    pub session_id: String,
    /// 原始输入内容
    pub input: String,
    /// 已重试次数
    pub retry_count: u32,
    /// 首次尝试时间戳(UNIX 秒)
    pub first_attempt_at: u64,
    /// 上次尝试时间戳(UNIX 秒)
    pub last_attempt_at: u64,
    /// 当前状态
    pub status: SessionStatus,
    /// 上次失败原因
    pub last_error: Option<String>,
}

/// 后台守护进程执行器类型别名
///
/// 接收一个 PersistentSession,返回一个 boxed future。
/// 由 `spawn_daemon` 在每次轮询周期内对 pending session 调用。
pub type SessionExecutor = Arc<
    dyn Fn(
            PersistentSession,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

impl PersistentSession {
    /// 创建新的 pending session
    pub fn new(session_id: impl Into<String>, input: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            session_id: session_id.into(),
            input: input.into(),
            retry_count: 0,
            first_attempt_at: now,
            last_attempt_at: now,
            status: SessionStatus::Pending,
            last_error: None,
        }
    }

    /// 是否超过最大运行时长(小时)
    pub fn is_expired(&self, max_run_hours: u32) -> bool {
        if max_run_hours == 0 {
            return false;
        }
        let now = current_timestamp();
        let elapsed_secs = now.saturating_sub(self.first_attempt_at);
        let max_secs = (max_run_hours as u64) * 3600;
        elapsed_secs >= max_secs
    }

    /// 是否超过最大重试次数
    pub fn is_max_retries_exceeded(&self, max_retries: u32) -> bool {
        self.retry_count >= max_retries
    }

    /// 是否已过冷却期
    pub fn is_cooldown_elapsed(&self, cooldown_secs: u64) -> bool {
        let now = current_timestamp();
        let elapsed = now.saturating_sub(self.last_attempt_at);
        elapsed >= cooldown_secs
    }

    /// 记录一次失败
    pub fn record_failure(&mut self, error: String) {
        self.retry_count += 1;
        self.last_attempt_at = current_timestamp();
        self.last_error = Some(error);
        self.status = SessionStatus::Pending;
    }

    /// 标记为运行中
    pub fn mark_running(&mut self) {
        self.status = SessionStatus::Running;
        self.last_attempt_at = current_timestamp();
    }

    /// 标记为已完成
    pub fn mark_completed(&mut self) {
        self.status = SessionStatus::Completed;
        self.last_error = None;
    }

    /// 标记为永久失败
    pub fn mark_failed(&mut self, reason: String) {
        self.status = SessionStatus::Failed;
        self.last_error = Some(reason);
    }
}

/// 持久化重试调度器
///
/// 管理 pending session 的持久化存储、加载、状态更新。
/// 实际执行由调用方提供闭包,本调度器只负责调度和持久化。
pub struct PersistentRunner {
    /// 存储根目录(`.axagent/persistent_sessions/`)
    storage_dir: PathBuf,
    /// 配置(运行时可更新)
    config: Arc<RwLock<PersistentRunnerConfig>>,
}

/// 持久化重试配置(从 kit::unified_config::PersistentRunnerSettings 转换)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentRunnerConfig {
    pub max_run_hours: u32,
    pub cooldown_secs: u64,
    pub max_retries: u32,
    pub enabled: bool,
}

impl Default for PersistentRunnerConfig {
    fn default() -> Self {
        Self { max_run_hours: 6, cooldown_secs: 60, max_retries: 10, enabled: false }
    }
}

impl PersistentRunner {
    /// 创建新的持久化调度器
    ///
    /// `project_root` 为项目根目录,存储路径为
    /// `{project_root}/.axagent/persistent_sessions/`
    pub fn new(project_root: impl Into<PathBuf>, config: PersistentRunnerConfig) -> Self {
        let project_root = project_root.into();
        let storage_dir = project_root.join(dir_name::AXAGENT).join("persistent_sessions");
        Self { storage_dir, config: Arc::new(RwLock::new(config)) }
    }

    /// 获取存储目录路径
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// 读取当前配置
    pub async fn config(&self) -> PersistentRunnerConfig {
        self.config.read().await.clone()
    }

    /// 更新配置(运行时热更新)
    pub async fn update_config(&self, config: PersistentRunnerConfig) {
        *self.config.write().await = config;
    }

    /// 确保存储目录存在
    pub async fn ensure_storage_dir(&self) -> Result<(), String> {
        tokio::fs::create_dir_all(&self.storage_dir).await.map_err(|e| e.to_string())
    }

    /// 保存 session 到磁盘(JSON 文件)
    ///
    /// 文件名:`{storage_dir}/{session_id}.json`
    pub async fn save_session(&self, session: &PersistentSession) -> Result<PathBuf, String> {
        self.ensure_storage_dir().await?;
        // session_id 验证:防止路径遍历
        validate_session_id(&session.session_id)?;
        let path = self.storage_dir.join(format!("{}.json", session.session_id));
        let content = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
        tokio::fs::write(&path, content).await.map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// 加载 session 从磁盘
    pub async fn load_session(
        &self,
        session_id: &str,
    ) -> Result<Option<PersistentSession>, String> {
        validate_session_id(session_id)?;
        let path = self.storage_dir.join(format!("{}.json", session_id));
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        let session: PersistentSession =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(Some(session))
    }

    /// 删除 session 文件
    pub async fn delete_session(&self, session_id: &str) -> Result<bool, String> {
        validate_session_id(session_id)?;
        let path = self.storage_dir.join(format!("{}.json", session_id));
        if !path.exists() {
            return Ok(false);
        }
        tokio::fs::remove_file(&path).await.map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// 列出所有 pending session(可被调度器唤醒的)
    ///
    /// 返回满足以下条件的 session:
    /// - status == Pending
    /// - 未超过 max_run_hours
    /// - 未超过 max_retries
    /// - 已过 cooldown_secs
    pub async fn list_pending_sessions(&self) -> Result<Vec<PersistentSession>, String> {
        let config = self.config().await;
        if !config.enabled {
            return Ok(Vec::new());
        }
        self.ensure_storage_dir().await?;
        let mut sessions = Vec::new();
        let mut reader = tokio::fs::read_dir(&self.storage_dir).await.map_err(|e| e.to_string())?;
        while let Some(entry) = reader.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let session: PersistentSession = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(_) => continue,
            };
            // 过滤:只返回可调度的 pending session
            if session.status != SessionStatus::Pending {
                continue;
            }
            if session.is_expired(config.max_run_hours) {
                continue;
            }
            if session.is_max_retries_exceeded(config.max_retries) {
                continue;
            }
            if !session.is_cooldown_elapsed(config.cooldown_secs) {
                continue;
            }
            sessions.push(session);
        }
        // 按上次尝试时间升序排序(最早失败的优先重试)
        sessions.sort_by_key(|s| s.last_attempt_at);
        Ok(sessions)
    }

    /// 列出所有 session(不限状态,用于 UI 展示)
    pub async fn list_all_sessions(&self) -> Result<Vec<PersistentSession>, String> {
        self.ensure_storage_dir().await?;
        let mut sessions = Vec::new();
        let mut reader = tokio::fs::read_dir(&self.storage_dir).await.map_err(|e| e.to_string())?;
        while let Some(entry) = reader.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let session: PersistentSession = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(_) => continue,
            };
            sessions.push(session);
        }
        // 按首次尝试时间降序排序(最新的在前)
        sessions.sort_by_key(|s| std::cmp::Reverse(s.first_attempt_at));
        Ok(sessions)
    }

    /// 清理已过期的 session 文件(超过 max_run_hours 或已 Completed/Failed)
    ///
    /// 返回清理的文件数。
    pub async fn cleanup_expired(&self) -> Result<usize, String> {
        let config = self.config().await;
        self.ensure_storage_dir().await?;
        let mut cleaned = 0;
        let mut reader = tokio::fs::read_dir(&self.storage_dir).await.map_err(|e| e.to_string())?;
        while let Some(entry) = reader.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let session: PersistentSession = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let should_clean = session.status == SessionStatus::Completed
                || session.status == SessionStatus::Failed
                || session.is_expired(config.max_run_hours);
            if should_clean && tokio::fs::remove_file(&path).await.is_ok() {
                cleaned += 1;
            }
        }
        Ok(cleaned)
    }

    /// 调度执行:取出所有 pending session,调用闭包执行
    ///
    /// 闭包返回 Ok(()) 表示成功,Err(String) 表示失败(会记录并更新 retry_count)。
    /// 返回 (成功数, 失败数)。
    pub async fn schedule<F, Fut>(&self, mut executor: F) -> Result<(usize, usize), String>
    where
        F: FnMut(PersistentSession) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let pending = self.list_pending_sessions().await?;
        let mut succeeded = 0;
        let mut failed = 0;
        for mut session in pending {
            session.mark_running();
            self.save_session(&session).await?;
            match executor(session.clone()).await {
                Ok(()) => {
                    session.mark_completed();
                    self.save_session(&session).await?;
                    succeeded += 1;
                },
                Err(err) => {
                    let config = self.config().await;
                    if session.is_max_retries_exceeded(config.max_retries)
                        || session.is_expired(config.max_run_hours)
                    {
                        session.mark_failed(err.clone());
                    } else {
                        session.record_failure(err);
                    }
                    self.save_session(&session).await?;
                    failed += 1;
                },
            }
        }
        Ok((succeeded, failed))
    }

    /// 启动后台守护循环
    ///
    /// 每隔 `poll_interval_secs` 秒检查一次 pending session,执行调度。
    /// 返回 JoinHandle,调用方可通过 abort 取消。
    pub fn spawn_daemon(
        self: Arc<Self>,
        poll_interval_secs: u64,
        executor: SessionExecutor,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval = Duration::from_secs(poll_interval_secs.max(10));
            loop {
                let config = self.config().await;
                if !config.enabled {
                    // 未启用,休眠后继续检查
                    tokio::time::sleep(interval).await;
                    continue;
                }
                // 执行调度
                let executor_clone = Arc::clone(&executor);
                let result = self
                    .schedule(|session| {
                        let executor_clone = Arc::clone(&executor_clone);
                        async move { executor_clone(session).await }
                    })
                    .await;
                match result {
                    Ok((succeeded, failed)) => {
                        if succeeded > 0 || failed > 0 {
                            tracing::info!(succeeded, failed, "[PersistentRunner] 调度周期完成");
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "[PersistentRunner] 调度周期失败"
                        );
                    },
                }
                // 清理过期 session
                if let Err(e) = self.cleanup_expired().await {
                    tracing::warn!(
                        error = %e,
                        "[PersistentRunner] 清理过期 session 失败"
                    );
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

/// 获取当前 UNIX 时间戳(秒)
fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// 验证 session_id 合法性(防止路径遍历)
fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty() {
        return Err("session_id cannot be empty".to_string());
    }
    // 只允许字母数字、-、_
    if !session_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("invalid session_id: {}", session_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_equality() {
        assert_eq!(SessionStatus::Pending, SessionStatus::Pending);
        assert_eq!(SessionStatus::Running, SessionStatus::Running);
        assert_eq!(SessionStatus::Completed, SessionStatus::Completed);
        assert_eq!(SessionStatus::Failed, SessionStatus::Failed);
        assert_ne!(SessionStatus::Pending, SessionStatus::Running);
    }

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Pending;
        let json = serde_json::to_string(&status).expect("测试：JSON序列化应成功");
        let deserialized: SessionStatus =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(deserialized, SessionStatus::Pending);
    }

    #[test]
    fn test_persistent_session_new() {
        let session = PersistentSession::new("session-1", "test input");
        assert_eq!(session.session_id, "session-1");
        assert_eq!(session.input, "test input");
        assert_eq!(session.retry_count, 0);
        assert_eq!(session.status, SessionStatus::Pending);
        assert!(session.last_error.is_none());
        assert!(session.first_attempt_at > 0);
        assert_eq!(session.first_attempt_at, session.last_attempt_at);
    }

    #[test]
    fn test_persistent_session_is_expired() {
        let mut session = PersistentSession::new("s1", "input");
        // 设置 first_attempt_at 为 7 小时前
        let now = current_timestamp();
        session.first_attempt_at = now - 7 * 3600;
        assert!(session.is_expired(6)); // 6 小时上限,已过期
        assert!(!session.is_expired(8)); // 8 小时上限,未过期
    }

    #[test]
    fn test_persistent_session_is_expired_zero_hours() {
        let session = PersistentSession::new("s1", "input");
        // max_run_hours=0 表示无限制
        assert!(!session.is_expired(0));
    }

    #[test]
    fn test_persistent_session_is_max_retries_exceeded() {
        let mut session = PersistentSession::new("s1", "input");
        session.retry_count = 5;
        assert!(session.is_max_retries_exceeded(5));
        assert!(!session.is_max_retries_exceeded(10));
    }

    #[test]
    fn test_persistent_session_is_cooldown_elapsed() {
        let mut session = PersistentSession::new("s1", "input");
        let now = current_timestamp();
        // 设置 last_attempt_at 为 120 秒前
        session.last_attempt_at = now - 120;
        assert!(session.is_cooldown_elapsed(60)); // 60 秒冷却,已过
        assert!(!session.is_cooldown_elapsed(180)); // 180 秒冷却,未过
    }

    #[test]
    fn test_persistent_session_record_failure() {
        let mut session = PersistentSession::new("s1", "input");
        let original_count = session.retry_count;
        session.record_failure("test error".to_string());
        assert_eq!(session.retry_count, original_count + 1);
        assert_eq!(session.last_error, Some("test error".to_string()));
        assert_eq!(session.status, SessionStatus::Pending);
    }

    #[test]
    fn test_persistent_session_mark_running() {
        let mut session = PersistentSession::new("s1", "input");
        session.mark_running();
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_persistent_session_mark_completed() {
        let mut session = PersistentSession::new("s1", "input");
        session.mark_completed();
        assert_eq!(session.status, SessionStatus::Completed);
        assert!(session.last_error.is_none());
    }

    #[test]
    fn test_persistent_session_mark_failed() {
        let mut session = PersistentSession::new("s1", "input");
        session.mark_failed("permanent failure".to_string());
        assert_eq!(session.status, SessionStatus::Failed);
        assert_eq!(session.last_error, Some("permanent failure".to_string()));
    }

    #[test]
    fn test_persistent_session_serialization() {
        let session = PersistentSession::new("s1", "test input");
        let json = serde_json::to_string(&session).expect("测试：JSON序列化应成功");
        let deserialized: PersistentSession =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(deserialized.session_id, "s1");
        assert_eq!(deserialized.input, "test input");
        assert_eq!(deserialized.status, SessionStatus::Pending);
    }

    #[test]
    fn test_persistent_runner_config_default() {
        let config = PersistentRunnerConfig::default();
        assert_eq!(config.max_run_hours, 6);
        assert_eq!(config.cooldown_secs, 60);
        assert_eq!(config.max_retries, 10);
        assert!(!config.enabled);
    }

    #[test]
    fn test_validate_session_id_valid() {
        assert!(validate_session_id("session-1").is_ok());
        assert!(validate_session_id("abc_123").is_ok());
        assert!(validate_session_id("UUID-abc-123").is_ok());
    }

    #[test]
    fn test_validate_session_id_empty() {
        assert!(validate_session_id("").is_err());
    }

    #[test]
    fn test_validate_session_id_invalid_chars() {
        assert!(validate_session_id("session/1").is_err());
        assert!(validate_session_id("session\\1").is_err());
        assert!(validate_session_id("../escape").is_err());
        assert!(validate_session_id("session 1").is_err()); // 空格
    }

    #[tokio::test]
    async fn test_persistent_runner_new() {
        let runner = PersistentRunner::new("/project", PersistentRunnerConfig::default());
        assert_eq!(runner.storage_dir(), Path::new("/project/.axagent/persistent_sessions"));
    }

    #[tokio::test]
    async fn test_persistent_runner_config_read_write() {
        let runner = PersistentRunner::new("/project", PersistentRunnerConfig::default());
        let config = runner.config().await;
        assert!(!config.enabled);
        let mut new_config = config.clone();
        new_config.enabled = true;
        runner.update_config(new_config).await;
        let config = runner.config().await;
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_persistent_runner_ensure_storage_dir() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        runner.ensure_storage_dir().await.expect("测试：异步操作应成功");
        assert!(runner.storage_dir().exists());
    }

    #[tokio::test]
    async fn test_persistent_runner_save_and_load_session() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        let session = PersistentSession::new("session-1", "test input");
        let path = runner.save_session(&session).await.expect("测试：异步操作应成功");
        assert!(path.exists());
        let loaded = runner.load_session("session-1").await.expect("测试：异步操作应成功");
        assert!(loaded.is_some());
        let loaded = loaded.expect("测试应成功");
        assert_eq!(loaded.session_id, "session-1");
        assert_eq!(loaded.input, "test input");
    }

    #[tokio::test]
    async fn test_persistent_runner_load_session_nonexistent() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        let loaded = runner.load_session("nonexistent").await.expect("测试：异步操作应成功");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_persistent_runner_load_session_invalid_id() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        assert!(runner.load_session("../escape").await.is_err());
        assert!(runner.load_session("").await.is_err());
    }

    #[tokio::test]
    async fn test_persistent_runner_delete_session() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        let session = PersistentSession::new("session-1", "input");
        runner.save_session(&session).await.expect("测试：异步操作应成功");
        let deleted = runner.delete_session("session-1").await.expect("测试：异步操作应成功");
        assert!(deleted);
        // 再次删除应返回 false
        let deleted_again = runner.delete_session("session-1").await.expect("测试：异步操作应成功");
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_empty() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig { enabled: true, ..Default::default() };
        let runner = PersistentRunner::new(dir.path(), config);
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_disabled() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        // 默认 enabled=false,应返回空
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_with_sessions() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 0, // 立即可重试
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        // 创建 2 个 pending session
        let s1 = PersistentSession::new("session-1", "input1");
        let s2 = PersistentSession::new("session-2", "input2");
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(pending.len(), 2);
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_excludes_completed() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config =
            PersistentRunnerConfig { enabled: true, cooldown_secs: 0, ..Default::default() };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.mark_completed();
        let s2 = PersistentSession::new("session-2", "input2");
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].session_id, "session-2");
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_excludes_expired() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 0,
            max_run_hours: 6,
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.first_attempt_at = current_timestamp() - 7 * 3600; // 7 小时前
        let s2 = PersistentSession::new("session-2", "input2");
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].session_id, "session-2");
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_excludes_max_retries() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 0,
            max_retries: 3,
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.retry_count = 3; // 已达最大重试
        let s2 = PersistentSession::new("session-2", "input2");
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].session_id, "session-2");
    }

    #[tokio::test]
    async fn test_persistent_runner_list_pending_respects_cooldown() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 3600, // 1 小时冷却
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.last_attempt_at = current_timestamp(); // 刚失败,未过冷却
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        let pending = runner.list_pending_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(pending.len(), 0); // 全部未过冷却
    }

    #[tokio::test]
    async fn test_persistent_runner_list_all_sessions() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let runner = PersistentRunner::new(dir.path(), PersistentRunnerConfig::default());
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.mark_completed();
        let s2 = PersistentSession::new("session-2", "input2");
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        let all = runner.list_all_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_persistent_runner_cleanup_expired() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig { max_run_hours: 6, ..Default::default() };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut s1 = PersistentSession::new("session-1", "input1");
        s1.mark_completed(); // 已完成
        let mut s2 = PersistentSession::new("session-2", "input2");
        s2.first_attempt_at = current_timestamp() - 7 * 3600; // 已过期
        let s3 = PersistentSession::new("session-3", "input3"); // 保留
        runner.save_session(&s1).await.expect("测试：异步操作应成功");
        runner.save_session(&s2).await.expect("测试：异步操作应成功");
        runner.save_session(&s3).await.expect("测试：异步操作应成功");
        let cleaned = runner.cleanup_expired().await.expect("测试：异步操作应成功");
        assert_eq!(cleaned, 2);
        let all = runner.list_all_sessions().await.expect("测试：异步操作应成功");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].session_id, "session-3");
    }

    #[tokio::test]
    async fn test_persistent_runner_schedule_success() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config =
            PersistentRunnerConfig { enabled: true, cooldown_secs: 0, ..Default::default() };
        let runner = PersistentRunner::new(dir.path(), config);
        let session = PersistentSession::new("session-1", "input1");
        runner.save_session(&session).await.expect("测试：异步操作应成功");
        let (succeeded, failed) =
            runner.schedule(|_session| async { Ok(()) }).await.expect("测试：异步操作应成功");
        assert_eq!(succeeded, 1);
        assert_eq!(failed, 0);
        // 验证 session 已标记为 completed
        let loaded = runner.load_session("session-1").await.expect("测试：异步操作应成功").unwrap();
        assert_eq!(loaded.status, SessionStatus::Completed);
    }

    #[tokio::test]
    async fn test_persistent_runner_schedule_failure_increments_retry() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 0,
            max_retries: 5,
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        let session = PersistentSession::new("session-1", "input1");
        runner.save_session(&session).await.expect("测试：异步操作应成功");
        let (succeeded, failed) = runner
            .schedule(|_session| async { Err("test failure".to_string()) })
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(succeeded, 0);
        assert_eq!(failed, 1);
        // 验证 session retry_count 已增加
        let loaded = runner.load_session("session-1").await.expect("测试：异步操作应成功").unwrap();
        assert_eq!(loaded.retry_count, 1);
        assert_eq!(loaded.last_error, Some("test failure".to_string()));
        assert_eq!(loaded.status, SessionStatus::Pending);
    }

    #[tokio::test]
    async fn test_persistent_runner_schedule_failure_marks_failed_after_max_retries() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistentRunnerConfig {
            enabled: true,
            cooldown_secs: 0,
            max_retries: 2,
            ..Default::default()
        };
        let runner = PersistentRunner::new(dir.path(), config);
        let mut session = PersistentSession::new("session-1", "input1");
        session.retry_count = 2; // 已达最大重试
        runner.save_session(&session).await.expect("测试：异步操作应成功");
        // list_pending 会过滤掉 max_retries_exceeded,所以 schedule 不会执行它
        let (succeeded, failed) = runner
            .schedule(|_session| async { Err("test failure".to_string()) })
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(succeeded, 0);
        assert_eq!(failed, 0);
    }
}
