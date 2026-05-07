use crate::reasoning_state::ActionType;
use crate::thought_chain::ThoughtStep;
use async_trait::async_trait;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonValidationResult {
    pub is_valid_json: bool,
    pub schema_compliant: bool,
    pub errors: Vec<String>,
    pub parsed_type: Option<JsonType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonType {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub keys_added: Vec<String>,
    pub keys_removed: Vec<String>,
    pub keys_modified: Vec<String>,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field_path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
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

pub struct LlmSemanticValidator {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
}

impl LlmSemanticValidator {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: impl Into<String>,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model: model.into(),
        }
    }

    fn build_validation_prompt(tool_name: &str, input: &str, output: &str) -> String {
        format!(
            "You are a tool output validator. Analyze whether the tool output is semantically correct for the given tool and input.\n\n\
            Tool: {}\n\
            Input: {}\n\
            Output: {}\n\n\
            Respond with a JSON object containing:\n\
            - \"is_valid\": boolean\n\
            - \"confidence\": float between 0 and 1\n\
            - \"reason\": string explaining your assessment\n\
            - \"corrections\": array of suggested corrections (if any)\n\n\
            Consider:\n\
            1. Does the output make sense for this tool type?\n\
            2. Is the output consistent with the input?\n\
            3. Are there any logical contradictions?\n\
            4. Is the output complete and informative?",
            tool_name, input, output
        )
    }
}

#[async_trait]
impl SemanticValidator for LlmSemanticValidator {
    async fn validate_semantically(
        &self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let prompt = Self::build_validation_prompt(tool_name, input, output);

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(
                    "You are a precise tool output validator. Always respond with valid JSON."
                        .to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            temperature: Some(0.1),
            top_p: None,
            max_tokens: Some(512),
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
            .map_err(|e| VerificationError::LlmError(e.to_string()))?;

        let content = response.content.trim();

        let json_str = if content.starts_with("```json") {
            content
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if content.starts_with("```") {
            content
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            content
        };

        let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            VerificationError::ParseError(format!("Failed to parse LLM response as JSON: {}", e))
        })?;

        let is_valid = parsed
            .get("is_valid")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let confidence = parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let reason = parsed
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("LLM validation completed")
            .to_string();

        let mut result = if is_valid {
            VerificationResult::uncertain(confidence, reason)
        } else {
            VerificationResult::invalid(reason)
        };

        if let Some(corrections) = parsed.get("corrections").and_then(|v| v.as_array()) {
            for correction in corrections {
                if let Some(text) = correction.as_str() {
                    result = result.with_correction(text);
                }
            }
        }

        Ok(result)
    }
}

pub struct RuleBasedValidator {
    error_patterns: Vec<String>,
    success_patterns: HashMap<String, Vec<String>>,
    format_expectations: HashMap<String, OutputFormat>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    FilePath,
    List,
    KeyValue,
    Tabular,
    Mixed,
}

impl RuleBasedValidator {
    pub fn new() -> Self {
        let error_patterns = vec![
            "error".to_string(),
            "failed".to_string(),
            "exception".to_string(),
            "traceback".to_string(),
            "segmentation fault".to_string(),
            "core dumped".to_string(),
            "panic".to_string(),
            "stack overflow".to_string(),
            "out of memory".to_string(),
            "timeout".to_string(),
            "connection refused".to_string(),
            "permission denied".to_string(),
            "access denied".to_string(),
            "not found".to_string(),
            "no such file".to_string(),
            "invalid argument".to_string(),
            "syntax error".to_string(),
            "null pointer".to_string(),
            "undefined".to_string(),
            "nan".to_string(),
        ];

        let success_patterns = {
            let mut m = HashMap::new();
            m.insert(
                "read_file".to_string(),
                vec![
                    "contents".to_string(),
                    "file".to_string(),
                    "lines".to_string(),
                ],
            );
            m.insert(
                "write_file".to_string(),
                vec![
                    "written".to_string(),
                    "created".to_string(),
                    "saved".to_string(),
                ],
            );
            m.insert(
                "edit_file".to_string(),
                vec![
                    "applied".to_string(),
                    "updated".to_string(),
                    "modified".to_string(),
                ],
            );
            m.insert(
                "execute_command".to_string(),
                vec![
                    "completed".to_string(),
                    "finished".to_string(),
                    "done".to_string(),
                    "exit code 0".to_string(),
                ],
            );
            m.insert(
                "web_search".to_string(),
                vec![
                    "results".to_string(),
                    "found".to_string(),
                    "matches".to_string(),
                ],
            );
            m
        };

        let format_expectations = {
            let mut m = HashMap::new();
            m.insert("web_search".to_string(), OutputFormat::List);
            m.insert("glob_search".to_string(), OutputFormat::List);
            m.insert("file_search".to_string(), OutputFormat::List);
            m.insert("read_file".to_string(), OutputFormat::Text);
            m.insert("execute_command".to_string(), OutputFormat::Text);
            m
        };

        Self {
            error_patterns,
            success_patterns,
            format_expectations,
        }
    }

