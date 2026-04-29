use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use crate::steer_manager::SteerManager;
use async_trait::async_trait;
use axagent_runtime::{prompt_cache::PromptCache, CacheGuard, HookChain};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

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
    pub fn new(implementation: Arc<tokio::sync::Mutex<T>>, event_bus: Option<Arc<AgentEventBus>>) -> Self {
        let event_bus = event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("typed_coordinator")));
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

        self.emit_event(AgentEventType::StateChanged, serde_json::json!({
            "previous": "Initializing",
            "current": "Idle"
        })).await;

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
        self.emit_event(AgentEventType::TurnStarted, serde_json::json!({
            "input_preview": input.content.chars().take(100).collect::<String>(),
            "cache_valid": cache_was_valid,
            "has_pending_changes": self.prompt_cache.has_pending_changes().await,
        })).await;

        let correlation_id = self.next_correlation_id();
        let result = {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.execute(input).await
        };

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = output.status.clone();
                self.emit_event(AgentEventType::TurnCompleted, serde_json::json!({
                    "correlation_id": correlation_id,
                    "iterations": output.iterations,
                    "status": output.status.to_string(),
                    "cache_was_valid": cache_was_valid,
                })).await;
            }
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
                self.emit_event(AgentEventType::Error, serde_json::json!({
                    "correlation_id": correlation_id,
                    "error": e.to_string(),
                    "cache_was_valid": cache_was_valid,
                })).await;
            }
        }

        result
    }

    pub async fn force_now(&self) {
        self.cache_guard.set_force_immediate(true).await;
        self.prompt_cache.invalidate("--now flag: immediate invalidation").await;
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

        self.emit_event(AgentEventType::StateChanged, serde_json::json!({
            "from": "Running",
            "to": "Paused"
        })).await;

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

        self.emit_event(AgentEventType::StateChanged, serde_json::json!({
            "from": "Paused",
            "to": "Running"
        })).await;

        Ok(())
    }

    pub async fn cancel(&self) -> Result<(), AgentError> {
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.cancel().await?;
        }

        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        self.emit_event(AgentEventType::StateChanged, serde_json::json!({
            "to": "Idle"
        })).await;

        Ok(())
    }

    pub async fn get_status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    fn next_correlation_id(&self) -> u64 {
        self.correlation_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
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
