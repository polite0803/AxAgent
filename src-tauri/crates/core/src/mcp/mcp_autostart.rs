//! MCP 自动发现 — 从 settings.json 扫描 mcpServers 并在启动时自动启动
//!
//! 扫描来源（按优先级）：
//! 1. `~/.axagent/settings.json` → mcpServers
//! 2. `<project>/.axagent/settings.json` → mcpServers
//! 3. `<project>/.axagent/settings.local.json` → mcpServers

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// 已启动的 MCP 服务器进程
#[derive(Debug)]
pub struct McpServerProcess {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub child: Option<Child>,
}

/// MCP 自动发现结果
#[derive(Debug, Default)]
pub struct McpAutoStartResult {
    pub started: Vec<McpServerProcess>,
    pub failed: Vec<(String, String)>, // (name, error)
    pub skipped: Vec<String>,          // 已禁用的服务器名称
}

/// 从 settings.json 扫描 mcpServers 配置并自动启动
///
/// 返回启动结果，包含成功/失败/跳过的服务器列表。
pub fn discover_and_start() -> McpAutoStartResult {
    let mut result = McpAutoStartResult::default();

    // 收集所有 settings 文件中的 mcpServers
    let servers = collect_mcp_servers();
    if servers.is_empty() {
        return result;
    }

    for (name, config) in servers {
        let enabled = config
            .get("enabled")
            .is_none_or(|v| v != "false" && v != "0");

        if !enabled {
            result.skipped.push(name);
            continue;
        }

        let command = match config.get("command") {
            Some(cmd) => cmd.clone(),
            None => {
                result.failed.push((name, "缺少 command 字段".into()));
                continue;
            },
        };

        let args: Vec<String> = config
            .get("args")
            .map(|a| serde_json::from_str(a).unwrap_or_default())
            .unwrap_or_default();

        let env: BTreeMap<String, String> = config
            .get("env")
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or_default();

        match start_mcp_process(&command, &args, &env) {
            Ok(child) => {
                tracing::info!("[MCP] 已启动服务器: {} ({})", name, command);
                result.started.push(McpServerProcess {
                    name,
                    command,
                    args,
                    child: Some(child),
                });
            },
            Err(e) => {
                tracing::warn!("[MCP] 启动失败: {} ({})", name, e);
                result.failed.push((name, e));
            },
        }
    }

    result
}

/// 启动单个 MCP 进程
fn start_mcp_process(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> Result<Child, String> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in env {
        cmd.env(key, value);
    }

    cmd.spawn().map_err(|e| format!("无法启动进程: {}", e))
}

/// 收集所有配置文件中的 mcpServers
fn collect_mcp_servers() -> BTreeMap<String, BTreeMap<String, String>> {
    let mut all_servers = BTreeMap::new();
    let config_paths = discover_config_paths();

    for path in config_paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mcp_servers) = root.get("mcpServers").and_then(|v| v.as_object()) {
                    for (name, config_value) in mcp_servers {
                        // 后面的配置覆盖前面的（项目 > 用户）
                        if let Some(config) = parse_mcp_server_config(config_value) {
                            all_servers.insert(name.clone(), config);
                        }
                    }
                }
            }
        }
    }

    all_servers
}

/// 解析单个 MCP 服务器配置为键值对
fn parse_mcp_server_config(value: &serde_json::Value) -> Option<BTreeMap<String, String>> {
    let obj = value.as_object()?;
    let mut config = BTreeMap::new();

    if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
        config.insert("command".into(), cmd.to_string());
    }
    if let Some(args) = obj.get("args") {
        if let Ok(args_str) = serde_json::to_string(args) {
            config.insert("args".into(), args_str);
        }
    }
    if let Some(env) = obj.get("env") {
        if let Ok(env_str) = serde_json::to_string(env) {
            config.insert("env".into(), env_str);
        }
    }
    if let Some(enabled) = obj.get("enabled").and_then(|v| v.as_bool()) {
        config.insert("enabled".into(), enabled.to_string());
    }

    if config.contains_key("command") {
        Some(config)
    } else {
        None
    }
}

/// 发现所有可能的 settings 文件路径
fn discover_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // 用户级配置
    paths.push(home.join(".axagent").join("settings.json"));

    // 项目级配置
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".axagent").join("settings.json"));
        paths.push(cwd.join(".axagent").join("settings.local.json"));
    }

    paths
}

/// 停止所有已启动的 MCP 服务器进程
pub fn stop_all(processes: &mut [McpServerProcess]) {
    for proc in processes.iter_mut() {
        if let Some(ref mut child) = proc.child {
            tracing::info!("[MCP] 停止服务器: {}", proc.name);
            let _ = child.kill();
            let _ = child.wait();
            proc.child = None;
        }
    }
}
