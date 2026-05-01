use crate::reasoning_state::ActionType;
use crate::thought_chain::ThoughtStep;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

    pub fn with_correction(mut self, correction: impl Into<String>) -> Self {
        self.suggested_corrections.push(correction.into());
        self
    }
}

#[async_trait]
pub trait SemanticValidator: Send + Sync {
    async fn validate_semantically(
        &self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> Result<VerificationResult, VerificationError>;
}

pub struct SelfVerifier {
    strict_mode: bool,
    semantic_validator: Option<Arc<dyn SemanticValidator>>,
}

impl SelfVerifier {
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            semantic_validator: None,
        }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub fn with_semantic_validator(mut self, validator: Arc<dyn SemanticValidator>) -> Self {
        self.semantic_validator = Some(validator);
        self
    }

    pub async fn verify(
        &self,
        step: &ThoughtStep,
        _original_goal: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let _result_str = step.result.as_deref().unwrap_or("");
        let action_type = step.action.as_ref().map(|a| a.action_type);
        let tool_name = step.action.as_ref().and_then(|a| a.tool_name.as_deref());

        let verification = match action_type {
            Some(ActionType::ToolCall) => {
                if let Some(name) = tool_name {
                    self.verify_specific_tool(name, step).await?
                } else {
                    self.verify_tool_result(step).await?
                }
            },
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

    async fn verify_specific_tool(
        &self,
        tool_name: &str,
        step: &ThoughtStep,
    ) -> Result<VerificationResult, VerificationError> {
        let result = step.result.as_deref().unwrap_or("");
        let input = step
            .action
            .as_ref()
            .and_then(|a| a.tool_input.as_ref())
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let base_verification = self.verify_tool_result(step).await?;
        if !base_verification.is_valid {
            return Ok(base_verification);
        }

        let specific_check = match tool_name {
            "read_file" | "read_multiple_files" => self.verify_file_read(result, input).await,
            "write_file" | "create_file" => self.verify_file_write(result, input).await,
            "glob_search" | "file_search" => self.verify_search_result(result, input).await,
            "execute_command" | "bash" | "shell" => self.verify_command_result(result, input).await,
            "web_search" | "search" => self.verify_web_search(result, input).await,
            "edit_file" | "apply_diff" => self.verify_file_edit(result, input).await,
            _ => Ok(VerificationResult::valid(
                "No specific validation available",
            )),
        }?;

        if let Some(ref validator) = self.semantic_validator {
            let semantic_result = validator
                .validate_semantically(tool_name, input, result)
                .await?;
            Ok(Self::combine_results(
                base_verification,
                specific_check,
                semantic_result,
            ))
        } else {
            Ok(specific_check)
        }
    }

    async fn verify_file_read(
        &self,
        result: &str,
        _input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        if result.contains("No such file or directory")
            || result.contains("Path does not exist")
            || result.contains("permission denied")
        {
            return Ok(VerificationResult::invalid(
                "File read failed - file not found or no permission",
            )
            .with_correction("Verify the file path is correct"));
        }

        if _input.contains("line_numbers") && !result.contains('\n') && !result.is_empty() {
            return Ok(VerificationResult::uncertain(
                0.7,
                "Expected multiple lines but got single line result",
            ));
        }

        Ok(VerificationResult::valid("File read verification passed"))
    }

    async fn verify_file_write(
        &self,
        result: &str,
        _input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        if result.to_lowercase().contains("error")
            || result.to_lowercase().contains("failed")
            || result.to_lowercase().contains("permission denied")
        {
            return Ok(VerificationResult::invalid("File write operation failed")
                .with_correction("Check disk space and file permissions"));
        }

        if result.contains("File written successfully")
            || result.contains("created successfully")
            || result.is_empty()
        {
            return Ok(VerificationResult::valid("File write verification passed"));
        }

        Ok(VerificationResult::uncertain(
            0.8,
            "File write completed with unexpected output",
        ))
    }

    async fn verify_search_result(
        &self,
        result: &str,
        input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        if result.is_empty() {
            return Ok(VerificationResult::uncertain(
                0.5,
                "Search returned no results - this may be expected",
            ));
        }

        let pattern = input
            .split("pattern")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().trim_matches('"').trim_matches('\''))
            .unwrap_or("");

        if !pattern.is_empty() && !result.contains(pattern) && !result.is_empty() {
            return Ok(VerificationResult::uncertain(
                0.6,
                format!("Search pattern '{}' not found in results", pattern),
            ));
        }

        Ok(VerificationResult::valid("Search verification passed"))
    }

    async fn verify_command_result(
        &self,
        result: &str,
        input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let cmd_lower = input.to_lowercase();

        if cmd_lower.contains("rm ")
            || cmd_lower.contains("delete ")
            || cmd_lower.contains("remove ")
        {
            if !result.contains("removed")
                && !result.contains("deleted")
                && !result.contains("cannot find")
                && !result.is_empty()
            {
                return Ok(VerificationResult::uncertain(
                    0.7,
                    "Deletion command completed but output is unclear",
                ));
            }
        }

        if result.contains("Segmentation fault")
            || result.contains("core dumped")
            || result.contains("panic")
        {
            return Ok(VerificationResult::invalid("Command caused a crash")
                .with_correction("Check command syntax and arguments"));
        }

        Ok(VerificationResult::valid("Command verification passed"))
    }

    async fn verify_web_search(
        &self,
        result: &str,
        input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        if result.is_empty() {
            return Ok(
                VerificationResult::invalid("Web search returned no results")
                    .with_correction("Try different search terms"),
            );
        }

        let query = input
            .split("query")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().trim_matches('"').trim_matches('\''))
            .unwrap_or("");

        if !query.is_empty() {
            let query_words: Vec<_> = query.split_whitespace().collect();
            let result_lower = result.to_lowercase();
            let matches: usize = query_words
                .iter()
                .filter(|w| result_lower.contains(&w.to_lowercase()))
                .count();

            let match_ratio = matches as f32 / query_words.len() as f32;
            if match_ratio < 0.3 && !result.is_empty() {
                return Ok(VerificationResult::uncertain(
                    0.6,
                    format!(
                        "Search results may not be relevant to query ({}% word match)",
                        (match_ratio * 100.0) as i32
                    ),
                ));
            }
        }

        Ok(VerificationResult::valid("Web search verification passed"))
    }