    fn detect_error_indicators(&self, output: &str) -> Vec<String> {
        let output_lower = output.to_lowercase();
        self.error_patterns
            .iter()
            .filter(|p| output_lower.contains(&p.to_lowercase()))
            .cloned()
            .collect()
    }

    fn check_success_patterns(&self, tool_name: &str, output: &str) -> Option<f32> {
        let patterns = self.success_patterns.get(tool_name)?;
        let output_lower = output.to_lowercase();
        let matches = patterns
            .iter()
            .filter(|p| output_lower.contains(&p.to_lowercase()))
            .count();
        if matches == 0 {
            Some(0.4)
        } else {
            Some(matches as f32 / patterns.len() as f32)
        }
    }

    fn detect_output_format(&self, output: &str) -> OutputFormat {
        if output.starts_with('{') || output.starts_with('[') {
            if serde_json::from_str::<serde_json::Value>(output).is_ok() {
                return if output.starts_with('{') {
                    OutputFormat::Json
                } else {
                    OutputFormat::Json
                };
            }
        }

        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > 1 {
            let has_consistent_delimiter = lines
                .iter()
                .all(|l| l.contains(": ") || l.contains(" = ") || l.contains("\t"));
            if has_consistent_delimiter {
                let has_colon = lines.iter().all(|l| l.contains(": "));
                return if has_colon {
                    OutputFormat::KeyValue
                } else {
                    OutputFormat::Tabular
                };
            }

            let has_path_chars = lines
                .iter()
                .all(|l| l.contains('/') || l.contains('\\') || l.contains('.'));
            if has_path_chars {
                return OutputFormat::FilePath;
            }

            return OutputFormat::List;
        }

        OutputFormat::Text
    }

    fn check_format_consistency(
        &self,
        tool_name: &str,
        output: &str,
    ) -> Option<VerificationResult> {
        let expected = self.format_expectations.get(tool_name)?;
        let actual = self.detect_output_format(output);

        if actual != *expected && !output.is_empty() {
            return Some(VerificationResult::uncertain(
                0.7,
                format!(
                    "Output format {:?} differs from expected {:?} for tool '{}'",
                    actual, expected, tool_name
                ),
            ));
        }

        None
    }

    fn check_output_completeness(
        &self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> VerificationResult {
        if output.is_empty() {
            return VerificationResult::invalid(format!(
                "Tool '{}' produced empty output",
                tool_name
            ));
        }

        let input_json = serde_json::from_str::<serde_json::Value>(input);
        if let Ok(ref parsed) = input_json {
            if let Some(obj) = parsed.as_object() {
                if obj.contains_key("path")
                    || obj.contains_key("file_path")
                    || obj.contains_key("filepath")
                {
                    if output.contains("No such file") || output.contains("not found") {
                        return VerificationResult::invalid("Referenced file path does not exist")
                            .with_correction("Verify the file path in the input");
                    }
                }

                if let Some(query) = obj.get("query").and_then(|v| v.as_str()) {
                    if !query.is_empty() && output.len() < query.len() / 2 {
                        return VerificationResult::uncertain(
                            0.5,
                            "Output seems too short relative to the query complexity",
                        );
                    }
                }
            }
        }

        let lines = output.lines().count();
        let truncated_indicators = ["...", "[truncated]", "(truncated)", "<truncated>"];
        let is_truncated = truncated_indicators.iter().any(|ind| output.contains(ind));
        if is_truncated && lines <= 3 {
            return VerificationResult::uncertain(0.6, "Output appears to be truncated")
                .with_correction("Consider requesting a larger output or paginating results");
        }

        VerificationResult::valid("Output completeness check passed")
    }
}

impl Default for RuleBasedValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SemanticValidator for RuleBasedValidator {
    async fn validate_semantically(
        &self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> Result<VerificationResult, VerificationError> {
        let mut results = Vec::new();

        let errors = self.detect_error_indicators(output);
        if !errors.is_empty() {
            let is_contextual = output.to_lowercase().contains("0 error")
                || output.to_lowercase().contains("no error")
                || output.to_lowercase().contains("0 errors")
                || output.to_lowercase().contains("no errors")
                || output.to_lowercase().contains("error: 0")
                || output.to_lowercase().contains("errors: 0");
            if !is_contextual {
                results.push(
                    VerificationResult::invalid(format!(
                        "Detected error indicators: {}",
                        errors.join(", ")
                    ))
                    .with_correction("Investigate the reported errors"),
                );
            }
        }

        if let Some(success_ratio) = self.check_success_patterns(tool_name, output) {
            if success_ratio < 0.5 && !output.is_empty() {
                results.push(VerificationResult::uncertain(
                    0.5 + success_ratio * 0.5,
                    format!(
                        "Low success pattern match ({:.0}%) for tool '{}'",
                        success_ratio * 100.0,
                        tool_name
                    ),
                ));
            }
        }

        if let Some(format_result) = self.check_format_consistency(tool_name, output) {
            results.push(format_result);
        }

        let completeness = self.check_output_completeness(tool_name, input, output);
        if !completeness.is_valid || completeness.confidence < 0.8 {
            results.push(completeness);
        }

        if results.is_empty() {
            return Ok(VerificationResult::valid("Rule-based validation passed"));
        }

        let any_invalid = results.iter().any(|r| !r.is_valid);
        let avg_confidence =
            results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32;
        let combined_reason = results
            .iter()
            .map(|r| r.reason.clone())
            .collect::<Vec<_>>()
            .join("; ");
        let mut corrections = Vec::new();
        for r in &results {
            corrections.extend(r.suggested_corrections.clone());
        }

        let result = if any_invalid {
            VerificationResult::invalid(combined_reason)
        } else {
            VerificationResult::uncertain(avg_confidence, "Rule-based validation found concerns")
        };

        let mut result = result;
        for c in corrections {
            result = result.with_correction(c);
        }

        Ok(result)
    }
}

pub fn validate_json_output(
    output: &str,
    schema: Option<&serde_json::Value>,
) -> JsonValidationResult {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);

