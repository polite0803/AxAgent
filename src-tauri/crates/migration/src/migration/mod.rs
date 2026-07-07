// SPDX-License-Identifier: AGPL-3.0-only

use axagent_harness::migration_types::{
    BackupInfo, DetectedPlatform, MigrationEntry, MigrationItem, MigrationReport,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use axagent_kit::secure_store::SecureStore;

pub(crate) fn axagent_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent")
}

pub(crate) fn openclaw_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".openclaw")
}

pub(crate) fn hermes_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".hermes")
}

pub(crate) fn timestamp_str() -> String {
    // P2-5: 追加毫秒 + UUID 后缀，避免快速连续两次备份产生同名目录
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let now = chrono::Utc::now();
    let millis = now.timestamp_millis();
    let nanos = now.timestamp_subsec_nanos();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    format!("{:013}-{:09}-{:04}-{}", millis, nanos, n & 0xFFFF, &suffix[..8])
}

/// P1-13: 进程级 migration 锁，防止并发迁移互相覆盖。
/// 通过 `migration.lock` 锁文件实现（不依赖第三方 crate）。
/// 锁文件持有者即拥有本次迁移的独占权。
pub(crate) struct MigrationLock {
    _file: fs::File,
    path: PathBuf,
}

impl MigrationLock {
    pub(crate) fn acquire() -> Result<Self, String> {
        let path = axagent_home().join("migration.lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建锁目录失败: {}", e))?;
        }
        // 用 create_new 保证只有一个进程能拿到锁
        let file =
            fs::OpenOptions::new().create_new(true).write(true).open(&path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "迁移正在进行中（锁文件 {} 已存在）。如确认无其他迁移，请删除该文件后重试",
                        path.display()
                    )
                } else {
                    format!("获取迁移锁失败: {}", e)
                }
            })?;
        Ok(Self { _file: file, path })
    }
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn make_item(
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

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
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

pub(crate) fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    Ok(())
}

pub(crate) fn migrate_file(
    src: &Path,
    dst: &Path,
    overwrite: bool,
) -> Result<MigrationEntry, MigrationEntry> {
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

pub(crate) fn migrate_dir(
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

pub(crate) fn merge_env_file(
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

    let store = axagent_kit::secure_store::CombinedSecureStore::with_default_paths();
    let is_secret = axagent_kit::secure_store::is_secret_key;
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

pub(crate) fn merge_yaml_config(
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

pub(crate) fn merge_yaml_values(
    mut base: serde_yaml::Value,
    overlay: serde_yaml::Value,
    overwrite: bool,
) -> serde_yaml::Value {
    match (&mut base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.get(&key) {
                    // P1-12: mapping 嵌套合并，递归调用
                    if existing.is_mapping() && value.is_mapping() {
                        let merged = merge_yaml_values(existing.clone(), value, overwrite);
                        base_map.insert(key, merged);
                    } else if overwrite {
                        // overwrite 模式下用 overlay 覆盖 base
                        base_map.insert(key, value);
                    } else {
                        // 非 overwrite 模式：保留 base
                        // 保留 base 中已有值，不做任何修改
                    }
                } else {
                    // base 中不存在的 key：直接插入
                    base_map.insert(key, value);
                }
            }
            base
        },
        // P1-12: 任一非 mapping 类型，overwrite=true 时用 overlay，否则保留 base
        (_, overlay) if overwrite => overlay,
        // 关键修复：避免 overlay 变成"替换为新值但返回了 base 的引用"
        (base, _) => std::mem::take(base),
    }
}

pub(crate) fn classify_entry(entry: MigrationEntry) -> ClassifiedEntry {
    if entry.reason.contains("目标已存在") {
        ClassifiedEntry::Skipped(entry)
    } else {
        ClassifiedEntry::Failed(entry)
    }
}

pub(crate) enum ClassifiedEntry {
    Skipped(MigrationEntry),
    Failed(MigrationEntry),
}

// Sub-modules
pub mod backup;
pub mod detect;
pub mod migration_ops;
pub mod preview;
pub mod rollback;

pub use backup::create_backup;
pub use detect::detect_platforms;
pub use migration_ops::{migrate_hermes, migrate_openclaw};
pub use preview::{preview_hermes, preview_openclaw};
pub use rollback::{DefaultMigrationRunner, list_backups, migrate_secrets, rollback};
