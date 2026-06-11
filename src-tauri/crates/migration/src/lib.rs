// SPDX-License-Identifier: AGPL-3.0-only

pub use axagent_core::ddl::run_initialization;
pub use axagent_harness::migration_types::{
    BackupInfo, DetectedPlatform, MigrationEntry, MigrationItem, MigrationReport,
};

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use axagent_core::secure_store::SecureStore;

fn axagent_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
}

fn openclaw_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
}

fn hermes_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
}

fn timestamp_str() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

pub fn detect_platforms() -> Vec<DetectedPlatform> {
    let mut platforms = Vec::new();

    let oc = openclaw_home();
    if oc.exists() && oc.is_dir() {
        let skill_dir = oc.join("skills");
        let skill_count = if skill_dir.exists() {
            fs::read_dir(&skill_dir)
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        platforms.push(DetectedPlatform {
            name: "OpenClaw".to_string(),
            base_path: oc.clone(),
            has_soul: oc.join("SOUL.md").exists(),
            has_memory: oc.join("MEMORY.md").exists(),
            has_skills: skill_dir.exists() && skill_count > 0,
            has_config: oc.join("config.yaml").exists() || oc.join("config.yml").exists(),
            has_env: oc.join(".env").exists(),
            has_cron: false,
            has_personalities: false,
            skill_count,
            memory_count: 0,
        });
    }

    let hm = hermes_home();
    if hm.exists() && hm.is_dir() {
        let skill_dir = hm.join("skills");
        let skill_count = if skill_dir.exists() {
            fs::read_dir(&skill_dir)
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let mem_dir = hm.join("memories");
        let memory_count = if mem_dir.exists() {
            fs::read_dir(&mem_dir)
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let personalities_dir = hm.join("personalities");
        platforms.push(DetectedPlatform {
            name: "Hermes".to_string(),
            base_path: hm.clone(),
            has_soul: false,
            has_memory: mem_dir.exists() && memory_count > 0,
            has_skills: skill_dir.exists() && skill_count > 0,
            has_config: hm.join("config.yaml").exists(),
            has_env: false,
            has_cron: hm.join("cron-tasks.json").exists(),
            has_personalities: personalities_dir.exists(),
            skill_count,
            memory_count,
        });
    }

    platforms
}

fn make_item(
    source: PathBuf,
    destination: PathBuf,
    item_type: &str,
    description: String,
) -> MigrationItem {
    let exists = destination.exists();
    MigrationItem {
        source,
        destination,
        item_type: item_type.to_string(),
        description,
        exists_at_dest: exists,
    }
}

pub fn preview_openclaw() -> Vec<MigrationItem> {
    let oc = openclaw_home();
    let home = axagent_home();
    let mut items = Vec::new();

    if oc.join("SOUL.md").exists() {
        items.push(make_item(
            oc.join("SOUL.md"),
            home.join("personalities")
                .join("openclaw-import")
                .join("SOUL.md"),
            "personality",
            "SOUL.md → personalities/openclaw-import/SOUL.md".to_string(),
        ));
    }

    if oc.join("MEMORY.md").exists() {
        items.push(make_item(
            oc.join("MEMORY.md"),
            home.join("memories").join("openclaw-import.md"),
            "memory",
            "MEMORY.md → memories/openclaw-import.md".to_string(),
        ));
    }

    let skill_dir = oc.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("skills").join("openclaw-imports").join(&name),
                    "skill",
                    format!("skills/{} → skills/openclaw-imports/{}", name, name),
                ));
            }
        }
    }

    let allowlist = oc.join("allowed-commands.json");
    if allowlist.exists() {
        items.push(make_item(
            allowlist,
            home.join("allowed-commands.json"),
            "allowlist",
            "allowed-commands.json → allowed-commands.json".to_string(),
        ));
    }

    let env_file = oc.join(".env");
    if env_file.exists() {
        items.push(make_item(
            env_file,
            home.join(".env"),
            "env",
            ".env → .env (API keys, merged)".to_string(),
        ));
    }

    items
}

