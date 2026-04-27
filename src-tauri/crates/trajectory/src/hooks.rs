//! Hooks service module
//!
//! Replaces TypeScript `hooksService.ts` with Rust implementation.
//! Provides hook lifecycle management for tool use events.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "pre_tool_use",
            HookEvent::PostToolUse => "post_tool_use",
            HookEvent::PostToolUseFailure => "post_tool_use_failure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCommand {
    pub id: String,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub event: HookEvent,
    pub commands: Vec<HookCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookExecutionStatus {
    Started,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecution {
    pub id: String,
    #[serde(rename = "hookId")]
    pub hook_id: String,
    pub event: HookEvent,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub status: HookExecutionStatus,
    #[serde(rename = "startedAt")]
    pub started_at: i64,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecutionResult {
    pub denied: bool,
    pub output: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksStore {
    #[serde(rename = "preToolUse")]
    pub pre_tool_use: Vec<HookCommand>,
    #[serde(rename = "postToolUse")]
    pub post_tool_use: Vec<HookCommand>,
    #[serde(rename = "postToolUseFailure")]
    pub post_tool_use_failure: Vec<HookCommand>,
}

fn generate_hook_id() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random: String = (0..9)
        .map(|_| {
            let idx = (timestamp % 36) as usize;
            let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
            chars[idx] as char
        })
        .collect();
    format!("hook_{}_{}", timestamp, random)
}

#[allow(dead_code)]
fn generate_execution_id() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random: String = (0..9)
        .map(|_| {
            let idx = (timestamp % 36) as usize;
            let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
            chars[idx] as char
        })
        .collect();
    format!("exec_{}_{}", timestamp, random)
}

pub struct HooksService {
    store: HooksStore,
    executions: Vec<HookExecution>,
    max_executions_stored: usize,
}

impl Default for HooksService {
    fn default() -> Self {
        Self::new()
    }
}

impl HooksService {
    pub fn new() -> Self {
        Self {
            store: HooksStore::default(),
            executions: Vec::new(),
            max_executions_stored: 100,
        }
    }

    pub fn get_default_hooks() -> HooksStore {
        HooksStore::default()
    }

    pub fn load_hooks(&self) -> HooksStore {
        self.store.clone()
    }

    pub fn save_hooks(&mut self, store: HooksStore) {
        self.store = store;
    }

    pub fn add_hook_command(
        &mut self,
        event: HookEvent,
        command: String,
        args: Option<Vec<String>>,
        description: Option<String>,
    ) -> HookCommand {
        let new_hook = HookCommand {
            id: generate_hook_id(),
            command,
            args,
            description,
            enabled: true,
        };

        match event {
            HookEvent::PreToolUse => self.store.pre_tool_use.push(new_hook.clone()),
            HookEvent::PostToolUse => self.store.post_tool_use.push(new_hook.clone()),
            HookEvent::PostToolUseFailure => {
                self.store.post_tool_use_failure.push(new_hook.clone())
            }
        }

        new_hook
    }

    pub fn remove_hook_command(&mut self, event: HookEvent, hook_id: &str) -> bool {
        let commands = match event {
            HookEvent::PreToolUse => &mut self.store.pre_tool_use,
            HookEvent::PostToolUse => &mut self.store.post_tool_use,
            HookEvent::PostToolUseFailure => &mut self.store.post_tool_use_failure,
        };

        if let Some(pos) = commands.iter().position(|h| h.id == hook_id) {
            commands.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn update_hook_command(
        &mut self,
        event: HookEvent,
        hook_id: &str,
        updates: HookCommandUpdate,
    ) -> Option<HookCommand> {
        let commands = match event {
            HookEvent::PreToolUse => &mut self.store.pre_tool_use,
            HookEvent::PostToolUse => &mut self.store.post_tool_use,
            HookEvent::PostToolUseFailure => &mut self.store.post_tool_use_failure,
        };

        if let Some(hook) = commands.iter_mut().find(|h| h.id == hook_id) {
            if let Some(command) = updates.command {
                hook.command = command;
            }
            if let Some(args) = updates.args {
                hook.args = Some(args);
            }
            if let Some(description) = updates.description {
                hook.description = Some(description);
            }
            if let Some(enabled) = updates.enabled {
                hook.enabled = enabled;
            }
            Some(hook.clone())
        } else {
            None
        }
    }

    pub fn toggle_hook_enabled(&mut self, event: HookEvent, hook_id: &str) -> Option<bool> {
        let commands = match event {
            HookEvent::PreToolUse => &mut self.store.pre_tool_use,
            HookEvent::PostToolUse => &mut self.store.post_tool_use,
            HookEvent::PostToolUseFailure => &mut self.store.post_tool_use_failure,
        };

        if let Some(hook) = commands.iter_mut().find(|h| h.id == hook_id) {
            hook.enabled = !hook.enabled;
            Some(hook.enabled)
        } else {
            None
        }
    }

    pub fn get_hook_commands(&self, event: HookEvent) -> Vec<&HookCommand> {
        let commands = match event {
            HookEvent::PreToolUse => &self.store.pre_tool_use,
            HookEvent::PostToolUse => &self.store.post_tool_use,
            HookEvent::PostToolUseFailure => &self.store.post_tool_use_failure,
        };

        commands.iter().filter(|h| h.enabled).collect()
    }

    pub fn get_all_hooks(&self) -> HooksStore {
        self.store.clone()
    }

    pub fn get_hooks_by_event(&self, event: HookEvent) -> Vec<HookCommand> {
        match event {
            HookEvent::PreToolUse => self.store.pre_tool_use.clone(),
            HookEvent::PostToolUse => self.store.post_tool_use.clone(),
            HookEvent::PostToolUseFailure => self.store.post_tool_use_failure.clone(),
        }
    }

    pub fn record_execution(&mut self, execution: HookExecution) {
        self.executions.push(execution);

        if self.executions.len() > self.max_executions_stored {
            let drain_count = self.executions.len() - self.max_executions_stored;
            self.executions.drain(..drain_count);
        }
    }

    pub fn get_executions(&self, limit: Option<usize>) -> Vec<&HookExecution> {
        let limit = limit.unwrap_or(self.max_executions_stored);
        self.executions.iter().rev().take(limit).collect()
    }

    pub fn clear_executions(&mut self) {
        self.executions.clear();
    }

    pub fn get_execution_stats(&self) -> HookExecutionStats {
        let total = self.executions.len();
        let started = self
            .executions
            .iter()
            .filter(|e| e.status == HookExecutionStatus::Started)
            .count();
        let completed = self
            .executions
            .iter()
            .filter(|e| e.status == HookExecutionStatus::Completed)
            .count();
        let failed = self
            .executions
            .iter()
            .filter(|e| e.status == HookExecutionStatus::Failed)
            .count();
        let cancelled = self
            .executions
            .iter()
            .filter(|e| e.status == HookExecutionStatus::Cancelled)
            .count();

        HookExecutionStats {
            total_executions: total,
            started_count: started,
            completed_count: completed,
            failed_count: failed,
            cancelled_count: cancelled,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HookCommandUpdate {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecutionStats {
    #[serde(rename = "totalExecutions")]
    pub total_executions: usize,
    #[serde(rename = "startedCount")]
    pub started_count: usize,
    #[serde(rename = "completedCount")]
    pub completed_count: usize,
    #[serde(rename = "failedCount")]
    pub failed_count: usize,
    #[serde(rename = "cancelledCount")]
    pub cancelled_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_hook_command() {
        let mut service = HooksService::new();

        let hook = service.add_hook_command(
            HookEvent::PreToolUse,
            "echo".to_string(),
            Some(vec!["test".to_string()]),
            Some("Test hook".to_string()),
        );

        assert!(!hook.id.is_empty());
        assert_eq!(hook.command, "echo");
        assert_eq!(hook.enabled, true);
    }

    #[test]
    fn test_remove_hook_command() {
        let mut service = HooksService::new();

        let hook = service.add_hook_command(HookEvent::PreToolUse, "echo".to_string(), None, None);

        let result = service.remove_hook_command(HookEvent::PreToolUse, &hook.id);
        assert!(result);

        let commands = service.get_hook_commands(HookEvent::PreToolUse);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_toggle_hook_enabled() {
        let mut service = HooksService::new();

        let hook = service.add_hook_command(HookEvent::PreToolUse, "echo".to_string(), None, None);

        assert_eq!(hook.enabled, true);

        let new_state = service.toggle_hook_enabled(HookEvent::PreToolUse, &hook.id);
        assert_eq!(new_state, Some(false));

        let new_state = service.toggle_hook_enabled(HookEvent::PreToolUse, &hook.id);
        assert_eq!(new_state, Some(true));
    }

    #[test]
    fn test_get_hook_commands_filters_disabled() {
        let mut service = HooksService::new();

        service.add_hook_command(
            HookEvent::PreToolUse,
            "enabled_command".to_string(),
            None,
            None,
        );

        let disabled_hook = service.add_hook_command(
            HookEvent::PreToolUse,
            "disabled_command".to_string(),
            None,
            None,
        );

        service.toggle_hook_enabled(HookEvent::PreToolUse, &disabled_hook.id);

        let commands = service.get_hook_commands(HookEvent::PreToolUse);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, "enabled_command");
    }

    #[test]
    fn test_execution_stats() {
        let mut service = HooksService::new();

        service.add_hook_command(HookEvent::PreToolUse, "echo".to_string(), None, None);

        let stats = service.get_execution_stats();
        assert_eq!(stats.total_executions, 0);

        let execution = HookExecution {
            id: generate_execution_id(),
            hook_id: "test_hook".to_string(),
            event: HookEvent::PreToolUse,
            tool_name: "test_tool".to_string(),
            status: HookExecutionStatus::Completed,
            started_at: chrono::Utc::now().timestamp_millis(),
            completed_at: Some(chrono::Utc::now().timestamp_millis()),
            output: Some("test output".to_string()),
            error: None,
        };

        service.record_execution(execution);

        let stats = service.get_execution_stats();
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.completed_count, 1);
    }

    #[test]
    fn test_get_hooks_by_event() {
        let mut service = HooksService::new();

        service.add_hook_command(
            HookEvent::PreToolUse,
            "pre_tool_cmd".to_string(),
            None,
            None,
        );

        service.add_hook_command(
            HookEvent::PostToolUse,
            "post_tool_cmd".to_string(),
            None,
            None,
        );

        let pre_hooks = service.get_hooks_by_event(HookEvent::PreToolUse);
        let post_hooks = service.get_hooks_by_event(HookEvent::PostToolUse);

        assert_eq!(pre_hooks.len(), 1);
        assert_eq!(pre_hooks[0].command, "pre_tool_cmd");
        assert_eq!(post_hooks.len(), 1);
        assert_eq!(post_hooks[0].command, "post_tool_cmd");
    }
}
