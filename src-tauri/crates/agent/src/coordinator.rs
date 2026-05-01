use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use crate::steer_manager::SteerManager;
use async_trait::async_trait;
use axagent_runtime::{prompt_cache::PromptCache, CacheGuard, HookChain};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// 工作者（Worker）Agent 模式
// 移植自 claude-code-main 的协调者/工作者模式
// ---------------------------------------------------------------------------

/// 协调者内部编排工具列表，工作者不可使用这些工具。
///
/// 工作者只能使用常规工具（文件操作、搜索、Web 请求等），
/// 不能创建子 Agent、发送跨 Agent 消息或生成综合输出。
pub const INTERNAL_ORCH_TOOLS: &[&str] = &[
    "agent_create",
    "agent_delete",
    "send_message",
    "synthetic_output",
];

/// 工作者 Agent 的定义。
///
/// 由协调者创建，指定工具集和系统提示，用于执行独立的并行任务。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDefinition {
    /// Agent 类型标识（通常为 "worker"）
    pub agent_type: String,
    /// 何时使用此工作者的描述（用于协调者决策）
    pub when_to_use: String,
    /// 受限工具集（不包含 INTERNAL_ORCH_TOOLS）
    pub tools: Vec<String>,
    /// 工作者的系统提示
    pub system_prompt: String,
}

impl WorkerDefinition {
    /// 创建一个新的工作者定义，自动过滤掉内部编排工具。
    pub fn new(
        agent_type: impl Into<String>,
        when_to_use: impl Into<String>,
        tools: Vec<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        // 自动过滤掉内部编排工具
        let filtered_tools: Vec<String> = tools
            .into_iter()
            .filter(|t| !INTERNAL_ORCH_TOOLS.contains(&t.as_str()))
            .collect();

        Self {
            agent_type: agent_type.into(),
            when_to_use: when_to_use.into(),
            tools: filtered_tools,
            system_prompt: system_prompt.into(),
        }
    }

    /// 验证工作者定义是否合法。
    pub fn validate(&self) -> Result<(), String> {
        if self.agent_type.is_empty() {
            return Err("agent_type 不能为空".to_string());
        }
        if self.tools.is_empty() {
            return Err("tools 不能为空".to_string());
        }
        if self.system_prompt.is_empty() {
            return Err("system_prompt 不能为空".to_string());
        }
        Ok(())
    }
}

/// 工作者与协调者之间的消息类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerMessageType {
    /// 进度更新
    Progress,
    /// 最终结果
    Result,
    /// 错误信息
    Error,
    /// 任务完成通知
    Completion,
}

impl std::fmt::Display for WorkerMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerMessageType::Progress => write!(f, "progress"),
            WorkerMessageType::Result => write!(f, "result"),
            WorkerMessageType::Error => write!(f, "error"),
            WorkerMessageType::Completion => write!(f, "completion"),
        }
    }
}

/// 工作者发送给协调者的消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMessage {
    /// 工作者唯一标识
    pub worker_id: String,
    /// 任务唯一标识
    pub task_id: String,
    /// 消息类型
    pub message_type: WorkerMessageType,
    /// 消息内容
    pub content: String,
    /// 附加元数据
    pub metadata: serde_json::Value,
}

