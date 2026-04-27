use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningState {
    Idle,
    Analyzing,
    Thinking,
    Planning,
    Acting,
    Observing,
    Reflecting,
    Finished,
    Failed,
}

impl ReasoningState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, ReasoningState::Finished | ReasoningState::Failed)
    }

    pub fn requires_observation(&self) -> bool {
        matches!(self, ReasoningState::Acting)
    }

    pub fn can_retry(&self) -> bool {
        matches!(self, ReasoningState::Observing | ReasoningState::Reflecting)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningState::Idle => "idle",
            ReasoningState::Analyzing => "analyzing",
            ReasoningState::Thinking => "thinking",
            ReasoningState::Planning => "planning",
            ReasoningState::Acting => "acting",
            ReasoningState::Observing => "observing",
            ReasoningState::Reflecting => "reflecting",
            ReasoningState::Finished => "finished",
            ReasoningState::Failed => "failed",
        }
    }
}

impl fmt::Display for ReasoningState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    ToolCall,
    LlmCall,
    UserConfirm,
    Validate,
    Analyze,
    Plan,
    Reflect,
    Synthesize,
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionType::ToolCall => write!(f, "tool_call"),
            ActionType::LlmCall => write!(f, "llm_call"),
            ActionType::UserConfirm => write!(f, "user_confirm"),
            ActionType::Validate => write!(f, "validate"),
            ActionType::Analyze => write!(f, "analyze"),
            ActionType::Plan => write!(f, "plan"),
            ActionType::Reflect => write!(f, "reflect"),
            ActionType::Synthesize => write!(f, "synthesize"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActConfig {
    pub max_iterations: usize,
    pub max_depth: usize,
    pub verification_enabled: bool,
    pub max_retry_attempts: usize,
    pub timeout_secs: u64,
    pub reflection_threshold: usize,
    pub enable_analyzing: bool,
    pub enable_reflection: bool,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_depth: 10,
            verification_enabled: true,
            max_retry_attempts: 3,
            timeout_secs: 300,
            reflection_threshold: 5,
            enable_analyzing: true,
            enable_reflection: true,
        }
    }
}

impl ReActConfig {
    pub fn for_simple_task() -> Self {
        Self {
            max_iterations: 20,
            max_depth: 5,
            verification_enabled: true,
            max_retry_attempts: 2,
            timeout_secs: 60,
            reflection_threshold: 3,
            enable_analyzing: false,
            enable_reflection: false,
        }
    }

    pub fn for_complex_task() -> Self {
        Self {
            max_iterations: 100,
            max_depth: 20,
            verification_enabled: true,
            max_retry_attempts: 5,
            timeout_secs: 600,
            reflection_threshold: 10,
            enable_analyzing: true,
            enable_reflection: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContext {
    pub original_input: String,
    pub current_goal: Option<String>,
    pub sub_goals: Vec<String>,
    pub constraints: Vec<String>,
    pub resources: Vec<String>,
    pub iteration: usize,
    pub depth: usize,
}

impl Default for ReasoningContext {
    fn default() -> Self {
        Self {
            original_input: String::new(),
            current_goal: None,
            sub_goals: Vec::new(),
            constraints: Vec::new(),
            resources: Vec::new(),
            iteration: 0,
            depth: 0,
        }
    }
}

impl ReasoningContext {
    pub fn new(input: &str) -> Self {
        Self {
            original_input: input.to_string(),
            ..Default::default()
        }
    }

    pub fn set_goal(&mut self, goal: String) {
        self.current_goal = Some(goal);
    }

    pub fn add_sub_goal(&mut self, sub_goal: String) {
        self.sub_goals.push(sub_goal);
    }

    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }

    pub fn increment_depth(&mut self) {
        self.depth += 1;
    }
}
