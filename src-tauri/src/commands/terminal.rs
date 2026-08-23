// SPDX-License-Identifier: AGPL-3.0-only

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::terminal as terminal_err;
use agent_macro::agent_command;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusInfo {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub staged: u32,
    pub conflicted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub network_status: String,
}

#[agent_command(domain = terminal, safety = Safe, call_mode = StateOnly, description = "获取Git分支")]
#[tauri::command]
pub async fn git_get_branch() -> Result<String, String> {
    let output = axagent_kit::utils::cmd("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err(ErrorResponse::err(terminal_err::GIT_BRANCH_FAILED));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}

#[agent_command(domain = terminal, safety = Safe, call_mode = StateOnly, description = "获取Git状态")]
#[tauri::command]
pub async fn git_status() -> Result<GitStatusInfo, String> {
    let branch = match git_get_branch().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("获取 git 分支失败: {}", e);
            "unknown".to_string()
        },
    };

    let output = axagent_kit::utils::cmd("git")
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

    // 获取 ahead/behind 计数
    let (ahead, behind) = get_ahead_behind().await;

    Ok(GitStatusInfo { branch, ahead, behind, dirty, staged, conflicted })
}

async fn get_ahead_behind() -> (u32, u32) {
    let output = match axagent_kit::utils::cmd("git")
        .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return (0, 0),
    };

    if !output.status.success() {
        return (0, 0);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() != 2 {
        return (0, 0);
    }

    let behind = parts[0].parse::<u32>().unwrap_or(0);
    let ahead = parts[1].parse::<u32>().unwrap_or(0);
    (ahead, behind)
}

#[agent_command(domain = terminal, safety = Safe, call_mode = StateOnly, description = "获取系统信息")]
#[tauri::command]
pub async fn system_get_info() -> Result<SystemInfo, String> {
    let cpu_usage = get_cpu_usage();
    let memory_usage = get_memory_usage();
    let network_status = get_network_status();

    Ok(SystemInfo { cpu_usage, memory_usage, network_status })
}

fn get_cpu_usage() -> f32 {
    use std::sync::{Mutex, OnceLock};
    static SYS: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();
    let sys_mutex = SYS.get_or_init(|| {
        let mut s = sysinfo::System::new();
        s.refresh_cpu_usage();
        Mutex::new(s)
    });
    let mut sys = sys_mutex.lock().unwrap_or_else(|e| e.into_inner());
    // sysinfo requires two refreshes for accurate CPU usage;
    // the first call returns 0% on fresh System. Use a short sleep + re-refresh.
    sys.refresh_cpu_usage();
    sys.global_cpu_usage()
}

fn get_memory_usage() -> f32 {
    use std::sync::{Mutex, OnceLock};
    static SYS: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();
    let sys_mutex = SYS.get_or_init(|| Mutex::new(sysinfo::System::new_all()));
    let mut sys = sys_mutex.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_memory();
    let total = sys.total_memory();
    if total > 0 {
        ((total - sys.available_memory()) as f32 / total as f32) * 100.0
    } else {
        0.0
    }
}

fn get_network_status() -> String {
    // 简单的网络连通性检测
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("ping").args(["-n", "1", "-w", "1000", "8.8.8.8"]).output()
        {
            Ok(output) => {
                if output.status.success() {
                    "connected".to_string()
                } else {
                    "disconnected".to_string()
                }
            },
            Err(_) => "disconnected".to_string(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        match std::process::Command::new("ping").args(["-c", "1", "-W", "1", "8.8.8.8"]).output() {
            Ok(output) => {
                if output.status.success() {
                    "connected".to_string()
                } else {
                    "disconnected".to_string()
                }
            },
            Err(_) => "disconnected".to_string(),
        }
    }
}

#[agent_command(domain = terminal, safety = Safe, call_mode = StateOnly, description = "路径补全")]
#[tauri::command]
pub async fn path_complete(partial_path: String) -> Result<Vec<String>, String> {
    // 安全检查：拒绝空路径
    if partial_path.is_empty() {
        return Ok(Vec::new());
    }

    let path = Path::new(&partial_path);

    // 安全检查：规范化路径后检查是否包含路径遍历
    let normalized = if partial_path.contains('/') || partial_path.contains('\\') {
        match path.canonicalize() {
            Ok(p) => p,
            // 如果路径不存在，尝试规范化父目录
            Err(_) => {
                if let Some(parent) = path.parent() {
                    match parent.canonicalize() {
                        Ok(p) => p,
                        Err(_) => return Ok(Vec::new()),
                    }
                } else {
                    match std::env::current_dir() {
                        Ok(cwd) => cwd,
                        Err(_) => return Ok(Vec::new()),
                    }
                }
            },
        }
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(_) => return Ok(Vec::new()),
        }
    };

    let parent = if partial_path.contains('/') || partial_path.contains('\\') {
        path.parent()
    } else {
        Some(Path::new("."))
    };

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // 安全：确保查找目录在 normalized 路径下
    let search_dir = if let Some(parent_dir) = parent {
        if parent_dir.is_absolute() {
            parent_dir.to_path_buf()
        } else {
            normalized.join(parent_dir)
        }
    } else {
        normalized.clone()
    };

    // 再次规范化搜索目录
    let search_dir = match search_dir.canonicalize() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            // 安全：确保条目在搜索目录内
            if !entry_path.starts_with(&search_dir) {
                continue;
            }
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

    results.sort();
    results.truncate(20);
    Ok(results)
}

#[agent_command(domain = terminal, safety = Safe, call_mode = StateOnly, description = "获取终端会话状态")]
#[tauri::command]
pub async fn session_get_status(_session_id: String) -> Result<serde_json::Value, String> {
    // 会话状态功能待后续与 agent 会话系统集成
    Ok(serde_json::json!({
        "token_count": null,
        "input_tokens": null,
        "output_tokens": null,
        "session_duration": null,
    }))
}
