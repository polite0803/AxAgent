// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use std::fs;

pub fn migrate_openclaw(overwrite: bool) -> MigrationReport {
    // P1-13: 获取进程级 migration 锁
    let _lock = match MigrationLock::acquire() {
        Ok(l) => l,
        Err(e) => {
            return MigrationReport {
                platform: "OpenClaw".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                migrated: Vec::new(),
                skipped: Vec::new(),
                failed: vec![MigrationEntry {
                    source: "OpenClaw".to_string(),
                    destination: "lock".to_string(),
                    item_type: "platform".to_string(),
                    description: e,
                    reason: "并发锁冲突".to_string(),
                }],
            };
        },
    };

    let oc = openclaw_home();
    let home = axagent_home();
    let ts = timestamp_str();
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if oc.join("SOUL.md").exists() {
        let dest = home.join("personalities").join("openclaw-import").join("SOUL.md");
        match migrate_file(&oc.join("SOUL.md"), &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    if oc.join("MEMORY.md").exists() {
        let dest = home.join("memories").join("openclaw-import.md");
        match migrate_file(&oc.join("MEMORY.md"), &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    let skill_dir = oc.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            if src_path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let dest = home.join("skills").join("openclaw-imports").join(&name);
                let (m, s, f) = migrate_dir(&src_path, &dest, overwrite);
                migrated.extend(m);
                skipped.extend(s);
                failed.extend(f);
            }
        }
    }

    let allowlist = oc.join("allowed-commands.json");
    if allowlist.exists() {
        let dest = home.join("allowed-commands.json");
        match migrate_file(&allowlist, &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    let env_file = oc.join(".env");
    if env_file.exists() {
        let dest = home.join(".env");
        match merge_env_file(&env_file, &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    MigrationReport { platform: "OpenClaw".to_string(), timestamp: ts, migrated, skipped, failed }
}

pub fn migrate_hermes(overwrite: bool) -> MigrationReport {
    // P1-13: 获取进程级 migration 锁
    let _lock = match MigrationLock::acquire() {
        Ok(l) => l,
        Err(e) => {
            return MigrationReport {
                platform: "Hermes".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                migrated: Vec::new(),
                skipped: Vec::new(),
                failed: vec![MigrationEntry {
                    source: "Hermes".to_string(),
                    destination: "lock".to_string(),
                    item_type: "platform".to_string(),
                    description: e,
                    reason: "并发锁冲突".to_string(),
                }],
            };
        },
    };

    let hm = hermes_home();
    let home = axagent_home();
    let ts = timestamp_str();
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    let skill_dir = hm.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            if src_path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let dest = home.join("skills").join("hermes-imports").join(&name);
                let (m, s, f) = migrate_dir(&src_path, &dest, overwrite);
                migrated.extend(m);
                skipped.extend(s);
                failed.extend(f);
            }
        }
    }

    let mem_dir = hm.join("memories");
    if mem_dir.exists()
        && let Ok(entries) = fs::read_dir(&mem_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            if src_path.is_file() && src_path.extension().is_some_and(|ext| ext == "md") {
                let name = entry.file_name().to_string_lossy().to_string();
                let dest = home.join("memories").join(&name);
                match migrate_file(&src_path, &dest, overwrite) {
                    Ok(e) => migrated.push(e),
                    Err(e) => match classify_entry(e) {
                        ClassifiedEntry::Skipped(e) => skipped.push(e),
                        ClassifiedEntry::Failed(e) => failed.push(e),
                    },
                }
            }
        }
    }

    let config = hm.join("config.yaml");
    if config.exists() {
        let dest = home.join("config.yaml");
        match merge_yaml_config(&config, &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    let cron = hm.join("cron-tasks.json");
    if cron.exists() {
        let dest = home.join("cron-tasks.json");
        match migrate_file(&cron, &dest, overwrite) {
            Ok(e) => migrated.push(e),
            Err(e) => match classify_entry(e) {
                ClassifiedEntry::Skipped(e) => skipped.push(e),
                ClassifiedEntry::Failed(e) => failed.push(e),
            },
        }
    }

    let personalities_dir = hm.join("personalities");
    if personalities_dir.exists()
        && let Ok(entries) = fs::read_dir(&personalities_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            if src_path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let dest = home.join("personalities").join(&name);
                let (m, s, f) = migrate_dir(&src_path, &dest, overwrite);
                migrated.extend(m);
                skipped.extend(s);
                failed.extend(f);
            }
        }
    }

    MigrationReport { platform: "Hermes".to_string(), timestamp: ts, migrated, skipped, failed }
}
