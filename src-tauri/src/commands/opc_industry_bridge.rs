// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业命令桥接器 — 将行业专属操作命令注册为 Agent 可调用的 Tool
//!
//! [AxInvest 本地专属] 设计对齐上游 `commands/agent/command_bridge.rs`，
//! 行业命令包含只读操作（获取配置）和写操作（执行操作、创建工作流）。
//! 直接调用 `opc_industry_actions` 模块中的函数。
//!
//! 合并纪律：本文件为 AxInvest 本地新增，上游无此文件 → 永不冲突。

use crate::commands::agent::command_bridge::TauriCommandToolDef;
use crate::commands::opc_industry_actions::{
    create_industry_workflow, execute_industry_action, get_action_config, get_all_industry_configs,
    get_industry_config, get_workflow_config,
};
use axagent_harness::types::{ChatTool, ChatToolFunction};
use axagent_tools::ToolError;
use axagent_tools::registry::SkillToolHandler;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, instrument, warn};

/// 构建可注册到 Agent 的行业命令工具列表
///
/// 命名空间 `opc_` 前缀，与上游 `tauri_` 前缀和股票 `stock_` 前缀区分。
pub fn build_opc_industry_tool_defs() -> Vec<TauriCommandToolDef> {
    vec![
        // ── 行业列表（只读） ──
        TauriCommandToolDef {
            name: "opc_list_industries",
            description: "获取所有 9 个行业的简要信息列表，包括行业 ID、名称、图标、描述、操作数量和工作流数量",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 行业配置（只读） ──
        TauriCommandToolDef {
            name: "opc_get_industry_config",
            description: "获取指定行业的完整配置，包括所有操作和工作流详情",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID，如 ai-research、software-dev、finance-invest、sales-growth、content-media、industry-consulting、accounting、ecommerce、education" },
                },
                "required": ["industry_id"],
            }),
            is_read_only: true,
        },
        // ── 行业操作配置（只读） ──
        TauriCommandToolDef {
            name: "opc_get_action_config",
            description: "获取行业特定操作的执行配置，包括 system prompt、user prompt 模板、图标、标签等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "action_key": { "type": "string", "description": "操作 key，如 ai-paper、sd-code-review" },
                },
                "required": ["industry_id", "action_key"],
            }),
            is_read_only: true,
        },
        // ── 行业工作流配置（只读） ──
        TauriCommandToolDef {
            name: "opc_get_workflow_config",
            description: "获取行业特定工作流的配置信息，包括名称、描述、版本、模板 ID",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流 ID，如 wf-ai-research-1" },
                },
                "required": ["industry_id", "workflow_id"],
            }),
            is_read_only: true,
        },
        // ── 构建行业对话 prompt（只读） ──
        TauriCommandToolDef {
            name: "opc_build_industry_prompt",
            description: "构建带行业上下文的对话 prompt，返回 system prompt 和初始 user prompt。可传入用户自定义输入替换模板变量",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "action_key": { "type": "string", "description": "操作 key" },
                    "user_input": { "type": "string", "description": "可选，用户自定义输入，用于替换 prompt 模板中的 {{input}} 变量" },
                },
                "required": ["industry_id", "action_key"],
            }),
            is_read_only: true,
        },
        // ── 列出行业所有操作（只读） ──
        TauriCommandToolDef {
            name: "opc_list_industry_actions",
            description: "获取指定行业的所有操作列表，返回每个操作的 key、标签、描述、图标等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                },
                "required": ["industry_id"],
            }),
            is_read_only: true,
        },
        // ── 列出行业所有工作流（只读） ──
        TauriCommandToolDef {
            name: "opc_list_industry_workflows",
            description: "获取指定行业的所有工作流列表，返回每个工作流的 ID、名称、描述、版本等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                },
                "required": ["industry_id"],
            }),
            is_read_only: true,
        },
        // ── 行业 UI 渲染（只读 → 写入 UI 事件） ──
        TauriCommandToolDef {
            name: "opc_render_ui",
            description: "将行业分析结果渲染为前端 UI 组件。支持卡片、表格、图表、列表等组件类型。用于在行业对话页面展示分析结果",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema": {
                        "type": "object",
                        "description": "UISchema 定义，必含 version/id/type，可选 props/children。组件类型: Card/Table/Chart/List/Markdown/Form",
                    },
                    "target_id": { "type": "string", "description": "渲染目标容器 ID，如 opc-industry-workspace" },
                    "replace": { "type": "boolean", "description": "可选，是否替换同名组件 (默认 true)" },
                },
                "required": ["schema"],
            }),
            is_read_only: true,
        },
        // ── 执行行业操作（写操作） ──
        TauriCommandToolDef {
            name: "opc_execute_industry_action",
            description: "【核心执行工具】执行行业专属操作。返回包含 System Prompt 和 User Prompt 的完整执行包，Agent 应以此作为当前任务的上下文。支持传入用户自定义输入",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "action_key": { "type": "string", "description": "操作 key，如 ai-paper、sd-code-review" },
                    "user_input": { "type": "string", "description": "可选，用户自定义输入，用于替换 prompt 模板中的 {{input}} 变量" },
                },
                "required": ["industry_id", "action_key"],
            }),
            is_read_only: false,
        },
        // ── 创建行业工作流（写操作） ──
        TauriCommandToolDef {
            name: "opc_create_industry_workflow",
            description: "【核心执行工具】根据行业模板创建一个新的工作流实例。返回实例 ID 和初始配置，Agent 可据此推进工作流步骤",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流模板 ID，如 wf-ai-research-1" },
                    "custom_name": { "type": "string", "description": "可选，自定义工作流实例名称" },
                },
                "required": ["industry_id", "workflow_id"],
            }),
            is_read_only: false,
        },
    ]
}

