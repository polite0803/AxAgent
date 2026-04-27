use crate::action_executor::{ActionError, ActionExecutor};
use crate::reasoning_state::{ActionType, ReActConfig, ReasoningState};
use crate::self_verifier::{SelfVerifier, VerificationResult};
use crate::thought_chain::{Action, ChainSummary, ThoughtChain, ThoughtEvent, ThoughtStep};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct ReActResult {
    pub final_response: String,
    pub thought_chain: ChainSummary,
    pub success: bool,
    pub iterations: usize,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

impl ReActResult {
    pub fn success(
        response: String,
        chain: ChainSummary,
        iterations: usize,
        duration: Duration,
    ) -> Self {
        Self {
            final_response: response,
            thought_chain: chain,
            success: true,
            iterations,
            total_duration_ms: duration.as_millis() as u64,
            error: None,
        }
    }

    pub fn failure(
        error: String,
        chain: ChainSummary,
        iterations: usize,
        duration: Duration,
    ) -> Self {
        Self {
            final_response: String::new(),
            thought_chain: chain,
            success: false,
            iterations,
            total_duration_ms: duration.as_millis() as u64,
            error: Some(error),
        }
    }
}

pub struct ReActEngine {
    executor: Arc<ActionExecutor>,
    verifier: Arc<SelfVerifier>,
    config: ReActConfig,
    event_sender: broadcast::Sender<ThoughtEvent>,
}

impl ReActEngine {
    pub fn new() -> Self {
        let executor = Arc::new(ActionExecutor::new());
        let verifier = Arc::new(SelfVerifier::new());
        let (event_sender, _) = broadcast::channel(100);

        Self {
            executor,
            verifier,
            config: ReActConfig::default(),
            event_sender,
        }
    }

    pub fn with_config(mut self, config: ReActConfig) -> Self {
        self.config = config;
        self
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ThoughtEvent> {
        self.event_sender.subscribe()
    }

    pub async fn run(&self, user_input: &str) -> ReActResult {
        let start = std::time::Instant::now();
        let mut chain = ThoughtChain::new();
        let mut state = ReasoningState::Thinking;
        let mut retry_count = 0;

        self.emit(ThoughtEvent::StateChanged(state));

        while !matches!(state, ReasoningState::Finished | ReasoningState::Failed) {
            if chain.iteration_count() >= self.config.max_iterations {
                return ReActResult::failure(
                    format!("Max iterations ({}) reached", self.config.max_iterations),
                    chain.to_summary(),
                    chain.iteration_count(),
                    start.elapsed(),
                );
            }

            let step_result = self.process_state(user_input, state, &mut chain).await;

            match step_result {
                Ok((new_state, should_continue)) => {
                    state = new_state;
                    self.emit(ThoughtEvent::StateChanged(state));

                    if matches!(state, ReasoningState::Observing) && !should_continue {
                        retry_count += 1;
                        if retry_count >= self.config.max_retry_attempts {
                            return ReActResult::failure(
                                format!("Max retries ({}) reached", self.config.max_retry_attempts),
                                chain.to_summary(),
                                chain.iteration_count(),
                                start.elapsed(),
                            );
                        }
                    } else {
                        retry_count = 0;
                    }

                    if matches!(state, ReasoningState::Finished) {
                        break;
                    }
                }
                Err(e) => {
                    self.emit(ThoughtEvent::Error(e.to_string()));
                    return ReActResult::failure(
                        e.to_string(),
                        chain.to_summary(),
                        chain.iteration_count(),
                        start.elapsed(),
                    );
                }
            }
        }

        let final_response = chain
            .latest_step()
            .and_then(|s| s.result.clone())
            .unwrap_or_else(|| "Task completed.".to_string());

        self.emit(ThoughtEvent::ChainComplete(chain.to_summary()));

        ReActResult::success(
            final_response,
            chain.to_summary(),
            chain.iteration_count(),
            start.elapsed(),
        )
    }

    async fn process_state(
        &self,
        user_input: &str,
        state: ReasoningState,
        chain: &mut ThoughtChain,
    ) -> Result<(ReasoningState, bool), ReActError> {
        match state {
            ReasoningState::Thinking => {
                let reasoning = format!(
                    "Analyzing user request: {}",
                    truncate_string(user_input, 100)
                );
                let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                chain.add_step(step);
                self.emit(ThoughtEvent::StepCompleted(
                    chain.latest_step().unwrap().clone(),
                ));
                Ok((ReasoningState::Planning, true))
            }
            ReasoningState::Planning => {
                let action = Action {
                    action_type: ActionType::LlmCall,
                    tool_name: None,
                    tool_input: None,
                    llm_prompt: Some(user_input.to_string()),
                    requires_confirmation: false,
                };
                let reasoning = format!("Planning next action: {:?}", action.action_type);
                let step = ThoughtStep::with_action(ReasoningState::Planning, reasoning, action);
                chain.add_step(step);
                self.emit(ThoughtEvent::StepCompleted(
                    chain.latest_step().unwrap().clone(),
                ));
                Ok((ReasoningState::Acting, true))
            }
            ReasoningState::Acting => {
                if let Some(latest) = chain.latest_step_mut() {
                    if let Some(ref action) = latest.action {
                        if action.requires_confirmation {
                            return Ok((ReasoningState::Observing, false));
                        }
                        let result = self.executor.execute(action.clone(), "").await;
                        match result {
                            Ok(action_result) => {
                                let observation = action_result.to_observation();
                                latest.result = Some(action_result.to_observation());
                                latest.observation = Some(observation.clone());
                                self.emit(ThoughtEvent::StepCompleted(latest.clone()));
                                return Ok((ReasoningState::Observing, action_result.is_success()));
                            }
                            Err(e) => {
                                latest.result = Some(format!("Error: {}", e));
                                latest.observation = Some(format!("Error: {}", e));
                                self.emit(ThoughtEvent::StepCompleted(latest.clone()));
                                return Ok((ReasoningState::Observing, false));
                            }
                        }
                    }
                }
                Ok((ReasoningState::Thinking, false))
            }
            ReasoningState::Observing => {
                if let Some(latest) = chain.latest_step() {
                    let verification = if self.config.verification_enabled {
                        self.verifier.verify(latest, user_input).await?
                    } else {
                        VerificationResult::valid("Verification skipped".to_string())
                    };

                    if let Some(step) = chain.latest_step_mut() {
                        step.is_verified = verification.is_valid;
                    }

                    if verification.is_valid {
                        Ok((ReasoningState::Finished, true))
                    } else {
                        let reasoning =
                            format!("Verification failed: {}. Retrying...", verification.reason);
                        let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                        chain.add_step(step);
                        Ok((ReasoningState::Thinking, false))
                    }
                } else {
                    Ok((ReasoningState::Finished, true))
                }
            }
            ReasoningState::Finished | ReasoningState::Failed => Ok((state, false)),
        }
    }

    fn emit(&self, event: ThoughtEvent) {
        let _ = self.event_sender.send(event);
    }
}

impl Default for ReActEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReActError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Action error: {0}")]
    ActionError(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

impl From<ActionError> for ReActError {
    fn from(e: ActionError) -> Self {
        ReActError::ActionError(e.to_string())
    }
}

impl From<crate::self_verifier::VerificationError> for ReActError {
    fn from(e: crate::self_verifier::VerificationError) -> Self {
        ReActError::VerificationFailed(e.to_string())
    }
}
