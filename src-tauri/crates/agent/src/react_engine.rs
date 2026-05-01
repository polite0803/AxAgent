use crate::action_executor::ActionExecutor;
use crate::reasoning_state::{ActionType, ReActConfig, ReasoningContext, ReasoningState};
use crate::self_verifier::{SelfVerifier, VerificationResult};
use crate::thought_chain::{Action, ChainSummary, ThoughtChain, ThoughtEvent, ThoughtStep};
use axagent_core::token_budget::{TokenBudgetDecision, TokenBudgetTracker};
use axagent_core::token_counter::estimate_tokens;
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
    pub context: ReasoningContext,
}

impl ReActResult {
    pub fn success(
        response: String,
        chain: ChainSummary,
        iterations: usize,
        duration: Duration,
        context: ReasoningContext,
    ) -> Self {
        Self {
            final_response: response,
            thought_chain: chain,
            success: true,
            iterations,
            total_duration_ms: duration.as_millis() as u64,
            error: None,
            context,
        }
    }

    pub fn failure(
        error: String,
        chain: ChainSummary,
        iterations: usize,
        duration: Duration,
        context: ReasoningContext,
    ) -> Self {
        Self {
            final_response: String::new(),
            thought_chain: chain,
            success: false,
            iterations,
            total_duration_ms: duration.as_millis() as u64,
            error: Some(error),
            context,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReActError {
    #[error("Action failed: {0}")]
    ActionError(String),
    #[error("Max iterations reached")]
    MaxIterations,
    #[error("Cancelled")]
    Cancelled,
    #[error("Verification failed: {0}")]
    VerificationError(String),
    #[error("Other: {0}")]
    Other(String),
}

pub struct ReActEngine {
    executor: Arc<ActionExecutor>,
    verifier: Arc<SelfVerifier>,
    config: ReActConfig,
    event_sender: broadcast::Sender<ThoughtEvent>,
    token_budget: TokenBudgetTracker,
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
            token_budget: TokenBudgetTracker::new(),
        }
    }

    pub fn with_config(mut self, config: ReActConfig) -> Self {
        self.config = config;
        self
    }

    /// 重置 token 预算跟踪器（新会话开始时调用）。
    pub fn reset_token_budget(&mut self) {
        self.token_budget.reset();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ThoughtEvent> {
        self.event_sender.subscribe()
    }

    pub async fn run(&mut self, user_input: &str) -> ReActResult {
        let start = std::time::Instant::now();
        let mut chain = ThoughtChain::new();
        let mut context = ReasoningContext::new(user_input);
        let mut state = if self.config.enable_analyzing {
            ReasoningState::Analyzing
        } else {
            ReasoningState::Thinking
        };
        let mut retry_count = 0;
        let mut consecutive_failures = 0;

        self.emit(ThoughtEvent::StateChanged(state));

        while !state.is_terminal() {
            context.increment_iteration();

            if context.iteration >= self.config.max_iterations {
                return ReActResult::failure(
                    format!("Max iterations ({}) reached", self.config.max_iterations),
                    chain.to_summary(),
                    context.iteration,
                    start.elapsed(),
                    context,
                );
            }

            if context.depth >= self.config.max_depth {
                return ReActResult::failure(
                    format!("Max depth ({}) reached", self.config.max_depth),
                    chain.to_summary(),
                    context.iteration,
                    start.elapsed(),
                    context,
                );
            }

            let step_result: Result<(ReasoningState, bool), ReActError> = self
                .process_state(user_input, state, &mut chain, &mut context)
                .await;

            match step_result {
                Ok((new_state, should_continue)) => {
                    let previous_state = state;
                    state = new_state;
                    self.emit(ThoughtEvent::StateChanged(state));

                    if previous_state.requires_observation() && !should_continue {
                        consecutive_failures += 1;
                        retry_count += 1;

                        if retry_count >= self.config.max_retry_attempts {
                            return ReActResult::failure(
                                format!("Max retries ({}) reached", self.config.max_retry_attempts),
                                chain.to_summary(),
                                context.iteration,
                                start.elapsed(),
                                context,
                            );
                        }
                    } else {
                        retry_count = 0;
                        consecutive_failures = 0;
                    }

                    if self.config.enable_reflection
                        && consecutive_failures >= self.config.reflection_threshold
                        && matches!(state, ReasoningState::Thinking)
                    {
                        state = ReasoningState::Reflecting;
                        consecutive_failures = 0;
                        self.emit(ThoughtEvent::StateChanged(state));
                    }

                    if state.is_terminal() {
                        break;
                    }

                    // Token 预算检查：防止无效循环耗尽上下文窗口
                    if self.config.token_budget_enabled {
                        let estimated_tokens = estimate_chain_tokens(&chain);
                        let decision = self.token_budget.check(
                            self.config.token_budget_limit,
                            estimated_tokens,
                        );

                        match decision {
                            TokenBudgetDecision::Continue { nudge_message, .. } => {
                                // 在接近预算上限时向链中添加提示
                                if context.iteration > 0 && context.iteration % 5 == 0 {
                                    let step = ThoughtStep::new(
                                        ReasoningState::Reflecting,
                                        nudge_message,
                                    );
                                    chain.add_step(step);
                                }
                            }
                            TokenBudgetDecision::Stop { completion_event } => {
                                if let Some(event) = completion_event {
                                    let reason = if event.diminishing_returns {
                                        format!(
                                            "Token budget exhausted: diminishing returns detected after {} continuations ({}% of {} tokens used in {}ms)",
                                            event.continuation_count,
                                            event.pct_used,
                                            event.budget,
                                            event.duration_ms,
                                        )
                                    } else {
                                        format!(
                                            "Token budget exhausted: {}% of {} tokens used",
                                            event.pct_used, event.budget,
                                        )
                                    };
                                    self.emit(ThoughtEvent::Error(reason.clone()));
                                    return ReActResult::failure(
                                        reason,
                                        chain.to_summary(),
                                        context.iteration,
                                        start.elapsed(),
                                        context,
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    self.emit(ThoughtEvent::Error(e.to_string()));
                    consecutive_failures += 1;

                    if consecutive_failures >= self.config.max_retry_attempts {
                        return ReActResult::failure(
                            e.to_string(),
                            chain.to_summary(),
                            context.iteration,
                            start.elapsed(),
                            context,
                        );
                    }

                    state = ReasoningState::Thinking;
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
            context.iteration,
            start.elapsed(),
            context,
        )
    }

    async fn process_state(
        &self,
        user_input: &str,
        state: ReasoningState,
        chain: &mut ThoughtChain,
        context: &mut ReasoningContext,
    ) -> Result<(ReasoningState, bool), ReActError> {
        match state {
            ReasoningState::Idle => Ok((ReasoningState::Analyzing, true)),

            ReasoningState::Analyzing => {
                let reasoning = self.analyze_input(user_input);
                let step = ThoughtStep::new(ReasoningState::Analyzing, reasoning.clone());
                chain.add_step(step);

                context.set_goal(reasoning);
                self.extract_sub_goals(user_input, context);

                self.emit(ThoughtEvent::StepCompleted(
                    chain.latest_step().unwrap().clone(),
                ));

                Ok((ReasoningState::Thinking, true))
            }

            ReasoningState::Thinking => {
                let reasoning = self.generate_reasoning(user_input, context);
                let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(
                    chain.latest_step().unwrap().clone(),
                ));

                Ok((ReasoningState::Planning, true))
            }

            ReasoningState::Planning => {
                let plan = self.create_plan(user_input, context);
                let action = Action {
                    action_type: ActionType::Plan,
                    tool_name: None,
                    tool_input: None,
                    llm_prompt: Some(plan.clone()),
                    requires_confirmation: false,
                };
                let reasoning = format!("Creating plan: {}", plan);
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
                                latest.result = Some(action_result.to_observation());
                                latest.observation = Some(action_result.to_observation());
                                self.emit(ThoughtEvent::StepCompleted(latest.clone()));
                                return Ok((ReasoningState::Observing, action_result.is_success()));
                            }
                            Err(e) => {
                                latest.result = Some(format!("Error: {}", e));
                                latest.observation = Some(format!("Error: {}", e));
                                self.emit(ThoughtEvent::StepCompleted(latest.clone()));
                                return Err(ReActError::ActionError(e.to_string()));
                            }
                        }
                    }
                }
                Ok((ReasoningState::Thinking, false))
            }

            ReasoningState::Observing => {
                if let Some(latest) = chain.latest_step() {
                    let verification = if self.config.verification_enabled {
                        self.verifier
                            .verify(latest, user_input)
                            .await
                            .map_err(|e| ReActError::VerificationError(e.to_string()))?
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

            ReasoningState::Reflecting => {
                let reflection = self.generate_reflection(chain, context);
                let step = ThoughtStep::new(ReasoningState::Reflecting, reflection);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(
                    chain.latest_step().unwrap().clone(),
                ));

                self.adjust_strategy(context);

                Ok((ReasoningState::Thinking, true))
            }

            ReasoningState::Finished | ReasoningState::Failed => Ok((state, false)),
        }
    }

    fn analyze_input(&self, input: &str) -> String {
        let word_count = input.split_whitespace().count();
        let has_code =
            input.contains("```") || input.contains("function") || input.contains("class");
        let has_questions = input.contains('?');

        let complexity = if word_count > 100 {
            "high"
        } else if word_count > 30 {
            "medium"
        } else {
            "low"
        };

        format!(
            "Input analysis: {} words, complexity={}, contains_code={}, contains_questions={}",
            word_count, complexity, has_code, has_questions
        )
    }

    fn extract_sub_goals(&self, input: &str, context: &mut ReasoningContext) {
        let sentences: Vec<&str> = input.split('.').filter(|s| !s.trim().is_empty()).collect();

        for (i, sentence) in sentences.iter().take(5).enumerate() {
            if sentence.contains(',') || sentence.len() > 50 {
                context.add_sub_goal(format!("Sub-goal {}: {}", i + 1, sentence.trim()));
            }
        }
    }

    fn generate_reasoning(&self, input: &str, context: &mut ReasoningContext) -> String {
        let goal = context.current_goal.as_deref().unwrap_or("Unknown goal");
        let sub_goals_count = context.sub_goals.len();

        format!(
            "Working toward goal: '{}'. {} sub-goals identified. Current iteration: {}. Input: '{}'",
            truncate_string(goal, 50),
            sub_goals_count,
            context.iteration,
            truncate_string(input, 80)
        )
    }

    fn create_plan(&self, input: &str, context: &mut ReasoningContext) -> String {
        context.increment_depth();

        let plan_steps = if context.depth == 1 {
            let truncated = truncate_string(input, 60);
            vec![
                format!("Analyze the requirements for: '{}'", truncated),
                "Execute necessary actions".to_string(),
                "Verify results".to_string(),
                "Synthesize response".to_string(),
            ]
        } else {
            vec![
                "Execute next step".to_string(),
                "Verify result".to_string(),
                "Iterate if needed".to_string(),
            ]
        };

        plan_steps.join(" -> ")
    }

    fn generate_reflection(&self, chain: &ThoughtChain, context: &mut ReasoningContext) -> String {
        let total_steps = chain.steps.len();
        let successful_steps = chain.steps.iter().filter(|s| s.is_verified).count();
        let failed_steps = total_steps - successful_steps;

        format!(
            "Reflection: {} total steps, {} successful, {} failed. Current depth: {}. Strategy adjustment needed.",
            total_steps, successful_steps, failed_steps, context.depth
        )
    }

    fn adjust_strategy(&self, context: &mut ReasoningContext) {
        context.depth = 0;
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

/// 从 ThoughtChain 估算 token 数量，用于预算跟踪。
fn estimate_chain_tokens(chain: &ThoughtChain) -> u64 {
    let mut total: usize = 0;
    for step in &chain.steps {
        total += estimate_tokens(&step.reasoning);
        if let Some(ref result) = step.result {
            total += estimate_tokens(result);
        }
        if let Some(ref observation) = step.observation {
            total += estimate_tokens(observation);
        }
    }
    total as u64
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_react_engine_basic() {
        let mut engine = ReActEngine::new();
        let result = engine.run("Hello, how are you?").await;

        assert!(result.iterations > 0);
        assert!(!result.final_response.is_empty());
    }

    #[tokio::test]
    async fn test_react_engine_with_analyzing_disabled() {
        let mut engine = ReActEngine::new().with_config(ReActConfig::for_simple_task());
        let result = engine.run("Simple question").await;

        assert!(result.success || result.error.is_some());
    }

    #[tokio::test]
    async fn test_reasoning_context() {
        let mut context = ReasoningContext::new("Test input");
        context.add_sub_goal("Goal 1".to_string());
        context.add_sub_goal("Goal 2".to_string());
        context.increment_iteration();
        context.increment_depth();

        assert_eq!(context.sub_goals.len(), 2);
        assert_eq!(context.iteration, 1);
        assert_eq!(context.depth, 1);
    }

    #[tokio::test]
    async fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }
}
