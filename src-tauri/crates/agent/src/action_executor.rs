use crate::reasoning_state::ActionType;
use crate::thought_chain::{Action, ThoughtStep};
use chrono::Utc;
use serde_json::Value;
use std::time::{Duration, Instant};

pub struct ActionExecutor {
    _private: (),
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub async fn execute(
        &self,
        action: Action,
        _conversation_id: &str,
    ) -> Result<ActionResult, ActionError> {
        let start = Instant::now();
        match action.action_type {
            ActionType::ToolCall => {
                let tool_name = action.tool_name.as_ref().ok_or(ActionError::InvalidAction(
                    "ToolCall action missing tool_name".to_string(),
                ))?;
                let input = action.tool_input.clone().unwrap_or(serde_json::json!({}));
                self.execute_tool(tool_name, input).await
            },
            ActionType::LlmCall => {
                let prompt = action
                    .llm_prompt
                    .as_ref()
                    .ok_or(ActionError::InvalidAction(
                        "LlmCall action missing prompt".to_string(),
                    ))?;
                Ok(ActionResult::LlmResponse(prompt.to_string()))
            },
            ActionType::UserConfirm => {
                let message = action.llm_prompt.clone().unwrap_or_default();
                Ok(ActionResult::UserConfirmationRequired(message))
            },
            ActionType::Validate => {
                let description = action.llm_prompt.clone().unwrap_or_default();
                Ok(ActionResult::Validation(description))
            },
            ActionType::Analyze => {
                Ok(ActionResult::Analysis(action.llm_prompt.clone().unwrap_or_default()))
            },
            ActionType::Plan => {
                Ok(ActionResult::Planning(action.llm_prompt.clone().unwrap_or_default()))
            },
            ActionType::Reflect => {
                Ok(ActionResult::Reflection(action.llm_prompt.clone().unwrap_or_default()))
            },
            ActionType::Synthesize => {
                Ok(ActionResult::Synthesis(action.llm_prompt.clone().unwrap_or_default()))
            },
        }
        .map(|result| result.with_duration(start.elapsed()))
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        input: Value,
    ) -> Result<ActionResult, ActionError> {
        let (server_name, local_name) = parse_full_tool_name(tool_name);

        let args = if let Some(obj) = input.as_object() {
            serde_json::to_value(obj.clone()).unwrap_or(input.clone())
        } else {
            serde_json::json!({ "input": input })
        };

        match axagent_tools::builtin_handlers::dispatch(server_name, local_name, args).await {
            Ok(mcp_result) => {
                Ok(ActionResult::ToolSuccess(mcp_result.content, tool_name.to_string()))
            },
            Err(e) => Err(ActionError::ToolExecution(e.to_string())),
        }
    }
}

fn parse_full_tool_name(full_name: &str) -> (&str, &str) {
    if let Some(idx) = full_name.find('/') {
        let server = &full_name[..idx];
        let tool = &full_name[idx + 1..];
        (server, tool)
    } else {
        ("", full_name)
    }
}

#[derive(Debug, Clone)]
pub enum ActionResult {
    ToolSuccess(String, String),
    LlmResponse(String),
    UserConfirmationRequired(String),
    Validation(String),
    Analysis(String),
    Planning(String),
    Reflection(String),
    Synthesis(String),
}

impl ActionResult {
    pub fn with_duration(self, _duration: Duration) -> Self {
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self,
            ActionResult::ToolSuccess(_, _)
                | ActionResult::LlmResponse(_)
                | ActionResult::Analysis(_)
                | ActionResult::Planning(_)
                | ActionResult::Reflection(_)
                | ActionResult::Synthesis(_)
        )
    }

    pub fn to_observation(&self) -> String {
        match self {
            ActionResult::ToolSuccess(output, tool) => {
                format!("Tool '{}' returned: {}", tool, truncate_string(output, 500))
            },
            ActionResult::LlmResponse(text) => {
                format!("LLM response: {}", truncate_string(text, 500))
            },
            ActionResult::UserConfirmationRequired(msg) => {
                format!("Awaiting user confirmation: {}", msg)
            },
            ActionResult::Validation(desc) => {
                format!("Validation: {}", desc)
            },
            ActionResult::Analysis(desc) => {
                format!("Analysis: {}", desc)
            },
            ActionResult::Planning(desc) => {
                format!("Planning: {}", desc)
            },
            ActionResult::Reflection(desc) => {
                format!("Reflection: {}", desc)
            },
            ActionResult::Synthesis(desc) => {
                format!("Synthesis: {}", desc)
            },
        }
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
pub enum ActionError {
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("LLM call failed: {0}")]
    LlmError(String),

    #[error("Invalid action: {0}")]
    InvalidAction(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl ActionError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ActionError::Timeout(_) | ActionError::LlmError(_) | ActionError::ToolExecution(_)
        )
    }
}

