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
    Synthesizing,
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
            ReasoningState::Synthesizing => "synthesizing",
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
    /// 是否启用 token 预算跟踪（检测收益递减并防止上下文窗口耗尽）
    pub token_budget_enabled: bool,
    /// Token 预算上限（None = 使用模型上下文窗口大小）
    pub token_budget_limit: Option<u64>,
    /// 是否启用循环检测
    pub cycle_detection_enabled: bool,
    /// 同工具+同参数最大允许重复调用次数
    pub max_repeated_calls: usize,
    /// 最大允许的无进展迭代次数
    pub max_no_progress_iterations: usize,
    /// 是否启用断点续执行
    pub checkpoint_enabled: bool,
    /// 每 N 次迭代自动保存一次 checkpoint
    pub checkpoint_interval: usize,
    /// 当前智能体角色（影响可调用工具范围）
    pub agent_role: String,
    /// 是否启用目标达成判定
    pub goal_evaluation_enabled: bool,
    /// 是否启用动态自适应反思阈值
    pub adaptive_reflection: bool,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_depth: 10,
            verification_enabled: true,
            max_retry_attempts: 3,
            timeout_secs: 300,
            reflection_threshold: 2,
            enable_analyzing: true,
            enable_reflection: true,
            token_budget_enabled: true,
            token_budget_limit: Some(180_000),
            cycle_detection_enabled: true,
            max_repeated_calls: 3,
            max_no_progress_iterations: 5,
            checkpoint_enabled: false,
            checkpoint_interval: 10,
            agent_role: "executor".to_string(),
            goal_evaluation_enabled: false,
            adaptive_reflection: true,
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
            token_budget_enabled: false,
            token_budget_limit: None,
            cycle_detection_enabled: true,
            max_repeated_calls: 3,
            max_no_progress_iterations: 5,
            checkpoint_enabled: false,
            checkpoint_interval: 10,
            agent_role: "executor".to_string(),
            goal_evaluation_enabled: false,
            adaptive_reflection: false,
        }
    }

    pub fn for_complex_task() -> Self {
        Self {
            max_iterations: 100,
            max_depth: 20,
            verification_enabled: true,
            max_retry_attempts: 5,
            timeout_secs: 600,
            reflection_threshold: 5,
            enable_analyzing: true,
            enable_reflection: true,
            token_budget_enabled: true,
            token_budget_limit: Some(200_000),
            cycle_detection_enabled: true,
            max_repeated_calls: 5,
            max_no_progress_iterations: 8,
            checkpoint_enabled: true,
            checkpoint_interval: 5,
            agent_role: "executor".to_string(),
            goal_evaluation_enabled: true,
            adaptive_reflection: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningContext {
    pub original_input: String,
    pub current_goal: Option<String>,
    pub sub_goals: Vec<String>,
    pub constraints: Vec<String>,
    pub resources: Vec<String>,
    pub iteration: usize,
    pub depth: usize,
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

/// 推理策略：根据智能体角色定制 ReAct 循环行为
#[derive(Debug, Clone)]
pub struct ReasoningStrategy {
    pub entry_state: ReasoningState,
    pub enable_reflection: bool,
    pub reflection_threshold: usize,
    pub temperature: f32,
    pub allow_tool_calls: bool,
    pub max_iterations: usize,
    pub timeout_secs: u64,
    pub enable_analyzing: bool,
}

impl ReasoningStrategy {
    /// 根据角色返回对应的推理策略
    pub fn for_role(role: &str) -> Self {
        match role {
            "executor" => Self {
                entry_state: ReasoningState::Analyzing,
                enable_reflection: true,
                reflection_threshold: 2,
                temperature: 0.3,
                allow_tool_calls: true,
                max_iterations: 50,
                timeout_secs: 300,
                enable_analyzing: true,
            },
            "planner" => Self {
                entry_state: ReasoningState::Thinking,
                enable_reflection: true,
                reflection_threshold: 2,
                temperature: 0.4,
                allow_tool_calls: false,
                max_iterations: 30,
                timeout_secs: 120,
                enable_analyzing: true,
            },
            "researcher" => Self {
                entry_state: ReasoningState::Analyzing,
                enable_reflection: true,
                reflection_threshold: 3,
                temperature: 0.3,
                allow_tool_calls: true,
                max_iterations: 40,
                timeout_secs: 180,
                enable_analyzing: true,
            },
            "code_reviewer" => Self {
                entry_state: ReasoningState::Thinking,
                enable_reflection: true,
                reflection_threshold: 2,
                temperature: 0.2,
                allow_tool_calls: true,
                max_iterations: 30,
                timeout_secs: 120,
                enable_analyzing: false,
            },
            "safety_guard" => Self {
                entry_state: ReasoningState::Thinking,
                enable_reflection: false,
                reflection_threshold: 1,
                temperature: 0.1,
                allow_tool_calls: false,
                max_iterations: 20,
                timeout_secs: 60,
                enable_analyzing: false,
            },
            _ => Self::default(),
        }
    }
}

impl Default for ReasoningStrategy {
    fn default() -> Self {
        Self::for_role("executor")
    }
}

impl From<ReasoningStrategy> for ReActConfig {
    fn from(strategy: ReasoningStrategy) -> Self {
        ReActConfig {
            max_iterations: strategy.max_iterations,
            max_depth: 10,
            verification_enabled: true,
            max_retry_attempts: 3,
            timeout_secs: strategy.timeout_secs,
            reflection_threshold: strategy.reflection_threshold,
            enable_analyzing: strategy.enable_analyzing,
            enable_reflection: strategy.enable_reflection,
            token_budget_enabled: strategy.max_iterations > 30,
            token_budget_limit: Some(180_000),
            cycle_detection_enabled: true,
            max_repeated_calls: 3,
            max_no_progress_iterations: 5,
            checkpoint_enabled: false,
            checkpoint_interval: 10,
            agent_role: "executor".to_string(),
            goal_evaluation_enabled: false,
            adaptive_reflection: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_state_as_str() {
        assert_eq!(ReasoningState::Idle.as_str(), "idle");
        assert_eq!(ReasoningState::Analyzing.as_str(), "analyzing");
        assert_eq!(ReasoningState::Thinking.as_str(), "thinking");
        assert_eq!(ReasoningState::Planning.as_str(), "planning");
        assert_eq!(ReasoningState::Acting.as_str(), "acting");
        assert_eq!(ReasoningState::Observing.as_str(), "observing");
        assert_eq!(ReasoningState::Reflecting.as_str(), "reflecting");
        assert_eq!(ReasoningState::Synthesizing.as_str(), "synthesizing");
        assert_eq!(ReasoningState::Finished.as_str(), "finished");
        assert_eq!(ReasoningState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_reasoning_state_display() {
        assert_eq!(ReasoningState::Idle.to_string(), "idle");
        assert_eq!(ReasoningState::Thinking.to_string(), "thinking");
        assert_eq!(ReasoningState::Acting.to_string(), "acting");
        assert_eq!(ReasoningState::Finished.to_string(), "finished");
        assert_eq!(ReasoningState::Failed.to_string(), "failed");
    }

    #[test]
    fn test_reasoning_state_is_terminal() {
        assert!(ReasoningState::Finished.is_terminal());
        assert!(ReasoningState::Failed.is_terminal());
        assert!(!ReasoningState::Idle.is_terminal());
        assert!(!ReasoningState::Thinking.is_terminal());
        assert!(!ReasoningState::Acting.is_terminal());
        assert!(!ReasoningState::Observing.is_terminal());
        assert!(!ReasoningState::Reflecting.is_terminal());
        assert!(!ReasoningState::Planning.is_terminal());
        assert!(!ReasoningState::Analyzing.is_terminal());
        assert!(!ReasoningState::Synthesizing.is_terminal());
    }

    #[test]
    fn test_reasoning_state_requires_observation() {
        assert!(ReasoningState::Acting.requires_observation());
        assert!(!ReasoningState::Thinking.requires_observation());
        assert!(!ReasoningState::Observing.requires_observation());
        assert!(!ReasoningState::Idle.requires_observation());
    }

    #[test]
    fn test_reasoning_state_can_retry() {
        assert!(ReasoningState::Observing.can_retry());
        assert!(ReasoningState::Reflecting.can_retry());
        assert!(!ReasoningState::Idle.can_retry());
        assert!(!ReasoningState::Thinking.can_retry());
        assert!(!ReasoningState::Acting.can_retry());
        assert!(!ReasoningState::Failed.can_retry());
        assert!(!ReasoningState::Finished.can_retry());
    }

    #[test]
    fn test_reasoning_state_equality() {
        assert_eq!(ReasoningState::Thinking, ReasoningState::Thinking);
        assert_ne!(ReasoningState::Thinking, ReasoningState::Acting);
    }

    #[test]
    fn test_reasoning_state_serialization() {
        let state = ReasoningState::Acting;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ReasoningState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ReasoningState::Acting);
    }

    #[test]
    fn test_action_type_display() {
        assert_eq!(ActionType::ToolCall.to_string(), "tool_call");
        assert_eq!(ActionType::LlmCall.to_string(), "llm_call");
        assert_eq!(ActionType::UserConfirm.to_string(), "user_confirm");
        assert_eq!(ActionType::Validate.to_string(), "validate");
        assert_eq!(ActionType::Analyze.to_string(), "analyze");
        assert_eq!(ActionType::Plan.to_string(), "plan");
        assert_eq!(ActionType::Reflect.to_string(), "reflect");
        assert_eq!(ActionType::Synthesize.to_string(), "synthesize");
    }

    #[test]
    fn test_action_type_equality() {
        assert_eq!(ActionType::ToolCall, ActionType::ToolCall);
        assert_ne!(ActionType::ToolCall, ActionType::LlmCall);
    }

    #[test]
    fn test_action_type_serialization() {
        let action_type = ActionType::Reflect;
        let json = serde_json::to_string(&action_type).unwrap();
        let deserialized: ActionType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ActionType::Reflect);
    }

    #[test]
    fn test_react_config_default() {
        let config = ReActConfig::default();
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_depth, 10);
        assert!(config.verification_enabled);
        assert_eq!(config.max_retry_attempts, 3);
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.reflection_threshold, 2);
        assert!(config.enable_analyzing);
        assert!(config.enable_reflection);
        assert!(config.token_budget_enabled);
        assert_eq!(config.token_budget_limit, Some(180_000));
    }

    #[test]
    fn test_react_config_for_simple_task() {
        let config = ReActConfig::for_simple_task();
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.max_depth, 5);
        assert!(config.verification_enabled);
        assert_eq!(config.max_retry_attempts, 2);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.reflection_threshold, 3);
        assert!(!config.enable_analyzing);
        assert!(!config.enable_reflection);
        assert!(!config.token_budget_enabled);
        assert!(config.token_budget_limit.is_none());
    }

    #[test]
    fn test_react_config_for_complex_task() {
        let config = ReActConfig::for_complex_task();
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.max_depth, 20);
        assert!(config.verification_enabled);
        assert_eq!(config.max_retry_attempts, 5);
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.reflection_threshold, 5);
        assert!(config.enable_analyzing);
        assert!(config.enable_reflection);
        assert!(config.token_budget_enabled);
        assert_eq!(config.token_budget_limit, Some(200_000));
    }

    #[test]
    fn test_react_config_serialization() {
        let config = ReActConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ReActConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_iterations, 50);
        assert_eq!(deserialized.max_depth, 10);
    }

    #[test]
    fn test_reasoning_context_new() {
        let ctx = ReasoningContext::new("test input");
        assert_eq!(ctx.original_input, "test input");
        assert!(ctx.current_goal.is_none());
        assert!(ctx.sub_goals.is_empty());
        assert!(ctx.constraints.is_empty());
        assert!(ctx.resources.is_empty());
        assert_eq!(ctx.iteration, 0);
        assert_eq!(ctx.depth, 0);
    }

    #[test]
    fn test_reasoning_context_default() {
        let ctx = ReasoningContext::default();
        assert!(ctx.original_input.is_empty());
        assert!(ctx.current_goal.is_none());
        assert!(ctx.sub_goals.is_empty());
        assert_eq!(ctx.iteration, 0);
        assert_eq!(ctx.depth, 0);
    }

    #[test]
    fn test_reasoning_context_set_goal() {
        let mut ctx = ReasoningContext::new("input");
        assert!(ctx.current_goal.is_none());
        ctx.set_goal("solve problem".to_string());
        assert_eq!(ctx.current_goal.unwrap(), "solve problem");
    }

    #[test]
    fn test_reasoning_context_add_sub_goals() {
        let mut ctx = ReasoningContext::new("input");
        ctx.add_sub_goal("goal 1".to_string());
        ctx.add_sub_goal("goal 2".to_string());
        ctx.add_sub_goal("goal 3".to_string());
        assert_eq!(ctx.sub_goals.len(), 3);
        assert_eq!(ctx.sub_goals[0], "goal 1");
        assert_eq!(ctx.sub_goals[2], "goal 3");
    }

    #[test]
    fn test_reasoning_context_increment_iteration() {
        let mut ctx = ReasoningContext::new("input");
        assert_eq!(ctx.iteration, 0);
        ctx.increment_iteration();
        assert_eq!(ctx.iteration, 1);
        ctx.increment_iteration();
        ctx.increment_iteration();
        assert_eq!(ctx.iteration, 3);
    }

    #[test]
    fn test_reasoning_context_increment_depth() {
        let mut ctx = ReasoningContext::new("input");
        assert_eq!(ctx.depth, 0);
        ctx.increment_depth();
        assert_eq!(ctx.depth, 1);
        ctx.increment_depth();
        assert_eq!(ctx.depth, 2);
    }

    #[test]
    fn test_reasoning_context_serialization() {
        let mut ctx = ReasoningContext::new("test input");
        ctx.set_goal("goal".to_string());
        ctx.add_sub_goal("sub1".to_string());
        ctx.increment_iteration();
        ctx.increment_depth();

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: ReasoningContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.original_input, "test input");
        assert_eq!(deserialized.current_goal.unwrap(), "goal");
        assert_eq!(deserialized.sub_goals.len(), 1);
        assert_eq!(deserialized.iteration, 1);
        assert_eq!(deserialized.depth, 1);
    }

    #[test]
    fn test_all_reasoning_state_variants() {
        let states = vec![
            ReasoningState::Idle,
            ReasoningState::Analyzing,
            ReasoningState::Thinking,
            ReasoningState::Planning,
            ReasoningState::Acting,
            ReasoningState::Observing,
            ReasoningState::Reflecting,
            ReasoningState::Synthesizing,
            ReasoningState::Finished,
            ReasoningState::Failed,
        ];
        for state in &states {
            assert!(!state.as_str().is_empty());
        }
        assert_eq!(states.len(), 10);
    }
}