/// 将行业工具定义转换为 ChatTool 列表
pub fn build_opc_industry_chat_tools() -> Vec<ChatTool> {
    build_opc_industry_tool_defs()
        .into_iter()
        .map(|def| ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: def.name.to_string(),
                description: Some(def.description.to_string()),
                parameters: Some(def.input_schema),
            },
        })
        .collect()
}

/// 为每个行业工具创建 SkillToolHandler
///
/// handler 直接调用 `opc_industry_actions` 中的纯函数，无需数据库连接。
/// `opc_render_ui` 例外 —— 需要 `AppHandle` 发射 UI 渲染事件。
pub fn build_opc_industry_handlers<R: tauri::Runtime>(
    app_handle: AppHandle<R>,
) -> Vec<(String, SkillToolHandler)> {
    let mut handlers = Vec::new();

    for def in build_opc_industry_tool_defs() {
        let handler = create_opc_industry_handler(def.name, app_handle.clone());
        handlers.push((def.name.to_string(), handler));
    }

    handlers
}

/// 创建单个行业命令的 handler
fn create_opc_industry_handler<R: tauri::Runtime>(
    command_name: &str,
    app_handle: AppHandle<R>,
) -> SkillToolHandler {
    let name = command_name.to_string();
    Box::new(move |input: &str| {
        let input_value: Value =
            serde_json::from_str(input).unwrap_or_else(|_| serde_json::json!({}));

        execute_opc_industry_command(&name, &input_value, &app_handle)
    })
}

/// 同步 handler 内部的执行逻辑
///
/// 所有行业命令均为同步操作（纯函数调用），直接在当前线程执行。
fn execute_opc_industry_command<R: tauri::Runtime>(
    command_name: &str,
    input: &Value,
    app_handle: &AppHandle<R>,
) -> Result<String, ToolError> {
    let app = app_handle.clone();
    let name = command_name.to_string();

    // 安全地获取或创建 runtime 执行异步操作
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.block_on(async { dispatch_opc_industry_command(&name, input, &app).await })
        },
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| ToolError::execution_failed(command_name))?;
            runtime.block_on(async { dispatch_opc_industry_command(&name, input, &app).await })
        },
    }
    .map_err(|e| ToolError::execution_failed_for(command_name, e))
}

