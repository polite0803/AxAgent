//! Agent Role System - Defines agent archetypes and their capabilities
//!
//! DB-first lookup: checks `agent_roles` table first, falls back to built-in enum.
//! Custom roles imported from Open Agent Spec or other sources are stored in the DB
//! and take precedence over the hardcoded 8 variants.

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 8 built-in role variants — used as enum fallback.
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
            AgentRole::Coordinator => "You are a coordinator agent responsible for task decomposition, worker assignment, and result synthesis. Think carefully about task dependencies and optimal execution order. You excel at breaking complex problems into manageable sub-tasks and coordinating multiple agents to work in parallel.",
            AgentRole::Researcher => "You are a research agent specialized in gathering information, analyzing data, and providing comprehensive research findings. Use web search, document analysis, and reasoning tools. Your strength is deep investigation and thorough analysis.",
            AgentRole::Developer => "You are a developer agent focused on writing, editing, and refactoring code. Use terminal, file operations, and git tools to accomplish development tasks. You follow best practices and write clean, maintainable code.",
            AgentRole::Reviewer => "You are a reviewer agent responsible for evaluating work quality, providing constructive feedback, and ensuring standards are met. Check code correctness, style, security, and adherence to requirements. Be thorough but constructive.",
            AgentRole::Browser => "You are a browser agent specialized in interacting with web pages, filling forms, and verifying visual content. Use browser automation tools. Your strength is precise UI interaction and data extraction from web sources.",
            AgentRole::Synthesizer => "You are a synthesizer agent responsible for aggregating results from multiple agents into a unified, coherent output. Combine findings, resolve conflicts, and present clear conclusions. Excel at condensing complex information.",
            AgentRole::Planner => "You are a planner agent focused on strategic thinking, risk assessment, and timeline planning. Analyze requirements, identify dependencies, estimate effort, and create actionable plans. Think several steps ahead.",
            AgentRole::Executor => "You are an executor agent responsible for carrying out discrete tasks with precision. Follow instructions carefully, report progress clearly, and handle errors gracefully. Reliable and detail-oriented.",
        }
    }

    pub fn default_tools(&self) -> Vec<&'static str> {
        match self {
            AgentRole::Coordinator => vec!["web_search","read_file","list_directory","search_files","grep_content","skill_manage","session_search","memory_flush","get_system_info","get_storage_info","list_storage_files"],
            AgentRole::Researcher => vec!["web_search","fetch_url","fetch_markdown","read_file","list_directory","search_files","grep_content","search_knowledge","list_knowledge_bases","session_search","list_storage_files","download_storage_file"],
            AgentRole::Developer => vec!["write_file","edit_file","search_replace","read_file","list_directory","search_files","grep_content","run_command","file_exists","get_file_info","create_directory","delete_file","move_file","get_system_info","list_processes","get_storage_info","list_storage_files","upload_storage_file","download_storage_file","delete_storage_file","git_status","git_diff","git_commit","git_log","git_branch","git_review"],
            AgentRole::Reviewer => vec!["read_file","list_directory","search_files","grep_content","run_command","file_exists","get_file_info","get_system_info","list_processes","git_status","git_diff","git_log","git_review"],
            AgentRole::Browser => vec!["fetch_url","fetch_markdown","web_search"],
            AgentRole::Synthesizer => vec!["write_file","read_file","list_directory","search_files","grep_content"],
            AgentRole::Planner => vec!["read_file","list_directory","search_files","grep_content","web_search","session_search","memory_flush","get_system_info","get_storage_info","list_storage_files"],
            AgentRole::Executor => vec!["run_command","write_file","edit_file","read_file","list_directory","search_files","grep_content","create_directory","delete_file","move_file","file_exists","get_system_info","list_processes","upload_storage_file","download_storage_file","delete_storage_file"],
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

impl AgentRole {
    /// DB-first role resolver: look up `agent_roles` table, fall back to enum.
    pub async fn resolve(
        db: &DatabaseConnection,
        role_name: &str,
    ) -> Option<ResolvedRole> {
        if let Ok(Some(row)) = get_role_from_db(db, role_name).await {
            return Some(ResolvedRole {
                name: row.name,
                system_prompt: if row.system_prompt.is_empty() {
                    Self::from_str_opt(role_name)
                        .map(|r| r.system_prompt().to_string())
                        .unwrap_or_default()
                } else {
                    row.system_prompt
                },
                default_tools: row.default_tools,
                max_concurrent: row.max_concurrent as usize,
                timeout_seconds: row.timeout_seconds as u64,
                source: row.source,
            });
        }
        Self::from_str_opt(role_name).map(|r| ResolvedRole {
            name: role_name.to_string(),
            system_prompt: r.system_prompt().to_string(),
            default_tools: r.default_tools().iter().map(|s| s.to_string()).collect(),
            max_concurrent: r.max_concurrent(),
            timeout_seconds: r.timeout_seconds(),
            source: "builtin".to_string(),
        })
    }
}

/// Resolved role data from DB or enum
#[derive(Debug, Clone)]
pub struct ResolvedRole {
    pub name: String,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
    pub source: String,
}

/// DB accessor
pub mod db_access {
    use sea_orm::{DatabaseConnection, EntityTrait};

    pub struct AgentRoleRow {
        pub name: String,
        pub system_prompt: String,
        pub default_tools: Vec<String>,
        pub max_concurrent: i32,
        pub timeout_seconds: i64,
        pub source: String,
    }

    pub async fn get_role_from_db(
        db: &DatabaseConnection,
        role_id: &str,
    ) -> Result<Option<AgentRoleRow>, sea_orm::DbErr> {
        use axagent_core::entity::agent_roles;
        let row = agent_roles::Entity::find_by_id(role_id).one(db).await?;
        Ok(row.map(|r| {
            let tools: Vec<String> = r
                .default_tools
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            AgentRoleRow {
                name: r.name,
                system_prompt: r.system_prompt,
                default_tools: tools,
                max_concurrent: r.max_concurrent,
                timeout_seconds: r.timeout_seconds,
                source: r.source,
            }
        }))
    }
}

use db_access::get_role_from_db;

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
    pub custom_prompt: Option<String>,
    pub custom_tools: Option<Vec<String>>,
    pub custom_max_concurrent: Option<usize>,
    pub custom_timeout_seconds: Option<u64>,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::Executor,
            enabled: true,
            custom_prompt: None,
            custom_tools: None,
            custom_max_concurrent: None,
            custom_timeout_seconds: None,
        }
    }
}

