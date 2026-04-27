use std::collections::HashMap;

use axagent_core::builtin_tools_registry::{
    get_all_builtin_tools_flat, get_handler, FlatBuiltinTool,
};
use axagent_core::repo::local_tool;
use axagent_core::types::{ChatTool, ChatToolFunction};
use sea_orm::DatabaseConnection;
use serde_json::Value;

// ── Data models ────────────────────────────────────────────────────────

/// Definition of a single local tool.
#[derive(Debug, Clone)]
pub struct LocalToolDef {
    pub group_id: String,
    pub group_name: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
    pub env_json: Option<String>,
    pub timeout_secs: Option<i32>,
}

/// A group of local tools (for UI display and enable/disable control).
#[derive(Debug, Clone)]
pub struct LocalToolGroup {
    pub group_id: String,
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<LocalToolDef>,
}

// ── LocalToolRegistry ──────────────────────────────────────────────────

/// Registry of local (builtin) tools that are executed directly without MCP.
///
/// Tools are loaded from `builtin_tools_registry` static definitions.
/// Enable/disable state is persisted in the `settings` table via `repo::local_tool`.
pub struct LocalToolRegistry {
    /// group_id → enabled
    enabled_map: HashMap<String, bool>,
    /// tool_name → LocalToolDef (full registry)
    tool_defs: HashMap<String, LocalToolDef>,
    /// group_id → Vec<tool_name>
    group_tools: HashMap<String, Vec<String>>,
    /// group_id → group_name
    group_names: HashMap<String, String>,
}

impl LocalToolRegistry {
    /// Initialize from builtin_tools_registry static definitions.
    /// All tools are enabled by default; call `load_enabled_state()` to
    /// read persisted state from the database.
    pub fn init_from_registry() -> Self {
        let mut tool_defs: HashMap<String, LocalToolDef> = HashMap::new();
        let mut group_tools: HashMap<String, Vec<String>> = HashMap::new();
        let mut group_names: HashMap<String, String> = HashMap::new();

        // Load from the flat list which includes both static and dynamic tools
        let flat_tools: Vec<FlatBuiltinTool> = get_all_builtin_tools_flat();

        for ft in flat_tools {
            let tool_name = ft.tool_name.clone();
            group_names
                .entry(ft.server_id.clone())
                .or_insert_with(|| ft.server_name.clone());
            group_tools
                .entry(ft.server_id.clone())
                .or_default()
                .push(tool_name.clone());

            tool_defs.insert(
                tool_name,
                LocalToolDef {
                    group_id: ft.server_id,
                    group_name: ft.server_name,
                    tool_name: ft.tool_name,
                    description: ft.description,
                    input_schema: ft.input_schema,
                    env_json: ft.env_json,
                    timeout_secs: ft.timeout_secs,
                },
            );
        }

        // Default: all groups enabled
        let enabled_map: HashMap<String, bool> =
            group_tools.keys().map(|gid| (gid.clone(), true)).collect();

        Self {
            enabled_map,
            tool_defs,
            group_tools,
            group_names,
        }
    }

    /// Load enable/disable state from the database.
    pub async fn load_enabled_state(&mut self, db: &DatabaseConnection) {
        for group_id in self.group_tools.keys() {
            let default = local_tool::get_default_enabled(group_id);
            let enabled = local_tool::get_enabled(db, group_id, default).await;
            self.enabled_map.insert(group_id.clone(), enabled);
        }
    }

    /// Execute a local tool directly (bypassing MCP entirely).
    ///
    /// Returns the tool output string on success, or an error message on failure.
    pub async fn execute(&self, tool_name: &str, input: Value) -> Result<String, String> {
        // Check if tool exists
        let def = self
            .tool_defs
            .get(tool_name)
            .ok_or_else(|| format!("Unknown local tool: {}", tool_name))?;

        // Check if tool is enabled
        if !self.is_enabled(tool_name) {
            return Err(format!("Tool '{}' is disabled", tool_name));
        }

        // Merge env_json into arguments (env_json takes precedence)
        let mut merged_args = input;
        if let Some(env_str) = &def.env_json {
            if let Ok(Value::Object(env_map)) = serde_json::from_str::<Value>(env_str) {
                if let Value::Object(args_map) = &mut merged_args {
                    for (k, v) in env_map {
                        args_map.insert(k, v);
                    }
                }
            }
        }

        // Look up the handler directly from builtin_tools_registry
        let handler = get_handler(&def.group_name, tool_name)
            .ok_or_else(|| format!("No handler registered for {}/{}", def.group_name, tool_name))?;

        // Execute with timeout
        let timeout_secs = def.timeout_secs.unwrap_or(30) as u64;
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        let result = tokio::time::timeout(timeout_duration, handler(merged_args))
            .await
            .map_err(|_| format!("Tool '{}' timed out after {}s", tool_name, timeout_secs))?
            .map_err(|e| e.to_string())?;

        if result.is_error {
            Err(result.content)
        } else {
            Ok(result.content)
        }
    }

