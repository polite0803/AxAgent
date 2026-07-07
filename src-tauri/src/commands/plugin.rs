// SPDX-License-Identifier: AGPL-3.0-only

use tauri::{State, command};
use tracing::warn;

use crate::app_state::AppState;

#[command]
pub async fn plugin_list(state: State<'_, AppState>) -> Result<Vec<PluginSummaryDto>, String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let manager = plugin_manager.blocking_read();
        manager
            .list_plugins()
            .map(|plugins| {
                plugins
                    .into_iter()
                    .map(|p| PluginSummaryDto {
                        id: p.metadata.id,
                        name: p.metadata.name,
                        version: p.metadata.version,
                        description: p.metadata.description,
                        kind: p.metadata.kind.to_string(),
                        enabled: p.enabled,
                        tools: p.tool_names,
                        mcp_servers: p.mcp_server_names,
                        skills: p.skill_names,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("plugin list task panicked: {e}"))?
}

#[command]
pub async fn plugin_validate_source(
    state: State<'_, AppState>,
    source: String,
) -> Result<PluginManifestDto, String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let manager = plugin_manager.blocking_read();
        let manifest = manager.validate_plugin_source(&source).map_err(|e| e.to_string())?;
        Ok(PluginManifestDto {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            permissions: manifest.permissions.iter().map(|p| p.as_str().to_string()).collect(),
            default_enabled: manifest.default_enabled,
            hooks: {
                let mut hooks = serde_json::Map::new();
                hooks.insert(
                    "PreToolUse".to_string(),
                    serde_json::Value::Array(
                        manifest
                            .hooks
                            .pre_tool_use
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
                hooks.insert(
                    "PostToolUse".to_string(),
                    serde_json::Value::Array(
                        manifest
                            .hooks
                            .post_tool_use
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
                hooks
            },
            tools: manifest
                .tools
                .iter()
                .map(|t| ToolDto { name: t.name.clone(), description: t.description.clone() })
                .collect(),
            mcp_servers: manifest
                .mcp_servers
                .iter()
                .map(|m| McpServerDto { name: m.name.clone(), command: m.command.clone() })
                .collect(),
            skills: manifest
                .skills
                .iter()
                .map(|s| SkillDto { name: s.name.clone(), path: s.path.clone() })
                .collect(),
        })
    })
    .await
    .map_err(|e| format!("plugin validate task panicked: {e}"))?
}

/// SECURITY (S9): 远程插件源（Git URL、npm 包）安装时无 SHA-256 完整性校验或签名验证。
/// 用户应从可信源（如官方 AxHub 市场）安装插件，避免安装来源不明的远程插件。
/// 前端应在安装前通过 `plugin_validate_source` 验证插件元数据。
#[command]
pub async fn plugin_install(
    state: State<'_, AppState>,
    source: String,
) -> Result<InstallOutcomeDto, String> {
    // 安全日志：记录远程插件源安装（无完整性校验）
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || source.starts_with('@')
    {
        warn!(
            "SECURITY: Installing plugin from remote source without integrity verification: {}",
            source
        );
    }
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = plugin_manager.blocking_write();
        let outcome = manager.install(&source).map_err(|e| e.to_string())?;
        Ok(InstallOutcomeDto {
            plugin_id: outcome.plugin_id,
            version: outcome.version,
            install_path: outcome.install_path.display().to_string(),
        })
    })
    .await
    .map_err(|e| format!("plugin install task panicked: {e}"))?
}

#[command]
pub async fn plugin_enable(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = plugin_manager.blocking_write();
        manager.enable(&plugin_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("plugin enable task panicked: {e}"))?
}

#[command]
pub async fn plugin_disable(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = plugin_manager.blocking_write();
        manager.disable(&plugin_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("plugin disable task panicked: {e}"))?
}

#[command]
pub async fn plugin_uninstall(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = plugin_manager.blocking_write();
        manager.uninstall(&plugin_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("plugin uninstall task panicked: {e}"))?
}

#[command]
pub async fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<UpdateOutcomeDto, String> {
    let plugin_manager = state.plugin_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = plugin_manager.blocking_write();
        let outcome = manager.update(&plugin_id).map_err(|e| e.to_string())?;
        Ok(UpdateOutcomeDto {
            plugin_id: outcome.plugin_id,
            old_version: outcome.old_version,
            new_version: outcome.new_version,
            install_path: outcome.install_path.display().to_string(),
        })
    })
    .await
    .map_err(|e| format!("plugin update task panicked: {e}"))?
}

#[derive(Debug, serde::Serialize)]
pub struct PluginSummaryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub kind: String,
    pub enabled: bool,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct PluginManifestDto {
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub default_enabled: bool,
    pub hooks: serde_json::Map<String, serde_json::Value>,
    pub tools: Vec<ToolDto>,
    pub mcp_servers: Vec<McpServerDto>,
    pub skills: Vec<SkillDto>,
}

#[derive(Debug, serde::Serialize)]
pub struct ToolDto {
    pub name: String,
    pub description: String,
}

#[derive(Debug, serde::Serialize)]
pub struct McpServerDto {
    pub name: String,
    pub command: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SkillDto {
    pub name: String,
    pub path: String,
}

#[derive(Debug, serde::Serialize)]
pub struct InstallOutcomeDto {
    pub plugin_id: String,
    pub version: String,
    pub install_path: String,
}

#[derive(Debug, serde::Serialize)]
pub struct UpdateOutcomeDto {
    pub plugin_id: String,
    pub old_version: String,
    pub new_version: String,
    pub install_path: String,
}
