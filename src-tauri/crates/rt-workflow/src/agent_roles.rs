// SPDX-License-Identifier: AGPL-3.0-only

//! Agent Role System - Defines agent archetypes and their capabilities
//!
//! Lookup priority: 1) DB `agent_roles` table → 2) config file (YAML/JSON).
//! Custom roles imported from Open Agent Spec or other sources are stored in the DB
//! and take precedence over file-defined roles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

/// Resolved role data from DB or file registry
#[derive(Debug, Clone)]
pub struct ResolvedRole {
    pub name: String,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub active_domains: Vec<String>,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
    pub source: String,
}

/// DB accessor —— 经 harness `AgentRoleRepository` 抽象，不直接依赖 axagent-entities。
pub mod db_access {
    use axagent_harness::repo_dtos::AgentRoleDto;

    pub struct AgentRoleRow {
        pub name: String,
        pub system_prompt: String,
        pub default_tools: Vec<String>,
        pub active_domains: Vec<String>,
        pub max_concurrent: i32,
        pub timeout_seconds: i64,
        pub source: String,
    }

    pub async fn get_role_from_db(role_id: &str) -> Result<Option<AgentRoleRow>, String> {
        let dto: Option<AgentRoleDto> =
            axagent_harness::repositories::agent_role_repository().get_agent_role(role_id).await?;
        Ok(dto.map(|r| AgentRoleRow {
            name: r.name,
            system_prompt: r.system_prompt,
            default_tools: r.default_tools,
            active_domains: r.active_domains,
            max_concurrent: r.max_concurrent,
            timeout_seconds: r.timeout_seconds,
            source: r.source,
        }))
    }
}

use db_access::get_role_from_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub role_name: String,
    pub enabled: bool,
    pub custom_prompt: Option<String>,
    pub custom_tools: Option<Vec<String>>,
    pub custom_max_concurrent: Option<usize>,
    pub custom_timeout_seconds: Option<u64>,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            role_name: "executor".to_string(),
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
        self.custom_prompt.clone().unwrap_or_else(|| self.role_name.clone())
    }

    pub fn effective_tools(&self) -> Vec<String> {
        self.custom_tools.clone().unwrap_or_default()
    }

    pub fn effective_max_concurrent(&self) -> usize {
        self.custom_max_concurrent.unwrap_or(3)
    }

    pub fn effective_timeout_seconds(&self) -> u64 {
        self.custom_timeout_seconds.unwrap_or(300)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoleRegistry {
    roles: HashMap<String, RoleConfig>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self { roles: HashMap::new() }
    }

    pub fn register(&mut self, config: RoleConfig) {
        self.roles.insert(config.role_name.clone(), config);
    }

    pub fn get(&self, role_name: &str) -> Option<&RoleConfig> {
        self.roles.get(role_name)
    }

    pub fn is_enabled(&self, role_name: &str) -> bool {
        self.roles.get(role_name).map(|c| c.enabled).unwrap_or(true)
    }
}

// ── 配置文件驱动的角色定义（扩展性核心） ──

/// 从外部配置文件（YAML 或 JSON）加载的角色定义。
/// 字段语义与 RoleConfig 对齐，但角色名是字符串（支持自定义角色名）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRoleDefinition {
    /// 角色名（如 "financial_auditor"）
    pub name: String,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// System prompt 模板
    pub system_prompt: String,
    /// 允许的工具列表
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 该角色自动激活的工具功能域（如 ["core", "general", "invest"]）
    #[serde(default)]
    pub active_domains: Vec<String>,
    /// 最大并发数
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// 超时秒数
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// 来源标注（如 "file:roles.yaml"）
    #[serde(default)]
    pub source: String,
}

fn default_enabled() -> bool {
    true
}
fn default_max_concurrent() -> usize {
    3
}
fn default_timeout_seconds() -> u64 {
    300
}

/// 角色配置文件顶层结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfigFile {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub roles: Vec<FileRoleDefinition>,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for RoleConfigFile {
    fn default() -> Self {
        Self { schema_version: 1, roles: Vec::new() }
    }
}

/// 从文件加载的自定义角色注册表（线程安全）。
#[derive(Debug, Default)]
pub struct FileRoleRegistry {
    roles: RwLock<HashMap<String, FileRoleDefinition>>,
}

impl FileRoleRegistry {
    pub fn new() -> Self {
        Self { roles: RwLock::new(HashMap::new()) }
    }

    /// 从 YAML 或 JSON 文件加载角色定义。
    /// 支持 .yaml / .yml / .json 扩展名。
    pub fn load_from_file(&self, path: &Path) -> Result<usize, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("无法读取角色配置文件 {}: {}", path.display(), e))?;

        let config: RoleConfigFile = if path.extension().is_some_and(|e| e == "yaml" || e == "yml")
        {
            serde_yaml::from_str(&content)
                .map_err(|e| format!("YAML 解析失败 {}: {}", path.display(), e))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| format!("JSON 解析失败 {}: {}", path.display(), e))?
        };

        let count = config.roles.len();
        let source = format!("file:{}", path.display());
        let mut map = self.roles.write().map_err(|e| format!("Lock poisoned: {}", e))?;

        for mut role in config.roles {
            if role.source.is_empty() {
                role.source = source.clone();
            }
            map.insert(role.name.clone(), role);
        }

        Ok(count)
    }

    /// 按角色名查找文件定义的 ResolvedRole。
    pub fn resolve(&self, role_name: &str) -> Option<ResolvedRole> {
        let map = self.roles.read().ok()?;
        map.get(role_name).map(|r| ResolvedRole {
            name: r.name.clone(),
            system_prompt: r.system_prompt.clone(),
            default_tools: r.allowed_tools.clone(),
            active_domains: r.active_domains.clone(),
            max_concurrent: r.max_concurrent,
            timeout_seconds: r.timeout_seconds,
            source: r.source.clone(),
        })
    }

    /// 列出所有文件角色的名称。
    pub fn role_names(&self) -> Vec<String> {
        self.roles.read().map(|map| map.keys().cloned().collect()).unwrap_or_default()
    }

    /// 列出所有文件角色定义。
    pub fn all_roles(&self) -> Vec<FileRoleDefinition> {
        self.roles.read().map(|map| map.values().cloned().collect()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub role_name: String,
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
    pub fn new(role_name: impl Into<String>) -> Self {
        Self {
            role_name: role_name.into(),
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

/// DB-first role resolver: check DB then file registry.
pub async fn resolve(role_name: &str) -> Option<ResolvedRole> {
    if let Ok(Some(row)) = get_role_from_db(role_name).await {
        return Some(ResolvedRole {
            name: row.name,
            system_prompt: row.system_prompt,
            default_tools: row.default_tools,
            active_domains: row.active_domains,
            max_concurrent: row.max_concurrent as usize,
            timeout_seconds: row.timeout_seconds as u64,
            source: row.source,
        });
    }
    None
}

/// Three-level lookup: DB -> file registry -> None (no more built-in enum).
pub async fn resolve_with_file_registry(
    file_registry: &FileRoleRegistry,
    role_name: &str,
) -> Option<ResolvedRole> {
    if let Ok(Some(row)) = get_role_from_db(role_name).await {
        return Some(ResolvedRole {
            name: row.name,
            system_prompt: row.system_prompt,
            default_tools: row.default_tools,
            active_domains: row.active_domains,
            max_concurrent: row.max_concurrent as usize,
            timeout_seconds: row.timeout_seconds as u64,
            source: row.source,
        });
    }
    file_registry.resolve(role_name)
}
