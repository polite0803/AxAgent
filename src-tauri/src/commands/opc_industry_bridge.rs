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
    get_all_industry_learning_configs, get_industry_config, get_industry_learning_config,
    get_workflow_config, load_rl_config,
};
use axagent_harness::types::{ChatTool, ChatToolFunction};
use axagent_tools::ToolError;
use axagent_tools::registry::SkillToolHandler;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
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
        // ── 行业包 manifest（只读） ──
        TauriCommandToolDef {
            name: "opc_get_industry_pack",
            description: "获取行业包的 manifest 基本信息，包括 ID、名称、图标、描述、版本和启用状态",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
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
        // ── 学习配置（只读） ──
        TauriCommandToolDef {
            name: "opc_get_learning_config",
            description: "获取指定行业的学习配置详情，包括反思、进化、自我改进和强化学习功能的启用状态及参数",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                },
                "required": ["industry_id"],
            }),
            is_read_only: true,
        },
        // ── 列出所有学习配置（只读） ──
        TauriCommandToolDef {
            name: "opc_list_learning_configs",
            description: "获取所有 9 个行业的学习配置概览，显示每个行业的反思、进化、自我改进和强化学习的启用状态",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 触发工作流反思（写操作） ──
        TauriCommandToolDef {
            name: "opc_reflect_on_workflow",
            description: "【学习工具】对指定工作流的执行结果进行反思评估。系统会根据行业反思模板分析结果质量，生成改进建议",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流 ID" },
                    "workflow_result": { "type": "object", "description": "工作流执行结果数据，用于反思分析" },
                },
                "required": ["industry_id", "workflow_id", "workflow_result"],
            }),
            is_read_only: false,
        },
        // ── 触发工作流进化（写操作） ──
        TauriCommandToolDef {
            name: "opc_evolve_workflow",
            description: "【学习工具】触发工作流进化。当工作流多次失败或效率低下时，可调用此工具尝试自动优化工作流结构",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流 ID" },
                    "reason": { "type": "string", "description": "进化原因说明，如连续失败、效率低下等" },
                },
                "required": ["industry_id", "workflow_id", "reason"],
            }),
            is_read_only: false,
        },
        // ── 执行自我改进（写操作） ──
        TauriCommandToolDef {
            name: "opc_run_self_improvement",
            description: "【学习工具】执行行业特定的自我改进流程。系统会基于历史执行数据优化参数和策略",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "target": { "type": "string", "description": "改进目标，如 workflow_optimization、prompt_refinement 等" },
                },
                "required": ["industry_id", "target"],
            }),
            is_read_only: false,
        },
        // ── 触发行业学习闭环（自动模式） ──
        TauriCommandToolDef {
            name: "opc_trigger_industry_learning",
            description: "【学习工具】自动触发行业学习闭环。根据行业配置依次执行反思、进化和自我改进。适用于工作流执行完成后触发自动学习",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流 ID" },
                    "workflow_result": { "type": "object", "description": "工作流执行结果数据" },
                },
                "required": ["industry_id", "workflow_id", "workflow_result"],
            }),
            is_read_only: false,
        },
        // ── 获取 RL 经验池统计（只读） ──
        TauriCommandToolDef {
            name: "opc_get_rl_stats",
            description: "【学习工具】获取强化学习经验池的统计数据，包括总经验数、平均奖励、成功率等。可按行业筛选或获取全局统计",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "可选，行业 ID。不传则返回全局统计" },
                },
            }),
            is_read_only: true,
        },
        // ── 记录 RL 经验（写操作） ──
        TauriCommandToolDef {
            name: "opc_record_rl_experience",
            description: "【学习工具】记录一次工作流执行的强化学习经验。系统会基于质量评分和工作流结果自动计算效率、成本、创新、满意度等维度分数",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                    "workflow_id": { "type": "string", "description": "工作流 ID" },
                    "quality_score": { "type": "number", "description": "质量评分 0.0-1.0" },
                    "workflow_result": { "type": "object", "description": "工作流执行结果数据，包含 steps（步骤数组）和 status（执行状态）等" },
                },
                "required": ["industry_id", "workflow_id", "quality_score", "workflow_result"],
            }),
            is_read_only: false,
        },
        // ── 触发 RL 策略优化（写操作） ──
        TauriCommandToolDef {
            name: "opc_trigger_rl_optimization",
            description: "【学习工具】触发强化学习策略优化。基于积累的经验池数据，分析奖励趋势并生成优化建议，调整反思阈值和进化触发条件",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "industry_id": { "type": "string", "description": "行业 ID" },
                },
                "required": ["industry_id"],
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
            serde_json::to_string_pretty(&list).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_industry_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let config = get_industry_config(industry_id)
                .ok_or_else(|| format!("行业不存在: {industry_id}"))?;
            serde_json::to_string_pretty(&config).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_industry_pack" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let config = get_industry_config(industry_id)
                .ok_or_else(|| format!("行业不存在: {industry_id}"))?;
            let manifest = serde_json::json!({
                "id": config.id,
                "name": config.name,
                "icon": config.icon,
                "description": config.description,
                "version": 1,
                "enabled": true,
            });
            let result = serde_json::json!({ "manifest": manifest });
            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_action_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let action_key =
                input["action_key"].as_str().ok_or_else(|| "缺少 action_key 参数".to_string())?;
            let action = get_action_config(industry_id, action_key)
                .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;
            serde_json::to_string_pretty(&action).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_workflow_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let workflow = get_workflow_config(industry_id, workflow_id)
                .ok_or_else(|| format!("工作流不存在: {industry_id}/{workflow_id}"))?;
            serde_json::to_string_pretty(&workflow).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
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
            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
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
            serde_json::to_string_pretty(&actions).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
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
            serde_json::to_string_pretty(&workflows).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
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
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_execute_industry_action" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let action_key =
                input["action_key"].as_str().ok_or_else(|| "缺少 action_key 参数".to_string())?;
            let user_input = input["user_input"].as_str();

            let result = execute_industry_action(industry_id, action_key, user_input)?;
            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_create_industry_workflow" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let custom_name = input["custom_name"].as_str();

            let result = create_industry_workflow(industry_id, workflow_id, custom_name)?;
            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_learning_config" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let app_state = app_handle.state::<crate::AppState>();
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;
            serde_json::to_string_pretty(&config).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_list_learning_configs" => {
            let app_state = app_handle.state::<crate::AppState>();
            let configs = get_all_industry_learning_configs(Some(&app_state.app_data_dir));
            serde_json::to_string_pretty(&configs).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_reflect_on_workflow" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let workflow_result = input["workflow_result"].clone();

            // 从 AppState 获取学习配置目录与学习引擎
            let app_state = app_handle.state::<crate::AppState>();
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

            if !config.reflection_enabled {
                return Err(format!("行业 {} 的反思功能未启用", industry_id));
            }

            let registry = app_state.learning.industry_adapter_registry.lock().await;
            let adapter = registry
                .get(industry_id)
                .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

            let template = adapter.reflection_template().clone();
            drop(registry);

            let request = axagent_orchestrator::ReflectionRequest {
                industry_id: industry_id.to_string(),
                workflow_id: workflow_id.to_string(),
                workflow_result,
                ..Default::default()
            };

            let engine = &app_state.learning.industry_learning_engine;
            let result = engine.reflect_on_workflow(&template, &request).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_evolve_workflow" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let reason = input["reason"].as_str().ok_or_else(|| "缺少 reason 参数".to_string())?;

            // 从 AppState 获取学习配置目录与学习引擎
            let app_state = app_handle.state::<crate::AppState>();
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

            if !config.evolution_enabled {
                return Err(format!("行业 {} 的进化功能未启用", industry_id));
            }

            let registry = app_state.learning.industry_adapter_registry.lock().await;
            let adapter = registry
                .get(industry_id)
                .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

            let constraints = adapter.evolution_constraints().clone();
            drop(registry);

            let request = axagent_orchestrator::EvolutionRequest {
                industry_id: industry_id.to_string(),
                workflow_id: workflow_id.to_string(),
                reason: reason.to_string(),
            };

            let engine = &app_state.learning.industry_learning_engine;
            let result = engine.evolve_workflow(&constraints, &request).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_run_self_improvement" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let target = input["target"].as_str().ok_or_else(|| "缺少 target 参数".to_string())?;

            // 从 AppState 获取学习配置目录与学习引擎
            let app_state = app_handle.state::<crate::AppState>();
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

            if !config.self_improvement_enabled {
                return Err(format!("行业 {} 的自我改进功能未启用", industry_id));
            }

            let request = axagent_orchestrator::SelfImprovementRequest {
                industry_id: industry_id.to_string(),
                target: target.to_string(),
            };

            let engine = &app_state.learning.industry_learning_engine;
            let result = engine.run_self_improvement(&request).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_trigger_industry_learning" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let workflow_result = input["workflow_result"].clone();

            // 从 AppState 获取学习配置目录与学习引擎
            let app_state = app_handle.state::<crate::AppState>();
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

            let registry = app_state.learning.industry_adapter_registry.lock().await;
            let adapter = registry
                .get(industry_id)
                .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

            let template = adapter.reflection_template().clone();
            let constraints = adapter.evolution_constraints().clone();
            let rl_config_from_adapter = adapter.learning_config().reinforcement_learning.clone();
            drop(registry);

            let engine = &app_state.learning.industry_learning_engine;
            let mut last_quality_score: f64 = 0.0;

            let mut reflection_result = serde_json::json!({
                "status": "skipped",
            });
            let mut evolution_result: Option<serde_json::Value> = None;
            let mut self_improvement_result: Option<serde_json::Value> = None;
            let mut rl_result: Option<serde_json::Value> = None;

            // 1. 触发反思
            if config.reflection_enabled {
                let request = axagent_orchestrator::ReflectionRequest {
                    industry_id: industry_id.to_string(),
                    workflow_id: workflow_id.to_string(),
                    workflow_result: workflow_result.clone(),
                    ..Default::default()
                };

                match engine.reflect_on_workflow(&template, &request).await {
                    Ok(result) => {
                        let quality_score = result.quality_score;
                        last_quality_score = quality_score / 100.0; // 转换为 0.0-1.0 范围
                        reflection_result = serde_json::json!({
                            "status": "success",
                            "quality_score": quality_score / 100.0,
                            "message": result.summary,
                        });

                        // 如果质量分数低于 70，自动触发进化
                        if quality_score < 70.0 && config.evolution_enabled {
                            let evolution_request = axagent_orchestrator::EvolutionRequest {
                                industry_id: industry_id.to_string(),
                                workflow_id: workflow_id.to_string(),
                                reason: format!(
                                    "质量分数较低 ({:.2})，触发进化优化",
                                    quality_score
                                ),
                            };

                            match engine.evolve_workflow(&constraints, &evolution_request).await {
                                Ok(evo_result) => {
                                    evolution_result = Some(serde_json::json!({
                                        "status": "success",
                                        "reason": evolution_request.reason,
                                        "message": evo_result.message,
                                    }));
                                },
                                Err(e) => {
                                    evolution_result = Some(serde_json::json!({
                                        "status": "failed",
                                        "reason": evolution_request.reason,
                                        "message": format!("进化失败: {}", e),
                                    }));
                                },
                            }
                        }
                    },
                    Err(e) => {
                        reflection_result = serde_json::json!({
                            "status": "failed",
                            "message": format!("反思失败: {}", e),
                        });
                    },
                }
            }

            // 2. 触发自我改进
            if config.self_improvement_enabled && reflection_result["status"] != "failed" {
                let improvement_request = axagent_orchestrator::SelfImprovementRequest {
                    industry_id: industry_id.to_string(),
                    target: format!("workflow_{}_optimization", workflow_id),
                };

                match engine.run_self_improvement(&improvement_request).await {
                    Ok(result) => {
                        self_improvement_result = Some(serde_json::json!({
                            "status": "success",
                            "target": improvement_request.target,
                            "message": result.message,
                        }));
                    },
                    Err(e) => {
                        self_improvement_result = Some(serde_json::json!({
                            "status": "failed",
                            "target": improvement_request.target,
                            "message": format!("自我改进失败: {}", e),
                        }));
                    },
                }
            }

            // 3. 触发强化学习（如果启用）
            let rl_config = rl_config_from_adapter;
            if rl_config.enabled {
                // 读取 YAML 配置中的完整 RL 参数
                let full_rl_config =
                    load_rl_config(industry_id, Some(&app_state.app_data_dir)).unwrap_or(rl_config);

                match engine
                    .run_reinforcement_learning(
                        industry_id,
                        workflow_id,
                        last_quality_score,
                        &workflow_result,
                        &full_rl_config,
                    )
                    .await
                {
                    Ok(rl_data) => {
                        let has_experience =
                            rl_data.get("experienceRecorded").is_some_and(|v| !v.is_null());
                        let pool_size =
                            rl_data.get("poolSize").and_then(|v| v.as_u64()).unwrap_or(0);
                        let policy_optimized = rl_data
                            .get("policyOptimized")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        rl_result = Some(serde_json::json!({
                            "status": "success",
                            "experience_recorded": has_experience,
                            "pool_size": pool_size,
                            "policy_optimized": policy_optimized,
                            "message": format!("RL 状态: {}", rl_data["status"]),
                        }));
                    },
                    Err(e) => {
                        rl_result = Some(serde_json::json!({
                            "status": "failed",
                            "message": format!("强化学习失败: {}", e),
                        }));
                    },
                }
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let result = serde_json::json!({
                "reflection": reflection_result,
                "evolution": evolution_result,
                "self_improvement": self_improvement_result,
                "reinforcement_learning": rl_result,
                "triggered_at": now,
            });

            serde_json::to_string_pretty(&result).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_get_rl_stats" => {
            let app_state = app_handle.state::<crate::AppState>();
            let engine = &app_state.learning.industry_learning_engine;
            let stats = engine.get_experience_pool_stats().await;
            serde_json::to_string_pretty(&stats).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "opc_record_rl_experience" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;
            let workflow_id =
                input["workflow_id"].as_str().ok_or_else(|| "缺少 workflow_id 参数".to_string())?;
            let raw_quality_score = input["quality_score"].as_f64().unwrap_or(0.5);
            // 自适应转换：如果大于 1，视为 0-100 范围，转换为 0-1
            let quality_score = if raw_quality_score > 1.0 {
                raw_quality_score / 100.0
            } else {
                raw_quality_score
            };
            let workflow_result =
                input.get("workflow_result").cloned().unwrap_or_else(|| serde_json::json!({}));

            let app_state = app_handle.state::<crate::AppState>();
            let engine = &app_state.learning.industry_learning_engine;

            // 获取行业 RL 配置
            let config = get_industry_learning_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

            // 加载 YAML 中的完整 RL 配置
            let rl_config = load_rl_config(industry_id, Some(&app_state.app_data_dir))
                .unwrap_or(config.reinforcement_learning.clone());

            match engine
                .record_experience(
                    industry_id,
                    workflow_id,
                    quality_score,
                    &workflow_result,
                    &rl_config,
                )
                .await
            {
                Ok(experience) => {
                    let result = serde_json::json!({
                        "success": true,
                        "experienceId": experience.id,
                        "totalReward": experience.total_reward,
                        "message": "经验记录成功",
                    });
                    serde_json::to_string_pretty(&result).map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })
                },
                Err(e) => {
                    let result = serde_json::json!({
                        "success": false,
                        "message": e,
                    });
                    serde_json::to_string_pretty(&result).map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })
                },
            }
        },
        "opc_trigger_rl_optimization" => {
            let industry_id =
                input["industry_id"].as_str().ok_or_else(|| "缺少 industry_id 参数".to_string())?;

            let app_state = app_handle.state::<crate::AppState>();
            let engine = &app_state.learning.industry_learning_engine;

            // 加载 YAML 中的完整 RL 配置
            let rl_config = load_rl_config(industry_id, Some(&app_state.app_data_dir))
                .ok_or_else(|| format!("行业 RL 配置不存在或无法加载: {industry_id}"))?;

            match engine.optimize_policy(industry_id, &rl_config).await {
                Ok(update) => serde_json::to_string_pretty(&update).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                }),
                Err(e) => {
                    let result = serde_json::json!({
                        "error": e,
                        "industryId": industry_id,
                    });
                    serde_json::to_string_pretty(&result).map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })
                },
            }
        },
        other => {
            warn!("Unknown OPC industry command: {}", other);
            Err(format!("未知行业命令: {other}"))
        },
    }
}
