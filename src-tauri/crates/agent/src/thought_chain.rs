use crate::reasoning_state::{ActionType, ReasoningState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    pub id: usize,
    pub state: ReasoningState,
    pub reasoning: String,
    pub action: Option<Action>,
    pub observation: Option<String>,
    pub result: Option<String>,
    pub is_verified: bool,
    pub timestamp: String,
}

impl ThoughtStep {
    pub fn new(state: ReasoningState, reasoning: impl Into<String>) -> Self {
        Self {
            id: 0,
            state,
            reasoning: reasoning.into(),
            action: None,
            observation: None,
            result: None,
            is_verified: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_action(
        state: ReasoningState,
        reasoning: impl Into<String>,
        action: Action,
    ) -> Self {
        Self {
            id: 0,
            state,
            reasoning: reasoning.into(),
            action: Some(action),
            observation: None,
            result: None,
            is_verified: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub llm_prompt: Option<String>,
    pub requires_confirmation: bool,
}

impl Action {
    pub fn tool_call(tool_name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            action_type: ActionType::ToolCall,
            tool_name: Some(tool_name.into()),
            tool_input: Some(input),
            llm_prompt: None,
            requires_confirmation: false,
        }
    }

    pub fn llm_call(prompt: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::LlmCall,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some(prompt.into()),
            requires_confirmation: false,
        }
    }

    pub fn user_confirm(message: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::UserConfirm,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some(message.into()),
            requires_confirmation: true,
        }
    }

    pub fn validate(description: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::Validate,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some(description.into()),
            requires_confirmation: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtChain {
    pub steps: Vec<ThoughtStep>,
    pub current_state: ReasoningState,
    pub iteration: usize,
}

impl ThoughtChain {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            current_state: ReasoningState::Thinking,
            iteration: 0,
        }
    }

    pub fn add_step(&mut self, step: ThoughtStep) {
        self.current_state = step.state;
        if step.state == ReasoningState::Thinking {
            self.iteration += 1;
        }
        self.steps.push(step);
    }

    pub fn latest_step(&self) -> Option<&ThoughtStep> {
        self.steps.last()
    }

    pub fn latest_step_mut(&mut self) -> Option<&mut ThoughtStep> {
        self.steps.last_mut()
    }

    pub fn update_step_result(&mut self, result: impl Into<String>, verified: bool) {
        if let Some(step) = self.steps.last_mut() {
            step.result = Some(result.into());
            step.is_verified = verified;
        }
    }

    pub fn update_step_observation(&mut self, observation: impl Into<String>) {
        if let Some(step) = self.steps.last_mut() {
            step.observation = Some(observation.into());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn iteration_count(&self) -> usize {
        self.iteration
    }

    pub fn to_summary(&self) -> ChainSummary {
        ChainSummary {
            total_steps: self.steps.len(),
            iterations: self.iteration,
            current_state: self.current_state.to_string(),
            steps: self.steps.clone(),
        }
    }
}

impl Default for ThoughtChain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    pub total_steps: usize,
    pub iterations: usize,
    pub current_state: String,
    pub steps: Vec<ThoughtStep>,
}

#[derive(Debug, Clone)]
pub enum ThoughtEvent {
    StepStarted(ThoughtStep),
    StepCompleted(ThoughtStep),
    StateChanged(ReasoningState),
    IterationComplete(usize),
    ChainComplete(ChainSummary),
    Error(String),
}

pub struct ThoughtChainEmitter {
    sender: broadcast::Sender<ThoughtEvent>,
}

impl ThoughtChainEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ThoughtEvent> {
        self.sender.subscribe()
    }

    pub fn emit(&self, event: ThoughtEvent) {
        let _ = self.sender.send(event);
    }
}

impl Default for ThoughtChainEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl ThoughtChain {
    pub fn with_emitter(_emitter: Arc<ThoughtChainEmitter>) -> Self {
        Self::new()
    }
}