impl RoleConfig {
    pub fn effective_system_prompt(&self) -> String {
        self.custom_prompt
            .clone()
            .unwrap_or_else(|| self.role.system_prompt().to_string())
    }

    pub fn effective_tools(&self) -> Vec<String> {
        self.custom_tools.clone().unwrap_or_else(|| {
            self.role.default_tools().iter().map(|s| s.to_string()).collect()
        })
    }

    pub fn effective_max_concurrent(&self) -> usize {
        self.custom_max_concurrent.unwrap_or_else(|| self.role.max_concurrent())
    }

    pub fn effective_timeout_seconds(&self) -> u64 {
        self.custom_timeout_seconds.unwrap_or_else(|| self.role.timeout_seconds())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRegistry {
    roles: HashMap<AgentRole, RoleConfig>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }

    pub fn register(&mut self, config: RoleConfig) {
        self.roles.insert(config.role, config);
    }

    pub fn get(&self, role: &AgentRole) -> Option<&RoleConfig> {
        self.roles.get(role)
    }

    pub fn is_enabled(&self, role: &AgentRole) -> bool {
        self.roles.get(role).map(|c| c.enabled).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub role: AgentRole,
    pub current_task: Option<String>,
    pub status: AgentStatus,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Error,
}

impl AgentInfo {
    pub fn new(role: AgentRole) -> Self {
        Self {
            role,
            current_task: None,
            status: AgentStatus::Idle,
            completed_tasks: 0,
            failed_tasks: 0,
        }
    }

    pub fn start_task(&mut self, task: String) {
        self.current_task = Some(task);
        self.status = AgentStatus::Running;
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
