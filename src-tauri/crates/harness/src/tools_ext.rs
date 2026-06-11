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