    match parsed {
        Ok(value) => {
            let json_type = match &value {
                serde_json::Value::Object(_) => Some(JsonType::Object),
                serde_json::Value::Array(_) => Some(JsonType::Array),
                serde_json::Value::String(_) => Some(JsonType::String),
                serde_json::Value::Number(_) => Some(JsonType::Number),
                serde_json::Value::Bool(_) => Some(JsonType::Boolean),
                serde_json::Value::Null => Some(JsonType::Null),
            };

            let mut errors = Vec::new();
            let schema_compliant = if let Some(schema) = schema {
                validate_against_schema(&value, schema, "", &mut errors)
            } else {
                true
            };

            JsonValidationResult {
                is_valid_json: true,
                schema_compliant,
                errors,
                parsed_type: json_type,
            }
        },
        Err(e) => JsonValidationResult {
            is_valid_json: false,
            schema_compliant: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            parsed_type: None,
        },
    }
}

fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    errors: &mut Vec<String>,
) -> bool {
    let mut valid = true;

    if let Some(schema_type) = schema.get("type").and_then(|t| t.as_str()) {
        let type_match = match schema_type {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => true,
        };
        if !type_match {
            errors.push(format!(
                "Type mismatch at '{}': expected '{}'",
                if path.is_empty() { "root" } else { path },
                schema_type
            ));
            valid = false;
        }
    }

    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        if let Some(obj) = value.as_object() {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if !obj.contains_key(field_name) {
                        errors.push(format!(
                            "Missing required field '{}' at '{}'",
                            field_name,
                            if path.is_empty() { "root" } else { path }
                        ));
                        valid = false;
                    }
                }
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        if let Some(obj) = value.as_object() {
            for (key, prop_schema) in properties {
                if let Some(child_value) = obj.get(key) {
                    let child_path = if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    if !validate_against_schema(child_value, prop_schema, &child_path, errors) {
                        valid = false;
                    }
                }
            }
        }
    }

    if let Some(items_schema) = schema.get("items") {
        if let Some(arr) = value.as_array() {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{}[{}]", if path.is_empty() { "root" } else { path }, i);
                if !validate_against_schema(item, items_schema, &item_path, errors) {
                    valid = false;
                }
            }
        }
    }

    if let Some(min) = schema.get("minLength").and_then(|v| v.as_u64()) {
        if let Some(s) = value.as_str() {
            if (s.len() as u64) < min {
                errors.push(format!(
                    "String at '{}' is too short (min length: {})",
                    if path.is_empty() { "root" } else { path },
                    min
                ));
                valid = false;
            }
        }
    }

    if let Some(max) = schema.get("maxLength").and_then(|v| v.as_u64()) {
        if let Some(s) = value.as_str() {
            if (s.len() as u64) > max {
                errors.push(format!(
                    "String at '{}' is too long (max length: {})",
                    if path.is_empty() { "root" } else { path },
                    max
                ));
                valid = false;
            }
        }
    }

    if let Some(enum_values) = schema.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.iter().any(|e| e == value) {
            errors.push(format!(
                "Value at '{}' is not one of the allowed enum values",
                if path.is_empty() { "root" } else { path }
            ));
            valid = false;
        }
    }

    valid
}