#[derive(Clone)]
pub struct ThoughtStepBuilder {
    state: crate::reasoning_state::ReasoningState,
    reasoning: String,
    action: Option<Action>,
}

impl ThoughtStepBuilder {
    pub fn new(
        state: crate::reasoning_state::ReasoningState,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            state,
            reasoning: reasoning.into(),
            action: None,
        }
    }

    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    pub fn build(self, step_id: usize) -> ThoughtStep {
        ThoughtStep {
            id: step_id,
            state: self.state,
            reasoning: self.reasoning,
            action: self.action,
            observation: None,
            result: None,
            is_verified: false,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_state::ActionType;
    use crate::thought_chain::Action;

    #[test]
    fn test_action_result_is_success_tool_success() {
        let result = ActionResult::ToolSuccess("output".to_string(), "tool".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_success_llm_response() {
        let result = ActionResult::LlmResponse("response".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_success_analysis() {
        let result = ActionResult::Analysis("analysis".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_success_planning() {
        let result = ActionResult::Planning("plan".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_success_reflection() {
        let result = ActionResult::Reflection("reflection".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_success_synthesis() {
        let result = ActionResult::Synthesis("synthesis".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_action_result_is_not_success_validation() {
        let result = ActionResult::Validation("desc".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_action_result_is_not_success_user_confirmation() {
        let result = ActionResult::UserConfirmationRequired("msg".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_action_result_to_observation_tool_success() {
        let result = ActionResult::ToolSuccess("output data".to_string(), "my_tool".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("my_tool"));
        assert!(obs.contains("output data"));
    }

    #[test]
    fn test_action_result_to_observation_llm_response() {
        let result = ActionResult::LlmResponse("llm text".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("LLM response"));
        assert!(obs.contains("llm text"));
    }

    #[test]
    fn test_action_result_to_observation_user_confirmation() {
        let result = ActionResult::UserConfirmationRequired("confirm?".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Awaiting user confirmation"));
        assert!(obs.contains("confirm?"));
    }

    #[test]
    fn test_action_result_to_observation_validation() {
        let result = ActionResult::Validation("valid".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Validation"));
        assert!(obs.contains("valid"));
    }

    #[test]
    fn test_action_result_to_observation_analysis() {
        let result = ActionResult::Analysis("analyzed".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Analysis"));
    }

    #[test]
    fn test_action_result_to_observation_planning() {
        let result = ActionResult::Planning("planned".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Planning"));
    }

    #[test]
    fn test_action_result_to_observation_reflection() {
        let result = ActionResult::Reflection("reflected".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Reflection"));
    }

    #[test]
    fn test_action_result_to_observation_synthesis() {
        let result = ActionResult::Synthesis("synthesized".to_string());
        let obs = result.to_observation();
        assert!(obs.contains("Synthesis"));
    }

    #[test]
    fn test_action_result_to_observation_truncates_long_output() {
        let long_output = "a".repeat(600);
        let result = ActionResult::ToolSuccess(long_output.clone(), "tool".to_string());
        let obs = result.to_observation();
        assert!(obs.len() < long_output.len() + 50);
    }

    #[test]
    fn test_action_result_with_duration() {
        let result = ActionResult::LlmResponse("text".to_string());
        let result = result.with_duration(Duration::from_millis(100));
        assert!(matches!(result, ActionResult::LlmResponse(_)));
    }

    #[test]
    fn test_action_error_is_retryable_timeout() {
        let err = ActionError::Timeout("timed out".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_action_error_is_retryable_llm_error() {
        let err = ActionError::LlmError("llm failed".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_action_error_is_retryable_tool_execution() {
        let err = ActionError::ToolExecution("tool failed".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_action_error_not_retryable_invalid_action() {
        let err = ActionError::InvalidAction("bad action".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_action_error_not_retryable_permission_denied() {
        let err = ActionError::PermissionDenied("no access".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_action_error_display() {
        let err = ActionError::ToolExecution("tool err".to_string());
        assert!(err.to_string().contains("tool err"));
        let err = ActionError::LlmError("llm err".to_string());
        assert!(err.to_string().contains("llm err"));
        let err = ActionError::InvalidAction("invalid".to_string());
        assert!(err.to_string().contains("invalid"));
        let err = ActionError::Timeout("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
        let err = ActionError::PermissionDenied("denied".to_string());
        assert!(err.to_string().contains("denied"));
    }

    #[test]
    fn test_parse_full_tool_name_with_slash() {
        let (server, tool) = parse_full_tool_name("myserver/mytool");
        assert_eq!(server, "myserver");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_full_tool_name_without_slash() {
        let (server, tool) = parse_full_tool_name("mytool");
        assert_eq!(server, "");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_full_tool_name_multiple_slashes() {
        let (server, tool) = parse_full_tool_name("server/path/tool");
        assert_eq!(server, "server");
        assert_eq!(tool, "path/tool");
    }

    #[test]
    fn test_truncate_string_short() {
        let result = truncate_string("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        let long_str = "a".repeat(600);
        let result = truncate_string(&long_str, 500);
        assert!(result.len() <= 500);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_string_exact_length() {
        let s = "a".repeat(500);
        let result = truncate_string(&s, 500);
        assert_eq!(result.len(), 500);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_truncate_string_empty() {
        let result = truncate_string("", 10);
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_action_executor_tool_call_missing_name() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::ToolCall,
            tool_name: None,
            tool_input: None,
            llm_prompt: None,
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ActionError::InvalidAction(_)));
    }

    #[tokio::test]
    async fn test_action_executor_llm_call() {
        let executor = ActionExecutor::new();
        let action = Action::llm_call("test prompt");
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::LlmResponse(text) => assert_eq!(text, "test prompt"),
            _ => panic!("Expected LlmResponse"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_llm_call_missing_prompt() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::LlmCall,
            tool_name: None,
            tool_input: None,
            llm_prompt: None,
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ActionError::InvalidAction(_)));
    }

    #[tokio::test]
    async fn test_action_executor_user_confirm() {
        let executor = ActionExecutor::new();
        let action = Action::user_confirm("confirm this?");
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::UserConfirmationRequired(msg) => assert_eq!(msg, "confirm this?"),
            _ => panic!("Expected UserConfirmationRequired"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_user_confirm_no_message() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::UserConfirm,
            tool_name: None,
            tool_input: None,
            llm_prompt: None,
            requires_confirmation: true,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::UserConfirmationRequired(msg) => assert_eq!(msg, ""),
            _ => panic!("Expected UserConfirmationRequired"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_validate() {
        let executor = ActionExecutor::new();
        let action = Action::validate("check this");
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Validation(desc) => assert_eq!(desc, "check this"),
            _ => panic!("Expected Validation"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_analyze() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::Analyze,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("analyze this".to_string()),
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Analysis(desc) => assert_eq!(desc, "analyze this"),
            _ => panic!("Expected Analysis"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_plan() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::Plan,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("plan this".to_string()),
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Planning(desc) => assert_eq!(desc, "plan this"),
            _ => panic!("Expected Planning"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_reflect() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::Reflect,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("reflect on this".to_string()),
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Reflection(desc) => assert_eq!(desc, "reflect on this"),
            _ => panic!("Expected Reflection"),
        }
    }

    #[tokio::test]
    async fn test_action_executor_synthesize() {
        let executor = ActionExecutor::new();
        let action = Action {
            action_type: ActionType::Synthesize,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("synthesize this".to_string()),
            requires_confirmation: false,
        };
        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Synthesis(desc) => assert_eq!(desc, "synthesize this"),
            _ => panic!("Expected Synthesis"),
        }
    }

    #[test]
    fn test_action_executor_default() {
        let executor = ActionExecutor::default();
        let _ = executor;
    }

    #[test]
    fn test_thought_step_builder_basic() {
        let builder = ThoughtStepBuilder::new(
            crate::reasoning_state::ReasoningState::Thinking,
            "test reasoning",
        );
        let step = builder.build(1);
        assert_eq!(step.id, 1);
        assert_eq!(step.state, crate::reasoning_state::ReasoningState::Thinking);
        assert_eq!(step.reasoning, "test reasoning");
        assert!(step.action.is_none());
        assert!(step.observation.is_none());
        assert!(step.result.is_none());
        assert!(!step.is_verified);
    }

    #[test]
    fn test_thought_step_builder_with_action() {
        let action = Action::llm_call("test prompt");
        let builder = ThoughtStepBuilder::new(
            crate::reasoning_state::ReasoningState::Acting,
            "acting on something",
        )
        .with_action(action);
        let step = builder.build(2);
        assert_eq!(step.id, 2);
        assert!(step.action.is_some());
    }

    #[test]
    fn test_thought_step_builder_different_states() {
        let states = vec![
            crate::reasoning_state::ReasoningState::Idle,
            crate::reasoning_state::ReasoningState::Analyzing,
            crate::reasoning_state::ReasoningState::Thinking,
            crate::reasoning_state::ReasoningState::Planning,
            crate::reasoning_state::ReasoningState::Acting,
            crate::reasoning_state::ReasoningState::Observing,
            crate::reasoning_state::ReasoningState::Reflecting,
            crate::reasoning_state::ReasoningState::Synthesizing,
            crate::reasoning_state::ReasoningState::Finished,
            crate::reasoning_state::ReasoningState::Failed,
        ];
        for (idx, state) in states.into_iter().enumerate() {
            let builder = ThoughtStepBuilder::new(state, "reasoning");
            let step = builder.build(idx);
            assert_eq!(step.state, state);
        }
    }
}
