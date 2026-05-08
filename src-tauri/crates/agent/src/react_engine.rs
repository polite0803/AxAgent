use crate::action_executor::ActionExecutor;
use crate::reasoning_state::{ActionType, ReActConfig, ReasoningContext, ReasoningState};
use crate::self_verifier::{SelfVerifier, VerificationResult};
use crate::thought_chain::{Action, ChainSummary, ThoughtChain, ThoughtEvent, ThoughtStep};
use axagent_core::token_budget::{TokenBudgetDecision, TokenBudgetTracker};
use axagent_core::token_counter::estimate_tokens;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
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
    #[error("LLM reasoning failed: {0}")]
    LlmReasoningError(String),
    #[error("Other: {0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait LlmReasoningProvider: Send + Sync {
    async fn analyze(&self, input: &str, context: &ReasoningContext) -> Result<String, ReActError>;
    async fn think(
        &self,
        input: &str,
        context: &ReasoningContext,
        chain: &ThoughtChain,
    ) -> Result<String, ReActError>;
    async fn plan(
        &self,
        input: &str,
        context: &mut ReasoningContext,
        chain: &ThoughtChain,
    ) -> Result<Action, ReActError>;
    async fn reflect(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError>;
    async fn synthesize(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError>;
}

pub struct DefaultReasoningProvider;

impl DefaultReasoningProvider {
    pub fn new() -> Self {
        Self
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

    fn generate_reasoning(&self, input: &str, context: &ReasoningContext) -> String {
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

    fn generate_reflection(&self, chain: &ThoughtChain, context: &ReasoningContext) -> String {
        let total_steps = chain.steps.len();
        let successful_steps = chain.steps.iter().filter(|s| s.is_verified).count();
        let failed_steps = total_steps - successful_steps;

        format!(
            "Reflection: {} total steps, {} successful, {} failed. Current depth: {}. Strategy adjustment needed.",
            total_steps, successful_steps, failed_steps, context.depth
        )
    }

    fn generate_synthesis(&self, chain: &ThoughtChain, context: &ReasoningContext) -> String {
        let total_steps = chain.steps.len();
        let verified_steps = chain.steps.iter().filter(|s| s.is_verified).count();

        format!(
            "Synthesis: Completed {} steps ({} verified) toward goal: '{}'. Final response ready.",
            total_steps,
            verified_steps,
            context.current_goal.as_deref().unwrap_or("Unknown goal")
        )
    }
}

impl Default for DefaultReasoningProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmReasoningProvider for DefaultReasoningProvider {
    async fn analyze(
        &self,
        input: &str,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok(self.analyze_input(input))
    }

    async fn think(
        &self,
        input: &str,
        context: &ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<String, ReActError> {
        Ok(self.generate_reasoning(input, context))
    }

    async fn plan(
        &self,
        input: &str,
        context: &mut ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<Action, ReActError> {
        let plan = self.create_plan(input, context);
        Ok(Action {
            action_type: ActionType::Plan,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some(plan.clone()),
            requires_confirmation: false,
        })
    }

    async fn reflect(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok(self.generate_reflection(chain, context))
    }

    async fn synthesize(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok(self.generate_synthesis(chain, context))
    }
}

pub struct LlmDrivenReasoningProvider {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
    fallback: DefaultReasoningProvider,
}

impl LlmDrivenReasoningProvider {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model,
            fallback: DefaultReasoningProvider::new(),
        }
    }

    async fn call_llm(&self, system_prompt: &str, user_prompt: &str) -> Result<String, ReActError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(system_prompt.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(user_prompt.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(2048),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = self
            .adapter
            .chat(&self.ctx, request)
            .await
            .map_err(|e| ReActError::LlmReasoningError(e.to_string()))?;

        Ok(response.content)
    }

    fn parse_action_from_response(&self, response: &str) -> Option<Action> {
        let lower = response.to_lowercase();

        if let Some(start) = lower.find("tool_call:") {
            let remainder = &response[start + "tool_call:".len()..];
            let tool_name = remainder.lines().next().unwrap_or("").trim().to_string();
            if !tool_name.is_empty() {
                let tool_input = remainder
                    .lines()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
                return Some(Action {
                    action_type: ActionType::ToolCall,
                    tool_name: Some(tool_name),
                    tool_input: Some(serde_json::Value::String(tool_input)),
                    llm_prompt: None,
                    requires_confirmation: false,
                });
            }
        }

        if lower.contains("llm_call:") || lower.contains("llm prompt:") {
            let prompt = response
                .lines()
                .skip_while(|line| {
                    !line.to_lowercase().contains("llm_call:")
                        && !line.to_lowercase().contains("llm prompt:")
                })
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            if !prompt.is_empty() {
                return Some(Action::llm_call(prompt));
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl LlmReasoningProvider for LlmDrivenReasoningProvider {
    async fn analyze(&self, input: &str, context: &ReasoningContext) -> Result<String, ReActError> {
        let system_prompt = "You are a reasoning analysis engine. Analyze the user input and provide a structured analysis including: complexity level (low/medium/high), key topics, whether it contains code or questions, and the primary goal. Be concise.";
        let user_prompt = format!(
            "Analyze this input:\n\n{}\n\nContext: iteration={}, depth={}",
            input, context.iteration, context.depth
        );

        match self.call_llm(system_prompt, &user_prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => self.fallback.analyze(input, context).await,
        }
    }

    async fn think(
        &self,
        input: &str,
        context: &ReasoningContext,
        chain: &ThoughtChain,
    ) -> Result<String, ReActError> {
        let steps_summary = chain
            .steps
            .iter()
            .take(5)
            .map(|s| format!("[{}] {}", s.state, truncate_string(&s.reasoning, 80)))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "You are a reasoning engine in a ReAct loop. Generate the next thinking step. Consider the current goal, progress so far, and what needs to be done next. Be concise and focused.";
        let user_prompt = format!(
            "Goal: {}\nSub-goals: {}\nIteration: {}\nDepth: {}\nPrevious steps:\n{}\n\nInput: {}",
            context.current_goal.as_deref().unwrap_or("Unknown"),
            context.sub_goals.len(),
            context.iteration,
            context.depth,
            steps_summary,
            input
        );

        match self.call_llm(system_prompt, &user_prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => self.fallback.think(input, context, chain).await,
        }
    }

    async fn plan(
        &self,
        input: &str,
        context: &mut ReasoningContext,
        chain: &ThoughtChain,
    ) -> Result<Action, ReActError> {
        let steps_summary = chain
            .steps
            .iter()
            .take(5)
            .map(|s| format!("[{}] {}", s.state, truncate_string(&s.reasoning, 80)))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "You are a planning engine in a ReAct loop. Create an action plan. If a tool should be called, respond with 'tool_call:<tool_name>' on the first line followed by the tool input as JSON. If an LLM call is needed, respond with 'llm_call:' followed by the prompt. Otherwise, provide a step-by-step plan.";
        let user_prompt = format!(
            "Goal: {}\nSub-goals: {:?}\nIteration: {}\nDepth: {}\nPrevious steps:\n{}\n\nInput: {}",
            context.current_goal.as_deref().unwrap_or("Unknown"),
            context.sub_goals,
            context.iteration,
            context.depth,
            steps_summary,
            input
        );

        match self.call_llm(system_prompt, &user_prompt).await {
            Ok(result) if !result.trim().is_empty() => {
                if let Some(action) = self.parse_action_from_response(&result) {
                    Ok(action)
                } else {
                    context.increment_depth();
                    Ok(Action {
                        action_type: ActionType::Plan,
                        tool_name: None,
                        tool_input: None,
                        llm_prompt: Some(result),
                        requires_confirmation: false,
                    })
                }
            },
            _ => self.fallback.plan(input, context, chain).await,
        }
    }

    async fn reflect(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        let steps_summary = chain
            .steps
            .iter()
            .take(10)
            .map(|s| {
                format!(
                    "[{}] {} (verified: {})",
                    s.state,
                    truncate_string(&s.reasoning, 60),
                    s.is_verified
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "You are a reflection engine in a ReAct loop. Review the progress so far and provide insights on what went well, what went wrong, and how to adjust the strategy. Be concise.";
        let user_prompt = format!(
            "Goal: {}\nIteration: {}\nDepth: {}\nSteps:\n{}\n\nProvide reflection and strategy adjustment.",
            context.current_goal.as_deref().unwrap_or("Unknown"),
            context.iteration,
            context.depth,
            steps_summary
        );

        match self.call_llm(system_prompt, &user_prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => self.fallback.reflect(chain, context).await,
        }
    }

    async fn synthesize(
        &self,
        chain: &ThoughtChain,
        context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        let steps_summary = chain
            .steps
            .iter()
            .take(10)
            .map(|s| format!("[{}] {}", s.state, truncate_string(&s.reasoning, 80)))
            .collect::<Vec<_>>()
            .join("\n");

        let observations = chain
            .steps
            .iter()
            .filter_map(|s| s.observation.as_ref().map(|o| truncate_string(o, 100)))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "You are a synthesis engine in a ReAct loop. Synthesize all the reasoning steps and observations into a coherent final response that addresses the original goal. Be comprehensive yet concise.";
        let user_prompt = format!(
            "Original goal: {}\nIteration: {}\n\nReasoning steps:\n{}\n\nObservations:\n{}\n\nSynthesize a final response.",
            context.current_goal.as_deref().unwrap_or("Unknown"),
            context.iteration,
            steps_summary,
            observations
        );

        match self.call_llm(system_prompt, &user_prompt).await {
            Ok(result) if !result.trim().is_empty() => Ok(result),
            _ => self.fallback.synthesize(chain, context).await,
        }
    }
}

pub struct ReActEngine {
    executor: Arc<ActionExecutor>,
    verifier: Arc<SelfVerifier>,
    config: ReActConfig,
    event_sender: broadcast::Sender<ThoughtEvent>,
    token_budget: TokenBudgetTracker,
    reasoning_provider: Arc<dyn LlmReasoningProvider>,
    planner: Option<Arc<tokio::sync::Mutex<crate::hierarchical_planner::HierarchicalPlanner>>>,
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
            reasoning_provider: Arc::new(DefaultReasoningProvider::new()),
            planner: None,
        }
    }

    pub fn with_config(mut self, config: ReActConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_reasoning_provider(mut self, provider: Arc<dyn LlmReasoningProvider>) -> Self {
        self.reasoning_provider = provider;
        self
    }

    pub fn with_planner(
        mut self,
        planner: Arc<tokio::sync::Mutex<crate::hierarchical_planner::HierarchicalPlanner>>,
    ) -> Self {
        self.planner = Some(planner);
        self
    }

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

                    if self.config.token_budget_enabled {
                        let estimated_tokens = estimate_chain_tokens(&chain);
                        let decision = self
                            .token_budget
                            .check(self.config.token_budget_limit, estimated_tokens);

                        match decision {
                            TokenBudgetDecision::Continue { nudge_message, .. } => {
                                if context.iteration > 0 && context.iteration.is_multiple_of(5) {
                                    let step =
                                        ThoughtStep::new(ReasoningState::Reflecting, nudge_message);
                                    chain.add_step(step);
                                }
                            },
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
                            },
                        }
                    }
                },
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

                    if consecutive_failures >= 2 {
                        if let Some(ref planner) = self.planner {
                            let task_id = format!("iteration_{}", context.iteration);
                            let error_msg = truncate_string(&e.to_string(), 100);
                            let replan_result = planner.lock().await.replan(
                                crate::hierarchical_planner::ReplanReason::StepFailed {
                                    task_id: task_id.clone(),
                                    error: error_msg.clone(),
                                },
                                vec![crate::hierarchical_planner::ReplanAction::ModifyTask {
                                    task_id: task_id.clone(),
                                    modifications: serde_json::json!({
                                        "description": format!("Retry after error: {}", error_msg)
                                    }),
                                }],
                            );

                            match replan_result {
                                Ok(record) => {
                                    tracing::warn!(
                                        version = record.version,
                                        reason = ?record.reason,
                                        "Replan triggered after consecutive failures, transitioning to Planning"
                                    );
                                    state = ReasoningState::Planning;
                                    continue;
                                },
                                Err(replan_err) => {
                                    tracing::warn!(
                                        error = %replan_err,
                                        "Replan failed, falling back to Thinking state"
                                    );
                                    state = ReasoningState::Thinking;
                                },
                            }
                        } else {
                            state = ReasoningState::Thinking;
                        }
                    } else {
                        state = ReasoningState::Thinking;
                    }
                },
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
                let reasoning = self.reasoning_provider.analyze(user_input, context).await?;
                let step = ThoughtStep::new(ReasoningState::Analyzing, reasoning.clone());
                chain.add_step(step);

                context.set_goal(reasoning);
                self.extract_sub_goals(user_input, context);

                self.emit(ThoughtEvent::StepCompleted(chain.latest_step().unwrap().clone()));

                Ok((ReasoningState::Thinking, true))
            },

            ReasoningState::Thinking => {
                let reasoning = self
                    .reasoning_provider
                    .think(user_input, context, chain)
                    .await?;
                let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(chain.latest_step().unwrap().clone()));

                Ok((ReasoningState::Planning, true))
            },

            ReasoningState::Planning => {
                let action = self
                    .reasoning_provider
                    .plan(user_input, context, chain)
                    .await?;
                let reasoning = format!(
                    "Creating plan: {}",
                    action.llm_prompt.as_deref().unwrap_or("execute action")
                );
                let step = ThoughtStep::with_action(ReasoningState::Planning, reasoning, action);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(chain.latest_step().unwrap().clone()));

                Ok((ReasoningState::Acting, true))
            },

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
                            },
                            Err(e) => {
                                latest.result = Some(format!("Error: {}", e));
                                latest.observation = Some(format!("Error: {}", e));
                                self.emit(ThoughtEvent::StepCompleted(latest.clone()));
                                return Err(ReActError::ActionError(e.to_string()));
                            },
                        }
                    }
                }
                Ok((ReasoningState::Thinking, false))
            },

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
                        Ok((ReasoningState::Synthesizing, true))
                    } else {
                        let reasoning =
                            format!("Verification failed: {}. Retrying...", verification.reason);
                        let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                        chain.add_step(step);
                        Ok((ReasoningState::Thinking, false))
                    }
                } else {
                    Ok((ReasoningState::Synthesizing, true))
                }
            },

            ReasoningState::Reflecting => {
                let reflection = self.reasoning_provider.reflect(chain, context).await?;
                let step = ThoughtStep::new(ReasoningState::Reflecting, reflection);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(chain.latest_step().unwrap().clone()));

                self.adjust_strategy(context);

                if let Some(ref planner) = self.planner {
                    let actions: Vec<crate::hierarchical_planner::ReplanAction> = context
                        .sub_goals
                        .iter()
                        .take(3)
                        .enumerate()
                        .map(|(i, _)| crate::hierarchical_planner::ReplanAction::Reorder {
                            task_id: format!("subgoal_{}", i),
                            new_position: i,
                        })
                        .collect();

                    let replan_result = planner.lock().await.replan(
                        crate::hierarchical_planner::ReplanReason::StepFailed {
                            task_id: "subgoal_0".to_string(),
                            error: "Reflection triggered replan".to_string(),
                        },
                        actions,
                    );

                    if let Ok(record) = replan_result {
                        tracing::warn!(
                            version = record.version,
                            reason = ?record.reason,
                            "Replan triggered during reflection"
                        );
                    }
                }

                Ok((ReasoningState::Thinking, true))
            },

            ReasoningState::Synthesizing => {
                let synthesis = self.reasoning_provider.synthesize(chain, context).await?;
                let step = ThoughtStep::new(ReasoningState::Synthesizing, synthesis.clone());
                chain.add_step(step);

                if let Some(latest) = chain.latest_step_mut() {
                    latest.result = Some(synthesis);
                }

                self.emit(ThoughtEvent::StepCompleted(chain.latest_step().unwrap().clone()));

                Ok((ReasoningState::Finished, true))
            },

            ReasoningState::Finished | ReasoningState::Failed => Ok((state, false)),
        }
    }

    fn extract_sub_goals(&self, input: &str, context: &mut ReasoningContext) {
        let sentences: Vec<&str> = input.split('.').filter(|s| !s.trim().is_empty()).collect();

        for (i, sentence) in sentences.iter().take(5).enumerate() {
            if sentence.contains(',') || sentence.len() > 50 {
                context.add_sub_goal(format!("Sub-goal {}: {}", i + 1, sentence.trim()));
            }
        }
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

        assert!(result.iterations > 0 || result.error.is_some());
        if result.success {
            assert!(!result.final_response.is_empty());
        }
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

    #[tokio::test]
    async fn test_default_reasoning_provider_analyze() {
        let provider = DefaultReasoningProvider::new();
        let context = ReasoningContext::new("Hello world");
        let result = provider.analyze("Hello world", &context).await.unwrap();
        assert!(result.contains("2 words"));
        assert!(result.contains("complexity=low"));
    }

    #[tokio::test]
    async fn test_default_reasoning_provider_think() {
        let provider = DefaultReasoningProvider::new();
        let mut context = ReasoningContext::new("Test input");
        context.set_goal("Test goal".to_string());
        let chain = ThoughtChain::new();
        let result = provider
            .think("Test input", &context, &chain)
            .await
            .unwrap();
        assert!(result.contains("Test goal"));
    }

    #[tokio::test]
    async fn test_default_reasoning_provider_plan() {
        let provider = DefaultReasoningProvider::new();
        let mut context = ReasoningContext::new("Test input");
        let chain = ThoughtChain::new();
        let action = provider
            .plan("Test input", &mut context, &chain)
            .await
            .unwrap();
        assert_eq!(action.action_type, ActionType::Plan);
        assert!(action.llm_prompt.is_some());
    }

    #[tokio::test]
    async fn test_default_reasoning_provider_reflect() {
        let provider = DefaultReasoningProvider::new();
        let context = ReasoningContext::new("Test input");
        let chain = ThoughtChain::new();
        let result = provider.reflect(&chain, &context).await.unwrap();
        assert!(result.contains("Reflection:"));
    }

    #[tokio::test]
    async fn test_default_reasoning_provider_synthesize() {
        let provider = DefaultReasoningProvider::new();
        let mut context = ReasoningContext::new("Test input");
        context.set_goal("Test goal".to_string());
        let chain = ThoughtChain::new();
        let result = provider.synthesize(&chain, &context).await.unwrap();
        assert!(result.contains("Synthesis:"));
        assert!(result.contains("Test goal"));
    }

    #[tokio::test]
    async fn test_react_engine_with_custom_provider() {
        struct TestProvider;

        #[async_trait::async_trait]
        impl LlmReasoningProvider for TestProvider {
            async fn analyze(
                &self,
                _input: &str,
                _context: &ReasoningContext,
            ) -> Result<String, ReActError> {
                Ok("Custom analysis".to_string())
            }
            async fn think(
                &self,
                _input: &str,
                _context: &ReasoningContext,
                _chain: &ThoughtChain,
            ) -> Result<String, ReActError> {
                Ok("Custom thinking".to_string())
            }
            async fn plan(
                &self,
                _input: &str,
                context: &mut ReasoningContext,
                _chain: &ThoughtChain,
            ) -> Result<Action, ReActError> {
                context.increment_depth();
                Ok(Action {
                    action_type: ActionType::Plan,
                    tool_name: None,
                    tool_input: None,
                    llm_prompt: Some("Custom plan".to_string()),
                    requires_confirmation: false,
                })
            }
            async fn reflect(
                &self,
                _chain: &ThoughtChain,
                _context: &ReasoningContext,
            ) -> Result<String, ReActError> {
                Ok("Custom reflection".to_string())
            }
            async fn synthesize(
                &self,
                _chain: &ThoughtChain,
                _context: &ReasoningContext,
            ) -> Result<String, ReActError> {
                Ok("Custom synthesis result".to_string())
            }
        }

        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(TestProvider));
        let result = engine.run("Test with custom provider").await;

        if result.success {
            assert!(!result.final_response.is_empty());
        }
    }
}
