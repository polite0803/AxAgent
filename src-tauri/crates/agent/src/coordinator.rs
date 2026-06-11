// SPDX-License-Identifier: AGPL-3.0-only

use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use crate::steer_manager::SteerManager;
use crate::tree_of_thoughts::{LlmReasoningProvider as ToTReasoningProvider, TreeOfThoughtsEngine};
use async_trait::async_trait;
use axagent_runtime_core::{CacheGuard, HookChain, prompt_cache::PromptCache};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
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
    // i18n-note: Validation error messages. Future: accept language parameter or convert to error codes.
    pub fn validate(&self) -> Result<(), String> {
        if self.agent_type.is_empty() {
            return Err("agent_type 不能为空".to_string());
        }
        if self.when_to_use.is_empty() {
            return Err("when_to_use 不能为空".to_string());
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
        let def =
            WorkerDefinition::new("worker", "For parallel tasks", tools, "You are a worker agent.");

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

// ---------------------------------------------------------------------------
// 状态机判别值（用于 lock-free 原子状态机）
// ---------------------------------------------------------------------------
//
// 真实状态机由 `AtomicU8` 驱动；`Arc<RwLock<AgentStatus>>` 仍保留以返回
// `Failed(String)` 等携带详情的状态给调用方。状态判别值映射：
//
// 0 = Idle
// 1 = Initializing
// 2 = Running
// 3 = WaitingForConfirmation
// 4 = Paused
// 5 = Completed
// 6 = Failed
//
// 所有比较/转换统一通过 `compare_exchange(SeqCst)` 完成，避免
// `RwLock` 写锁释放后并发 cancel/get_status 读到错乱中间态。

const STATE_IDLE: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_RUNNING: u8 = 2;
const STATE_WAITING_FOR_CONFIRMATION: u8 = 3;
const STATE_PAUSED: u8 = 4;
const STATE_COMPLETED: u8 = 5;
const STATE_FAILED: u8 = 6;

/// 将 `AgentStatus` 映射到原子判别值（`Failed(_)` 一律映射到 `STATE_FAILED`，详情保留在 RwLock 中）。
fn state_discriminant(status: &AgentStatus) -> u8 {
    match status {
        AgentStatus::Idle => STATE_IDLE,
        AgentStatus::Initializing => STATE_INITIALIZING,
        AgentStatus::Running => STATE_RUNNING,
        AgentStatus::WaitingForConfirmation => STATE_WAITING_FOR_CONFIRMATION,
        AgentStatus::Paused => STATE_PAUSED,
        AgentStatus::Completed => STATE_COMPLETED,
        AgentStatus::Failed(_) => STATE_FAILED,
    }
}

/// 由原子判别值构造无详情 `AgentStatus`（`Failed` 分支 detail 为空字符串，真实 detail 由 RwLock 提供）。
fn state_from_discriminant(value: u8) -> AgentStatus {
    match value {
        STATE_IDLE => AgentStatus::Idle,
        STATE_INITIALIZING => AgentStatus::Initializing,
        STATE_RUNNING => AgentStatus::Running,
        STATE_WAITING_FOR_CONFIRMATION => AgentStatus::WaitingForConfirmation,
        STATE_PAUSED => AgentStatus::Paused,
        STATE_COMPLETED => AgentStatus::Completed,
        STATE_FAILED => AgentStatus::Failed(String::new()),
        // 未知判别值兜底为 Idle，避免污染状态机
        _ => AgentStatus::Idle,
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
    /// 状态机判别值（lock-free 状态机；具体含义见 `STATE_*` 常量注释）。
    /// 状态转换通过 `compare_exchange(SeqCst)` 完成，避免 `RwLock` 写锁
    /// 释放后并发 cancel/get_status 读到错乱中间态。
    state: Arc<AtomicU8>,
    /// 完整 `AgentStatus`（含 `Failed(String)` 详情），由 atomic 状态机驱动刷新。
    status: Arc<RwLock<AgentStatus>>,
    config: Arc<RwLock<AgentConfig>>,
    implementation: Arc<tokio::sync::Mutex<T>>,
    event_bus: Arc<AgentEventBus>,
    correlation_counter: std::sync::atomic::AtomicU64,
    pub prompt_cache: Arc<PromptCache>,
    pub cache_guard: Arc<CacheGuard>,
    pub hook_chain: Arc<HookChain>,
    pub steer_manager: Arc<SteerManager>,
    tot_engine: Option<TreeOfThoughtsEngine>,
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
            state: Arc::new(AtomicU8::new(STATE_IDLE)),
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            config: Arc::new(RwLock::new(AgentConfig::default())),
            implementation,
            event_bus,
            correlation_counter: std::sync::atomic::AtomicU64::new(0),
            prompt_cache: prompt_cache.clone(),
            cache_guard: Arc::new(CacheGuard::new(prompt_cache)),
            hook_chain: Arc::new(HookChain::new()),
            steer_manager: Arc::new(SteerManager::new()),
            tot_engine: None,
        }
    }

    pub fn with_tot_engine(mut self, engine: TreeOfThoughtsEngine) -> Self {
        self.tot_engine = Some(engine);
        self
    }

    pub async fn reason_with_tot(
        &mut self,
        _problem: &str,
        context: &str,
        provider: &Arc<dyn ToTReasoningProvider>,
    ) -> Option<Vec<String>> {
        let engine = self.tot_engine.as_mut()?;

        let root_id = engine.root_id.clone();
        let child_ids = engine
            .generate_branching_options(root_id, context, provider)
            .await
            .ok()?;

        let mut scored_ids = Vec::new();
        for child_id in &child_ids {
            if let Ok(score) = engine
                .evaluate_and_score_node(child_id, context, provider)
                .await
            {
                scored_ids.push((child_id.clone(), score));
            }
        }

        engine.prune_below_threshold(0.3);

        let best_path = engine.select_best_path();
        if !best_path.is_empty() {
            tracing::info!(
                path_length = best_path.len(),
                "Tree of Thoughts selected best reasoning path"
            );
        }

        Some(best_path)
    }

    pub async fn initialize(&self, config: AgentConfig) -> Result<(), AgentError> {
        // 1. 原子守卫：仅允许从 Idle 进入 Initializing；并发进入返回 InvalidState
        if !self.try_transition(&[STATE_IDLE], STATE_INITIALIZING) {
            let current = self.current_state();
            return Err(AgentError::InvalidState(format!(
                "Cannot initialize from state {}",
                state_from_discriminant(current)
            )));
        }
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Initializing;
        }

        // 2. 调用实现；失败时复位状态为 Idle，再传播错误（避免卡在 Initializing）
        let init_result = {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.initialize(config.clone()).await
        };
        if let Err(err) = init_result {
            self.set_state(STATE_IDLE);
            {
                let mut status = self.status.write().await;
                *status = AgentStatus::Idle;
            }
            return Err(err);
        }

        // 3. 成功：原子置 Idle，刷新 config，发出 StateChanged 事件
        self.set_state(STATE_IDLE);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Idle;
        }
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
        // 1. 原子守卫：仅允许从 Idle|Paused 进入 Running；并发进入时
        //    - 当前已是 Running → AlreadyRunning
        //    - 其余状态 → InvalidState
        // 通过 `compare_exchange` 实现 lock-free 状态获取，状态在
        // 整个 impl.execute() 期间持久可见，避免 `RwLock` 写锁释放
        // 后并发 cancel/get_status 读到错乱中间态。
        if !self.try_transition(&[STATE_IDLE, STATE_PAUSED], STATE_RUNNING) {
            let current = self.current_state();
            if current == STATE_RUNNING {
                return Err(AgentError::AlreadyRunning);
            }
            return Err(AgentError::InvalidState(format!(
                "Cannot execute from state {}",
                state_from_discriminant(current)
            )));
        }
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Running;
        }

        let mut input = input;
        if self.steer_manager.has_pending().await
            && let Some(steer_block) = self.steer_manager.format_steer_block().await
        {
            let mut ctx = input
                .context
                .take()
                .and_then(|v| {
                    serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(v).ok()
                })
                .unwrap_or_default();
            ctx.insert("steer".to_string(), serde_json::json!(steer_block));
            input.context = Some(serde_json::Value::Object(ctx));
            tracing::info!("Injecting steer instructions into agent turn");
        }

        // For complex tasks, use Tree of Thoughts to explore multiple reasoning paths
        // before delegating to workers

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

        // 4. 写回终态：以 atomic 为准，detail 走 RwLock
        match &result {
            Ok(output) => {
                self.set_state(state_discriminant(&output.status));
                {
                    let mut status = self.status.write().await;
                    *status = output.status.clone();
                }
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
            },
            Err(e) => {
                self.set_state(STATE_FAILED);
                {
                    let mut status = self.status.write().await;
                    *status = AgentStatus::Failed(e.to_string());
                }
                self.emit_event(
                    AgentEventType::Error,
                    serde_json::json!({
                        "correlation_id": correlation_id,
                        "error": e.to_string(),
                        "cache_was_valid": cache_was_valid,
                    }),
                )
                .await;
            },
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
        // 1. 原子检查：仅当状态为 Running 时才进入（不预占状态，避免失败后回滚）
        let current = self.current_state();
        if current != STATE_RUNNING {
            return Err(AgentError::InvalidState(format!(
                "Cannot pause from state {}",
                state_from_discriminant(current)
            )));
        }

        // 2. 调用实现，失败时原状态（Running）保持不变
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.pause().await?;
        }

        // 3. 成功：原子置 Paused，刷新 detail
        self.set_state(STATE_PAUSED);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Paused;
        }

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
        // 1. 原子检查：仅当状态为 Paused 时才进入
        let current = self.current_state();
        if current != STATE_PAUSED {
            return Err(AgentError::InvalidState(format!(
                "Cannot resume from state {}",
                state_from_discriminant(current)
            )));
        }

        // 2. 调用实现，失败时原状态（Paused）保持不变
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.resume().await?;
        }

        // 3. 成功：原子置 Running，刷新 detail
        self.set_state(STATE_RUNNING);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Running;
        }

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
        // 1. 调用实现；失败时原状态保持不变，错误直接传播（避免掩盖实现层错误）
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.cancel().await?;
        }

        // 2. 成功：原子置 Idle，刷新 detail
        self.set_state(STATE_IDLE);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Idle;
        }

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

    /// 读取当前状态判别值（SeqCst load）。
    fn current_state(&self) -> u8 {
        self.state.load(Ordering::SeqCst)
    }

    /// 原子地、无锁地将状态从 `from` 中的任一判别值转换为 `to`。
    ///
    /// 使用 `compare_exchange` 循环避免 ABA；当前判别值不在
    /// `from` 中或被并发修改时返回 `false`，由调用方决定如何响应。
    fn try_transition(&self, from: &[u8], to: u8) -> bool {
        let mut current = self.current_state();
        loop {
            if !from.contains(&current) {
                return false;
            }
            match self
                .state
                .compare_exchange(current, to, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// 无条件原子地写入状态判别值（仅在锁/事件已正确同步时使用）。
    fn set_state(&self, to: u8) {
        self.state.store(to, Ordering::SeqCst);
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