    async fn verify_file_edit(
        &self,
        result: &str,
        _input: &str,
    ) -> Result<VerificationResult, VerificationError> {
        if result.to_lowercase().contains("error")
            || result.to_lowercase().contains("failed to apply")
        {
            return Ok(VerificationResult::invalid("File edit operation failed")
                .with_correction("Check the diff syntax and file permissions"));
        }

        if result.contains("Applied successfully")
            || result.contains("edit applied")
            || result.contains("File updated")
        {
            return Ok(VerificationResult::valid("File edit verification passed"));
        }

        if result.is_empty() {
            return Ok(VerificationResult::uncertain(
                0.7,
                "Edit completed but output is empty",
            ));
        }

        Ok(VerificationResult::valid("File edit verification passed"))
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

    fn combine_results(
        basic: VerificationResult,
        specific: VerificationResult,
        semantic: VerificationResult,
    ) -> VerificationResult {
        let is_valid = basic.is_valid && specific.is_valid && semantic.is_valid;
        let confidence = (basic.confidence + specific.confidence + semantic.confidence) / 3.0;

        let mut reasons = Vec::new();
        if !basic.is_valid {
            reasons.push(format!("Basic: {}", basic.reason));
        }
        if !specific.is_valid {
            reasons.push(format!("Specific: {}", specific.reason));
        }
        if !semantic.is_valid {
            reasons.push(format!("Semantic: {}", semantic.reason));
        }

        let reason = if reasons.is_empty() {
            "All verifications passed".to_string()
        } else {
            reasons.join("; ")
        };

        let mut corrections = Vec::new();
        corrections.extend(basic.suggested_corrections);
        corrections.extend(specific.suggested_corrections);
        corrections.extend(semantic.suggested_corrections);

        VerificationResult {
            is_valid,
            confidence,
            reason,
            suggested_corrections: corrections,
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
