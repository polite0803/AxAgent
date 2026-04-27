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
            }
            ActionType::LlmCall => {
                let prompt = action
                    .llm_prompt
                    .as_ref()
                    .ok_or(ActionError::InvalidAction(
                        "LlmCall action missing prompt".to_string(),
                    ))?;
                Ok(ActionResult::LlmResponse(prompt.to_string()))
            }
            ActionType::UserConfirm => {
                let message = action.llm_prompt.clone().unwrap_or_default();
                Ok(ActionResult::UserConfirmationRequired(message))
            }
            ActionType::Validate => {
                let description = action.llm_prompt.clone().unwrap_or_default();
                Ok(ActionResult::Validation(description))
            }
            ActionType::Analyze => {
                Ok(ActionResult::Analysis(action.llm_prompt.clone().unwrap_or_default()))
            }
            ActionType::Plan => {
                Ok(ActionResult::Planning(action.llm_prompt.clone().unwrap_or_default()))
            }
            ActionType::Reflect => {
                Ok(ActionResult::Reflection(action.llm_prompt.clone().unwrap_or_default()))
            }
            ActionType::Synthesize => {
                Ok(ActionResult::Synthesis(action.llm_prompt.clone().unwrap_or_default()))
            }
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

        match axagent_core::builtin_tools::dispatch(server_name, local_name, args).await {
            Ok(mcp_result) => Ok(ActionResult::ToolSuccess(
                mcp_result.content,
                tool_name.to_string(),
            )),
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
            }
            ActionResult::LlmResponse(text) => {
                format!("LLM response: {}", truncate_string(text, 500))
            }
            ActionResult::UserConfirmationRequired(msg) => {
                format!("Awaiting user confirmation: {}", msg)
            }
            ActionResult::Validation(desc) => {
                format!("Validation: {}", desc)
            }
            ActionResult::Analysis(desc) => {
                format!("Analysis: {}", desc)
            }
            ActionResult::Planning(desc) => {
                format!("Planning: {}", desc)
            }
            ActionResult::Reflection(desc) => {
                format!("Reflection: {}", desc)
            }
            ActionResult::Synthesis(desc) => {
                format!("Synthesis: {}", desc)
            }
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
