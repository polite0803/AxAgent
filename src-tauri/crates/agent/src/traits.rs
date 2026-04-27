use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not initialized")]
    NotInitialized,
    #[error("Agent already running")]
    AlreadyRunning,
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
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

#[async_trait]
pub trait Agent: Send + Sync {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;
    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError>;
    async fn pause(&mut self) -> Result<(), AgentError>;
    async fn resume(&mut self) -> Result<(), AgentError>;
    async fn cancel(&mut self) -> Result<(), AgentError>;
    fn status(&self) -> AgentStatus;
    fn agent_type(&self) -> &'static str;
}
