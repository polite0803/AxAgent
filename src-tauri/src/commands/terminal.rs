use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusInfo {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub staged: u32,
    pub conflicted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub network_status: String,
}

#[tauri::command]
pub async fn git_get_branch() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get git branch".to_string());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}

#[tauri::command]
pub async fn git_status() -> Result<GitStatusInfo, String> {
    let branch = git_get_branch()
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    let status_output = String::from_utf8_lossy(&output.stdout);
    let mut staged = 0u32;
    let mut dirty = false;
    let mut conflicted = 0u32;

    for line in status_output.lines() {
        if line.len() < 2 {
            continue;
        }
        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');

        if index_status == 'U' || worktree_status == 'U' {
            conflicted += 1;
        } else if index_status != ' ' && index_status != '?' {
            staged += 1;
        }

        if worktree_status != ' ' && worktree_status != '?' {
            dirty = true;
        }
    }

    Ok(GitStatusInfo {
        branch,
        ahead: 0,
        behind: 0,
        dirty,
        staged,
        conflicted,
    })
}

#[tauri::command]
pub async fn system_get_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        cpu_usage: 0.0,
        memory_usage: 0.0,
        network_status: "connected".to_string(),
    })
}

#[tauri::command]
pub async fn path_complete(partial_path: String) -> Result<Vec<String>, String> {
    let mut results = Vec::new();

    let path = Path::new(&partial_path);
    let parent = if partial_path.contains('/') || partial_path.contains('\\') {
        path.parent()
    } else {
        Some(Path::new("."))
    };

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Some(parent_dir) = parent {
        if let Ok(entries) = std::fs::read_dir(parent_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase().starts_with(&file_name.to_lowercase()) {
                        let is_dir = entry_path.is_dir();
                        let display_name = if is_dir {
                            format!("{}/", name)
                        } else {
                            name.to_string()
                        };
                        results.push(display_name);
                    }
                }
            }
        }
    }

    results.sort();
    results.truncate(20);
    Ok(results)
}

#[tauri::command]
pub async fn session_get_status(_session_id: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "token_count": 0,
        "input_tokens": 0,
        "output_tokens": 0,
        "session_duration": 0,
    }))
}
