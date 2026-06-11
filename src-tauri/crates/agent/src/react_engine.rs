// SPDX-License-Identifier: AGPL-3.0-only

use crate::action_executor::ActionExecutor;
use crate::cycle_detector::CycleDetector;
use crate::reasoning_state::{ActionType, ReActConfig, ReasoningContext, ReasoningState};
use crate::self_verifier::{SelfVerifier, VerificationResult};
use crate::thought_chain::{Action, ChainSummary, ThoughtChain, ThoughtEvent, ThoughtStep};
use axagent_core::token_budget::{TokenBudgetDecision, TokenBudgetTracker};
use axagent_core::token_counter::estimate_tokens;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use axagent_runtime_core::{LlmCallConfig, execute_llm};
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
    #[error("Cycle detected: {0}")]
    CycleDetected(String),
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
    /// 中心化 LLM 调用配置（可选，设置后走 execute_llm 路径）
    llm_call_config: Option<LlmCallConfig>,
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
            llm_call_config: None,
        }
    }

    /// 注入中心化 LLM 调用配置
    pub fn with_llm_call_config(mut self, config: LlmCallConfig) -> Self {
        self.llm_call_config = Some(config);
        self
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
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(user_prompt.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
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

        // ── 中心化路径：如果配置了 LlmCallConfig，走 execute_llm() ──
        if let Some(ref config) = self.llm_call_config {
            return match execute_llm(&*self.adapter, &self.ctx, request, config).await {
                Ok(result) => Ok(result.response.content),
                Err(e) => Err(ReActError::LlmReasoningError(e)),
            };
        }

        // ── 旧路径：直接 adapter.chat() ──
        const MAX_RETRIES: usize = 2;
        let mut last_error: Option<String> = None;

        for attempt in 0..=MAX_RETRIES {
            match self.adapter.chat(&self.ctx, request.clone()).await {
                Ok(response) => return Ok(response.content),
                Err(e) => {
                    let err_str = e.to_string();
                    let is_retryable = err_str.contains("timeout")
                        || err_str.contains("connection")
                        || err_str.contains("reset")
                        || err_str.contains("429")
                        || err_str.contains("503")
                        || err_str.contains("502")
                        || err_str.contains("rate limit");

                    if !is_retryable || attempt >= MAX_RETRIES {
                        return Err(ReActError::LlmReasoningError(err_str));
                    }

                    last_error = Some(err_str);
                    let delay = std::time::Duration::from_millis(500 * 2u64.pow(attempt as u32));
                    tokio::time::sleep(delay).await;
                },
            }
        }

        Err(ReActError::LlmReasoningError(
            last_error.unwrap_or_else(|| "LLM call failed after retries".to_string()),
        ))
    }

    fn parse_action_from_response(&self, response: &str) -> Option<Action> {
        // 尝试 JSON 解析（优先）
        if let Some(action) = self.try_parse_json_action(response) {
            return Some(action);
        }

        // 回退：旧版字符串匹配
        self.try_parse_legacy_action(response)
    }

    fn try_parse_json_action(&self, response: &str) -> Option<Action> {
        let json_str = extract_json_from_response(response);

        #[derive(serde::Deserialize)]
        struct ActionResponse {
            action_type: Option<String>,
            tool_name: Option<String>,
            tool_input: Option<serde_json::Value>,
            llm_prompt: Option<String>,
            requires_confirmation: Option<bool>,
        }

        let parsed: ActionResponse = serde_json::from_str(&json_str).ok()?;

        let action_type = parsed
            .action_type
            .as_deref()
            .unwrap_or("plan")
            .to_lowercase();

        match action_type.as_str() {
            "tool_call" | "toolcall" => {
                let tool_name = parsed.tool_name?;
                if tool_name.is_empty() {
                    return None;
                }
                Some(Action {
                    action_type: ActionType::ToolCall,
                    tool_name: Some(tool_name),
                    tool_input: parsed.tool_input,
                    llm_prompt: None,
                    requires_confirmation: parsed.requires_confirmation.unwrap_or(false),
                })
            },
            "llm_call" | "llmcall" => {
                let prompt = parsed.llm_prompt?;
                if prompt.is_empty() {
                    return None;
                }
                Some(Action::llm_call(prompt))
            },
            "user_confirm" | "userconfirm" => {
                let message = parsed.llm_prompt.unwrap_or_default();
                Some(Action::user_confirm(message))
            },
            _ => {
                // plan / analyze / reflect / synthesize
                Some(Action {
                    action_type: ActionType::Plan,
                    tool_name: None,
                    tool_input: None,
                    llm_prompt: parsed.llm_prompt.or_else(|| {
                        Some(
                            parsed
                                .tool_name
                                .as_deref()
                                .unwrap_or("execute plan")
                                .to_string(),
                        )
                    }),
                    requires_confirmation: false,
                })
            },
        }
    }

    fn try_parse_legacy_action(&self, response: &str) -> Option<Action> {
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
        let ctx_config = crate::context_window::ContextWindowConfig::default();
        let ctx_window = crate::context_window::ContextWindow::from_chain(chain, &ctx_config);
        let steps_summary = ctx_window.to_prompt_string();

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
        let ctx_config = crate::context_window::ContextWindowConfig::default();
        let ctx_window = crate::context_window::ContextWindow::from_chain(chain, &ctx_config);
        let steps_summary = ctx_window.to_prompt_string();

        let system_prompt = "You are a planning engine in a ReAct loop. Respond ONLY with a single JSON object (no markdown, no extra text):\n{\n  \"action_type\": \"tool_call\" | \"llm_call\" | \"user_confirm\" | \"plan\",\n  \"tool_name\": \"<name>\",        // required for tool_call\n  \"tool_input\": {<params>},      // required for tool_call, must be valid JSON object\n  \"llm_prompt\": \"<prompt>\",      // required for llm_call or plan\n  \"requires_confirmation\": false  // optional, set true for destructive operations\n}\nChoose the action that best advances the goal.";
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
        let ctx_config = crate::context_window::ContextWindowConfig {
            recent_count: 8,
            max_summary_chars: 300,
            summarize_older_than: 15,
            deduplicate_similar: true,
        };
        let ctx_window = crate::context_window::ContextWindow::from_chain(chain, &ctx_config);
        let steps_summary = ctx_window.to_prompt_string();

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
        let ctx_config = crate::context_window::ContextWindowConfig {
            recent_count: 10,
            max_summary_chars: 500,
            summarize_older_than: 20,
            deduplicate_similar: true,
        };
        let ctx_window = crate::context_window::ContextWindow::from_chain(chain, &ctx_config);
        let steps_summary = ctx_window.to_prompt_string();

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
    cycle_detector: Option<CycleDetector>,
    checkpoint_manager: Option<Arc<crate::checkpoint::CheckpointManager>>,
    checkpoint_session_id: Option<String>,
    checkpoint_interval: usize,
    goal_evaluator: Option<std::sync::Mutex<crate::goal_evaluator::GoalEvaluator>>,
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
            cycle_detector: None,
            checkpoint_manager: None,
            checkpoint_session_id: None,
            checkpoint_interval: 0,
            goal_evaluator: None,
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

    pub fn with_cycle_detection(mut self, max_repeat_calls: usize, max_no_progress: usize) -> Self {
        self.cycle_detector = Some(CycleDetector::new(max_repeat_calls, max_no_progress));
        self
    }

    pub fn with_checkpoint(
        mut self,
        manager: Arc<crate::checkpoint::CheckpointManager>,
        session_id: String,
        interval: usize,
    ) -> Self {
        self.checkpoint_manager = Some(manager);
        self.checkpoint_session_id = Some(session_id);
        self.checkpoint_interval = interval;
        self
    }

    pub fn with_goal_evaluation(mut self, max_not_achieved: usize) -> Self {
        self.goal_evaluator = Some(std::sync::Mutex::new(
            crate::goal_evaluator::GoalEvaluator::new(max_not_achieved),
        ));
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

        // 根据配置自动启用循环检测
        if self.config.cycle_detection_enabled && self.cycle_detector.is_none() {
            self.cycle_detector = Some(CycleDetector::new(
                self.config.max_repeated_calls,
                self.config.max_no_progress_iterations,
            ));
        }

        // 根据配置自动启用目标达成判定
        if self.config.goal_evaluation_enabled && self.goal_evaluator.is_none() {
            self.goal_evaluator =
                Some(std::sync::Mutex::new(crate::goal_evaluator::GoalEvaluator::new(3)));
        }

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
                        && consecutive_failures >= self.effective_reflection_threshold(&context)
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

                    // 循环检测
                    if let Some(ref mut detector) = self.cycle_detector {
                        let latest_obs = chain.latest_step().and_then(|s| s.observation.as_deref());
                        let alerts = detector.record_step(
                            chain
                                .latest_step()
                                .and_then(|s| s.action.as_ref())
                                .and_then(|a| a.tool_name.as_deref())
                                .unwrap_or(""),
                            chain
                                .latest_step()
                                .and_then(|s| s.action.as_ref())
                                .and_then(|a| a.tool_input.as_ref())
                                .map(|v| v.to_string())
                                .as_deref()
                                .unwrap_or(""),
                            chain.steps.len(),
                            latest_obs,
                            context.iteration,
                        );

                        if let Some(alert) = alerts.into_iter().next() {
                            let msg = match alert {
                                crate::cycle_detector::CycleAlert::RepeatCall {
                                    tool_name,
                                    count,
                                    first_seen_at_iteration,
                                } => format!(
                                    "检测到循环调用: 工具 '{}' 已重复 {} 次 (首次出现在第 {} 次迭代)",
                                    tool_name, count, first_seen_at_iteration
                                ),
                                crate::cycle_detector::CycleAlert::NoProgress {
                                    stagnant_iterations,
                                } => format!(
                                    "检测到状态停滞: 连续 {} 次迭代无实质性进展",
                                    stagnant_iterations
                                ),
                            };
                            self.emit(ThoughtEvent::Error(msg.clone()));
                            return ReActResult::failure(
                                msg,
                                chain.to_summary(),
                                context.iteration,
                                start.elapsed(),
                                context,
                            );
                        }
                    }

                    // 断点续执行：每 N 次迭代自动保存
                    if let Some(ref cm) = self.checkpoint_manager
                        && self.checkpoint_interval > 0
                        && context.iteration.is_multiple_of(self.checkpoint_interval)
                        && let Some(ref sid) = self.checkpoint_session_id
                    {
                        let cp = crate::checkpoint::ReActEngineCheckpoint {
                            session_id: sid.clone(),
                            iteration: context.iteration,
                            chain: chain.clone(),
                            context: context.clone(),
                            current_state: state,
                            token_budget_used: estimate_chain_tokens(&chain),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        if let Err(e) = cm.save_react_checkpoint(&cp).await {
                            tracing::warn!(
                                error = %e,
                                iteration = context.iteration,
                                "Failed to save ReAct checkpoint"
                            );
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
                            let mut planner_guard = planner.lock().await;
                            let failed_ids = planner_guard.get_failed_steps();
                            let pending_ids = planner_guard.get_pending_steps();
                            let target_id = failed_ids
                                .first()
                                .or(pending_ids.first())
                                .cloned()
                                .unwrap_or_else(|| format!("iteration_{}", context.iteration));
                            let error_msg = truncate_string(&e.to_string(), 100);
                            let replan_result = planner_guard.replan(
                                crate::hierarchical_planner::ReplanReason::StepFailed {
                                    task_id: target_id.clone(),
                                    error: error_msg.clone(),
                                },
                                vec![crate::hierarchical_planner::ReplanAction::ModifyTask {
                                    task_id: target_id.clone(),
                                    modifications: serde_json::json!({
                                        "description": format!("Retry after error: {}", error_msg)
                                    }),
                                }],
                            );

                            match replan_result {
                                Ok(record) => {
                                    tracing::info!(
                                        version = record.version,
                                        reason = ?record.reason,
                                        "Replan triggered after consecutive failures, transitioning to Planning"
                                    );
                                    state = ReasoningState::Planning;
                                    continue;
                                },
                                Err(replan_err) => {
                                    tracing::info!(
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

    /// 从最近的 checkpoint 恢复执行
    ///
    /// 需要先通过 `with_checkpoint()` 配置 CheckpointManager 和 session_id。
    /// 如果找不到 checkpoint，回退到普通的 `run()`。
    pub async fn resume(&mut self, user_input: &str) -> ReActResult {
        let session_id = match &self.checkpoint_session_id {
            Some(sid) => sid.clone(),
            None => return self.run(user_input).await,
        };

        let cm = match &self.checkpoint_manager {
            Some(cm) => cm.clone(),
            None => return self.run(user_input).await,
        };

        let loaded = match cm.load_react_checkpoint(&session_id).await {
            Ok(Some(cp)) => cp,
            _ => return self.run(user_input).await,
        };

        // 从 checkpoint 恢复状态
        let start = std::time::Instant::now();
        let mut chain = loaded.chain;
        let mut context = loaded.context;
        let mut state = loaded.current_state;
        let mut retry_count = 0;
        let mut consecutive_failures = 0;

        tracing::info!(
            iteration = loaded.iteration,
            session_id = %session_id,
            "Resumed ReAct engine from checkpoint"
        );

        // 恢复后继续执行主循环（与 run() 相同逻辑）
        self.emit(ThoughtEvent::StateChanged(state));

        if self.config.cycle_detection_enabled && self.cycle_detector.is_none() {
            self.cycle_detector = Some(CycleDetector::new(
                self.config.max_repeated_calls,
                self.config.max_no_progress_iterations,
            ));
        }

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
                        && consecutive_failures >= self.effective_reflection_threshold(&context)
                        && matches!(state, ReasoningState::Thinking)
                    {
                        state = ReasoningState::Reflecting;
                        consecutive_failures = 0;
                        self.emit(ThoughtEvent::StateChanged(state));
                    }

                    if state.is_terminal() {
                        break;
                    }

                    // 断点续执行：每 N 次迭代自动保存
                    if let Some(ref cm) = self.checkpoint_manager
                        && self.checkpoint_interval > 0
                        && context.iteration.is_multiple_of(self.checkpoint_interval)
                        && let Some(ref sid) = self.checkpoint_session_id
                    {
                        let cp = crate::checkpoint::ReActEngineCheckpoint {
                            session_id: sid.clone(),
                            iteration: context.iteration,
                            chain: chain.clone(),
                            context: context.clone(),
                            current_state: state,
                            token_budget_used: estimate_chain_tokens(&chain),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        if let Err(e) = cm.save_react_checkpoint(&cp).await {
                            tracing::warn!(
                                error = %e,
                                iteration = context.iteration,
                                "Failed to save ReAct checkpoint"
                            );
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

                    state = ReasoningState::Thinking;
                },
            }
        }

        let final_response = chain
            .latest_step()
            .and_then(|s| s.result.clone())
            .unwrap_or_else(|| "Task completed.".to_string());

        self.emit(ThoughtEvent::ChainComplete(chain.to_summary()));

        // 成功完成后删除 checkpoint
        if let Some(ref cm) = self.checkpoint_manager
            && let Some(ref sid) = self.checkpoint_session_id
        {
            let _ = cm.delete_react_checkpoint(sid).await;
        }

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

                self.emit(ThoughtEvent::StepCompleted(
                    chain
                        .latest_step()
                        .expect("step just added via add_step")
                        .clone(),
                ));

                Ok((ReasoningState::Thinking, true))
            },

            ReasoningState::Thinking => {
                let reasoning = self
                    .reasoning_provider
                    .think(user_input, context, chain)
                    .await?;
                let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                chain.add_step(step);

                self.emit(ThoughtEvent::StepCompleted(
                    chain
                        .latest_step()
                        .expect("step just added via add_step")
                        .clone(),
                ));

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

                self.emit(ThoughtEvent::StepCompleted(
                    chain
                        .latest_step()
                        .expect("step just added via add_step")
                        .clone(),
                ));

                Ok((ReasoningState::Acting, true))
            },

            ReasoningState::Acting => {
                if let Some(latest) = chain.latest_step_mut()
                    && let Some(ref action) = latest.action
                {
                    if action.requires_confirmation {
                        return Ok((ReasoningState::Observing, false));
                    }

                    let timeout_duration = Duration::from_secs(self.config.timeout_secs);
                    let result = match tokio::time::timeout(
                        timeout_duration,
                        self.executor.execute(action.clone(), ""),
                    )
                    .await
                    {
                        Ok(exec_result) => exec_result,
                        Err(_elapsed) => {
                            return Err(ReActError::ActionError(format!(
                                "操作超时 ({} 秒)",
                                self.config.timeout_secs
                            )));
                        },
                    };

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
                        // 目标达成判定
                        if let Some(ref evaluator) = self.goal_evaluator {
                            let mut guard = evaluator
                                .lock()
                                .expect("goal evaluator mutex should not be poisoned");
                            let evaluation = guard.evaluate(chain, context);
                            if !evaluation.achieved {
                                let reasoning = format!(
                                    "目标未达成 (置信度 {:.0}%): {}。缺失: {}。返回 Thinking 继续处理。",
                                    evaluation.confidence * 100.0,
                                    evaluation.reason,
                                    evaluation.missing.join(", ")
                                );
                                let step = ThoughtStep::new(ReasoningState::Thinking, reasoning);
                                chain.add_step(step);
                                return Ok((ReasoningState::Thinking, false));
                            }
                        }
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

                self.emit(ThoughtEvent::StepCompleted(
                    chain
                        .latest_step()
                        .expect("step just added via add_step")
                        .clone(),
                ));

                self.adjust_strategy(context);

                if let Some(ref planner) = self.planner {
                    let mut planner_guard = planner.lock().await;
                    let pending = planner_guard.get_pending_steps();
                    let completed = planner_guard.get_completed_steps();

                    let reference_ids: Vec<String> = if !pending.is_empty() {
                        pending
                    } else {
                        completed
                    };
                    let actions: Vec<crate::hierarchical_planner::ReplanAction> = reference_ids
                        .iter()
                        .take(3)
                        .enumerate()
                        .map(|(i, tid)| crate::hierarchical_planner::ReplanAction::Reorder {
                            task_id: tid.clone(),
                            new_position: i,
                        })
                        .collect();

                    if !actions.is_empty() {
                        let reason = crate::hierarchical_planner::ReplanReason::StepFailed {
                            task_id: reference_ids[0].clone(),
                            error: "Reflection triggered replan".to_string(),
                        };
                        if let Ok(record) = planner_guard.replan(reason, actions) {
                            tracing::info!(
                                version = record.version,
                                reason = ?record.reason,
                                "Replan triggered during reflection"
                            );
                        }
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

                self.emit(ThoughtEvent::StepCompleted(
                    chain
                        .latest_step()
                        .expect("step just added via add_step")
                        .clone(),
                ));

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

    fn effective_reflection_threshold(&self, context: &ReasoningContext) -> usize {
        if !self.config.adaptive_reflection {
            return self.config.reflection_threshold;
        }
        let base = self.config.reflection_threshold;
        let depth_penalty = context.depth / 2;
        let progress_penalty = context.iteration / 10;
        base.saturating_sub(depth_penalty + progress_penalty).max(1)
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

/// 从 LLM 响应中提取 JSON 内容
///
/// 处理 LLM 可能在 JSON 外包裹 markdown 代码块或额外文本的情况。
fn extract_json_from_response(response: &str) -> String {
    let trimmed = response.trim();

    // 尝试从 markdown 代码块中提取 JSON
    if let Some(json_start) = trimmed.find("```json") {
        let after_open = &trimmed[json_start + "```json".len()..];
        if let Some(json_end) = after_open.find("```") {
            return after_open[..json_end].trim().to_string();
        }
        return after_open.trim().to_string();
    }

    if let Some(json_start) = trimmed.find("```") {
        let after_open = &trimmed[json_start + "```".len()..];
        if let Some(json_end) = after_open.find("```") {
            return after_open[..json_end].trim().to_string();
        }
        return after_open.trim().to_string();
    }

    // 尝试找到 { 开始 } 结束的 JSON 对象
    if let Some(brace_start) = trimmed.find('{')
        && let Some(brace_end) = trimmed.rfind('}')
    {
        let candidate = trimmed[brace_start..=brace_end].to_string();
        if candidate.len() > trimmed.len() / 2 {
            return candidate;
        }
    }

    trimmed.to_string()
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