pub fn preview_hermes() -> Vec<MigrationItem> {
    let hm = hermes_home();
    let home = axagent_home();
    let mut items = Vec::new();

    let skill_dir = hm.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("skills").join("hermes-imports").join(&name),
                    "skill",
                    format!("skills/{} → skills/hermes-imports/{}", name, name),
                ));
            }
        }
    }

    let mem_dir = hm.join("memories");
    if mem_dir.exists()
        && let Ok(entries) = fs::read_dir(&mem_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("memories").join(&name),
                    "memory",
                    format!("memories/{} → memories/{}", name, name),
                ));
            }
        }
    }

    let config = hm.join("config.yaml");
    if config.exists() {
        items.push(make_item(
            config,
            home.join("config.yaml"),
            "config",
            "config.yaml → config.yaml (merged)".to_string(),
        ));
    }

    let cron = hm.join("cron-tasks.json");
    if cron.exists() {
        items.push(make_item(
            cron,
            home.join("cron-tasks.json"),
            "cron",
            "cron-tasks.json → cron-tasks.json".to_string(),
        ));
    }

    let personalities_dir = hm.join("personalities");
    if personalities_dir.exists()
        && let Ok(entries) = fs::read_dir(&personalities_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("personalities").join(&name),
                    "personality",
                    format!("personalities/{} → personalities/{}", name, name),
                ));
            }
        }
    }

    items
}

pub fn create_backup(_platform: &str) -> Result<BackupInfo, String> {
    let home = axagent_home();
    let ts = timestamp_str();
    let backup_dir = home.join("migration-backup").join(&ts);

    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let mut items_backed_up = Vec::new();

    let dirs_to_backup = [
        home.join("personalities"),
        home.join("memories"),
        home.join("skills"),
    ];
    let files_to_backup = [
        home.join("allowed-commands.json"),
        home.join(".env"),
        home.join("config.yaml"),
        home.join("cron-tasks.json"),
    ];

    for dir in &dirs_to_backup {
        if dir.exists() {
            let dir_name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let dest = backup_dir.join(&dir_name);
            copy_dir_recursive(dir, &dest)?;
            items_backed_up.push(dir_name);
        }
    }

    for file in &files_to_backup {
        if file.exists() {
            let file_name = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let dest = backup_dir.join(&file_name);
            fs::copy(file, &dest).map_err(|e| format!("Failed to backup {}: {}", file_name, e))?;
            items_backed_up.push(file_name);
        }
    }

    Ok(BackupInfo {
        backup_path: backup_dir,
        timestamp: ts,
        items_backed_up,
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| {
                    format!("Failed to copy {} → {}: {}", src_path.display(), dst_path.display(), e)
                })?;
            }
        }
    }
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    Ok(())
}

fn migrate_file(src: &Path, dst: &Path, overwrite: bool) -> Result<MigrationEntry, MigrationEntry> {
    let src_str = src.display().to_string();
    let dst_str = dst.display().to_string();
    let desc = format!("{} → {}", src_str, dst_str);

    if dst.exists() && !overwrite {
        return Err(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "file".to_string(),
            description: desc,
            reason: "目标已存在，跳过（使用 overwrite 覆盖）".to_string(),
        });
    }

    if let Err(e) = ensure_parent(dst) {
        return Err(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "file".to_string(),
            description: format!("{} → {}", src.display(), dst.display()),
            reason: format!("创建目标目录失败: {}", e),
        });
    }

    match fs::copy(src, dst) {
        Ok(_) => Ok(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "file".to_string(),
            description: desc,
            reason: "已迁移".to_string(),
        }),
        Err(e) => Err(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "file".to_string(),
            description: desc,
            reason: format!("复制失败: {}", e),
        }),
    }
}

fn migrate_dir(
    src: &Path,
    dst: &Path,
    overwrite: bool,
) -> (Vec<MigrationEntry>, Vec<MigrationEntry>, Vec<MigrationEntry>) {
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                let (m, s, f) = migrate_dir(&src_path, &dst_path, overwrite);
                migrated.extend(m);
                skipped.extend(s);
                failed.extend(f);
            } else {
                match migrate_file(&src_path, &dst_path, overwrite) {
                    Ok(entry) => migrated.push(entry),
                    Err(entry) => {
                        if entry.reason.contains("目标已存在") {
                            skipped.push(entry);
                        } else {
                            failed.push(entry);
                        }
                    },
                }
            }
        }
    }

    (migrated, skipped, failed)
}

