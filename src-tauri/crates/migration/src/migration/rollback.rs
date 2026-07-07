// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub fn rollback(backup_path: &Path) -> Result<MigrationReport, String> {
    // P1-13: 获取进程级 migration 锁
    let _lock = MigrationLock::acquire()?;

    let home = axagent_home();
    let backup_root = home.join("migration-backup");

    let canonical_backup = backup_path
        .canonicalize()
        .map_err(|_| format!("备份路径不存在: {}", backup_path.display()))?;
    let canonical_root = backup_root
        .canonicalize()
        .map_err(|_| format!("备份根目录不存在: {}", backup_root.display()))?;

    if !canonical_backup.starts_with(&canonical_root) {
        return Err(format!(
            "安全限制：回滚路径必须在 {} 内，实际: {}",
            backup_root.display(),
            backup_path.display()
        ));
    }

    let ts = timestamp_str();
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if !backup_path.exists() {
        return Err(format!("备份路径不存在: {}", backup_path.display()));
    }

    // P1-14: 收集备份中的所有条目；回滚前先删除目标 home 中备份不存在的条目
    // 防止"备份里没有的、迁移后新增的文件"残留
    let backup_names: HashSet<String> = match fs::read_dir(backup_path) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(e) => {
            return Err(format!("无法读取备份目录: {}", e));
        },
    };

    // 枚举 home 中的同名顶层条目（personalities/memories/skills/配置文件），
    // 删除那些不在备份中的（说明是迁移过程中新增的，需清理）
    let protected_top_level = [
        "personalities",
        "memories",
        "skills",
        "allowed-commands.json",
        ".env",
        "config.yaml",
        "cron-tasks.json",
    ];
    for name in protected_top_level {
        let path = home.join(name);
        if path.exists() && !backup_names.contains(name) {
            if path.is_dir() {
                if let Err(e) = fs::remove_dir_all(&path) {
                    failed.push(MigrationEntry {
                        source: path.display().to_string(),
                        destination: "deleted".to_string(),
                        item_type: "directory".to_string(),
                        description: format!("删除新增目录: {}", e),
                        reason: "回滚清理".to_string(),
                    });
                } else {
                    migrated.push(MigrationEntry {
                        source: path.display().to_string(),
                        destination: "deleted".to_string(),
                        item_type: "directory".to_string(),
                        description: "已删除迁移过程中新增的目录".to_string(),
                        reason: "回滚清理".to_string(),
                    });
                }
            } else if let Err(e) = fs::remove_file(&path) {
                failed.push(MigrationEntry {
                    source: path.display().to_string(),
                    destination: "deleted".to_string(),
                    item_type: "file".to_string(),
                    description: format!("删除新增文件: {}", e),
                    reason: "回滚清理".to_string(),
                });
            } else {
                migrated.push(MigrationEntry {
                    source: path.display().to_string(),
                    destination: "deleted".to_string(),
                    item_type: "file".to_string(),
                    description: "已删除迁移过程中新增的文件".to_string(),
                    reason: "回滚清理".to_string(),
                });
            }
        }
    }

    // 用备份文件覆盖/写回 home
    if let Ok(entries) = fs::read_dir(backup_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let dest = home.join(&name);

            if src_path.is_dir() {
                let (m, s, f) = migrate_dir(&src_path, &dest, true);
                migrated.extend(m);
                skipped.extend(s);
                failed.extend(f);
            } else {
                match migrate_file(&src_path, &dest, true) {
                    Ok(e) => migrated.push(e),
                    Err(e) => failed.push(e),
                }
            }
        }
    }

    Ok(MigrationReport {
        platform: "rollback".to_string(),
        timestamp: ts,
        migrated,
        skipped,
        failed,
    })
}

pub fn list_backups() -> Vec<BackupInfo> {
    let backup_root = axagent_home().join("migration-backup");
    let mut backups = Vec::new();

    if !backup_root.exists() {
        return backups;
    }

    if let Ok(entries) = fs::read_dir(&backup_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let ts = entry.file_name().to_string_lossy().to_string();
                let mut items = Vec::new();
                if let Ok(dir_entries) = fs::read_dir(&path) {
                    for de in dir_entries.filter_map(|e| e.ok()) {
                        items.push(de.file_name().to_string_lossy().to_string());
                    }
                }
                backups.push(BackupInfo {
                    backup_path: path,
                    timestamp: ts,
                    items_backed_up: items,
                });
            }
        }
    }

    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    backups
}

pub fn migrate_secrets(secrets: HashMap<String, String>) -> Vec<(String, Result<(), String>)> {
    let store = axagent_kit::secure_store::CombinedSecureStore::with_default_paths();
    axagent_kit::secure_store::migrate_secrets(&store, secrets)
}

// ── `axagent_harness::MigrationRunner` trait impl ──
//
// 把原来模块顶层的 8 个 free function 包成 trait impl，让 `tools` crate
// 不用直接 import `axagent_migration`，改为持有
// `Arc<dyn axagent_harness::MigrationRunner>`，由 wiring 层注入。

pub struct DefaultMigrationRunner;

impl axagent_harness::MigrationRunner for DefaultMigrationRunner {
    fn detect_platforms(&self) -> Vec<DetectedPlatform> {
        super::detect::detect_platforms()
    }
    fn preview_openclaw(&self) -> Vec<MigrationItem> {
        super::preview::preview_openclaw()
    }
    fn preview_hermes(&self) -> Vec<MigrationItem> {
        super::preview::preview_hermes()
    }
    fn create_backup(&self, platform: &str) -> Result<BackupInfo, String> {
        super::backup::create_backup(platform)
    }
    fn migrate_openclaw(&self, overwrite: bool) -> MigrationReport {
        super::migration_ops::migrate_openclaw(overwrite)
    }
    fn migrate_hermes(&self, overwrite: bool) -> MigrationReport {
        super::migration_ops::migrate_hermes(overwrite)
    }
    fn rollback(&self, backup_path: &Path) -> Result<MigrationReport, String> {
        super::rollback::rollback(backup_path)
    }
    fn list_backups(&self) -> Vec<BackupInfo> {
        super::rollback::list_backups()
    }
}