/// 命令分发 — 根据命令名调用行业配置函数
#[instrument(skip(app_handle), fields(command = %command_name))]
async fn dispatch_opc_industry_command<R: tauri::Runtime>(
    command_name: &str,
    input: &Value,
    app_handle: &AppHandle<R>,
) -> Result<String, String> {
    debug!("Executing OPC industry command: {}", command_name);

    match command_name {
        "opc_list_industries" => {
            let configs = get_all_industry_configs();
            let list: Vec<serde_json::Value> = configs
                .into_iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "name": c.name,
                        "icon": c.icon,
                        "description": c.description,
                        "actionCount": c.actions.len(),
                        "workflowCount": c.workflows.len(),
                    })
                })
                .collect();
            serde_json::to_string_pretty(&list).map_err(|e| e.to_string())
        },
        "opc_get_industry_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let config = get_industry_config(industry_id)
                .ok_or_else(|| format!("行业不存在: {industry_id}"))?;
            serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
        },
        "opc_get_action_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let action_key =
                input["action_key"].as_str().ok_or_else(|| "缺少 action_key 参数".to_string())?;
            let action = get_action_config(industry_id, action_key)
                .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;
            serde_json::to_string_pretty(&action).map_err(|e| e.to_string())
        },
        "opc_get_workflow_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let workflow = get_workflow_config(industry_id, workflow_id)
                .ok_or_else(|| format!("工作流不存在: {industry_id}/{workflow_id}"))?;
            serde_json::to_string_pretty(&workflow).map_err(|e| e.to_string())
        },
        "opc_build_industry_prompt" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let action_key =
                input["action_key"].as_str().ok_or_else(|| "缺少 action_key 参数".to_string())?;
            let user_input = input["user_input"].as_str().map(|s| s.to_string());

            let action = get_action_config(industry_id, action_key)
                .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;

            let user_prompt = match user_input {
                Some(input) if !input.trim().is_empty() => {
                    action.user_prompt_template.replace("{{input}}", &input)
                },
                _ => action.user_prompt_template.clone(),
            };

            let result = serde_json::json!({
                "systemPrompt": action.system_prompt,
                "userPrompt": user_prompt,
                "actionKey": action.key,
                "actionLabel": action.label,
                "industryId": industry_id,
            });
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
        },
        "opc_list_industry_actions" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let config = get_industry_config(industry_id)
                .ok_or_else(|| format!("行业不存在: {industry_id}"))?;
            let actions: Vec<serde_json::Value> = config
                .actions
                .into_iter()
                .map(|a| {
                    serde_json::json!({
                        "key": a.key,
                        "label": a.label,
                        "description": a.description,
                        "actionType": a.action_type,
                        "icon": a.icon,
                        "tags": a.tags,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&actions).map_err(|e| e.to_string())
        },
        "opc_list_industry_workflows" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let config = get_industry_config(industry_id)
                .ok_or_else(|| format!("行业不存在: {industry_id}"))?;
            let workflows: Vec<serde_json::Value> = config
                .workflows
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id,
                        "name": w.name,
                        "description": w.description,
                        "version": w.version,
                        "templateId": w.template_id,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&workflows).map_err(|e| e.to_string())
        },
        "opc_render_ui" => {
            let schema =
                input["schema"].as_object().ok_or_else(|| "缺少 schema 参数".to_string())?;
            let target_id = input["target_id"].as_str().map(|s| s.to_string());
            let replace = input["replace"].as_bool().unwrap_or(true);
            let schema_id = schema.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

            let payload = serde_json::json!({
                "schema": schema,
                "targetId": target_id,
                "replace": replace,
            });

            app_handle.emit("agent-render-ui", &payload).map_err(|e| {
                warn!("[opc-industry-bridge] 派发 UI 渲染事件失败: {}", e);
                format!("派发 UI 渲染事件失败: {e}")
            })?;

            debug!(
                "[opc-industry-bridge] UI rendered: schemaId={}, replace={}",
                schema_id, replace
            );
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "action": "render",
                "schemaId": schema_id,
            }))
            .map_err(|e| e.to_string())
        },
        "opc_execute_industry_action" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let action_key =
                input["action_key"].as_str().ok_or_else(|| "缺少 action_key 参数".to_string())?;
            let user_input = input["user_input"].as_str();

            let result = execute_industry_action(industry_id, action_key, user_input)?;
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
        },
        "opc_create_industry_workflow" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let custom_name = input["custom_name"].as_str();

            let result = create_industry_workflow(industry_id, workflow_id, custom_name)?;
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
        },
        other => {
            warn!("Unknown OPC industry command: {}", other);
            Err(format!("未知行业命令: {other}"))
        },
    }
}