fn merge_env_file(
    src: &Path,
    dst: &Path,
    overwrite: bool,
) -> Result<MigrationEntry, MigrationEntry> {
    let src_str = src.display().to_string();
    let dst_str = dst.display().to_string();
    let desc = format!("{} → {} (merged)", src_str, dst_str);

    let src_content = fs::read_to_string(src).map_err(|e| MigrationEntry {
        source: src_str.clone(),
        destination: dst_str.clone(),
        item_type: "env".to_string(),
        description: desc.clone(),
        reason: format!("读取源文件失败: {}", e),
    })?;

    let mut existing_keys = HashSet::new();
    let mut existing_lines = Vec::new();
    if dst.exists() {
        let dst_content = fs::read_to_string(dst).unwrap_or_default();
        for line in dst_content.lines() {
            existing_lines.push(line.to_string());
            if let Some(key) = line.split('=').next()
                && !line.starts_with('#')
                && !key.trim().is_empty()
            {
                existing_keys.insert(key.trim().to_string());
            }
        }
    }

    let mut new_lines = Vec::new();
    for line in src_content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            new_lines.push(line.to_string());
            continue;
        }
        if let Some(key) = line.split('=').next() {
            let key = key.trim().to_string();
            if existing_keys.contains(&key) && !overwrite {
                continue;
            }
            if existing_keys.contains(&key) {
                existing_lines.retain(|l| {
                    if let Some(k) = l.split('=').next() {
                        k.trim() != key
                    } else {
                        true
                    }
                });
            }
            existing_keys.insert(key);
        }
        new_lines.push(line.to_string());
    }

    if let Err(e) = ensure_parent(dst) {
        return Err(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "env".to_string(),
            description: desc,
            reason: format!("创建目标目录失败: {}", e),
        });
    }

    let mut all_lines = existing_lines;
    if !all_lines.is_empty() && !all_lines.last().unwrap().is_empty() {
        all_lines.push(String::new());
    }
    all_lines.extend(new_lines);

    let store = axagent_core::secure_store::CombinedSecureStore::with_default_paths();
    let is_secret = axagent_core::secure_store::is_secret_key;
    let mut non_secret_lines = Vec::new();
    let mut secret_count = 0usize;

    for line in all_lines {
        let line_is_secret = if let Some(key_part) = line.split('=').next() {
            let key_trimmed = key_part.trim();
            !line.starts_with('#') && !key_trimmed.is_empty() && is_secret(key_trimmed)
        } else {
            false
        };

        if line_is_secret {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if let Err(e) = store.store_secret(key, value) {
                    tracing::warn!("Failed to store secret '{}' securely: {}", key, e);
                    non_secret_lines.push(line);
                } else {
                    secret_count += 1;
                }
            }
        } else {
            non_secret_lines.push(line);
        }
    }

    fs::write(dst, non_secret_lines.join("\n")).map_err(|e| MigrationEntry {
        source: src_str,
        destination: dst_str,
        item_type: "env".to_string(),
        description: format!("{} → {} (merged)", src.display(), dst.display()),
        reason: format!("写入失败: {}", e),
    })?;

    let reason = if secret_count > 0 {
        format!("已合并 ({} 个密钥已安全存储)", secret_count)
    } else {
        "已合并".to_string()
    };

    Ok(MigrationEntry {
        source: src.display().to_string(),
        destination: dst.display().to_string(),
        item_type: "env".to_string(),
        description: format!("{} → {} (merged)", src.display(), dst.display()),
        reason,
    })
}

