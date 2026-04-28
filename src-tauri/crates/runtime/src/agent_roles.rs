//! Agent Role System - Defines agent archetypes and their capabilities
//!
//! Integrated into the agent execution path:
//! - `AgentSession::role` is set via `session.with_role(role)` in `agent_query`
//! - Role determines the system prompt, allowed tools, and concurrency limits
//! - Tools are filtered based on role's `default_tools()` in `agent_query`
//!
//! Permission control is handled by `PermissionMode` + `PermissionPolicy` in
//! addition to role-based tool filtering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Coordinator,
    Researcher,
    Developer,
    Reviewer,
    Browser,
    Synthesizer,
    Planner,
    Executor,
}

impl AgentRole {
    /// Parse a role from a string (case-insensitive). Returns None for unknown roles.
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "coordinator" => Some(AgentRole::Coordinator),
            "researcher" => Some(AgentRole::Researcher),
            "developer" => Some(AgentRole::Developer),
            "reviewer" => Some(AgentRole::Reviewer),
            "browser" => Some(AgentRole::Browser),
            "synthesizer" => Some(AgentRole::Synthesizer),
            "planner" => Some(AgentRole::Planner),
            "executor" => Some(AgentRole::Executor),
            _ => None,
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::Coordinator => {
                "You are a coordinator agent responsible for task decomposition, worker assignment, and result synthesis. Think carefully about task dependencies and optimal execution order. You excel at breaking complex problems into manageable sub-tasks and coordinating multiple agents to work in parallel."
            }
            AgentRole::Researcher => {
                "You are a research agent specialized in gathering information, analyzing data, and providing comprehensive research findings. Use web search, document analysis, and reasoning tools. Your strength is deep investigation and thorough analysis."
            }
            AgentRole::Developer => {
                "You are a developer agent focused on writing, editing, and refactoring code. Use terminal, file operations, and git tools to accomplish development tasks. You follow best practices and write clean, maintainable code."
            }
            AgentRole::Reviewer => {
                "You are a reviewer agent responsible for evaluating work quality, providing constructive feedback, and ensuring standards are met. Check code correctness, style, security, and adherence to requirements. Be thorough but constructive."
            }
            AgentRole::Browser => {
                "You are a browser agent specialized in interacting with web pages, filling forms, and verifying visual content. Use browser automation tools. Your strength is precise UI interaction and data extraction from web sources."
            }
            AgentRole::Synthesizer => {
                "You are a synthesizer agent responsible for aggregating results from multiple agents into a unified, coherent output. Combine findings, resolve conflicts, and present clear conclusions. Excel at condensing complex information."
            }
            AgentRole::Planner => {
                "You are a planner agent focused on strategic thinking, risk assessment, and timeline planning. Analyze requirements, identify dependencies, estimate effort, and create actionable plans. Think several steps ahead."
            }
            AgentRole::Executor => {
                "You are an executor agent responsible for carrying out discrete tasks with precision. Follow instructions carefully, report progress clearly, and handle errors gracefully. Reliable and detail-oriented."
            }
        }
    }

    pub fn default_tools(&self) -> Vec<&'static str> {
        match self {
            AgentRole::Coordinator => vec![
                "web_search",
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "skill_manage",
                "session_search",
                "memory_flush",
                "get_system_info",
                "get_storage_info",
                "list_storage_files",
            ],
            AgentRole::Researcher => vec![
                "web_search",
                "fetch_url",
                "fetch_markdown",
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "search_knowledge",
                "list_knowledge_bases",
                "session_search",
                "list_storage_files",
                "download_storage_file",
            ],
            AgentRole::Developer => vec![
                "write_file",
                "edit_file",
                "search_replace",
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "run_command",
                "file_exists",
                "get_file_info",
                "create_directory",
                "delete_file",
                "move_file",
                "get_system_info",
                "list_processes",
                "get_storage_info",
                "list_storage_files",
                "upload_storage_file",
                "download_storage_file",
                "delete_storage_file",
                "git_status",
                "git_diff",
                "git_commit",
                "git_log",
                "git_branch",
                "git_review",
            ],
            AgentRole::Reviewer => vec![
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "run_command",
                "file_exists",
                "get_file_info",
                "get_system_info",
                "list_processes",
                "git_status",
                "git_diff",
                "git_log",
                "git_review",
            ],
            AgentRole::Browser => vec!["fetch_url", "fetch_markdown", "web_search"],
            AgentRole::Synthesizer => vec![
                "write_file",
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
            ],
            AgentRole::Planner => vec![
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "web_search",
                "session_search",
                "memory_flush",
                "get_system_info",
                "get_storage_info",
                "list_storage_files",
            ],
            AgentRole::Executor => vec![
                "run_command",
                "write_file",
                "edit_file",
                "read_file",
                "list_directory",
                "search_files",
                "grep_content",
                "create_directory",
                "delete_file",
                "move_file",
                "file_exists",
                "get_system_info",
                "list_processes",
                "upload_storage_file",
                "download_storage_file",
                "delete_storage_file",
            ],
        }
    }

    pub fn max_concurrent(&self) -> usize {
        match self {
            AgentRole::Coordinator => 1,
            AgentRole::Researcher => 4,
            AgentRole::Developer => 3,
            AgentRole::Reviewer => 2,
            AgentRole::Browser => 3,
            AgentRole::Synthesizer => 1,
            AgentRole::Planner => 2,
            AgentRole::Executor => 5,
        }
    }

    pub fn timeout_seconds(&self) -> u64 {
        match self {
            AgentRole::Coordinator => 300,
            AgentRole::Researcher => 600,
            AgentRole::Developer => 900,
            AgentRole::Reviewer => 600,
            AgentRole::Browser => 300,
            AgentRole::Synthesizer => 180,
            AgentRole::Planner => 300,
            AgentRole::Executor => 600,
        }
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coordinator => write!(f, "coordinator"),
            Self::Researcher => write!(f, "researcher"),
            Self::Developer => write!(f, "developer"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Browser => write!(f, "browser"),
            Self::Synthesizer => write!(f, "synthesizer"),
            Self::Planner => write!(f, "planner"),
            Self::Executor => write!(f, "executor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub role: AgentRole,
    pub enabled: bool,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
    pub allowed_tools: Vec<String>,
    pub custom_prompt: Option<String>,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::Executor,
            enabled: true,
            max_concurrent: RoleConfig::default_max_concurrent_for_role(&AgentRole::Executor),
            timeout_seconds: AgentRole::Executor.timeout_seconds(),
            allowed_tools: AgentRole::Executor
                .default_tools()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            custom_prompt: None,
        }
    }
}