pub fn detect_state_change(before: &serde_json::Value, after: &serde_json::Value) -> StateDiff {
    let mut keys_added = Vec::new();
    let mut keys_removed = Vec::new();
    let mut keys_modified = Vec::new();
    let mut changes = Vec::new();

    collect_diff(
        before,
        after,
        "",
        &mut keys_added,
        &mut keys_removed,
        &mut keys_modified,
        &mut changes,
    );

    StateDiff {
        keys_added,
        keys_removed,
        keys_modified,
        changes,
    }
}

fn collect_diff(
    before: &serde_json::Value,
    after: &serde_json::Value,
    path: &str,
    keys_added: &mut Vec<String>,
    keys_removed: &mut Vec<String>,
    keys_modified: &mut Vec<String>,
    changes: &mut Vec<FieldChange>,
) {
    match (before, after) {
        (serde_json::Value::Object(b_obj), serde_json::Value::Object(a_obj)) => {
            let b_keys: HashSet<&String> = b_obj.keys().collect();
            let a_keys: HashSet<&String> = a_obj.keys().collect();

            for key in a_keys.difference(&b_keys) {
                let field_path = format_path(path, key);
                keys_added.push(field_path.clone());
                changes.push(FieldChange {
                    field_path: field_path.clone(),
                    old_value: None,
                    new_value: Some(a_obj[*key].clone()),
                });
            }

            for key in b_keys.difference(&a_keys) {
                let field_path = format_path(path, key);
                keys_removed.push(field_path.clone());
                changes.push(FieldChange {
                    field_path: field_path.clone(),
                    old_value: Some(b_obj[*key].clone()),
                    new_value: None,
                });
            }

            for key in b_keys.intersection(&a_keys) {
                let b_val = &b_obj[*key];
                let a_val = &a_obj[*key];
                if b_val != a_val {
                    let field_path = format_path(path, key);
                    keys_modified.push(field_path.clone());
                    changes.push(FieldChange {
                        field_path: field_path.clone(),
                        old_value: Some(b_val.clone()),
                        new_value: Some(a_val.clone()),
                    });
                }
            }
        },
        (serde_json::Value::Array(b_arr), serde_json::Value::Array(a_arr)) => {
            let max_len = b_arr.len().max(a_arr.len());
            for i in 0..max_len {
                let item_path = format!("{}[{}]", if path.is_empty() { "root" } else { path }, i);
                match (b_arr.get(i), a_arr.get(i)) {
                    (Some(b_val), Some(a_val)) => {
                        if b_val != a_val {
                            keys_modified.push(item_path.clone());
                            changes.push(FieldChange {
                                field_path: item_path.clone(),
                                old_value: Some(b_val.clone()),
                                new_value: Some(a_val.clone()),
                            });
                            collect_diff(
                                b_val,
                                a_val,
                                &item_path,
                                keys_added,
                                keys_removed,
                                keys_modified,
                                changes,
                            );
                        }
                    },
                    (None, Some(a_val)) => {
                        keys_added.push(item_path.clone());
                        changes.push(FieldChange {
                            field_path: item_path.clone(),
                            old_value: None,
                            new_value: Some(a_val.clone()),
                        });
                    },
                    (Some(b_val), None) => {
                        keys_removed.push(item_path.clone());
                        changes.push(FieldChange {
                            field_path: item_path.clone(),
                            old_value: Some(b_val.clone()),
                            new_value: None,
                        });
                    },
                    (None, None) => {},
                }
            }
        },
        _ => {
            if before != after {
                let field_path = if path.is_empty() {
                    "root".to_string()
                } else {
                    path.to_string()
                };
                keys_modified.push(field_path.clone());
                changes.push(FieldChange {
                    field_path: field_path.clone(),
                    old_value: Some(before.clone()),
                    new_value: Some(after.clone()),
                });
            }
        },
    }
}

fn format_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", parent, key)
    }
}

pub struct SelfVerifier {
    strict_mode: bool,
    semantic_validator: Option<Arc<dyn SemanticValidator>>,
    rule_based_validator: Option<Arc<RuleBasedValidator>>,
    llm_validator: Option<Arc<LlmSemanticValidator>>,
}

impl SelfVerifier {
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            semantic_validator: None,
            rule_based_validator: Some(Arc::new(RuleBasedValidator::new())),
            llm_validator: None,
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

    pub fn with_rule_based_validator(mut self, validator: Arc<RuleBasedValidator>) -> Self {
        self.rule_based_validator = Some(validator);
        self
    }

    pub fn with_llm_validator(mut self, validator: Arc<LlmSemanticValidator>) -> Self {
        self.llm_validator = Some(validator);
        self
    }