impl WorkerMessage {
    pub fn new(
        worker_id: impl Into<String>,
        task_id: impl Into<String>,
        message_type: WorkerMessageType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            worker_id: worker_id.into(),
            task_id: task_id.into(),
            message_type,
            content: content.into(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn progress(worker_id: &str, task_id: &str, content: &str) -> Self {
        Self::new(worker_id, task_id, WorkerMessageType::Progress, content)
    }

    pub fn result(worker_id: &str, task_id: &str, content: &str) -> Self {
        Self::new(worker_id, task_id, WorkerMessageType::Result, content)
    }

    pub fn error(worker_id: &str, task_id: &str, content: &str) -> Self {
        Self::new(worker_id, task_id, WorkerMessageType::Error, content)
    }

    pub fn completion(worker_id: &str, task_id: &str, content: &str) -> Self {
        Self::new(worker_id, task_id, WorkerMessageType::Completion, content)
    }
}

/// 工作者的运行时状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// 已创建，等待调度
    Created,
    /// 正在运行
    Running,
    /// 已成功完成
    Completed,
    /// 失败
    Failed(String),
    /// 被取消
    Cancelled,
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerStatus::Created => write!(f, "created"),
            WorkerStatus::Running => write!(f, "running"),
            WorkerStatus::Completed => write!(f, "completed"),
            WorkerStatus::Failed(_) => write!(f, "failed"),
            WorkerStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 工作者执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// 工作者 ID
    pub worker_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 执行状态
    pub status: WorkerStatus,
    /// 输出内容
    pub output: Option<String>,
    /// 收到的消息历史
    pub messages: Vec<WorkerMessage>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

impl WorkerResult {
    pub fn success(worker_id: &str, task_id: &str, output: &str, duration_ms: u64) -> Self {
        Self {
            worker_id: worker_id.to_string(),
            task_id: task_id.to_string(),
            status: WorkerStatus::Completed,
            output: Some(output.to_string()),
            messages: vec![WorkerMessage::completion(worker_id, task_id, output)],
            duration_ms,
        }
    }

    pub fn failure(
        worker_id: &str,
        task_id: &str,
        error: &str,
        messages: Vec<WorkerMessage>,
        duration_ms: u64,
    ) -> Self {
        Self {
            worker_id: worker_id.to_string(),
            task_id: task_id.to_string(),
            status: WorkerStatus::Failed(error.to_string()),
            output: None,
            messages,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;

    #[test]
    fn test_worker_definition_filters_internal_tools() {
        let tools = vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "agent_create".to_string(),
            "bash".to_string(),
            "send_message".to_string(),
        ];
        let def = WorkerDefinition::new(
            "worker",
            "For parallel tasks",
            tools,
            "You are a worker agent.",
        );

        assert!(!def.tools.contains(&"agent_create".to_string()));
        assert!(!def.tools.contains(&"send_message".to_string()));
        assert!(def.tools.contains(&"read_file".to_string()));
        assert!(def.tools.contains(&"write_file".to_string()));
        assert!(def.tools.contains(&"bash".to_string()));
    }

    #[test]
    fn test_worker_definition_validate() {
        let valid = WorkerDefinition::new(
            "worker",
            "For parallel tasks",
            vec!["read_file".to_string()],
            "You are a worker.",
        );
        assert!(valid.validate().is_ok());

        let empty_type = WorkerDefinition::new("", "", vec!["t".to_string()], "prompt");
        assert!(empty_type.validate().is_err());
    }

    #[test]
    fn test_worker_message_constructors() {
        let msg = WorkerMessage::progress("w1", "t1", "50% done");
        assert_eq!(msg.message_type, WorkerMessageType::Progress);

        let msg = WorkerMessage::result("w1", "t1", "completed successfully");
        assert_eq!(msg.message_type, WorkerMessageType::Result);

        let msg = WorkerMessage::error("w1", "t1", "something went wrong");
        assert_eq!(msg.message_type, WorkerMessageType::Error);
    }

    #[test]
    fn test_worker_result_success() {
        let result = WorkerResult::success("w1", "t1", "done", 1500);
        assert!(matches!(result.status, WorkerStatus::Completed));
        assert_eq!(result.output, Some("done".to_string()));
        assert_eq!(result.duration_ms, 1500);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Initializing,
    Running,
    WaitingForConfirmation,
    Paused,
    Completed,
    Failed(String),
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "Idle"),
            AgentStatus::Initializing => write!(f, "Initializing"),
            AgentStatus::Running => write!(f, "Running"),
            AgentStatus::WaitingForConfirmation => write!(f, "WaitingForConfirmation"),
            AgentStatus::Paused => write!(f, "Paused"),
            AgentStatus::Completed => write!(f, "Completed"),
            AgentStatus::Failed(msg) => write!(f, "Failed({})", msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub timeout_secs: Option<u64>,
    pub enable_self_verification: bool,
    pub enable_error_recovery: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            timeout_secs: Some(300),
            enable_self_verification: false,
            enable_error_recovery: true,
        }
    }
}

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not initialized")]
    NotInitialized,
    #[error("Agent already running")]
    AlreadyRunning,
    #[error("Agent is in invalid state: {0}")]
    InvalidState(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub content: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorOutput {
    pub content: String,
    pub status: AgentStatus,
    pub iterations: usize,
    pub metadata: serde_json::Value,
}

impl CoordinatorOutput {
    pub fn success(content: String, iterations: usize) -> Self {
        Self {
            content,
            status: AgentStatus::Completed,
            iterations,
            metadata: serde_json::json!({}),
        }
    }

    pub fn failure(message: String, iterations: usize) -> Self {
        Self {
            content: message.clone(),
            status: AgentStatus::Failed(message),
            iterations,
            metadata: serde_json::json!({}),
        }
    }
}

#[async_trait]
pub trait AgentImpl: Send + Sync {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;
    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError>;
    async fn pause(&mut self) -> Result<(), AgentError>;
    async fn resume(&mut self) -> Result<(), AgentError>;
    async fn cancel(&mut self) -> Result<(), AgentError>;
    fn status(&self) -> AgentStatus;
    fn agent_type(&self) -> &'static str;
}

pub struct AgentCoordinator<T: AgentImpl> {
    status: Arc<RwLock<AgentStatus>>,
    config: Arc<RwLock<AgentConfig>>,
    implementation: Arc<tokio::sync::Mutex<T>>,
    event_bus: Arc<AgentEventBus>,
    correlation_counter: std::sync::atomic::AtomicU64,
    pub prompt_cache: Arc<PromptCache>,
    pub cache_guard: Arc<CacheGuard>,
    pub hook_chain: Arc<HookChain>,
    pub steer_manager: Arc<SteerManager>,
}

impl<T: AgentImpl> AgentCoordinator<T> {
    pub fn new(
        implementation: Arc<tokio::sync::Mutex<T>>,
        event_bus: Option<Arc<AgentEventBus>>,
    ) -> Self {
        let event_bus =
            event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("typed_coordinator")));
        let prompt_cache = Arc::new(PromptCache::new());

        Self {
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            config: Arc::new(RwLock::new(AgentConfig::default())),
            implementation,
            event_bus,
            correlation_counter: std::sync::atomic::AtomicU64::new(0),
            prompt_cache: prompt_cache.clone(),
            cache_guard: Arc::new(CacheGuard::new(prompt_cache)),
            hook_chain: Arc::new(HookChain::new()),
            steer_manager: Arc::new(SteerManager::new()),
        }
    }

    pub async fn initialize(&self, config: AgentConfig) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if *status != AgentStatus::Idle {
            return Err(AgentError::InvalidState(format!(
                "Cannot initialize from status {}",
                status
            )));
        }

        *status = AgentStatus::Initializing;
        drop(status);

        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.initialize(config.clone()).await?;
        }

        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;
        let mut cfg = self.config.write().await;
        *cfg = config;

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "previous": "Initializing",
                "current": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    pub async fn execute(&self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        let mut status = self.status.write().await;
        let current_status = status.clone();

        if matches!(current_status, AgentStatus::Running) {
            return Err(AgentError::AlreadyRunning);
        }

        if !matches!(current_status, AgentStatus::Idle | AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!(
                "Cannot execute from status {}",
                current_status
            )));
        }

        *status = AgentStatus::Running;
        drop(status);

        let mut input = input;
        if self.steer_manager.has_pending().await {
            if let Some(steer_block) = self.steer_manager.format_steer_block().await {
                input.context = Some(serde_json::json!({
                    "steer": steer_block,
                }));
                tracing::info!("Injecting steer instructions into agent turn");
            }
        }

        let cache_was_valid = self.prompt_cache.is_cache_valid().await;
        self.emit_event(
            AgentEventType::TurnStarted,
            serde_json::json!({
                "input_preview": input.content.chars().take(100).collect::<String>(),
                "cache_valid": cache_was_valid,
                "has_pending_changes": self.prompt_cache.has_pending_changes().await,
            }),
        )
        .await;

        let correlation_id = self.next_correlation_id();
        let result = {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.execute(input).await
        };

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = output.status.clone();
                self.emit_event(
                    AgentEventType::TurnCompleted,
                    serde_json::json!({
                        "correlation_id": correlation_id,
                        "iterations": output.iterations,
                        "status": output.status.to_string(),
                        "cache_was_valid": cache_was_valid,
                    }),
                )
                .await;
            }
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
                self.emit_event(
                    AgentEventType::Error,
                    serde_json::json!({
                        "correlation_id": correlation_id,
                        "error": e.to_string(),
                        "cache_was_valid": cache_was_valid,
                    }),
                )
                .await;
            }
        }

        result
    }

    pub async fn force_now(&self) {
        self.cache_guard.set_force_immediate(true).await;
        self.prompt_cache
            .invalidate("--now flag: immediate invalidation")
            .await;
    }

    pub async fn prepare_for_new_session(&self) {
        self.prompt_cache.invalidate_for_new_session().await;
        self.cache_guard.set_force_immediate(false).await;
    }

    pub async fn pause(&self) -> Result<(), AgentError> {
        let status = self.status.read().await;
        if !matches!(*status, AgentStatus::Running) {
            return Err(AgentError::InvalidState(format!(
                "Cannot pause from status {}",
                status
            )));
        }
        drop(status);

        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.pause().await?;
        }

        let mut status = self.status.write().await;
        *status = AgentStatus::Paused;

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Running",
                "to": "Paused"
            }),
        )
        .await;

        Ok(())
    }

    pub async fn resume(&self) -> Result<(), AgentError> {
        let status = self.status.read().await;
        if !matches!(*status, AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!(
                "Cannot resume from status {}",
                status
            )));
        }
        drop(status);

        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.resume().await?;
        }

        let mut status = self.status.write().await;
        *status = AgentStatus::Running;

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Paused",
                "to": "Running"
            }),
        )
        .await;

        Ok(())
    }

    pub async fn cancel(&self) -> Result<(), AgentError> {
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.cancel().await?;
        }

        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "to": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    pub async fn get_status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    fn next_correlation_id(&self) -> u64 {
        self.correlation_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    async fn emit_event(&self, event_type: AgentEventType, payload: serde_json::Value) {
        let event = UnifiedAgentEvent::new("AgentCoordinator", event_type, payload);
        if let Err(e) = self.event_bus.emit(event) {
            tracing::warn!("Failed to emit event: {:?}", e);
        }
    }
}

impl<T: AgentImpl> std::fmt::Debug for AgentCoordinator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCoordinator")
            .field("event_bus", &self.event_bus.name())
            .finish()
    }
}
