use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// 单个本地工具信息（前端 DTO）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolInfo {
    #[serde(rename = "name")]
    pub name: String,
    pub description: String,
    #[serde(rename = "category")]
    pub category: String,
    #[serde(rename = "isDestructive")]
    pub is_destructive: bool,
    #[serde(rename = "isReadOnly")]
    pub is_read_only: bool,
    #[serde(rename = "isConcurrencySafe")]
    pub is_concurrency_safe: bool,
    /// 此单独工具是否被启用（仅当分类已启用时有效）
    pub enabled: bool,
}

/// 本地工具组信息（前端 DTO）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolGroupInfo {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub description: String,
    pub enabled: bool,
    pub tools: Vec<LocalToolInfo>,
}

/// 分组描述映射
fn group_description(gid: &str) -> &str {
    match gid {
        "builtin-file-read" => "只读文件操作：读取、搜索、列出目录和文件信息",
        "builtin-file-write" => "写入文件操作：创建、编辑、删除、移动文件",
        "builtin-shell" => "Shell 命令执行和代码 REPL",
        "builtin-network" => "网络请求：网页抓取、搜索、浏览器自动化",
        "builtin-system-tools" => "系统工具：配置、缓存、终端、通知",
        "builtin-agent" => "Agent 管理：子 Agent、Skill、任务列表、计划模式",
        "builtin-vcs" => "版本控制：Git 状态、差异、提交、分支、审查",
        "builtin-automation" => "自动化：定时任务、后台任务、工作流执行",
        "builtin-communication" => "通信：消息发送、文件传输、团队管理",
        "builtin-ai-media" => "AI 媒体：图片生成、图表、推理思考",
        "builtin-integration" => "外部集成：Dify 知识库、Obsidian 笔记",
        "builtin-storage" => "存储管理：文件上传、下载、删除",
        "builtin-knowledge" => "知识库：知识实体、流程、文档管理",
        "builtin-browser" => "浏览器自动化：导航、截图、点击、填写表单",
        "builtin-desktop" => "桌面控制：截图、鼠标点击、键盘输入",
        _ => "其他工具",
    }
}

fn to_local_group(
    g: axagent_tools::registry::ToolGroupInfo,
    disabled_tools: &std::collections::HashSet<String>,
) -> LocalToolGroupInfo {
    let gid = g.group_id.clone();
    LocalToolGroupInfo {
        group_id: gid.clone(),
        group_name: g.group_name,
        description: group_description(&gid).to_string(),
        enabled: g.enabled,
        tools: g
            .tools
            .into_iter()
            .map(|t| {
                let tool_name = t.name.clone();
                LocalToolInfo {
                    name: t.name,
                    description: t.description,
                    category: t.category.as_str().to_string(),
                    is_destructive: t.is_destructive,
                    is_read_only: t.is_read_only,
                    is_concurrency_safe: t.is_concurrency_safe,
                    enabled: !disabled_tools.contains(&tool_name),
                }
            })
            .collect(),
    }
}

// ── 列出所有工具（含单工具启用状态） ──

#[tauri::command]
pub async fn list_local_tools(
    state: State<'_, AppState>,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;
    let disabled = registry.disabled_tools.clone();
    Ok(registry
        .get_tool_groups()
        .into_iter()
        .map(|g| to_local_group(g, &disabled))
        .collect())
}

// ── 切换工具分类启禁 ──

#[tauri::command]
pub async fn toggle_local_tool_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<LocalToolGroupInfo, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;
    registry
        .toggle_group(&state.sea_db, &group_id)
        .await
        .map_err(|e| e.to_string())?;

    let disabled = registry.disabled_tools.clone();
    let groups = registry.get_tool_groups();
    let group = groups
        .into_iter()
        .find(|g| g.group_id == group_id)
        .ok_or_else(|| format!("Group '{}' not found", group_id))?;
    Ok(to_local_group(group, &disabled))
}

// ── 切换单个工具启禁 ──

#[tauri::command]
pub async fn toggle_single_tool(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;
    registry
        .toggle_tool(&state.sea_db, &tool_name)
        .await
        .map_err(|e| e.to_string())?;

    let disabled = registry.disabled_tools.clone();
    Ok(registry
        .get_tool_groups()
        .into_iter()
        .map(|g| to_local_group(g, &disabled))
        .collect())
}