    /// Check whether a tool is enabled.
    pub fn is_enabled(&self, tool_name: &str) -> bool {
        self.get_group_id(tool_name)
            .map(|gid| self.enabled_map.get(gid).copied().unwrap_or(true))
            .unwrap_or(false)
    }

    /// Check whether a tool is registered in the local registry.
    pub fn contains(&self, tool_name: &str) -> bool {
        self.tool_defs.contains_key(tool_name)
    }

    /// Get the group_id for a tool, if it exists.
    pub fn get_group_id(&self, tool_name: &str) -> Option<&str> {
        self.tool_defs.get(tool_name).map(|d| d.group_id.as_str())
    }

    /// Get ChatTool definitions for all enabled tools (for injection into LLM).
    pub fn get_enabled_chat_tools(&self) -> Vec<ChatTool> {
        let mut tools = Vec::new();
        for (tool_name, def) in &self.tool_defs {
            if !self.is_enabled(tool_name) {
                continue;
            }
            tools.push(ChatTool {
                r#type: "function".to_string(),
                function: ChatToolFunction {
                    name: tool_name.clone(),
                    description: Some(def.description.clone()),
                    parameters: Some(def.input_schema.clone()),
                },
            });
        }
        tools
    }

    /// Get all tool groups with their enable/disable state (for UI display).
    pub fn get_tool_groups(&self) -> Vec<LocalToolGroup> {
        let mut groups = Vec::new();
        for (group_id, tool_names) in &self.group_tools {
            let group_name = self.group_names.get(group_id).cloned().unwrap_or_default();
            let enabled = self.enabled_map.get(group_id).copied().unwrap_or(true);
            let tools: Vec<LocalToolDef> = tool_names
                .iter()
                .filter_map(|tn| self.tool_defs.get(tn).cloned())
                .collect();
            groups.push(LocalToolGroup {
                group_id: group_id.clone(),
                group_name,
                enabled,
                tools,
            });
        }
        groups
    }

    /// Toggle the enable/disable state of a tool group and persist to DB.
    /// Returns the new enabled state.
    pub async fn toggle_group(
        &mut self,
        db: &DatabaseConnection,
        group_id: &str,
    ) -> Result<bool, String> {
        let current = self.enabled_map.get(group_id).copied().unwrap_or(true);
        let new_state = !current;
        local_tool::set_enabled(db, group_id, new_state)
            .await
            .map_err(|e| e.to_string())?;
        self.enabled_map.insert(group_id.to_string(), new_state);
        Ok(new_state)
    }

    /// Dynamically set env_json for a specific tool (e.g. web_search with API key).
    pub fn set_env_json(&mut self, tool_name: &str, env_json: String) {
        if let Some(def) = self.tool_defs.get_mut(tool_name) {
            def.env_json = Some(env_json);
        }
    }

    /// Dynamically set timeout for a specific tool.
    pub fn set_timeout_secs(&mut self, tool_name: &str, timeout_secs: i32) {
        if let Some(def) = self.tool_defs.get_mut(tool_name) {
            def.timeout_secs = Some(timeout_secs);
        }
    }

    /// Get all registered tool names (regardless of enabled state).
    pub fn all_tool_names(&self) -> Vec<String> {
        self.tool_defs.keys().cloned().collect()
    }

    /// Get enabled tool names only.
    pub fn enabled_tool_names(&self) -> Vec<String> {
        self.tool_defs
            .keys()
            .filter(|name| self.is_enabled(name))
            .cloned()
            .collect()
    }

    /// Get all registered tool definitions (regardless of enabled state).
    pub fn all_tool_defs(&self) -> &HashMap<String, LocalToolDef> {
        &self.tool_defs
    }
}
