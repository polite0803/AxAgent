// SPDX-License-Identifier-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::agent::resolve_profile_tool_context;
use axagent_agent_macro::agent_command;
use axagent_harness::ToolDomain;
use axagent_tools::registry::{SCREEN_PERCEPTION_TOOL, is_disclosure_immune};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;

/// 单个本地工具信息（前端 DTO）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

/// 屏幕感知工具是否应被排除在可见工具集之外（**可见性侧**）。
///
/// **必须与 `agent_query` 工具策略块的判定同步**：本函数的语义契约是
/// 「显示的数量 = 实际传给 LLM 的数量」，而 `agent_query` 在
/// `settings.screen_perception_enabled` 关闭时会把 `SCREEN_PERCEPTION_TOOL` 并入
/// blocked 不下发。此处不同步就会出现「UI 多算一个 LLM 根本拿不到的工具」的漂移。
///
/// 只管**可见性**；执行期拦截在 `agent_query` 的 `tool_registry.tools.disable(..)`，
/// 两侧不可互相替代（前者挡「看到」，后者挡「调成」）。
async fn screen_perception_tool_hidden(state: &AppState) -> bool {
    let settings =
        axagent_dao::repo::settings::get_settings(state.harness.db()).await.unwrap_or_default();
    !settings.screen_perception_enabled
}

#[agent_command(domain = tool, safety = Safe, call_mode = StateInput, description = "获取已启用工具数量")]
#[tauri::command]
pub async fn get_tool_count(
    state: State<'_, AppState>,
    agent_profile_id: Option<String>,
) -> Result<u32, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;

    let Some(profile_id) = agent_profile_id else {
        // 无 profile：按自由对话默认域 General 筛选（与 agent_query 一致），
        // 避免显示全局已启用工具数导致用户看到的数量与实际传给 LLM 的不一致。
        let mut domains = HashSet::new();
        domains.insert(ToolDomain::General);
        let chat_tools = registry.get_chat_tools_for_domains(&domains, None);
        let mut names: HashSet<String> =
            chat_tools.iter().map(|t| t.function.name.clone()).collect();
        if screen_perception_tool_hidden(&state).await {
            names.remove(SCREEN_PERCEPTION_TOOL);
        }
        return Ok(names.len() as u32);
    };

    // 解析 profile 上下文（与 agent_query 一致的三源合并）；local_tool 无动态专家覆盖
    let Some(ctx) = resolve_profile_tool_context(&state, &profile_id, None).await else {
        // profile 不存在或查询失败：回退到默认自由对话域 General，
        // 避免显示全局已启用数导致与实际传给 LLM 的工具数不一致。
        let mut domains = HashSet::new();
        domains.insert(ToolDomain::General);
        let chat_tools = registry.get_chat_tools_for_domains(&domains, None);
        let mut names: HashSet<String> =
            chat_tools.iter().map(|t| t.function.name.clone()).collect();
        if screen_perception_tool_hidden(&state).await {
            names.remove(SCREEN_PERCEPTION_TOOL);
        }
        return Ok(names.len() as u32);
    };

    // 三源合并活跃域（与 agent_query 的 ② 分支保持一致：确保 General 始终存在）
    let active_domains: HashSet<ToolDomain> = if ctx.active_domains.is_empty() {
        let mut d = HashSet::new();
        d.insert(ToolDomain::General);
        d
    } else {
        let mut d = ctx.active_domains;
        d.insert(ToolDomain::General);
        d
    };

    // 按域筛选（注意：MCP 工具不受域过滤，与 agent_query 一致）
    let chat_tools = registry.get_chat_tools_for_domains(&active_domains, None);
    let mut names: HashSet<String> = chat_tools.iter().map(|t| t.function.name.clone()).collect();

    // 应用 recommended_tools 白名单：追加缺失的推荐工具。
    // 走 `get_chat_tools_by_names` 而非 `registry.tools.list_all()` —— 后者**不过滤**
    // registry 层的 disable()，会把用户在设置里关掉的工具也计数进来，导致展示的
    // 数量与实际传给 LLM 的不一致；前者复用 registry 统一的启用/禁用过滤。
    let extra = registry.get_chat_tools_by_names(ctx.recommended_tools.iter().map(String::as_str));
    for t in extra {
        names.insert(t.function.name);
    }

    // 应用 disallowed_tools 黑名单。必须放在推荐追加**之后**作最终兜底：
    // 原顺序（先 remove 黑名单、再 insert 推荐）会让「同时出现在 recommended_tools
    // 里的禁用工具」被重新注回，等于绕过 profile 禁用策略。此处与 `agent_query`
    // 保持严格一致的筛选顺序（禁区 12：禁止语义漂移）。
    //
    // 豁免 `DISCLOSURE_TOOLS`：与 `apply_tool_policy` 共用 `is_disclosure_immune` 判定。
    // 两侧必须一致——若这里算掉而 LLM 列表里仍在，UI 计数就会与实际工具数对不上。
    for name in &ctx.disallowed_tools {
        if !is_disclosure_immune(name) {
            names.remove(name);
        }
    }

    // 屏幕感知门控（可见性侧）：与 `agent_query` 策略块同步。放在**最后一道**，
    // 确保 recommended 追加之后不会把该工具又算回来。
    if screen_perception_tool_hidden(&state).await {
        names.remove(SCREEN_PERCEPTION_TOOL);
    }

    Ok(names.len() as u32)
}

// ── 列出所有工具（含单工具启用状态） ──

#[agent_command(domain = tool, safety = Safe, call_mode = StateOnly, description = "列出本地工具")]
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

#[agent_command(domain = tool, safety = Caution, call_mode = StateInput, description = "切换工具分组启用")]
#[tauri::command]
pub async fn toggle_local_tool_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<LocalToolGroupInfo, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;
    registry.toggle_group(state.harness.db(), &group_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let disabled = registry.groups.disabled_tools.clone();
    let groups = registry.get_tool_groups();
    let group = groups
        .into_iter()
        .find(|g| g.group_id == group_id)
        .ok_or_else(|| format!("Group '{}' not found", group_id))?;
    Ok(to_local_group(group, &disabled))
}

// ── 切换单个工具启禁 ──

#[agent_command(domain = tool, safety = Caution, call_mode = StateInput, description = "切换单个工具启用")]
#[tauri::command]
pub async fn toggle_single_tool(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(state.harness.db()).await;
    registry.toggle_tool(state.harness.db(), &tool_name).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let disabled = registry.groups.disabled_tools.clone();
    Ok(registry.get_tool_groups().into_iter().map(|g| to_local_group(g, &disabled)).collect())
}
