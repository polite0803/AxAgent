use std::time::Instant;

use serde_json::Value;

use super::types::{AtomicSkill, AtomicSkillError, AtomicSkillExecutionResult, EntryType};

/// Atomic skill executor that dispatches calls based on entry type
pub struct AtomicSkillExecutor;

impl AtomicSkillExecutor {
    /// Execute an atomic skill by dispatching to the appropriate executor
    /// based on the entry type.
    ///
    /// This is the primary entry point for the work engine to invoke atomic skills.
    pub async fn execute_builtin(entry_ref: &str, input: Value) -> Result<Value, AtomicSkillError> {
        let _ = (entry_ref, input);
        Err(AtomicSkillError {
            error_type: "not_implemented".to_string(),
            message: "Builtin execution requires axagent_core integration".to_string(),
        })
    }

    /// Execute an atomic skill via MCP tool
    pub async fn execute_mcp(
        entry_ref: &str,
        input: Value,
        mcp_call_fn: &dyn Fn(&str, Value) -> Result<Value, AtomicSkillError>,
    ) -> Result<Value, AtomicSkillError> {
        mcp_call_fn(entry_ref, input)
    }

    /// Execute an atomic skill via local tool registry
    pub async fn execute_local(
        entry_ref: &str,
        input: Value,
        local_execute_fn: &dyn Fn(&str, Value) -> Result<String, String>,
    ) -> Result<Value, AtomicSkillError> {
        let result = local_execute_fn(entry_ref, input).map_err(|e| AtomicSkillError {
            error_type: "local_tool_error".to_string(),
            message: e,
        })?;

        serde_json::from_str(&result).map_err(|e| AtomicSkillError {
            error_type: "parse_error".to_string(),
            message: format!("Failed to parse local tool result: {}", e),
        })
    }

    /// Execute an atomic skill via plugin tool
    pub async fn execute_plugin(
        entry_ref: &str,
        input: Value,
        plugin_call_fn: &dyn Fn(&str, Value) -> Result<Value, AtomicSkillError>,
    ) -> Result<Value, AtomicSkillError> {
        plugin_call_fn(entry_ref, input)
    }

    /// High-level execute method that dispatches based on entry type
    /// and records execution timing and result.
    pub async fn execute(
        skill: &AtomicSkill,
        input: Value,
        mcp_call_fn: &dyn Fn(&str, Value) -> Result<Value, AtomicSkillError>,
        local_execute_fn: &dyn Fn(&str, Value) -> Result<String, String>,
        plugin_call_fn: &dyn Fn(&str, Value) -> Result<Value, AtomicSkillError>,
    ) -> AtomicSkillExecutionResult {
        let start = Instant::now();

        let result = match &skill.entry_type {
            EntryType::Builtin => Self::execute_builtin(&skill.entry_ref, input).await,
            EntryType::Mcp => Self::execute_mcp(&skill.entry_ref, input, mcp_call_fn).await,
            EntryType::Local => {
                Self::execute_local(&skill.entry_ref, input, local_execute_fn).await
            }
            EntryType::Plugin => {
                Self::execute_plugin(&skill.entry_ref, input, plugin_call_fn).await
            }
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => AtomicSkillExecutionResult {
                skill_id: skill.id.clone(),
                success: true,
                output,
                execution_time_ms,
                error: None,
            },
            Err(error) => AtomicSkillExecutionResult {
                skill_id: skill.id.clone(),
                success: false,
                output: Value::Null,
                execution_time_ms,
                error: Some(error),
            },
        }
    }
}
