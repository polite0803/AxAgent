// SPDX-License-Identifier: AGPL-3.0-only

//! `tools` crate 跨层依赖下沉到 harness 层的 trait 集合。
//!
//! - `MigrationRunner`  — `axagent_migration` 的纯函数集合（detect/preview/migrate/backup/rollback/list）
//! - `PluginAgentProvider` — `axagent_plugins::global_plugin_agents()` 的全局只读视图
//!
//! `tools` 不再直接 import 这两个底层 crate，而是持有
//! `Arc<dyn MigrationRunner>` / `Arc<dyn PluginAgentProvider>`，
//! 由 wiring 层（runtime/gateway）注入。

use std::path::Path;

use crate::migration_types::{BackupInfo, DetectedPlatform, MigrationItem, MigrationReport};

/// 从其他 Agent 平台（OpenClaw / Hermes 等）迁移数据的能力契约。
pub trait MigrationRunner: Send + Sync {
    fn detect_platforms(&self) -> Vec<DetectedPlatform>;
    fn preview_openclaw(&self) -> Vec<MigrationItem>;
    fn preview_hermes(&self) -> Vec<MigrationItem>;
    fn create_backup(&self, platform: &str) -> Result<BackupInfo, String>;
    fn migrate_openclaw(&self, overwrite: bool) -> MigrationReport;
    fn migrate_hermes(&self, overwrite: bool) -> MigrationReport;
    fn rollback(&self, backup_path: &Path) -> Result<MigrationReport, String>;
    fn list_backups(&self) -> Vec<BackupInfo>;
}

/// 单个插件提供的 agent 完整定义。
#[derive(Debug, Clone)]
pub struct PluginAgentDescriptor {
    pub agent_type: String,
    pub description: String,
    pub tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub background: bool,
    pub system_prompt: Option<String>,
    pub source: String,
}

/// 插件提供 Agent 定义的全局只读视图。
pub trait PluginAgentProvider: Send + Sync {
    /// 返回所有已加载的插件 agent 完整定义。
    fn all(&self) -> Vec<PluginAgentDescriptor>;
}

use serde::{Deserialize, Serialize};

/// delegate_task 命令的输入参数（权威定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTaskInput {
    pub role_name: String,
    pub task: String,
    #[serde(default)]
    pub context: serde_json::Value,
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// delegate_task 命令的输出结果（权威定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTaskResult {
    pub delegation_id: String,
    pub role_name: String,
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub duration_ms: u64,
}

/// Multi-Agent 委派任务的执行契约。
///
/// wiring 层实现此 trait，通过 `set_delegate_task_runner()` 注入到 tools crate，
/// 使 `DelegateTaskTool` 无需直接依赖 Tauri 命令层或 providers crate。
#[async_trait::async_trait]
pub trait DelegateTaskRunner: Send + Sync {
    /// 委派任务给指定角色。
    async fn delegate(&self, input: DelegateTaskInput) -> Result<DelegateTaskResult, String>;
}
