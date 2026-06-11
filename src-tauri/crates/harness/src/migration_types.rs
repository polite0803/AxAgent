// SPDX-License-Identifier: AGPL-3.0-only

//! 跨平台 Agent 数据迁移 DTO 集合。
//!
//! 定义在 harness 层（最低层），让 `MigrationRunner` trait 也能
//! 落在 harness，而 `migration` crate 通过 `pub use` 重导出使用。

use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectedPlatform {
    pub name: String,
    pub base_path: PathBuf,
    pub has_soul: bool,
    pub has_memory: bool,
    pub has_skills: bool,
    pub has_config: bool,
    pub has_env: bool,
    pub has_cron: bool,
    pub has_personalities: bool,
    pub skill_count: usize,
    pub memory_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MigrationItem {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub item_type: String,
    pub description: String,
    pub exists_at_dest: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MigrationReport {
    pub platform: String,
    pub timestamp: String,
    pub migrated: Vec<MigrationEntry>,
    pub skipped: Vec<MigrationEntry>,
    pub failed: Vec<MigrationEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MigrationEntry {
    pub source: String,
    pub destination: String,
    pub item_type: String,
    pub description: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupInfo {
    pub backup_path: PathBuf,
    pub timestamp: String,
    pub items_backed_up: Vec<String>,
}
