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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thought_chain_new() {
        let chain = ThoughtChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert_eq!(chain.iteration_count(), 0);
        assert_eq!(chain.current_state, ReasoningState::Thinking);
    }

    #[test]
    fn test_thought_chain_default() {
        let chain = ThoughtChain::default();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_thought_step_new() {
        let step = ThoughtStep::new(ReasoningState::Thinking, "test reasoning");
        assert_eq!(step.id, 0);
        assert_eq!(step.state, ReasoningState::Thinking);
        assert_eq!(step.reasoning, "test reasoning");
        assert!(step.action.is_none());
        assert!(step.observation.is_none());
        assert!(step.result.is_none());
        assert!(!step.is_verified);
        assert!(!step.timestamp.is_empty());
    }

    #[test]
    fn test_thought_step_with_action() {
        let action = Action::tool_call("read_file", serde_json::json!({"path": "/tmp/test"}));
        let step = ThoughtStep::with_action(ReasoningState::Acting, "acting on data", action);
        assert_eq!(step.state, ReasoningState::Acting);
        assert_eq!(step.reasoning, "acting on data");
        assert!(step.action.is_some());
        let a = step.action.unwrap();
        assert_eq!(a.action_type, ActionType::ToolCall);
        assert_eq!(a.tool_name.unwrap(), "read_file");
    }

    #[test]
    fn test_add_step_updates_state() {
        let mut chain = ThoughtChain::new();
        let step = ThoughtStep::new(ReasoningState::Analyzing, "analyzing input");
        chain.add_step(step);
        assert_eq!(chain.current_state, ReasoningState::Analyzing);
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_add_step_thinking_increments_iteration() {
        let mut chain = ThoughtChain::new();
        let step1 = ThoughtStep::new(ReasoningState::Thinking, "first thought");
        chain.add_step(step1);
        assert_eq!(chain.iteration_count(), 1);

        let step2 = ThoughtStep::new(ReasoningState::Acting, "acting");
        chain.add_step(step2);
        assert_eq!(chain.iteration_count(), 1);

        let step3 = ThoughtStep::new(ReasoningState::Thinking, "second thought");
        chain.add_step(step3);
        assert_eq!(chain.iteration_count(), 2);
    }

    #[test]
    fn test_latest_step() {
        let mut chain = ThoughtChain::new();
        assert!(chain.latest_step().is_none());

        let step1 = ThoughtStep::new(ReasoningState::Thinking, "first");
        chain.add_step(step1);
        assert_eq!(chain.latest_step().unwrap().reasoning, "first");

        let step2 = ThoughtStep::new(ReasoningState::Acting, "second");
        chain.add_step(step2);
        assert_eq!(chain.latest_step().unwrap().reasoning, "second");
    }

    #[test]
    fn test_latest_step_mut() {
        let mut chain = ThoughtChain::new();
        let step = ThoughtStep::new(ReasoningState::Thinking, "original");
        chain.add_step(step);

        if let Some(s) = chain.latest_step_mut() {
            s.reasoning = "modified".to_string();
        }
        assert_eq!(chain.latest_step().unwrap().reasoning, "modified");
    }

    #[test]
    fn test_update_step_result() {
        let mut chain = ThoughtChain::new();
        let step = ThoughtStep::new(ReasoningState::Thinking, "thinking");
        chain.add_step(step);

        chain.update_step_result("done", true);
        let latest = chain.latest_step().unwrap();
        assert_eq!(latest.result.as_deref(), Some("done"));
        assert!(latest.is_verified);
    }

    #[test]
    fn test_update_step_result_empty_chain() {
        let mut chain = ThoughtChain::new();
        chain.update_step_result("result", false);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_update_step_observation() {
        let mut chain = ThoughtChain::new();
        let step = ThoughtStep::new(ReasoningState::Observing, "observing");
        chain.add_step(step);

        chain.update_step_observation("saw something");
        assert_eq!(
            chain.latest_step().unwrap().observation.as_deref(),
            Some("saw something")
        );
    }

    #[test]
    fn test_update_step_observation_empty_chain() {
        let mut chain = ThoughtChain::new();
        chain.update_step_observation("nothing");
        assert!(chain.is_empty());
    }

    #[test]
    fn test_chain_len_and_empty() {
        let mut chain = ThoughtChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);

        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "step 1"));
        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 1);

        chain.add_step(ThoughtStep::new(ReasoningState::Acting, "step 2"));
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_to_summary() {
        let mut chain = ThoughtChain::new();
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "think"));
        chain.add_step(ThoughtStep::new(ReasoningState::Acting, "act"));

        let summary = chain.to_summary();
        assert_eq!(summary.total_steps, 2);
        assert_eq!(summary.iterations, 1);
        assert_eq!(summary.current_state, "acting");
        assert_eq!(summary.steps.len(), 2);
    }

    #[test]
    fn test_action_tool_call() {
        let action = Action::tool_call("bash", serde_json::json!({"cmd": "ls"}));
        assert_eq!(action.action_type, ActionType::ToolCall);
        assert_eq!(action.tool_name.unwrap(), "bash");
        assert!(action.tool_input.is_some());
        assert!(action.llm_prompt.is_none());
        assert!(!action.requires_confirmation);
    }

    #[test]
    fn test_action_llm_call() {
        let action = Action::llm_call("explain this code");
        assert_eq!(action.action_type, ActionType::LlmCall);
        assert!(action.tool_name.is_none());
        assert!(action.tool_input.is_none());
        assert_eq!(action.llm_prompt.unwrap(), "explain this code");
        assert!(!action.requires_confirmation);
    }

    #[test]
    fn test_action_user_confirm() {
        let action = Action::user_confirm("proceed with delete?");
        assert_eq!(action.action_type, ActionType::UserConfirm);
        assert!(action.requires_confirmation);
        assert_eq!(action.llm_prompt.unwrap(), "proceed with delete?");
    }

    #[test]
    fn test_action_validate() {
        let action = Action::validate("check output correctness");
        assert_eq!(action.action_type, ActionType::Validate);
        assert!(!action.requires_confirmation);
        assert_eq!(action.llm_prompt.unwrap(), "check output correctness");
    }

    #[test]
    fn test_thought_chain_multiple_iterations() {
        let mut chain = ThoughtChain::new();
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "think 1"));
        chain.add_step(ThoughtStep::new(ReasoningState::Acting, "act 1"));
        chain.add_step(ThoughtStep::new(ReasoningState::Observing, "obs 1"));
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "think 2"));
        chain.add_step(ThoughtStep::new(ReasoningState::Planning, "plan 1"));

        assert_eq!(chain.len(), 5);
        assert_eq!(chain.iteration_count(), 2);
        assert_eq!(chain.current_state, ReasoningState::Planning);
    }

    #[test]
    fn test_thought_chain_emitter_creation() {
        let emitter = ThoughtChainEmitter::new();
        let _receiver = emitter.subscribe();
    }

    #[test]
    fn test_thought_chain_emitter_default() {
        let emitter = ThoughtChainEmitter::default();
        let _receiver = emitter.subscribe();
    }

    #[tokio::test]
    async fn test_thought_chain_emitter_emit_and_receive() {
        let emitter = ThoughtChainEmitter::new();
        let mut receiver = emitter.subscribe();

        let step = ThoughtStep::new(ReasoningState::Thinking, "test step");
        emitter.emit(ThoughtEvent::StepStarted(step.clone()));

        let event = receiver.recv().await.unwrap();
        match event {
            ThoughtEvent::StepStarted(s) => assert_eq!(s.reasoning, "test step"),
            _ => panic!("expected StepStarted event"),
        }
    }

    #[tokio::test]
    async fn test_thought_chain_emitter_state_changed() {
        let emitter = ThoughtChainEmitter::new();
        let mut receiver = emitter.subscribe();

        emitter.emit(ThoughtEvent::StateChanged(ReasoningState::Acting));

        let event = receiver.recv().await.unwrap();
        match event {
            ThoughtEvent::StateChanged(state) => assert_eq!(state, ReasoningState::Acting),
            _ => panic!("expected StateChanged event"),
        }
    }

    #[tokio::test]
    async fn test_thought_chain_emitter_iteration_complete() {
        let emitter = ThoughtChainEmitter::new();
        let mut receiver = emitter.subscribe();

        emitter.emit(ThoughtEvent::IterationComplete(3));

        let event = receiver.recv().await.unwrap();
        match event {
            ThoughtEvent::IterationComplete(n) => assert_eq!(n, 3),
            _ => panic!("expected IterationComplete event"),
        }
    }

    #[tokio::test]
    async fn test_thought_chain_emitter_chain_complete() {
        let emitter = ThoughtChainEmitter::new();
        let mut receiver = emitter.subscribe();

        let mut chain = ThoughtChain::new();
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "done"));
        let summary = chain.to_summary();

        emitter.emit(ThoughtEvent::ChainComplete(summary.clone()));

        let event = receiver.recv().await.unwrap();
        match event {
            ThoughtEvent::ChainComplete(s) => assert_eq!(s.total_steps, 1),
            _ => panic!("expected ChainComplete event"),
        }
    }

    #[tokio::test]
    async fn test_thought_chain_emitter_error() {
        let emitter = ThoughtChainEmitter::new();
        let mut receiver = emitter.subscribe();

        emitter.emit(ThoughtEvent::Error("something went wrong".to_string()));

        let event = receiver.recv().await.unwrap();
        match event {
            ThoughtEvent::Error(msg) => assert_eq!(msg, "something went wrong"),
            _ => panic!("expected Error event"),
        }
    }

    #[test]
    fn test_thought_chain_with_emitter() {
        let emitter = Arc::new(ThoughtChainEmitter::new());
        let chain = ThoughtChain::with_emitter(emitter);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_thought_step_serialization() {
        let step = ThoughtStep::new(ReasoningState::Thinking, "serialize me");
        let json = serde_json::to_string(&step).unwrap();
        let deserialized: ThoughtStep = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reasoning, "serialize me");
        assert_eq!(deserialized.state, ReasoningState::Thinking);
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::tool_call("grep", serde_json::json!({"pattern": "test"}));
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action_type, ActionType::ToolCall);
        assert_eq!(deserialized.tool_name.unwrap(), "grep");
    }

    #[test]
    fn test_chain_summary_serialization() {
        let mut chain = ThoughtChain::new();
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "step"));
        let summary = chain.to_summary();
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ChainSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_steps, 1);
    }
}