    pub fn with_llm(
        mut self,
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: impl Into<String>,
    ) -> Self {
        self.llm_validator = Some(Arc::new(LlmSemanticValidator::new(adapter, ctx, model)));
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
            return Ok(VerificationResult::invalid("Confidence below threshold in strict mode"));
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
            _ => Ok(VerificationResult::valid("No specific validation available")),
        }?;

        let rule_based_result = if let Some(ref rbv) = self.rule_based_validator {
            let rb = rbv.validate_semantically(tool_name, input, result).await?;
            if !rb.is_valid {
                return Ok(Self::combine_results(base_verification, specific_check, rb));
            }
            Some(rb)
        } else {
            None
        };

        if let Some(ref validator) = self.semantic_validator {
            let semantic_result = validator
                .validate_semantically(tool_name, input, result)
                .await?;
            if let Some(ref rb) = rule_based_result {
                let combined = Self::combine_results(base_verification, specific_check, rb.clone());
                return Ok(Self::merge_with_semantic(combined, semantic_result));
            }
            return Ok(Self::combine_results(base_verification, specific_check, semantic_result));
        }

        if let Some(ref llm) = self.llm_validator {
            let llm_result = llm.validate_semantically(tool_name, input, result).await?;
            if let Some(ref rb) = rule_based_result {
                let combined = Self::combine_results(base_verification, specific_check, rb.clone());
                return Ok(Self::merge_with_semantic(combined, llm_result));
            }
            return Ok(Self::combine_results(base_verification, specific_check, llm_result));
        }

        if let Some(rb) = rule_based_result {
            Ok(Self::combine_results(base_verification, specific_check, rb))
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
            return Ok(VerificationResult::invalid("Web search returned no results")
                .with_correction("Try different search terms"));
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
            return Ok(VerificationResult::uncertain(0.7, "Edit completed but output is empty"));
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

        Ok(VerificationResult::valid(format!("Tool '{}' executed successfully", tool_name)))
    }

