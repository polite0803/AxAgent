use crate::reasoning_state::ActionType;
use crate::thought_chain::ThoughtStep;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub confidence: f32,
    pub reason: String,
    pub suggested_corrections: Vec<String>,
}

impl VerificationResult {
    pub fn valid(reason: impl Into<String>) -> Self {
        Self {
            is_valid: true,
            confidence: 1.0,
            reason: reason.into(),
            suggested_corrections: Vec::new(),
        }
    }

    pub fn invalid(reason: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            confidence: 1.0,
            reason: reason.into(),
            suggested_corrections: Vec::new(),
        }
    }

    pub fn uncertain(confidence: f32, reason: impl Into<String>) -> Self {
        Self {
            is_valid: true,
            confidence: confidence.clamp(0.0, 1.0),
            reason: reason.into(),
            suggested_corrections: Vec::new(),
        }
    }
}

pub struct SelfVerifier {
    strict_mode: bool,
}

impl SelfVerifier {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub async fn verify(
        &self,
        step: &ThoughtStep,
        _original_goal: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let _result_str = step.result.as_deref().unwrap_or("");
        let action_type = step.action.as_ref().map(|a| a.action_type);

        let verification = match action_type {
            Some(ActionType::ToolCall) => self.verify_tool_result(step).await?,
            Some(ActionType::LlmCall) => self.verify_llm_result(step).await?,
            _ => VerificationResult::uncertain(0.5, "Unknown action type"),
        };

        if self.strict_mode && verification.confidence < 0.8 {
            return Ok(VerificationResult::invalid(
                "Confidence below threshold in strict mode",
            ));
        }

        Ok(verification)
    }

    async fn verify_tool_result(
        &self,
        step: &ThoughtStep,
    ) -> Result<VerificationResult, VerificationError> {
        let tool_name = step
            .action
            .as_ref()
            .and_then(|a| a.tool_name.as_deref())
            .unwrap_or("unknown");
        let result = step.result.as_deref().unwrap_or("");

        if result.to_lowercase().contains("error")
            || result.to_lowercase().contains("failed")
            || result.to_lowercase().contains("exception")
        {
            return Ok(VerificationResult::invalid(format!(
                "Tool '{}' returned an error: {}",
                tool_name,
                Self::truncate_string(result, 200)
            )));
        }

        if result.is_empty() && !Self::is_empty_ok_tool(tool_name) {
            return Ok(VerificationResult::invalid(format!(
                "Tool '{}' returned empty result",
                tool_name
            )));
        }

        Ok(VerificationResult::valid(format!(
            "Tool '{}' executed successfully",
            tool_name
        )))
    }

    async fn verify_llm_result(
        &self,
        step: &ThoughtStep,
    ) -> Result<VerificationResult, VerificationError> {
        let response = step.result.as_deref().unwrap_or("");

        if response.is_empty() {
            return Ok(VerificationResult::invalid(
                "LLM returned empty response".to_string(),
            ));
        }

        if response.len() < 10 {
            return Ok(VerificationResult::uncertain(
                0.6,
                "LLM response is unusually short",
            ));
        }

        Ok(VerificationResult::valid("LLM response received"))
    }

    fn is_empty_ok_tool(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "delete_file"
                | "move_file"
                | "create_directory"
                | "mouse_click"
                | "type_text"
                | "key_press"
                | "scroll"
        )
    }

    fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}

impl Default for SelfVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Verification failed: {0}")]
    Failed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("LLM error: {0}")]
    LlmError(String),
}