fn merge_yaml_config(
    src: &Path,
    dst: &Path,
    overwrite: bool,
) -> Result<MigrationEntry, MigrationEntry> {
    let src_str = src.display().to_string();
    let dst_str = dst.display().to_string();
    let desc = format!("{} → {} (merged)", src_str, dst_str);

    let src_content = fs::read_to_string(src).map_err(|e| MigrationEntry {
        source: src_str.clone(),
        destination: dst_str.clone(),
        item_type: "config".to_string(),
        description: desc.clone(),
        reason: format!("读取源文件失败: {}", e),
    })?;

    let src_yaml: serde_yaml::Value =
        serde_yaml::from_str(&src_content).unwrap_or(serde_yaml::Value::Null);

    let dst_yaml = if dst.exists() {
        let dst_content = fs::read_to_string(dst).unwrap_or_default();
        serde_yaml::from_str(&dst_content).unwrap_or(serde_yaml::Value::Null)
    } else {
        serde_yaml::Value::Null
    };

    let merged = merge_yaml_values(dst_yaml, src_yaml, overwrite);

    if let Err(e) = ensure_parent(dst) {
        return Err(MigrationEntry {
            source: src_str,
            destination: dst_str,
            item_type: "config".to_string(),
            description: desc,
            reason: format!("创建目标目录失败: {}", e),
        });
    }

    let output = serde_yaml::to_string(&merged).unwrap_or_default();
    fs::write(dst, output).map_err(|e| MigrationEntry {
        source: src_str,
        destination: dst_str,
        item_type: "config".to_string(),
        description: format!("{} → {} (merged)", src.display(), dst.display()),
        reason: format!("写入失败: {}", e),
    })?;

    Ok(MigrationEntry {
        source: src.display().to_string(),
        destination: dst.display().to_string(),
        item_type: "config".to_string(),
        description: format!("{} → {} (merged)", src.display(), dst.display()),
        reason: "已合并".to_string(),
    })
}

fn merge_yaml_values(
    mut base: serde_yaml::Value,
    overlay: serde_yaml::Value,
    overwrite: bool,
) -> serde_yaml::Value {
    match (&mut base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.get(&key) {
                    if existing.is_mapping() && value.is_mapping() {
                        let merged = merge_yaml_values(existing.clone(), value, overwrite);
                        base_map.insert(key, merged);
                    } else if overwrite {
                        base_map.insert(key, value);
                    }
                } else {
                    base_map.insert(key, value);
                }
            }
            base
        },
        (_, overlay) if overwrite => overlay,
        (base, _) => std::mem::take(base),
    }
}

fn classify_entry(entry: MigrationEntry) -> ClassifiedEntry {
    if entry.reason.contains("目标已存在") {
        ClassifiedEntry::Skipped(entry)
    } else {
        ClassifiedEntry::Failed(entry)
    }
}

enum ClassifiedEntry {
    Skipped(MigrationEntry),
    Failed(MigrationEntry),
}

pub fn migrate_openclaw(overwrite: bool) -> MigrationReport {
    let oc = openclaw_home();
    let home = axagent_home();
    let ts = timestamp_str();
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if oc.join("SOUL.md").exists() {
        let dest = home
            .join("personalities")
            .join("openclaw-import")
            .join("SOUL.md");
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

    MigrationReport {
        platform: "OpenClaw".to_string(),
        timestamp: ts,
        migrated,
        skipped,
        failed,
    }
}

pub fn migrate_hermes(overwrite: bool) -> MigrationReport {
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

    MigrationReport {
        platform: "Hermes".to_string(),
        timestamp: ts,
        migrated,
        skipped,
        failed,
    }
}

pub fn rollback(backup_path: &Path) -> Result<MigrationReport, String> {
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
    let store = axagent_core::secure_store::CombinedSecureStore::with_default_paths();
    axagent_core::secure_store::migrate_secrets(&store, secrets)
}

// ── `axagent_harness::MigrationRunner` trait impl ──
//
// 把原来模块顶层的 8 个 free function 包成 trait impl，让 `tools` crate
// 不用直接 import `axagent_migration`，改为持有
// `Arc<dyn axagent_harness::MigrationRunner>`，由 wiring 层注入。

pub struct DefaultMigrationRunner;

impl axagent_harness::MigrationRunner for DefaultMigrationRunner {
    fn detect_platforms(&self) -> Vec<DetectedPlatform> {
        detect_platforms()
    }
    fn preview_openclaw(&self) -> Vec<MigrationItem> {
        preview_openclaw()
    }
    fn preview_hermes(&self) -> Vec<MigrationItem> {
        preview_hermes()
    }
    fn create_backup(&self, platform: &str) -> Result<BackupInfo, String> {
        create_backup(platform)
    }
    fn migrate_openclaw(&self, overwrite: bool) -> MigrationReport {
        migrate_openclaw(overwrite)
    }
    fn migrate_hermes(&self, overwrite: bool) -> MigrationReport {
        migrate_hermes(overwrite)
    }
    fn rollback(&self, backup_path: &Path) -> Result<MigrationReport, String> {
        rollback(backup_path)
    }
    fn list_backups(&self) -> Vec<BackupInfo> {
        list_backups()
    }
}
