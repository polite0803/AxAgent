use tauri::{State, command};

use crate::app_state::AppState;

/// 列出已安装插件
#[command]
pub fn plugin_list(state: State<'_, AppState>) -> Result<Vec<PluginSummaryDto>, String> {
    let manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
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
}

/// 验证插件源（安装前预览清单）
#[command]
pub fn plugin_validate_source(
    state: State<'_, AppState>,
    source: String,
) -> Result<PluginManifestDto, String> {
    let manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let manifest = manager
        .validate_plugin_source(&source)
        .map_err(|e| e.to_string())?;
    Ok(PluginManifestDto {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        permissions: manifest
            .permissions
            .iter()
            .map(|p| p.as_str().to_string())
            .collect(),
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
            .map(|t| ToolDto {
                name: t.name.clone(),
                description: t.description.clone(),
            })
            .collect(),
        mcp_servers: manifest
            .mcp_servers
            .iter()
            .map(|m| McpServerDto {
                name: m.name.clone(),
                command: m.command.clone(),
            })
            .collect(),
        skills: manifest
            .skills
            .iter()
            .map(|s| SkillDto {
                name: s.name.clone(),
                path: s.path.clone(),
            })
            .collect(),
    })
}

/// 安装插件（同步命令，Tauri 在线程池上运行，避免嵌套 tokio runtime）
#[command]
pub fn plugin_install(
    state: State<'_, AppState>,
    source: String,
) -> Result<InstallOutcomeDto, String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let outcome = manager.install(&source).map_err(|e| e.to_string())?;
    Ok(InstallOutcomeDto {
        plugin_id: outcome.plugin_id,
        version: outcome.version,
        install_path: outcome.install_path.display().to_string(),
    })
}

/// 启用插件
#[command]
pub fn plugin_enable(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.enable(&plugin_id).map_err(|e| e.to_string())
}

/// 禁用插件
#[command]
pub fn plugin_disable(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.disable(&plugin_id).map_err(|e| e.to_string())
}

/// 卸载插件
#[command]
pub fn plugin_uninstall(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.uninstall(&plugin_id).map_err(|e| e.to_string())
}

/// 更新插件（同步命令）
#[command]
pub fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<UpdateOutcomeDto, String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let outcome = manager.update(&plugin_id).map_err(|e| e.to_string())?;
    Ok(UpdateOutcomeDto {
        plugin_id: outcome.plugin_id,
        old_version: outcome.old_version,
        new_version: outcome.new_version,
        install_path: outcome.install_path.display().to_string(),
    })
}

// —— DTO 类型（前端兼容） ——

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
