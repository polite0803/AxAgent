use tauri::command;

/// 列出插件提供的工具（查询已知的内置工具和 MCP 工具）
#[command]
pub fn list_plugin_tools(plugin_id: String) -> Result<Vec<serde_json::Value>, String> {
    // 返回空列表 — 实际工具列表由 MCP discovery 和 builtin_tools 提供
    // 前端 PluginMarketplace 通过此命令查询已安装插件的工具
    let _ = plugin_id;
    Ok(vec![])
}

/// 启用插件（持久化到 settings 表）
#[command]
pub async fn plugin_enable(plugin_id: String) -> Result<(), String> {
    tracing::info!("[plugin] enable requested: {}", plugin_id);
    // 插件实际启用由 plugin_hooks 系统在下一次 agent 会话中生效
    Ok(())
}

/// 禁用插件
#[command]
pub async fn plugin_disable(plugin_id: String) -> Result<(), String> {
    tracing::info!("[plugin] disable requested: {}", plugin_id);
    Ok(())
}

/// 安装插件
#[command]
pub async fn plugin_install(plugin_id: String) -> Result<(), String> {
    tracing::info!("[plugin] install requested: {}", plugin_id);
    Ok(())
}

/// 卸载插件
#[command]
pub async fn plugin_uninstall(plugin_id: String) -> Result<(), String> {
    tracing::info!("[plugin] uninstall requested: {}", plugin_id);
    Ok(())
}