    async fn verify_llm_result(
        &self,
        step: &ThoughtStep,
    ) -> Result<VerificationResult, VerificationError> {
        let response = step.result.as_deref().unwrap_or("");

        if response.is_empty() {
            return Ok(VerificationResult::invalid("LLM returned empty response".to_string()));
        }

        if response.len() < 10 {
            return Ok(VerificationResult::uncertain(0.6, "LLM response is unusually short"));
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

    fn merge_with_semantic(
        base: VerificationResult,
        semantic: VerificationResult,
    ) -> VerificationResult {
        let is_valid = base.is_valid && semantic.is_valid;
        let confidence = if semantic.is_valid {
            (base.confidence * 0.6) + (semantic.confidence * 0.4)
        } else {
            (base.confidence * 0.3) + (semantic.confidence * 0.7)
        };

        let mut reasons = Vec::new();
        if !base.is_valid {
            reasons.push(base.reason.clone());
        }
        if !semantic.is_valid {
            reasons.push(format!("Semantic: {}", semantic.reason));
        }

        let reason = if reasons.is_empty() {
            "All verifications passed".to_string()
        } else {
            reasons.join("; ")
        };

        let mut corrections = base.suggested_corrections;
        corrections.extend(semantic.suggested_corrections);

        VerificationResult {
            is_valid,
            confidence: confidence.clamp(0.0, 1.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_state::ReasoningState;
    use crate::thought_chain::{Action, ThoughtStep};

    #[test]
    fn test_verification_result_valid() {
        let r = VerificationResult::valid("all good");
        assert!(r.is_valid);
        assert!((r.confidence - 1.0).abs() < f32::EPSILON);
        assert_eq!(r.reason, "all good");
        assert!(r.suggested_corrections.is_empty());
    }

    #[test]
    fn test_verification_result_invalid() {
        let r = VerificationResult::invalid("bad output");
        assert!(!r.is_valid);
        assert!((r.confidence - 1.0).abs() < f32::EPSILON);
        assert_eq!(r.reason, "bad output");
    }

    #[test]
    fn test_verification_result_uncertain() {
        let r = VerificationResult::uncertain(0.6, "maybe ok");
        assert!(r.is_valid);
        assert!((r.confidence - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_verification_result_uncertain_clamped() {
        let r = VerificationResult::uncertain(1.5, "high");
        assert!((r.confidence - 1.0).abs() < f32::EPSILON);
        let r2 = VerificationResult::uncertain(-0.5, "low");
        assert!((r2.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_verification_result_with_correction() {
        let r = VerificationResult::invalid("err")
            .with_correction("fix it")
            .with_correction("try again");
        assert_eq!(r.suggested_corrections.len(), 2);
    }

    #[test]
    fn test_rule_based_validator_detect_error() {
        let v = RuleBasedValidator::new();
        let errors = v.detect_error_indicators("Error: something went wrong");
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.to_lowercase().contains("error")));
    }

    #[test]
    fn test_rule_based_validator_detect_panic() {
        let v = RuleBasedValidator::new();
        let errors = v.detect_error_indicators("thread panicked at 'overflow'");
        assert!(errors.iter().any(|e| e.to_lowercase().contains("panic")));
    }

    #[test]
    fn test_rule_based_validator_no_error() {
        let v = RuleBasedValidator::new();
        let errors = v.detect_error_indicators("File contents loaded successfully");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_rule_based_validator_success_patterns() {
        let v = RuleBasedValidator::new();
        let ratio = v.check_success_patterns("read_file", "File contents loaded");
        assert!(ratio.is_some());
        assert!(ratio.unwrap() > 0.0);
    }

    #[test]
    fn test_rule_based_validator_success_patterns_no_match() {
        let v = RuleBasedValidator::new();
        let ratio = v.check_success_patterns("read_file", "unexpected garbage");
        assert!(ratio.is_some());
        assert!((ratio.unwrap() - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_rule_based_validator_success_patterns_unknown_tool() {
        let v = RuleBasedValidator::new();
        assert!(v
            .check_success_patterns("unknown_tool", "anything")
            .is_none());
    }

    #[test]
    fn test_rule_based_validator_detect_format_json() {
        let v = RuleBasedValidator::new();
        assert_eq!(v.detect_output_format(r#"{"key": "value"}"#), OutputFormat::Json);
        assert_eq!(v.detect_output_format(r#"[1, 2, 3]"#), OutputFormat::Json);
    }

    #[test]
    fn test_rule_based_validator_detect_format_text() {
        let v = RuleBasedValidator::new();
        assert_eq!(v.detect_output_format("just some text"), OutputFormat::Text);
    }

    #[test]
    fn test_rule_based_validator_detect_format_key_value() {
        let v = RuleBasedValidator::new();
        let kv = "name: alice\nage: 30";
        assert_eq!(v.detect_output_format(kv), OutputFormat::KeyValue);
    }

    #[test]
    fn test_rule_based_validator_detect_format_list() {
        let v = RuleBasedValidator::new();
        let list = "item one\nitem two\nitem three";
        assert_eq!(v.detect_output_format(list), OutputFormat::List);
    }

    #[test]
    fn test_rule_based_validator_format_consistency_match() {
        let v = RuleBasedValidator::new();
        let result = v.check_format_consistency("web_search", "- result 1\n- result 2");
        assert!(result.is_none());
    }

    #[test]
    fn test_rule_based_validator_format_consistency_mismatch() {
        let v = RuleBasedValidator::new();
        let result = v.check_format_consistency("web_search", "just plain text");
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.confidence < 1.0);
    }

    #[test]
    fn test_rule_based_validator_format_consistency_unknown_tool() {
        let v = RuleBasedValidator::new();
        assert!(v
            .check_format_consistency("unknown_tool", "anything")
            .is_none());
    }

    #[test]
    fn test_rule_based_validator_completeness_empty() {
        let v = RuleBasedValidator::new();
        let r = v.check_output_completeness("tool", "{}", "");
        assert!(!r.is_valid);
    }

    #[test]
    fn test_rule_based_validator_completeness_truncated() {
        let v = RuleBasedValidator::new();
        let r = v.check_output_completeness("tool", "{}", "some output...[truncated]");
        assert!(r.confidence < 1.0);
    }

    #[test]
    fn test_rule_based_validator_completeness_ok() {
        let v = RuleBasedValidator::new();
        let r = v.check_output_completeness("tool", "{}", "normal output content");
        assert!(r.is_valid);
    }

    #[tokio::test]
    async fn test_rule_based_validator_semantic_valid() {
        let v = RuleBasedValidator::new();
        let result = v
            .validate_semantically("read_file", "{}", "File contents loaded successfully")
            .await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.is_valid);
    }

    #[tokio::test]
    async fn test_rule_based_validator_semantic_error() {
        let v = RuleBasedValidator::new();
        let result = v
            .validate_semantically("execute_command", "{}", "Error: command failed with exception")
            .await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(!r.is_valid);
    }

    #[tokio::test]
    async fn test_rule_based_validator_contextual_error() {
        let v = RuleBasedValidator::new();
        let result = v
            .validate_semantically("execute_command", "{}", "Build completed with 0 errors")
            .await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.is_valid);
    }

    #[test]
    fn test_validate_json_output_valid_object() {
        let r = validate_json_output(r#"{"name": "test"}"#, None);
        assert!(r.is_valid_json);
        assert!(r.schema_compliant);
        assert!(r.errors.is_empty());
        assert_eq!(r.parsed_type, Some(JsonType::Object));
    }

    #[test]
    fn test_validate_json_output_valid_array() {
        let r = validate_json_output(r#"[1, 2, 3]"#, None);
        assert!(r.is_valid_json);
        assert_eq!(r.parsed_type, Some(JsonType::Array));
    }

    #[test]
    fn test_validate_json_output_invalid() {
        let r = validate_json_output("not json at all", None);
        assert!(!r.is_valid_json);
        assert!(!r.schema_compliant);
        assert!(!r.errors.is_empty());
        assert!(r.parsed_type.is_none());
    }

    #[test]
    fn test_validate_json_output_schema_type_mismatch() {
        let schema = serde_json::json!({"type": "string"});
        let r = validate_json_output("42", Some(&schema));
        assert!(r.is_valid_json);
        assert!(!r.schema_compliant);
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn test_validate_json_output_schema_required() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "age"]
        });
        let r = validate_json_output(r#"{"name": "alice"}"#, Some(&schema));
        assert!(r.is_valid_json);
        assert!(!r.schema_compliant);
    }

    #[test]
    fn test_validate_json_output_schema_ok() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string"}
            }
        });
        let r = validate_json_output(r#"{"name": "alice"}"#, Some(&schema));
        assert!(r.is_valid_json);
        assert!(r.schema_compliant);
    }

    #[test]
    fn test_validate_json_output_schema_enum() {
        let schema = serde_json::json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });
        let r = validate_json_output(r#""red""#, Some(&schema));
        assert!(r.schema_compliant);
        let r2 = validate_json_output(r#""yellow""#, Some(&schema));
        assert!(!r2.schema_compliant);
    }

    #[test]
    fn test_validate_json_output_schema_min_max_length() {
        let schema = serde_json::json!({
            "type": "string",
            "minLength": 2,
            "maxLength": 5
        });
        let r_ok = validate_json_output(r#""abc""#, Some(&schema));
        assert!(r_ok.schema_compliant);
        let r_short = validate_json_output(r#""a""#, Some(&schema));
        assert!(!r_short.schema_compliant);
        let r_long = validate_json_output(r#""abcdef""#, Some(&schema));
        assert!(!r_long.schema_compliant);
    }

    #[test]
    fn test_detect_state_change_addition() {
        let before = serde_json::json!({"a": 1});
        let after = serde_json::json!({"a": 1, "b": 2});
        let diff = detect_state_change(&before, &after);
        assert!(diff.keys_added.contains(&"b".to_string()));
        assert!(diff.keys_removed.is_empty());
    }

    #[test]
    fn test_detect_state_change_deletion() {
        let before = serde_json::json!({"a": 1, "b": 2});
        let after = serde_json::json!({"a": 1});
        let diff = detect_state_change(&before, &after);
        assert!(diff.keys_removed.contains(&"b".to_string()));
        assert!(diff.keys_added.is_empty());
    }

    #[test]
    fn test_detect_state_change_modification() {
        let before = serde_json::json!({"a": 1});
        let after = serde_json::json!({"a": 2});
        let diff = detect_state_change(&before, &after);
        assert!(diff.keys_modified.contains(&"a".to_string()));
        assert!(diff.keys_added.is_empty());
        assert!(diff.keys_removed.is_empty());
    }

    #[test]
    fn test_detect_state_change_nested() {
        let before = serde_json::json!({"outer": {"inner": 1}});
        let after = serde_json::json!({"outer": {"inner": 2}});
        let diff = detect_state_change(&before, &after);
        assert!(!diff.changes.is_empty());
    }

    #[test]
    fn test_detect_state_change_no_change() {
        let before = serde_json::json!({"a": 1});
        let after = serde_json::json!({"a": 1});
        let diff = detect_state_change(&before, &after);
        assert!(diff.keys_added.is_empty());
        assert!(diff.keys_removed.is_empty());
        assert!(diff.keys_modified.is_empty());
        assert!(diff.changes.is_empty());
    }

    #[test]
    fn test_detect_state_change_array() {
        let before = serde_json::json!([1, 2, 3]);
        let after = serde_json::json!([1, 5, 3, 4]);
        let diff = detect_state_change(&before, &after);
        assert!(!diff.keys_modified.is_empty() || !diff.keys_added.is_empty());
    }

    #[test]
    fn test_self_verifier_default() {
        let sv = SelfVerifier::new();
        let sv2 = SelfVerifier::default();
        assert!(!sv.strict_mode);
        assert!(!sv2.strict_mode);
    }

    #[test]
    fn test_self_verifier_with_strict_mode() {
        let sv = SelfVerifier::new().with_strict_mode(true);
        assert!(sv.strict_mode);
    }

    #[test]
    fn test_self_verifier_is_empty_ok_tool() {
        assert!(SelfVerifier::is_empty_ok_tool("delete_file"));
        assert!(SelfVerifier::is_empty_ok_tool("move_file"));
        assert!(SelfVerifier::is_empty_ok_tool("create_directory"));
        assert!(!SelfVerifier::is_empty_ok_tool("read_file"));
        assert!(!SelfVerifier::is_empty_ok_tool("web_search"));
    }

    #[test]
    fn test_self_verifier_truncate_string() {
        assert_eq!(SelfVerifier::truncate_string("hello", 10), "hello");
        let long = "a".repeat(200);
        let truncated = SelfVerifier::truncate_string(&long, 100);
        assert!(truncated.len() <= 100);
        assert!(truncated.ends_with("..."));
    }

    #[tokio::test]
    async fn test_self_verifier_verify_tool_call() {
        let sv = SelfVerifier::new();
        let step = ThoughtStep {
            id: 0,
            state: ReasoningState::Acting,
            reasoning: "read a file".to_string(),
            action: Some(Action {
                action_type: ActionType::ToolCall,
                tool_name: Some("read_file".to_string()),
                tool_input: Some(serde_json::json!({"path": "/tmp/test.txt"})),
                llm_prompt: None,
                requires_confirmation: false,
            }),
            observation: None,
            result: Some("File contents loaded successfully".to_string()),
            is_verified: false,
            timestamp: String::new(),
        };
        let result = sv.verify(&step, "read file").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_self_verifier_verify_llm_call() {
        let sv = SelfVerifier::new();
        let step = ThoughtStep {
            id: 0,
            state: ReasoningState::Thinking,
            reasoning: "think".to_string(),
            action: Some(Action {
                action_type: ActionType::LlmCall,
                tool_name: None,
                tool_input: None,
                llm_prompt: Some("prompt".to_string()),
                requires_confirmation: false,
            }),
            observation: None,
            result: Some("This is a reasonable response from the LLM.".to_string()),
            is_verified: false,
            timestamp: String::new(),
        };
        let result = sv.verify(&step, "goal").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_valid);
    }

    #[tokio::test]
    async fn test_self_verifier_verify_strict_mode_low_confidence() {
        let sv = SelfVerifier::new().with_strict_mode(true);
        let step = ThoughtStep {
            id: 0,
            state: ReasoningState::Reflecting,
            reasoning: "reflect".to_string(),
            action: Some(Action {
                action_type: ActionType::Reflect,
                tool_name: None,
                tool_input: None,
                llm_prompt: None,
                requires_confirmation: false,
            }),
            observation: None,
            result: Some("ok".to_string()),
            is_verified: false,
            timestamp: String::new(),
        };
        let result = sv.verify(&step, "goal").await.unwrap();
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_self_verifier_verify_tool_error() {
        let sv = SelfVerifier::new();
        let step = ThoughtStep {
            id: 0,
            state: ReasoningState::Acting,
            reasoning: "run command".to_string(),
            action: Some(Action {
                action_type: ActionType::ToolCall,
                tool_name: Some("execute_command".to_string()),
                tool_input: Some(serde_json::json!({"command": "ls"})),
                llm_prompt: None,
                requires_confirmation: false,
            }),
            observation: None,
            result: Some("Error: command failed".to_string()),
            is_verified: false,
            timestamp: String::new(),
        };
        let result = sv.verify(&step, "goal").await.unwrap();
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_self_verifier_verify_llm_empty() {
        let sv = SelfVerifier::new();
        let step = ThoughtStep {
            id: 0,
            state: ReasoningState::Thinking,
            reasoning: "think".to_string(),
            action: Some(Action {
                action_type: ActionType::LlmCall,
                tool_name: None,
                tool_input: None,
                llm_prompt: None,
                requires_confirmation: false,
            }),
            observation: None,
            result: Some("".to_string()),
            is_verified: false,
            timestamp: String::new(),
        };
        let result = sv.verify(&step, "goal").await.unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_combine_results_all_valid() {
        let a = VerificationResult::valid("a");
        let b = VerificationResult::valid("b");
        let c = VerificationResult::valid("c");
        let combined = SelfVerifier::combine_results(a, b, c);
        assert!(combined.is_valid);
    }

    #[test]
    fn test_combine_results_one_invalid() {
        let a = VerificationResult::valid("a");
        let b = VerificationResult::invalid("b");
        let c = VerificationResult::valid("c");
        let combined = SelfVerifier::combine_results(a, b, c);
        assert!(!combined.is_valid);
    }

    #[test]
    fn test_merge_with_semantic_valid() {
        let base = VerificationResult::valid("base");
        let sem = VerificationResult::valid("sem");
        let merged = SelfVerifier::merge_with_semantic(base, sem);
        assert!(merged.is_valid);
    }

    #[test]
    fn test_merge_with_semantic_invalid() {
        let base = VerificationResult::valid("base");
        let sem = VerificationResult::invalid("sem");
        let merged = SelfVerifier::merge_with_semantic(base, sem);
        assert!(!merged.is_valid);
    }
}
