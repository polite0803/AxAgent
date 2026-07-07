// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use std::fs;

pub fn create_backup(_platform: &str) -> Result<BackupInfo, String> {
    let home = axagent_home();
    let ts = timestamp_str();
    let backup_dir = home.join("migration-backup").join(&ts);

    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let mut items_backed_up = Vec::new();

    let dirs_to_backup = [home.join("personalities"), home.join("memories"), home.join("skills")];
    let files_to_backup = [
        home.join("allowed-commands.json"),
        home.join(".env"),
        home.join("config.yaml"),
        home.join("cron-tasks.json"),
    ];

    for dir in &dirs_to_backup {
        if dir.exists() {
            let dir_name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            let dest = backup_dir.join(&dir_name);
            copy_dir_recursive(dir, &dest)?;
            items_backed_up.push(dir_name);
        }
    }

    for file in &files_to_backup {
        if file.exists() {
            let file_name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            let dest = backup_dir.join(&file_name);
            fs::copy(file, &dest).map_err(|e| format!("Failed to backup {}: {}", file_name, e))?;
            items_backed_up.push(file_name);
        }
    }

    Ok(BackupInfo { backup_path: backup_dir, timestamp: ts, items_backed_up })
}
