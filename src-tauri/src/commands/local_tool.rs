// SPDX-License-Identifier-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::agent::resolve_profile_tool_context;
use axagent_harness::ToolDomain;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

// ── 获取已启用工具总数 ──
//
// 当传入 `agent_profile_id` 时，返回按该 profile 的工具域筛选后的工具数；
// 否则返回全局已启用工具数（兼容旧调用方）。
// 筛选语义与 `agent_query` 一致（共享 `resolve_profile_tool_context`）。

#[tauri::command]
pub async fn get_tool_count(
    state: State<'_, AppState>,
    agent_profile_id: Option<String>,
) -> Result<u32, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;

    let Some(profile_id) = agent_profile_id else {
        // 无 profile：按自由对话默认域 Core + General 筛选（与 agent_query 一致），
        // 避免显示全局已启用工具数导致用户看到的数量与实际传给 LLM 的不一致。
        let mut domains = HashSet::new();
        domains.insert(ToolDomain::Core);
        domains.insert(ToolDomain::General);
        let chat_tools = registry.get_chat_tools_for_domains(&domains, None);
        let names: HashSet<String> = chat_tools.iter().map(|t| t.function.name.clone()).collect();
        return Ok(names.len() as u32);
    };

    // 解析 profile 上下文（与 agent_query 一致的三源合并）
    let Some(ctx) = resolve_profile_tool_context(&state, &profile_id).await else {
        // profile 不存在或查询失败：回退到默认自由对话域 Core + General，
        // 避免显示全局已启用数导致与实际传给 LLM 的工具数不一致。
        let mut domains = HashSet::new();
        domains.insert(ToolDomain::Core);
        domains.insert(ToolDomain::General);
        let chat_tools = registry.get_chat_tools_for_domains(&domains, None);
        let names: HashSet<String> = chat_tools.iter().map(|t| t.function.name.clone()).collect();
        return Ok(names.len() as u32);
    };

    // 三源合并活跃域（与 agent_query 的 ② 分支保持一致：确保 Core 始终存在）
    let active_domains: HashSet<ToolDomain> = if ctx.active_domains.is_empty() {
        let mut d = HashSet::new();
        d.insert(ToolDomain::Core);
        d.insert(ToolDomain::General);
        d
    } else {
        let mut d = ctx.active_domains;
        d.insert(ToolDomain::Core);
        d
    };

    // 按域筛选（注意：MCP 工具不受域过滤，与 agent_query 一致）
    let chat_tools = registry.get_chat_tools_for_domains(&active_domains, None);
    let mut names: HashSet<String> = chat_tools.iter().map(|t| t.function.name.clone()).collect();

    // 应用 disallowed_tools 黑名单
    for name in &ctx.disallowed_tools {
        names.remove(name);
    }

    // 应用 recommended_tools 白名单：从全量工具列表中追加缺失的推荐工具
    let all_info = registry.tools.list_all();
    for info in &all_info {
        if ctx.recommended_tools.contains(&info.name) {
            names.insert(info.name.clone());
        }
    }

    Ok(names.len() as u32)
}

// ── 列出所有工具（含单工具启用状态） ──

#[tauri::command]
pub async fn list_local_tools(
    state: State<'_, AppState>,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;
    let disabled = registry.groups.disabled_tools.clone();
    Ok(registry.get_tool_groups().into_iter().map(|g| to_local_group(g, &disabled)).collect())
}

// ── 切换工具分类启禁 ──

#[tauri::command]
pub async fn toggle_local_tool_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<LocalToolGroupInfo, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;
    registry.toggle_group(state.harness.db(), &group_id).await.map_err(|e| e.to_string())?;

    let disabled = registry.groups.disabled_tools.clone();
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
    registry.load_enabled_state(state.harness.db()).await;
    registry.toggle_tool(state.harness.db(), &tool_name).await.map_err(|e| e.to_string())?;

    let disabled = registry.groups.disabled_tools.clone();
    Ok(registry.get_tool_groups().into_iter().map(|g| to_local_group(g, &disabled)).collect())
}