impl RoleConfig {
    fn default_max_concurrent_for_role(role: &AgentRole) -> usize {
        role.max_concurrent()
    }

    pub fn for_role(role: AgentRole) -> Self {
        Self {
            role,
            enabled: true,
            max_concurrent: role.max_concurrent(),
            timeout_seconds: role.timeout_seconds(),
            allowed_tools: role.default_tools().iter().map(|s| s.to_string()).collect(),
            custom_prompt: None,
        }
    }

    pub fn with_custom_prompt(mut self, prompt: &str) -> Self {
        self.custom_prompt = Some(prompt.to_string());
        self
    }

    pub fn effective_prompt(&self) -> String {
        if let Some(ref custom) = self.custom_prompt {
            custom.clone()
        } else {
            self.role.system_prompt().to_string()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RoleRegistry {
    roles: HashMap<AgentRole, RoleConfig>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_default_roles();
        registry
    }

    fn register_default_roles(&mut self) {
        for role in [
            AgentRole::Coordinator,
            AgentRole::Researcher,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Browser,
            AgentRole::Synthesizer,
            AgentRole::Planner,
            AgentRole::Executor,
        ] {
            self.roles.insert(role, RoleConfig::for_role(role));
        }
    }

    pub fn get(&self, role: AgentRole) -> Option<&RoleConfig> {
        self.roles.get(&role)
    }

    pub fn register(&mut self, config: RoleConfig) {
        self.roles.insert(config.role, config);
    }

    pub fn list_enabled(&self) -> Vec<&RoleConfig> {
        self.roles.values().filter(|r| r.enabled).collect()
    }

    pub fn disable(&mut self, role: AgentRole) {
        if let Some(config) = self.roles.get_mut(&role) {
            config.enabled = false;
        }
    }

    pub fn enable(&mut self, role: AgentRole) {
        if let Some(config) = self.roles.get_mut(&role) {
            config.enabled = true;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Busy,
    Error,
    Offline,
}

impl AgentInfo {
    pub fn new(id: &str, name: &str, role: AgentRole) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            role,
            capabilities: role.default_tools().iter().map(|s| s.to_string()).collect(),
            status: AgentStatus::Idle,
            current_task: None,
            completed_tasks: 0,
            failed_tasks: 0,
        }
    }

    pub fn assign_task(&mut self, task_id: &str) {
        self.current_task = Some(task_id.to_string());
        self.status = AgentStatus::Busy;
    }

    pub fn complete_task(&mut self) {
        self.current_task = None;
        self.status = AgentStatus::Idle;
        self.completed_tasks += 1;
    }

    pub fn fail_task(&mut self) {
        self.current_task = None;
        self.status = AgentStatus::Error;
        self.failed_tasks += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.completed_tasks + self.failed_tasks;
        if total == 0 {
            return 0.0;
        }
        self.completed_tasks as f64 / total as f64
    }
}
