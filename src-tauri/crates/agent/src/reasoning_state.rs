use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningState {
    Thinking,
    Planning,
    Acting,
    Observing,
    Finished,
    Failed,
}

impl fmt::Display for ReasoningState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasoningState::Thinking => write!(f, "thinking"),
            ReasoningState::Planning => write!(f, "planning"),
            ReasoningState::Acting => write!(f, "acting"),
            ReasoningState::Observing => write!(f, "observing"),
            ReasoningState::Finished => write!(f, "finished"),
            ReasoningState::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    ToolCall,
    LlmCall,
    UserConfirm,
    Validate,
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionType::ToolCall => write!(f, "tool_call"),
            ActionType::LlmCall => write!(f, "llm_call"),
            ActionType::UserConfirm => write!(f, "user_confirm"),
            ActionType::Validate => write!(f, "validate"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReActConfig {
    pub max_iterations: usize,
    pub max_depth: usize,
    pub verification_enabled: bool,
    pub max_retry_attempts: usize,
    pub timeout_secs: u64,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            max_depth: 10,
            verification_enabled: true,
            max_retry_attempts: 3,
            timeout_secs: 120,
        }
    }
}
